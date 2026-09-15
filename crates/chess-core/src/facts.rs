//! Plain facts about a position, read off the board: where the pieces stand,
//! what attacks what, what is pinned, and where each king can go; and, for a
//! line of moves, what each move does.
//!
//! The coach hands these to a language model so that it reads the board
//! instead of picturing it from a FEN, which it gets wrong ("e7 is blocked
//! by its own pawn" when that pawn is on e5). Everything here is exact, and
//! [`BoardFacts::describe`] and [`MoveFacts::describe`] put it in words.

use shakmaty::{
    Bitboard, CastlingSide, Chess, Color, Piece, Position, Role, Square, attacks, san::SanPlus,
    uci::UciMove,
};

/// A piece and where it stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub piece: Piece,
    pub square: Square,
}

impl Placed {
    /// "the white queen on h5".
    pub fn name(self) -> String {
        format!(
            "the {} {} on {}",
            color_name(self.piece.color),
            role_name(self.piece.role),
            self.square
        )
    }
}

/// A piece (not a king) that the other side attacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub piece: Placed,
    pub attackers: Vec<Placed>,
    pub defenders: Vec<Placed>,
}

impl Target {
    /// Attacked by something worth less than it.
    pub fn attacked_by_cheaper(&self) -> bool {
        let value = value(self.piece.piece.role);
        self.attackers.iter().any(|a| value_of(a) < value)
    }
}

/// A piece that can't leave the line between its king and an enemy piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    pub pinned: Placed,
    pub by: Placed,
    pub king: Placed,
}

/// A square next to a king, from the king's point of view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Around {
    /// The king can go there (it's empty, or holds an undefended enemy piece).
    Free,
    /// One of its own pieces is there.
    Own(Piece),
    /// The other side covers it.
    Covered(Vec<Placed>),
}

/// Where a king can go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KingRoom {
    pub king: Placed,
    pub around: Vec<(Square, Around)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardFacts {
    pub turn: Color,
    /// Every piece, White's first, kings to pawns.
    pub pieces: Vec<Placed>,
    /// Material in the usual points (pawn 1, knight and bishop 3, rook 5, queen 9).
    pub material: [u32; 2],
    /// Pieces giving check to the side to move.
    pub checkers: Vec<Placed>,
    /// Pieces the other side attacks, White's first.
    pub targets: Vec<Target>,
    pub pins: Vec<Pin>,
    /// White's king first.
    pub kings: Vec<KingRoom>,
}

const ROLES: [Role; 6] = [
    Role::King,
    Role::Queen,
    Role::Rook,
    Role::Bishop,
    Role::Knight,
    Role::Pawn,
];
const COLORS: [Color; 2] = [Color::White, Color::Black];

/// The facts about `pos`.
pub fn board_facts(pos: &Chess) -> BoardFacts {
    let board = pos.board();
    let occupied = board.occupied();
    let placed = |sq: Square| board.piece_at(sq).map(|piece| Placed { piece, square: sq });
    let all = |bb: Bitboard| -> Vec<Placed> { sorted(bb.into_iter().filter_map(placed).collect()) };

    let pieces = all(occupied);
    let material = COLORS.map(|color| {
        board
            .by_color(color)
            .into_iter()
            .filter_map(|sq| board.piece_at(sq))
            .map(|p| value(p.role))
            .sum()
    });
    let checkers = all(pos.checkers());

    let mut targets = Vec::new();
    for p in &pieces {
        if p.piece.role == Role::King {
            continue;
        }
        let color = p.piece.color;
        let attackers = all(board.attacks_to(p.square, !color, occupied));
        if attackers.is_empty() {
            continue;
        }
        let defenders = all(board.attacks_to(p.square, color, occupied));
        targets.push(Target {
            piece: *p,
            attackers,
            defenders,
        });
    }

    let mut pins = Vec::new();
    let mut kings = Vec::new();
    for color in COLORS {
        let Some(king_sq) = board.king_of(color) else {
            continue;
        };
        let king = placed(king_sq).expect("a king on its square");
        // Enemy sliders that would see the king through at most one piece.
        let enemy = board.by_color(!color);
        let diagonal = (board.bishops() | board.queens()) & enemy;
        let straight = (board.rooks() | board.queens()) & enemy;
        let snipers = (attacks::bishop_attacks(king_sq, Bitboard::EMPTY) & diagonal)
            | (attacks::rook_attacks(king_sq, Bitboard::EMPTY) & straight);
        for sniper in snipers {
            let between = attacks::between(sniper, king_sq) & occupied;
            if let Some(sq) = between.single_square()
                && board.by_color(color).contains(sq)
            {
                pins.push(Pin {
                    pinned: placed(sq).expect("a piece"),
                    by: placed(sniper).expect("a piece"),
                    king,
                });
            }
        }
        // The king doesn't shield squares behind it from a slider.
        let without_king = occupied.without(king_sq);
        let around = attacks::king_attacks(king_sq)
            .into_iter()
            .map(|sq| {
                let status = match board.piece_at(sq) {
                    Some(piece) if piece.color == color => Around::Own(piece),
                    _ => {
                        let cover = all(board.attacks_to(sq, !color, without_king));
                        if cover.is_empty() {
                            Around::Free
                        } else {
                            Around::Covered(cover)
                        }
                    }
                };
                (sq, status)
            })
            .collect();
        kings.push(KingRoom { king, around });
    }

    BoardFacts {
        turn: pos.turn(),
        pieces,
        material,
        checkers,
        targets,
        pins,
        kings,
    }
}

impl BoardFacts {
    /// The facts in words, one per line.
    pub fn describe(&self) -> Vec<String> {
        let mut out = self.describe_pieces();
        out.extend(self.describe_tension());
        out
    }

    /// Where the pieces stand, and the material.
    pub fn describe_pieces(&self) -> Vec<String> {
        let mut out = Vec::new();
        for color in COLORS {
            let mut groups = Vec::new();
            for role in ROLES {
                let squares: Vec<String> = self
                    .pieces
                    .iter()
                    .filter(|p| p.piece == Piece { color, role })
                    .map(|p| p.square.to_string())
                    .collect();
                match squares.len() {
                    0 => {}
                    1 => groups.push(format!("{} {}", role_name(role), squares[0])),
                    _ => groups.push(format!("{}s {}", role_name(role), squares.join(", "))),
                }
            }
            out.push(format!(
                "{} pieces: {}.",
                capitalized(color_name(color)),
                groups.join("; ")
            ));
        }
        let [white, black] = self.material;
        let balance = match white.cmp(&black) {
            std::cmp::Ordering::Equal => "level".to_string(),
            std::cmp::Ordering::Greater => format!("White is {} up", white - black),
            std::cmp::Ordering::Less => format!("Black is {} up", black - white),
        };
        out.push(format!(
            "Material: White {white}, Black {black} ({balance})."
        ));
        out
    }

    /// Checks, pins, attacked pieces, and where each king can go.
    pub fn describe_tension(&self) -> Vec<String> {
        let mut out = Vec::new();
        if !self.checkers.is_empty() {
            out.push(format!(
                "The {} king is in check from {}.",
                color_name(self.turn),
                names(&self.checkers)
            ));
        }
        for pin in &self.pins {
            out.push(format!(
                "{} is pinned to its king by {}.",
                capitalized(&pin.pinned.name()),
                pin.by.name()
            ));
        }
        for t in &self.targets {
            let defended = if t.defenders.is_empty() {
                "not defended".to_string()
            } else {
                format!("defended by {}", names(&t.defenders))
            };
            let cheaper = if t.attacked_by_cheaper() {
                " It is attacked by a piece worth less than it."
            } else {
                ""
            };
            out.push(format!(
                "{} is attacked by {}, and {defended}.{cheaper}",
                capitalized(&t.piece.name()),
                names(&t.attackers)
            ));
        }
        for room in &self.kings {
            out.push(room.describe());
        }
        out
    }
}

impl KingRoom {
    /// "The black king on e8 can go to e7; its own pieces block d7 (pawn), d8 (queen)."
    pub fn describe(&self) -> String {
        let squares = |want: fn(&Around) -> bool| -> Vec<String> {
            self.around
                .iter()
                .filter(|(_, a)| want(a))
                .map(|(sq, _)| sq.to_string())
                .collect()
        };
        let free = squares(|a| matches!(a, Around::Free));
        // Named, or a model guesses: "its own pawns on f7 and f8" (f8 was a bishop).
        let own: Vec<String> = self
            .around
            .iter()
            .filter_map(|(sq, a)| match a {
                Around::Own(piece) => Some(format!("{sq} ({})", role_name(piece.role))),
                _ => None,
            })
            .collect();
        let covered: Vec<String> = self
            .around
            .iter()
            .filter_map(|(sq, a)| match a {
                Around::Covered(by) => Some(format!("{sq} by {}", names(by))),
                _ => None,
            })
            .collect();
        let mut parts = vec![if free.is_empty() {
            "has no free square next to it".to_string()
        } else {
            format!("can go to {}", free.join(", "))
        }];
        if !own.is_empty() {
            parts.push(format!("its own pieces block {}", own.join(", ")));
        }
        if !covered.is_empty() {
            parts.push(format!("covered: {}", covered.join("; ")));
        }
        format!("{} {}.", capitalized(&self.king.name()), parts.join("; "))
    }
}

/// What one move of a line does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveFacts {
    /// Numbered SAN: "4. Qxf7#" or "4... Kxe5".
    pub label: String,
    /// The piece that moves, where it stood.
    pub piece: Placed,
    pub to: Square,
    pub capture: Option<Placed>,
    pub promotion: Option<Role>,
    pub castle: Option<CastlingSide>,
    pub check: bool,
    pub checkmate: bool,
    pub stalemate: bool,
    /// Enemy pieces (not the king) that the moved piece attacks from its new square.
    pub attacks: Vec<Placed>,
}

/// What each move of `pv` does, stopping at the first move that isn't legal.
pub fn line_facts(pos: &Chess, pv: &[UciMove]) -> Vec<MoveFacts> {
    let mut pos = pos.clone();
    let mut out = Vec::with_capacity(pv.len());
    for uci in pv {
        let Ok(mv) = uci.to_move(&pos) else { break };
        let color = pos.turn();
        let number = pos.fullmoves();
        let from = mv.from().expect("standard chess has no drops");
        let piece = Placed {
            piece: Piece {
                color,
                role: mv.role(),
            },
            square: from,
        };
        let capture = mv.capture().map(|role| Placed {
            piece: Piece {
                color: !color,
                role,
            },
            square: if mv.is_en_passant() {
                Square::from_coords(mv.to().file(), from.rank())
            } else {
                mv.to()
            },
        });
        let castle = mv.castling_side();
        let to = match castle {
            Some(side) => side.king_to(color),
            None => mv.to(),
        };
        let san = SanPlus::from_move_and_play_unchecked(&mut pos, mv);
        let label = match color {
            Color::White => format!("{number}. {san}"),
            Color::Black => format!("{number}... {san}"),
        };
        let board = pos.board();
        let attacks = if castle.is_some() {
            Vec::new()
        } else {
            let enemy = board.by_color(!color) & !board.kings();
            sorted(
                (board.attacks_from(to) & enemy)
                    .into_iter()
                    .filter_map(|sq| board.piece_at(sq).map(|piece| Placed { piece, square: sq }))
                    .collect(),
            )
        };
        out.push(MoveFacts {
            label,
            piece,
            to,
            capture,
            promotion: mv.promotion(),
            castle,
            check: pos.is_check(),
            checkmate: pos.is_checkmate(),
            stalemate: pos.is_stalemate(),
            attacks,
        });
    }
    out
}

impl MoveFacts {
    /// "4. Qxf7#: the white queen on h5 takes the black pawn on f7, checkmate."
    pub fn describe(&self) -> String {
        let mut what = match self.castle {
            Some(CastlingSide::KingSide) => format!("{} castles kingside", self.piece.name()),
            Some(CastlingSide::QueenSide) => format!("{} castles queenside", self.piece.name()),
            None => match self.capture {
                Some(c) if c.square != self.to => {
                    format!(
                        "{} takes {} en passant, landing on {}",
                        self.piece.name(),
                        c.name(),
                        self.to
                    )
                }
                Some(c) => format!("{} takes {}", self.piece.name(), c.name()),
                None => format!("{} moves to {}", self.piece.name(), self.to),
            },
        };
        if let Some(role) = self.promotion {
            what.push_str(&format!(" and promotes to a {}", role_name(role)));
        }
        if self.checkmate {
            what.push_str(", checkmate");
        } else if self.stalemate {
            what.push_str(", stalemate (a draw)");
        } else {
            if self.check {
                what.push_str(", with check");
            }
            if !self.attacks.is_empty() {
                what.push_str(&format!(", and now attacks {}", names(&self.attacks)));
            }
        }
        format!("{}: {what}.", self.label)
    }
}

fn value(role: Role) -> u32 {
    match role {
        Role::Pawn => 1,
        Role::Knight | Role::Bishop => 3,
        Role::Rook => 5,
        Role::Queen => 9,
        Role::King => 0,
    }
}

/// For comparing attackers: a king is worth the most.
fn value_of(p: &Placed) -> u32 {
    match p.piece.role {
        Role::King => u32::MAX,
        role => value(role),
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Pawn => "pawn",
        Role::Knight => "knight",
        Role::Bishop => "bishop",
        Role::Rook => "rook",
        Role::Queen => "queen",
        Role::King => "king",
    }
}

fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "white",
        Color::Black => "black",
    }
}

fn capitalized(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// "the white queen on h5 and the white bishop on c4".
fn names(pieces: &[Placed]) -> String {
    let names: Vec<String> = pieces.iter().map(|p| p.name()).collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// White's pieces first, then kings to pawns, then by square.
fn sorted(mut pieces: Vec<Placed>) -> Vec<Placed> {
    let rank = |role: Role| ROLES.iter().position(|r| *r == role).unwrap_or(0);
    pieces.sort_by_key(|p| (p.piece.color == Color::Black, rank(p.piece.role), p.square));
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;
    use shakmaty::{CastlingMode, fen::Fen};

    fn pos(fen: &str) -> Chess {
        fen.parse::<Fen>()
            .unwrap()
            .into_position(CastlingMode::Standard)
            .unwrap()
    }

    fn uci(moves: &[&str]) -> Vec<UciMove> {
        moves.iter().map(|m| m.parse().unwrap()).collect()
    }

    /// Scholar's Mate, White to play Qxf7#.
    const SCHOLAR: &str = "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4";

    #[test]
    fn lists_the_pieces_and_material() {
        let facts = board_facts(&pos(SCHOLAR));
        let text = facts.describe();
        assert_eq!(
            text[0],
            "White pieces: king e1; queen h5; rooks a1, h1; bishops c1, c4; knights b1, g1; \
             pawns a2, b2, c2, d2, f2, g2, h2, e4."
        );
        assert_eq!(facts.material, [39, 39]);
        assert_eq!(text[2], "Material: White 39, Black 39 (level).");
    }

    #[test]
    fn says_what_is_attacked_and_who_defends_it() {
        let text = board_facts(&pos(SCHOLAR)).describe();
        assert!(
            text.contains(
                &"The black pawn on f7 is attacked by the white queen on h5 and the white bishop \
                  on c4, and defended by the black king on e8."
                    .to_string()
            ),
            "{text:#?}"
        );
        // The queen hits e5 too; the knight on c6 defends it.
        assert!(
            text.contains(
                &"The black pawn on e5 is attacked by the white queen on h5, and defended by the \
                  black knight on c6."
                    .to_string()
            ),
            "{text:#?}"
        );
    }

    #[test]
    fn knows_where_the_kings_can_go() {
        // Before the mate, e7 is free (the e-pawn went to e5).
        let facts = board_facts(&pos(SCHOLAR));
        let black = &facts.kings[1];
        assert_eq!(
            black.describe(),
            "The black king on e8 can go to e7; its own pieces block d7 (pawn), f7 (pawn), \
             d8 (queen), f8 (bishop)."
        );
        // After Qxf7#, e7 is covered and the queen can't be taken: it's defended.
        let mut after = pos(SCHOLAR);
        let mv = uci(&["h5f7"])[0].to_move(&after).unwrap();
        after.play_unchecked(mv);
        let facts = board_facts(&after);
        assert_eq!(
            facts.kings[1].describe(),
            "The black king on e8 has no free square next to it; its own pieces block d7 (pawn), \
             d8 (queen), f8 (bishop); covered: e7 by the white queen on f7; f7 by the white \
             bishop on c4."
        );
        assert_eq!(facts.checkers.len(), 1);
        assert!(
            facts
                .describe()
                .contains(&"The black king is in check from the white queen on f7.".to_string())
        );
    }

    #[test]
    fn a_king_cannot_hide_behind_itself() {
        // Rook checks along the rank: the square behind the king is covered too.
        let facts = board_facts(&pos("8/8/8/R3k3/8/8/8/4K3 b - - 0 1"));
        let room = facts.kings[1].describe();
        assert!(room.contains("f5 by the white rook on a5"), "{room}");
    }

    #[test]
    fn finds_pins_and_loose_pieces() {
        let facts = board_facts(&pos("4k3/8/8/8/1b6/2N5/8/4K3 w - - 0 1"));
        let text = facts.describe();
        assert!(
            text.contains(
                &"The white knight on c3 is pinned to its king by the black bishop on b4."
                    .to_string()
            ),
            "{text:#?}"
        );
        // A blocked line is no pin.
        let blocked = board_facts(&pos("4k3/8/8/8/1b6/2N5/3P4/4K3 w - - 0 1"));
        assert!(blocked.pins.is_empty());

        // The drill blunder Qe5+: the queen next to the king, undefended.
        let text = board_facts(&pos("8/8/8/3kQ3/8/8/8/4K3 b - - 1 1")).describe();
        assert!(
            text.contains(
                &"The white queen on e5 is attacked by the black king on d5, and not defended."
                    .to_string()
            ),
            "{text:#?}"
        );
        // Attacked by a cheaper piece, even though defended.
        let text = board_facts(&pos("4k3/8/8/3p4/4N3/5P2/8/4K3 w - - 0 1")).describe();
        assert!(
            text.contains(
                &"The white knight on e4 is attacked by the black pawn on d5, and defended by \
                  the white pawn on f3. It is attacked by a piece worth less than it."
                    .to_string()
            ),
            "{text:#?}"
        );
    }

    #[test]
    fn describes_each_move_of_a_line() {
        let line = line_facts(&pos(SCHOLAR), &uci(&["h5f7"]));
        assert_eq!(
            line[0].describe(),
            "4. Qxf7#: the white queen on h5 takes the black pawn on f7, checkmate."
        );

        // New attacks, check, and numbering for Black.
        let start = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let line = line_facts(&start, &uci(&["e2e4", "e7e5", "f1c4", "b8c6", "d1h5"]));
        let text: Vec<String> = line.iter().map(MoveFacts::describe).collect();
        assert_eq!(text[0], "1. e4: the white pawn on e2 moves to e4.");
        assert_eq!(text[1], "1... e5: the black pawn on e7 moves to e5.");
        assert_eq!(
            text[2],
            "2. Bc4: the white bishop on f1 moves to c4, and now attacks the black pawn on f7."
        );
        assert_eq!(
            text[4],
            "3. Qh5: the white queen on d1 moves to h5, and now attacks the black pawn on e5, \
             the black pawn on f7 and the black pawn on h7."
        );

        // An illegal tail is cut off.
        assert_eq!(line_facts(&start, &uci(&["e2e4", "e2e4"])).len(), 1);
    }

    #[test]
    fn describes_castling_en_passant_promotion_and_stalemate() {
        let castle = line_facts(
            &pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1"),
            &uci(&["e1g1"]),
        );
        assert_eq!(
            castle[0].describe(),
            "1. O-O: the white king on e1 castles kingside."
        );
        assert_eq!(castle[0].to, Square::G1);

        let ep = line_facts(&pos("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2"), &uci(&["e5d6"]));
        assert_eq!(
            ep[0].describe(),
            "2. exd6: the white pawn on e5 takes the black pawn on d5 en passant, landing on d6."
        );

        let promote = line_facts(&pos("8/4P3/8/8/8/8/k7/4K3 w - - 0 1"), &uci(&["e7e8q"]));
        assert_eq!(
            promote[0].describe(),
            "1. e8=Q: the white pawn on e7 moves to e8 and promotes to a queen."
        );

        let stalemate = line_facts(&pos("k7/8/1Q6/8/8/8/8/4K3 w - - 0 1"), &uci(&["b6c7"]));
        assert_eq!(
            stalemate[0].describe(),
            "1. Qc7: the white queen on b6 moves to c7, stalemate (a draw)."
        );
    }
}
