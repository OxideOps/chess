//! One game between two players, as the server sees it.
//!
//! `Room` is pure state: it knows nothing about sockets or time sources.
//! Callers pass `now` in and get back the messages to send out, which keeps
//! the rules testable without a runtime.

use std::time::{Duration, Instant};

use chess_core::{
    Color, Game, GameError,
    protocol::{
        Away, Category, ClientMessage, Clocks, GameEnd, GameOverReason, GameResult, Players,
        ServerMessage,
    },
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

/// Everything needed to rebuild a [`Room`] later, e.g. from the database.
/// Durations are milliseconds; `clock_running_for_ms` is how long the side
/// to move's clock had been running when the snapshot was taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub initial_ms: u64,
    pub increment_ms: u64,
    /// UCI moves from the initial position.
    pub moves: Vec<String>,
    pub white_ms: u64,
    pub black_ms: u64,
    pub clock_running_for_ms: Option<u64>,
    pub draw_offer: Option<Color>,
    pub ended: Option<GameEnd>,
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
    /// A player who left while their opponent is here, and when they lose
    /// the game for it. Not persisted: after a restart, presence is rebuilt
    /// from whoever reconnects.
    away: Option<(Color, Instant)>,
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
            away: None,
        }
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn time_control(&self) -> TimeControl {
        self.time_control
    }

    pub fn category(&self) -> Category {
        Category::of(
            self.time_control.initial.as_millis() as u64,
            self.time_control.increment.as_millis() as u64,
        )
    }

    /// Capture the room as data, as of `now`.
    pub fn snapshot(&self, now: Instant) -> Snapshot {
        Snapshot {
            initial_ms: self.time_control.initial.as_millis() as u64,
            increment_ms: self.time_control.increment.as_millis() as u64,
            moves: self
                .game
                .moves()
                .iter()
                .map(|m| m.uci.to_string())
                .collect(),
            white_ms: self.remaining[0].as_millis() as u64,
            black_ms: self.remaining[1].as_millis() as u64,
            clock_running_for_ms: self
                .clock_since
                .map(|since| now.saturating_duration_since(since).as_millis() as u64),
            draw_offer: self.draw_offer,
            ended: self.ended,
        }
    }

    /// Rebuild a room from a snapshot taken `age` ago (the clock, if it was
    /// running, has kept running meanwhile). Fails if the moves don't replay.
    pub fn restore(snapshot: &Snapshot, age: Duration, now: Instant) -> Result<Room, GameError> {
        let mut game = Game::new();
        for uci in &snapshot.moves {
            game.play_uci(uci)?;
        }
        let clock_since = snapshot.clock_running_for_ms.and_then(|running| {
            let since = now.checked_sub(Duration::from_millis(running) + age);
            // Only a running clock makes sense; a finished game has none.
            since.filter(|_| snapshot.ended.is_none())
        });
        Ok(Room {
            game,
            time_control: TimeControl {
                initial: Duration::from_millis(snapshot.initial_ms),
                increment: Duration::from_millis(snapshot.increment_ms),
            },
            remaining: [
                Duration::from_millis(snapshot.white_ms),
                Duration::from_millis(snapshot.black_ms),
            ],
            clock_since,
            draw_offer: snapshot.draw_offer,
            ended: snapshot.ended,
            away: None,
        })
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
            // Seats are the registry's business; it fills them in.
            players: Players::default(),
            away: self.away_at(now),
            // Rated or not is the registry's business too.
            rated: false,
            category: self.category(),
            rating_diffs: None,
        }
    }

    /// Tell the room who is connected. A countdown runs for a seated player
    /// with no connection while the other player is connected, both seats
    /// are taken, and the game is on; `grace` is how long it lasts. It keeps
    /// its deadline across calls, stops when they return (or when nobody is
    /// left to claim the game), and restarts from `grace` next time.
    /// Returns the broadcast when the countdown starts or stops.
    pub fn set_presence(
        &mut self,
        connected: [bool; 2],
        seated: [bool; 2],
        grace: Duration,
        now: Instant,
    ) -> Option<Outgoing> {
        let gone = if self.is_over() || !(seated[0] && seated[1]) {
            None
        } else {
            match connected {
                [false, true] => Some(Color::White),
                [true, false] => Some(Color::Black),
                _ => None,
            }
        };
        let next = match (self.away, gone) {
            (Some((side, deadline)), Some(g)) if side == g => Some((side, deadline)),
            (_, Some(g)) => Some((g, now + grace)),
            (_, None) => None,
        };
        if next == self.away {
            return None;
        }
        self.away = next;
        Some(Outgoing::Broadcast(ServerMessage::AwayChanged {
            away: self.away_at(now),
        }))
    }

    /// When the away player's countdown runs out, if one is running.
    pub fn away_deadline(&self) -> Option<Instant> {
        self.away.map(|(_, deadline)| deadline)
    }

    fn away_at(&self, now: Instant) -> Option<Away> {
        self.away.map(|(side, deadline)| Away {
            side,
            ms: deadline.saturating_duration_since(now).as_millis() as u64,
        })
    }

    /// End the game if the away player's countdown has run out: aborted if
    /// both sides hadn't moved yet, otherwise won by the player who stayed.
    /// A no-op before the deadline (or if they came back).
    pub fn check_abandonment(&mut self, now: Instant) -> Option<Outgoing> {
        let (side, deadline) = self.away?;
        if now < deadline || self.is_over() {
            return None;
        }
        let result = if self.game.moves().len() < 2 {
            GameResult::Aborted
        } else {
            GameResult::from_winner(Some(!side))
        };
        Some(self.end(GameEnd {
            result,
            reason: GameOverReason::Abandoned,
        }))
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
        self.away = None;
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
    fn snapshot_round_trips_and_keeps_the_clock_running() {
        let (mut r, t0) = room();
        r.handle(Some(Color::White), mv("e2e4"), t0);
        r.handle(Some(Color::Black), mv("e7e5"), t0);
        r.handle(Some(Color::White), ClientMessage::OfferDraw, t0);
        // White's clock has run 10s when we snapshot.
        let snap = r.snapshot(t0 + secs(10));
        assert_eq!(snap.moves, ["e2e4", "e7e5"]);
        assert_eq!((snap.white_ms, snap.black_ms), (60_000, 60_000));
        assert_eq!(snap.clock_running_for_ms, Some(10_000));
        assert_eq!(snap.draw_offer, Some(Color::White));

        // Restored 5s later: the clock ran the whole time.
        let t1 = t0 + secs(15);
        let restored = Room::restore(&snap, secs(5), t1).unwrap();
        assert_eq!(restored.game().movetext(), "1. e4 e5");
        assert_eq!(restored.clocks(t1).white_ms, 45_000);
        assert_eq!(restored.deadline(), Some(t0 + secs(60)));
        assert_eq!(
            restored.snapshot(t1),
            Snapshot {
                clock_running_for_ms: Some(15_000),
                ..snap
            }
        );

        // A finished game restores without a running clock.
        let mut r = restored;
        r.handle(Some(Color::Black), ClientMessage::AcceptDraw, t1);
        let snap = r.snapshot(t1);
        assert!(snap.ended.is_some());
        assert_eq!(snap.clock_running_for_ms, None);
        let again = Room::restore(&snap, secs(100), t1 + secs(100)).unwrap();
        assert!(again.is_over());
        assert_eq!(again.deadline(), None);

        // Garbage moves don't restore.
        let bad = Snapshot {
            moves: vec!["e2e5".into()],
            ..snap
        };
        assert!(Room::restore(&bad, secs(0), t1).is_err());
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

    const BOTH: [bool; 2] = [true, true];

    fn away(out: Option<Outgoing>) -> Option<Option<Away>> {
        match out {
            Some(Broadcast(ServerMessage::AwayChanged { away })) => Some(away),
            None => None,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_countdown_runs_only_for_a_seated_player_whose_opponent_is_here() {
        let (mut r, t0) = room();
        let grace = secs(60);
        // Nobody to blame while the Black seat is open.
        assert_eq!(
            r.set_presence([false, false], [true, false], grace, t0),
            None
        );
        assert_eq!(
            r.set_presence([true, false], [true, false], grace, t0),
            None
        );
        // Both seated, Black not connected: Black's countdown starts.
        let started = away(r.set_presence([true, false], BOTH, grace, t0)).unwrap();
        assert_eq!(
            started,
            Some(Away {
                side: Color::Black,
                ms: 60_000
            })
        );
        assert_eq!(r.away_deadline(), Some(t0 + grace));
        // Telling the room the same thing again changes nothing, and keeps the deadline.
        assert_eq!(
            r.set_presence([true, false], BOTH, grace, t0 + secs(10)),
            None
        );
        assert_eq!(r.away_deadline(), Some(t0 + grace));
        // Sync reports the time left.
        match r.sync(Some(Color::White), t0 + secs(15)) {
            ServerMessage::Sync { away, .. } => assert_eq!(away.unwrap().ms, 45_000),
            other => panic!("{other:?}"),
        }
        // Black comes back: stopped.
        assert_eq!(
            away(r.set_presence(BOTH, BOTH, grace, t0 + secs(20))),
            Some(None)
        );
        assert_eq!(r.check_abandonment(t0 + secs(61)), None);
        // White leaves: a fresh countdown for White.
        let white = away(r.set_presence([false, true], BOTH, grace, t0 + secs(30))).unwrap();
        assert_eq!(white.unwrap().side, Color::White);
        assert_eq!(r.away_deadline(), Some(t0 + secs(90)));
        // Black leaves too: nobody is here to claim the game, so no countdown.
        assert_eq!(
            away(r.set_presence([false, false], BOTH, grace, t0 + secs(40))),
            Some(None)
        );
        assert_eq!(r.away_deadline(), None);
    }

    #[test]
    fn leaving_before_both_have_moved_aborts_after_that_it_loses() {
        let (mut r, t0) = room();
        let grace = secs(60);
        r.handle(Some(Color::White), mv("e2e4"), t0);
        r.set_presence([true, false], BOTH, grace, t0);
        assert_eq!(r.check_abandonment(t0 + secs(59)), None);
        assert_eq!(
            r.check_abandonment(t0 + secs(60)),
            Some(Broadcast(ServerMessage::GameOver {
                end: GameEnd {
                    result: GameResult::Aborted,
                    reason: GameOverReason::Abandoned
                }
            }))
        );
        assert_eq!(r.away_deadline(), None);
        // Over is over: no countdown for a finished game.
        assert_eq!(
            r.set_presence([false, true], BOTH, grace, t0 + secs(70)),
            None
        );

        let (mut r, t0) = room();
        r.handle(Some(Color::White), mv("e2e4"), t0);
        r.handle(Some(Color::Black), mv("e7e5"), t0);
        r.set_presence([false, true], BOTH, grace, t0 + secs(1));
        assert_eq!(
            r.check_abandonment(t0 + secs(61)),
            Some(Broadcast(ServerMessage::GameOver {
                end: GameEnd {
                    result: GameResult::BlackWins,
                    reason: GameOverReason::Abandoned
                }
            }))
        );
        // The clock stopped with the game.
        assert_eq!(r.deadline(), None);
    }

    #[test]
    fn a_game_that_ends_otherwise_stops_the_countdown() {
        let (mut r, t0) = room();
        r.set_presence([true, false], BOTH, secs(60), t0);
        r.handle(Some(Color::White), ClientMessage::Resign, t0 + secs(5));
        assert_eq!(r.away_deadline(), None);
        assert_eq!(r.check_abandonment(t0 + secs(120)), None);
        assert_eq!(
            r.ended().map(|e| e.reason),
            Some(GameOverReason::Resignation)
        );
    }
}
