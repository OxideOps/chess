//! Plain facts about a position, read off the board: where the pieces stand,
//! what attacks what, what is pinned, and where each king can go; and, for a
//! line of moves, what each move does.
//!
//! The coach hands these to a language model so that it reads the board
//! instead of picturing it from a FEN, which it gets wrong ("e7 is blocked
//! by its own pawn" when that pawn is on e5). Everything here is exact, and
//! [`BoardFacts::describe`] and [`MoveFacts::describe`] put it in words.

use shakmaty::{
    Bitboard, CastlingSide, Chess, Color, File, Piece, Position, Rank, Role, Square, attacks,
    san::SanPlus, uci::UciMove,
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

/// The pawns that stand out, and the files they leave open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Structure {
    /// No enemy pawn ahead of it on its file or either neighbour.
    pub passed: Vec<Placed>,
    /// No friendly pawn on either neighbouring file.
    pub isolated: Vec<Placed>,
    /// Another friendly pawn on the same file.
    pub doubled: Vec<Placed>,
    /// No pawns of either colour.
    pub open_files: Vec<File>,
    /// No pawns of that colour, but some of the other's; White first.
    pub half_open: [Vec<File>; 2],
}

/// How far a side has got its pieces out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Development {
    pub color: Color,
    /// Knights and bishops still on the square they started on.
    pub at_home: Vec<Placed>,
    pub king: Square,
    /// The king can still castle one way or the other.
    pub can_castle: bool,
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
    pub structure: Structure,
    /// White first.
    pub development: Vec<Development>,
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
    let material = material_of(board);
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

    let mut structure = Structure::default();
    for file in File::ALL {
        let on_file = |color: Color| board.pawns() & board.by_color(color) & Bitboard::from(file);
        let (white, black) = (on_file(Color::White), on_file(Color::Black));
        match (white.is_empty(), black.is_empty()) {
            (true, true) => structure.open_files.push(file),
            (true, false) => structure.half_open[0].push(file),
            (false, true) => structure.half_open[1].push(file),
            (false, false) => {}
        }
    }
    for p in pieces.iter().filter(|p| p.piece.role == Role::Pawn) {
        let color = p.piece.color;
        let pawns = |c: Color| board.pawns() & board.by_color(c);
        let files = |file: File| Bitboard::from(file);
        let neighbours = [p.square.file().offset(-1), p.square.file().offset(1)]
            .into_iter()
            .flatten()
            .fold(Bitboard::EMPTY, |bb, f| bb | files(f));
        if (pawns(color) & neighbours).is_empty() {
            structure.isolated.push(*p);
        }
        if (pawns(color) & files(p.square.file())).count() > 1 {
            structure.doubled.push(*p);
        }
        let ahead = ranks_ahead(p.square.rank(), color);
        if ((neighbours | files(p.square.file())) & ahead & pawns(!color)).is_empty() {
            structure.passed.push(*p);
        }
    }

    let development = COLORS
        .iter()
        .filter_map(|&color| {
            let back = color.fold_wb(Rank::First, Rank::Eighth);
            let at_home = sorted(
                pieces
                    .iter()
                    .filter(|p| p.piece.color == color)
                    .filter(|p| matches!(p.piece.role, Role::Knight | Role::Bishop))
                    .filter(|p| p.square.rank() == back && home_square(p.piece.role, p.square))
                    .copied()
                    .collect(),
            );
            Some(Development {
                color,
                at_home,
                king: board.king_of(color)?,
                can_castle: pos.castles().has_color(color),
            })
        })
        .collect();

    BoardFacts {
        turn: pos.turn(),
        pieces,
        material,
        checkers,
        targets,
        pins,
        kings,
        structure,
        development,
    }
}

/// The squares on the ranks in front of `rank`, from `color`'s point of view.
fn ranks_ahead(rank: Rank, color: Color) -> Bitboard {
    Rank::ALL
        .iter()
        .filter(|r| match color {
            Color::White => **r > rank,
            Color::Black => **r < rank,
        })
        .fold(Bitboard::EMPTY, |bb, r| bb | Bitboard::from(*r))
}

/// Whether a knight or bishop stands where its kind starts.
fn home_square(role: Role, square: Square) -> bool {
    match role {
        Role::Knight => matches!(square.file(), File::B | File::G),
        Role::Bishop => matches!(square.file(), File::C | File::F),
        _ => false,
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
        out.push(format!("Material: {}.", balance(white, black)));
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
        if let Some(line) = self.structure.describe() {
            out.push(line);
        }
        for side in &self.development {
            out.push(side.describe());
        }
        out
    }
}

impl Structure {
    /// "Pawns: passed e5 (White); isolated d4 (White); open file d; …", or
    /// nothing when the pawns hold no such feature.
    pub fn describe(&self) -> Option<String> {
        let squares = |pawns: &[Placed]| -> String {
            pawns
                .iter()
                .map(|p| format!("{} ({})", p.square, color_name(p.piece.color)))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let files = |files: &[File]| -> String {
            files
                .iter()
                .map(|f| f.char().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let mut parts = Vec::new();
        for (name, pawns) in [
            ("passed", &self.passed),
            ("isolated", &self.isolated),
            ("doubled", &self.doubled),
        ] {
            if !pawns.is_empty() {
                parts.push(format!("{name}: {}", squares(pawns)));
            }
        }
        if !self.open_files.is_empty() {
            parts.push(format!("open files: {}", files(&self.open_files)));
        }
        for (color, half) in COLORS.iter().zip(&self.half_open) {
            if !half.is_empty() {
                parts.push(format!(
                    "half-open for {}: {}",
                    capitalized(color_name(*color)),
                    files(half)
                ));
            }
        }
        (!parts.is_empty()).then(|| format!("Pawns — {}.", parts.join("; ")))
    }
}

impl Development {
    /// "White: king on g1, can still castle; knights and bishops still at
    /// home: b1, c1."
    pub fn describe(&self) -> String {
        let home = if self.at_home.is_empty() {
            "every knight and bishop has moved".to_string()
        } else {
            format!(
                "still at home: {}",
                self.at_home
                    .iter()
                    .map(|p| format!("{} {}", role_name(p.piece.role), p.square))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        format!(
            "{}: king on {}, {}; {home}.",
            capitalized(color_name(self.color)),
            self.king,
            if self.can_castle {
                "can still castle"
            } else {
                "can no longer castle"
            }
        )
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
    /// The material after the move, when it changed it (a capture or a
    /// promotion): White's points, then Black's.
    pub material: Option<[u32; 2]>,
    /// What the other side can play next, when there is little choice (a
    /// check, or a position with at most three legal moves): the count, and
    /// the moves in SAN.
    pub replies: Option<(usize, Vec<String>)>,
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
        // After mate or stalemate the count is beside the point.
        let over = pos.is_checkmate() || pos.is_stalemate();
        let changed = (mv.is_capture() || mv.promotion().is_some()) && !over;
        let material = changed.then(|| material_of(pos.board()));
        // After a check, or when there is next to no choice, what can follow.
        let legal = pos.legal_moves();
        let replies = (!legal.is_empty() && (pos.is_check() || legal.len() <= 3)).then(|| {
            let moves = legal
                .iter()
                .take(4)
                .map(|m| SanPlus::from_move(pos.clone(), *m).to_string())
                .collect();
            (legal.len(), moves)
        });
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
            material,
            replies,
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
        let mut out = format!("{}: {what}.", self.label);
        if let Some([white, black]) = self.material {
            out.push_str(&format!(" Material now {}.", balance(white, black)));
        }
        if let Some((count, moves)) = &self.replies {
            out.push_str(&match (count, moves.as_slice()) {
                (1, [only]) => format!(" The only legal reply is {only}."),
                (n, moves) if *n <= 4 => format!(" The legal replies are {}.", moves.join(", ")),
                (n, _) => format!(" There are {n} legal replies."),
            });
        }
        out
    }
}

/// The points each side holds: White's, then Black's.
fn material_of(board: &shakmaty::Board) -> [u32; 2] {
    COLORS.map(|color| {
        board
            .by_color(color)
            .into_iter()
            .filter_map(|sq| board.piece_at(sq))
            .map(|p| value(p.role))
            .sum()
    })
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

/// "White 39, Black 36 (White is 3 up)".
fn balance(white: u32, black: u32) -> String {
    let who = match white.cmp(&black) {
        std::cmp::Ordering::Equal => "level".to_string(),
        std::cmp::Ordering::Greater => format!("White is {} up", white - black),
        std::cmp::Ordering::Less => format!("Black is {} up", black - white),
    };
    format!("White {white}, Black {black} ({who})")
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
    fn reads_the_pawn_structure_and_how_far_the_pieces_are_out() {
        // A middlegame with the c-file open and every minor piece out.
        let facts = board_facts(&pos(
            "r2q1rk1/pp3ppp/2n1pn2/3p4/3P4/2NBPN2/PP3PPP/R2Q1RK1 w - - 0 1",
        ));
        let text = facts.describe();
        let line = |start: &str| text.iter().find(|l| l.starts_with(start)).cloned();
        assert_eq!(line("Pawns — "), Some("Pawns — open files: c.".to_string()));
        assert_eq!(
            line("White: king"),
            Some(
                "White: king on g1, can no longer castle; every knight and bishop has moved."
                    .to_string()
            )
        );

        // Passed, isolated and doubled pawns, and the half-open files.
        let facts = board_facts(&pos("4k3/5p2/8/3P4/8/2P5/P1P5/4K3 w - - 0 1"));
        let squares = |pawns: &[Placed]| {
            pawns
                .iter()
                .map(|p| p.square.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let s = &facts.structure;
        // Black has one pawn, so most of White's are passed.
        assert_eq!(squares(&s.passed), "a2 c2 c3 d5 f7");
        assert_eq!(squares(&s.isolated), "a2 f7");
        assert_eq!(squares(&s.doubled), "c2 c3");
        assert_eq!(s.open_files.len(), 4); // b, e, g, h
        let pawns = s.describe().unwrap();
        assert!(pawns.contains("half-open for White: f"), "{pawns}");
        assert!(pawns.contains("half-open for Black: a, c, d"), "{pawns}");

        // Nothing to say about the pawns at the start; every piece is home.
        let start = board_facts(&pos(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ));
        assert_eq!(start.structure.describe(), None);
        assert_eq!(
            start.development[0].describe(),
            "White: king on e1, can still castle; still at home: bishop c1, bishop f1, \
             knight b1, knight g1."
        );
    }

    #[test]
    fn counts_the_material_a_move_wins_and_what_can_answer_it() {
        // A capture says what the material became, and a check what can reply.
        let line = line_facts(
            &pos("r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4"),
            &uci(&["h5e5"]),
        );
        let said = line[0].describe();
        assert!(
            said.contains("Material now White 39, Black 38 (White is 1 up)."),
            "{said}"
        );
        assert!(said.contains("legal replies"), "{said}");

        // With one way out, the reply is named.
        let line = line_facts(&pos("7k/6p1/8/8/8/8/8/R5K1 w - - 0 1"), &uci(&["a1a8"]));
        let said = line[0].describe();
        assert!(said.contains("The only legal reply is Kh7."), "{said}");

        // A quiet move says neither; mate says neither (the game is over).
        let quiet = line_facts(&pos("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1"), &uci(&["e2e4"]));
        assert_eq!(
            quiet[0].describe(),
            "1. e4: the white pawn on e2 moves to e4."
        );
        let mate = line_facts(&pos("7k/6pp/8/8/8/8/8/R5K1 w - - 0 1"), &uci(&["a1a8"]));
        assert_eq!(
            mate[0].describe(),
            "1. Ra8#: the white rook on a1 moves to a8, checkmate."
        );
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
            "2. exd6: the white pawn on e5 takes the black pawn on d5 en passant, landing on d6. \
             Material now White 1, Black 0 (White is 1 up)."
        );

        let promote = line_facts(&pos("8/4P3/8/8/8/8/k7/4K3 w - - 0 1"), &uci(&["e7e8q"]));
        assert_eq!(
            promote[0].describe(),
            "1. e8=Q: the white pawn on e7 moves to e8 and promotes to a queen. Material now \
             White 9, Black 0 (White is 9 up)."
        );

        let stalemate = line_facts(&pos("k7/8/1Q6/8/8/8/8/4K3 w - - 0 1"), &uci(&["b6c7"]));
        assert_eq!(
            stalemate[0].describe(),
            "1. Qc7: the white queen on b6 moves to c7, stalemate (a draw)."
        );
    }
}
