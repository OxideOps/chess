//! One game between two players, as the server sees it.
//!
//! `Room` is pure state: it knows nothing about sockets or time sources.
//! Callers pass `now` in and get back the messages to send out, which keeps
//! the rules testable without a runtime.

use std::time::{Duration, Instant};

use chess_core::{
    Color, Game,
    protocol::{ClientMessage, Clocks, GameEnd, GameOverReason, GameResult, ServerMessage},
};

/// Initial time and increment per move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeControl {
    pub initial: Duration,
    pub increment: Duration,
}

impl Default for TimeControl {
    fn default() -> Self {
        Self {
            initial: Duration::from_secs(5 * 60),
            increment: Duration::ZERO,
        }
    }
}

/// A message produced by the room, and who should get it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outgoing {
    /// Only to the connection that sent the request.
    Reply(ServerMessage),
    /// To everyone watching the game.
    Broadcast(ServerMessage),
}

#[derive(Debug)]
pub struct Room {
    game: Game,
    time_control: TimeControl,
    /// Remaining time as of `clock_since` (or as of the last move, if the
    /// clocks aren't running).
    remaining: [Duration; 2],
    /// When the side to move's clock started running. Clocks only run once
    /// both sides have moved, and stop when the game ends.
    clock_since: Option<Instant>,
    draw_offer: Option<Color>,
    ended: Option<GameEnd>,
}

fn idx(color: Color) -> usize {
    if color.is_white() { 0 } else { 1 }
}

impl Room {
    pub fn new(time_control: TimeControl) -> Self {
        Self {
            game: Game::new(),
            time_control,
            remaining: [time_control.initial; 2],
            clock_since: None,
            draw_offer: None,
            ended: None,
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn ended(&self) -> Option<GameEnd> {
        self.ended
    }

    pub fn is_over(&self) -> bool {
        self.ended.is_some()
    }

    /// Remaining time for both sides at `now`.
    pub fn clocks(&self, now: Instant) -> Clocks {
        let mut remaining = self.remaining;
        if let Some(since) = self.clock_since {
            let i = idx(self.game.turn());
            remaining[i] = remaining[i].saturating_sub(now.saturating_duration_since(since));
        }
        Clocks {
            white_ms: remaining[0].as_millis() as u64,
            black_ms: remaining[1].as_millis() as u64,
        }
    }

    /// When the side to move runs out of time, if the clock is running.
    pub fn deadline(&self) -> Option<Instant> {
        self.clock_since
            .map(|since| since + self.remaining[idx(self.game.turn())])
    }

    /// The message a newly connected client needs.
    pub fn sync(&self, your_color: Option<Color>, now: Instant) -> ServerMessage {
        ServerMessage::Sync {
            start_fen: self.game.start_fen(),
            moves: self.game.moves().iter().map(|m| m.uci).collect(),
            clocks: self.clocks(now),
            your_color,
            ended: self.ended,
            draw_offer: self.draw_offer,
        }
    }

    /// Flag the side to move if its clock has run out. Call this whenever
    /// the deadline passes; it is a no-op otherwise.
    pub fn check_timeout(&mut self, now: Instant) -> Option<Outgoing> {
        let deadline = self.deadline()?;
        if now < deadline || self.is_over() {
            return None;
        }
        let loser = self.game.turn();
        self.remaining[idx(loser)] = Duration::ZERO;
        Some(self.end(GameEnd {
            result: GameResult::from_winner(Some(!loser)),
            reason: GameOverReason::Timeout,
        }))
    }

    /// Handle a message from a player (`Some(color)`) or spectator (`None`).
    pub fn handle(
        &mut self,
        who: Option<Color>,
        msg: ClientMessage,
        now: Instant,
    ) -> Vec<Outgoing> {
        if let ClientMessage::Ping = msg {
            return vec![Outgoing::Reply(ServerMessage::Pong)];
        }
        let Some(who) = who else {
            return vec![reject("spectators can't do that")];
        };
        if let Some(timeout) = self.check_timeout(now) {
            // The clock ran out before this message arrived; that wins.
            return vec![timeout, reject("the game is over")];
        }
        if self.is_over() {
            return vec![reject("the game is over")];
        }
        match msg {
            ClientMessage::Move { uci } => self.play(who, &uci.to_string(), now),
            ClientMessage::Resign => vec![self.end(GameEnd {
                result: GameResult::from_winner(Some(!who)),
                reason: GameOverReason::Resignation,
            })],
            ClientMessage::OfferDraw => match self.draw_offer {
                Some(by) if by == who => vec![reject("you already offered a draw")],
                // Both sides offering is an agreement.
                Some(_) => vec![self.end(GameEnd {
                    result: GameResult::Draw,
                    reason: GameOverReason::Agreement,
                })],
                None => {
                    self.draw_offer = Some(who);
                    vec![Outgoing::Broadcast(ServerMessage::DrawOffered { by: who })]
                }
            },
            ClientMessage::AcceptDraw => match self.draw_offer {
                Some(by) if by != who => vec![self.end(GameEnd {
                    result: GameResult::Draw,
                    reason: GameOverReason::Agreement,
                })],
                _ => vec![reject("there is no draw offer to accept")],
            },
            ClientMessage::DeclineDraw => match self.draw_offer {
                Some(by) if by != who => {
                    self.draw_offer = None;
                    vec![Outgoing::Broadcast(ServerMessage::DrawDeclined)]
                }
                _ => vec![reject("there is no draw offer to decline")],
            },
            ClientMessage::Ping => unreachable!("handled above"),
        }
    }

    fn play(&mut self, who: Color, uci: &str, now: Instant) -> Vec<Outgoing> {
        if who != self.game.turn() {
            return vec![reject("it is not your turn")];
        }
        let played = match self.game.play_uci(uci) {
            Ok(m) => m.uci,
            Err(e) => return vec![reject(&e.to_string())],
        };
        // Stop the mover's clock (it was running against them) and give the increment.
        if let Some(since) = self.clock_since.take() {
            let i = idx(who);
            self.remaining[i] = self.remaining[i]
                .saturating_sub(now.saturating_duration_since(since))
                + self.time_control.increment;
        }
        // A move withdraws any draw offer on the table.
        self.draw_offer = None;

        let ply = self.game.ply_count() as u32;
        let mut out = Vec::new();
        if let Some(end) = GameEnd::from_status(self.game.status()) {
            out.push(Outgoing::Broadcast(ServerMessage::MovePlayed {
                ply,
                uci: played,
                clocks: self.clocks(now),
            }));
            out.push(self.end(end));
            return out;
        }
        // Clocks run once both sides have moved.
        if ply >= 2 {
            self.clock_since = Some(now);
        }
        out.push(Outgoing::Broadcast(ServerMessage::MovePlayed {
            ply,
            uci: played,
            clocks: self.clocks(now),
        }));
        out
    }

    fn end(&mut self, end: GameEnd) -> Outgoing {
        self.ended = Some(end);
        self.clock_since = None;
        self.draw_offer = None;
        Outgoing::Broadcast(ServerMessage::GameOver { end })
    }
}

fn reject(message: &str) -> Outgoing {
    Outgoing::Reply(ServerMessage::Rejected {
        message: message.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use Outgoing::{Broadcast, Reply};

    fn mv(uci: &str) -> ClientMessage {
        ClientMessage::Move {
            uci: uci.parse().unwrap(),
        }
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    fn room() -> (Room, Instant) {
        (
            Room::new(TimeControl {
                initial: secs(60),
                increment: secs(2),
            }),
            Instant::now(),
        )
    }

    #[test]
    fn moves_are_validated_and_broadcast() {
        let (mut r, t0) = room();
        let out = r.handle(Some(Color::White), mv("e2e4"), t0);
        assert!(matches!(
            out.as_slice(),
            [Broadcast(ServerMessage::MovePlayed { ply: 1, .. })]
        ));
        assert!(matches!(
            r.handle(Some(Color::White), mv("d2d4"), t0).as_slice(),
            [Reply(ServerMessage::Rejected { message })] if message == "it is not your turn"
        ));
        assert!(matches!(
            r.handle(Some(Color::Black), mv("e7e4"), t0).as_slice(),
            [Reply(ServerMessage::Rejected { message })] if message == "illegal move"
        ));
        assert!(matches!(
            r.handle(None, mv("e7e5"), t0).as_slice(),
            [Reply(ServerMessage::Rejected { .. })]
        ));
        assert_eq!(
            r.handle(None, ClientMessage::Ping, t0),
            vec![Reply(ServerMessage::Pong)]
        );
        assert_eq!(r.game().movetext(), "1. e4");
    }

    #[test]
    fn clocks_start_after_both_have_moved_and_get_the_increment() {
        let (mut r, t0) = room();
        r.handle(Some(Color::White), mv("e2e4"), t0);
        // White thought for 10s before the clocks started: no charge.
        r.handle(Some(Color::Black), mv("e7e5"), t0 + secs(10));
        assert_eq!(r.clocks(t0 + secs(10)).white_ms, 60_000);
        assert_eq!(r.clocks(t0 + secs(10)).black_ms, 60_000);
        assert_eq!(r.deadline(), Some(t0 + secs(70)));

        // Now White's clock runs: 5s later it shows 55s, then the move adds 2s.
        assert_eq!(r.clocks(t0 + secs(15)).white_ms, 55_000);
        r.handle(Some(Color::White), mv("g1f3"), t0 + secs(15));
        let c = r.clocks(t0 + secs(15));
        assert_eq!((c.white_ms, c.black_ms), (57_000, 60_000));
        assert_eq!(r.deadline(), Some(t0 + secs(75)));
    }

    #[test]
    fn running_out_of_time_loses() {
        let (mut r, t0) = room();
        r.handle(Some(Color::White), mv("e2e4"), t0);
        r.handle(Some(Color::Black), mv("e7e5"), t0);
        assert_eq!(r.check_timeout(t0 + secs(59)), None);
        let out = r.check_timeout(t0 + secs(61)).unwrap();
        assert_eq!(
            out,
            Broadcast(ServerMessage::GameOver {
                end: GameEnd {
                    result: GameResult::BlackWins,
                    reason: GameOverReason::Timeout
                }
            })
        );
        assert_eq!(r.clocks(t0 + secs(100)).white_ms, 0);
        assert!(r.is_over());
        // A move that arrives after the flag is rejected, and the flag is reported first.
        let out = r.handle(Some(Color::White), mv("g1f3"), t0 + secs(62));
        assert!(matches!(
            out.as_slice(),
            [Reply(ServerMessage::Rejected { .. })]
        ));
    }

    #[test]
    fn late_move_is_beaten_by_the_flag() {
        let (mut r, t0) = room();
        r.handle(Some(Color::White), mv("e2e4"), t0);
        r.handle(Some(Color::Black), mv("e7e5"), t0);
        let out = r.handle(Some(Color::White), mv("g1f3"), t0 + secs(61));
        assert!(matches!(
            out.as_slice(),
            [
                Broadcast(ServerMessage::GameOver { .. }),
                Reply(ServerMessage::Rejected { .. })
            ]
        ));
        assert_eq!(r.game().ply_count(), 2);
    }

    #[test]
    fn checkmate_ends_the_game_with_the_move() {
        let (mut r, t0) = room();
        let mut side = Color::White;
        for m in ["e2e4", "e7e5", "f1c4", "b8c6", "d1h5", "g8f6"] {
            r.handle(Some(side), mv(m), t0);
            side = !side;
        }
        let out = r.handle(Some(Color::White), mv("h5f7"), t0);
        assert!(matches!(
            out.as_slice(),
            [
                Broadcast(ServerMessage::MovePlayed { ply: 7, .. }),
                Broadcast(ServerMessage::GameOver {
                    end: GameEnd {
                        result: GameResult::WhiteWins,
                        reason: GameOverReason::Checkmate
                    }
                })
            ]
        ));
        assert_eq!(r.deadline(), None);
        assert!(matches!(
            r.handle(Some(Color::Black), ClientMessage::Resign, t0)
                .as_slice(),
            [Reply(ServerMessage::Rejected { .. })]
        ));
    }

    #[test]
    fn resign_and_draw_negotiation() {
        let (mut r, t0) = room();
        assert!(matches!(
            r.handle(Some(Color::Black), ClientMessage::AcceptDraw, t0)
                .as_slice(),
            [Reply(ServerMessage::Rejected { .. })]
        ));
        assert_eq!(
            r.handle(Some(Color::White), ClientMessage::OfferDraw, t0),
            vec![Broadcast(ServerMessage::DrawOffered { by: Color::White })]
        );
        assert!(matches!(
            r.handle(Some(Color::White), ClientMessage::OfferDraw, t0)
                .as_slice(),
            [Reply(ServerMessage::Rejected { .. })]
        ));
        assert_eq!(
            r.handle(Some(Color::Black), ClientMessage::DeclineDraw, t0),
            vec![Broadcast(ServerMessage::DrawDeclined)]
        );
        // A move withdraws an offer.
        r.handle(Some(Color::White), ClientMessage::OfferDraw, t0);
        r.handle(Some(Color::White), mv("e2e4"), t0);
        assert!(matches!(
            r.sync(None, t0),
            ServerMessage::Sync {
                draw_offer: None,
                ..
            }
        ));
        // Offer, accept.
        r.handle(Some(Color::Black), ClientMessage::OfferDraw, t0);
        assert_eq!(
            r.handle(Some(Color::White), ClientMessage::AcceptDraw, t0),
            vec![Broadcast(ServerMessage::GameOver {
                end: GameEnd {
                    result: GameResult::Draw,
                    reason: GameOverReason::Agreement
                }
            })]
        );

        let (mut r, t0) = room();
        assert_eq!(
            r.handle(Some(Color::White), ClientMessage::Resign, t0),
            vec![Broadcast(ServerMessage::GameOver {
                end: GameEnd {
                    result: GameResult::BlackWins,
                    reason: GameOverReason::Resignation
                }
            })]
        );
        assert!(matches!(
            r.sync(Some(Color::Black), t0),
            ServerMessage::Sync {
                ended: Some(_),
                your_color: Some(Color::Black),
                ..
            }
        ));
    }
}
