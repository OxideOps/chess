use chess_core::{
    Color, Game, GameError, Piece, Role, Square,
    shakmaty::{File, Position, Rank, uci::UciMove},
};
use dioxus::prelude::*;

const BOARD_CSS: Asset = asset!("/assets/board.css");

/// Everything one square needs to render itself.
struct SquareView {
    square: Square,
    piece: Option<Piece>,
    selected: bool,
    last_move: bool,
    check: bool,
    /// A legal destination of the selected piece.
    destination: bool,
    /// Coordinate labels go on the left column and bottom row of the board.
    rank_label: Option<char>,
    file_label: Option<char>,
}

/// An interactive board. Click a piece, then click where it should go.
///
/// In play mode the board only accepts moves at the latest position of a
/// game that isn't over. In `analysis` mode any position can be played from:
/// moving while viewing history discards the moves after it. Arrow keys step
/// through the move history when the board has focus. `arrows` are drawn
/// from square to square, e.g. for engine suggestions.
#[component]
pub fn Board(
    game: Signal<Game>,
    orientation: Color,
    #[props(default)] analysis: bool,
    #[props(default)] arrows: Vec<(Square, Square)>,
) -> Element {
    let mut selected: Signal<Option<Square>> = use_signal(|| None);
    // A move that needs a promotion piece before it can be played.
    let mut promotion: Signal<Option<(Square, Square)>> = use_signal(|| None);

    let g = game.read();
    let position = g.position();
    let turn = position.turn();
    let interactive = !g.status().is_game_over() && (analysis || !g.is_viewing_history());
    let selected_square = if interactive { selected() } else { None };
    let destinations = selected_square
        .map(|sq| g.legal_destinations(sq))
        .unwrap_or_default();
    let last_move = match g.last_move().map(|m| m.uci) {
        Some(UciMove::Normal { from, to, .. }) => Some((from, to)),
        _ => None,
    };
    let king_in_check = position
        .is_check()
        .then(|| position.board().king_of(turn))
        .flatten();

    let mut files = File::ALL;
    let mut ranks = Rank::ALL;
    if orientation.is_white() {
        ranks.reverse();
    } else {
        files.reverse();
    }
    let squares: Vec<SquareView> = ranks
        .iter()
        .flat_map(|&rank| files.iter().map(move |&file| (file, rank)))
        .map(|(file, rank)| {
            let square = Square::from_coords(file, rank);
            SquareView {
                square,
                piece: position.board().piece_at(square),
                selected: selected_square == Some(square),
                last_move: last_move.is_some_and(|(from, to)| from == square || to == square),
                check: king_in_check == Some(square),
                destination: destinations.contains(&square),
                rank_label: (file == files[0]).then(|| rank.char()),
                file_label: (rank == ranks[7]).then(|| file.char()),
            }
        })
        .collect();
    drop(g);
    let arrow_views: Vec<ArrowView> = arrows
        .iter()
        .map(|&(from, to)| ArrowView::new(from, to, orientation))
        .collect();

    // Plays at the end of the game, or from the viewed position in analysis mode.
    let play = move |from: Square, to: Square, promotion: Option<Role>| -> Result<(), GameError> {
        let mut game = game;
        let mut g = game.write();
        if analysis {
            g.play_here_from_to(from, to, promotion).map(|_| ())
        } else {
            g.play_from_to(from, to, promotion).map(|_| ())
        }
    };

    let mut on_square_click = move |square: Square| {
        if !interactive {
            return;
        }
        let owns_piece = move |sq: Square| {
            game.read()
                .position()
                .board()
                .piece_at(sq)
                .is_some_and(|p| p.color == turn)
        };
        match selected() {
            Some(from) if from == square => selected.set(None),
            Some(from) => {
                match play(from, square, None) {
                    Ok(()) => selected.set(None),
                    Err(GameError::PromotionRequired) => promotion.set(Some((from, square))),
                    // Clicking another of your own pieces selects it instead.
                    Err(_) => selected.set(owns_piece(square).then_some(square)),
                }
            }
            None => {
                if owns_piece(square) {
                    selected.set(Some(square));
                }
            }
        }
    };

    let on_key_down = move |event: Event<KeyboardData>| {
        let mut game = game;
        match event.key() {
            Key::ArrowLeft => game.write().go_back(),
            Key::ArrowRight => game.write().go_forward(),
            Key::ArrowUp => game.write().go_to_start(),
            Key::ArrowDown => game.write().go_to_end(),
            Key::Escape => {
                selected.set(None);
                promotion.set(None);
            }
            _ => return,
        }
        event.prevent_default();
    };

    rsx! {
        document::Stylesheet { href: BOARD_CSS }
        div {
            class: "board",
            class: if interactive { "interactive" },
            tabindex: "0",
            onkeydown: on_key_down,
            for view in squares {
                div {
                    key: "{view.square}",
                    class: "square",
                    class: if view.square.is_light() { "light" } else { "dark" },
                    class: if view.selected { "selected" },
                    class: if view.last_move { "last-move" },
                    class: if view.check { "check" },
                    onclick: move |_| on_square_click(view.square),
                    if let Some(piece) = view.piece {
                        img {
                            class: "piece",
                            src: piece_asset(piece),
                            alt: "{piece_name(piece)}",
                            draggable: false,
                        }
                    }
                    if view.destination {
                        span { class: if view.piece.is_some() { "capture-hint" } else { "move-hint" } }
                    }
                    if let Some(label) = view.rank_label {
                        span { class: "coord rank", "{label}" }
                    }
                    if let Some(label) = view.file_label {
                        span { class: "coord file", "{label}" }
                    }
                }
            }
            if !arrow_views.is_empty() {
                svg { class: "arrows", "viewBox": "0 0 8 8",
                    defs {
                        marker {
                            id: "arrowhead",
                            "viewBox": "0 0 10 10",
                            "refX": "5",
                            "refY": "5",
                            "markerWidth": "4",
                            "markerHeight": "4",
                            orient: "auto",
                            polygon { points: "0,0 10,5 0,10" }
                        }
                    }
                    for a in arrow_views {
                        line {
                            x1: "{a.x1}",
                            y1: "{a.y1}",
                            x2: "{a.x2}",
                            y2: "{a.y2}",
                            "marker-end": "url(#arrowhead)",
                        }
                    }
                }
            }
            if let Some((from, to)) = promotion() {
                PromotionPicker {
                    color: turn,
                    on_pick: move |role: Option<Role>| {
                        if let Some(role) = role
                            && let Err(err) = play(from, to, Some(role))
                        {
                            dioxus::logger::tracing::warn!("promotion failed: {err}");
                        }
                        promotion.set(None);
                        selected.set(None);
                    },
                }
            }
        }
    }
}

/// An arrow in board coordinates (8x8, origin top-left as drawn).
struct ArrowView {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl ArrowView {
    fn new(from: Square, to: Square, orientation: Color) -> Self {
        let centre = |sq: Square| {
            let (file, rank) = (
                f64::from(u32::from(sq.file())),
                f64::from(u32::from(sq.rank())),
            );
            if orientation.is_white() {
                (file + 0.5, 7.5 - rank)
            } else {
                (7.5 - file, rank + 0.5)
            }
        };
        let (x1, y1) = centre(from);
        let (x2, y2) = centre(to);
        // Stop short of the destination centre so the head sits inside the square.
        let (dx, dy) = (x2 - x1, y2 - y1);
        let len = (dx * dx + dy * dy).sqrt().max(f64::EPSILON);
        let shorten = 0.3;
        Self {
            x1,
            y1,
            x2: x2 - dx / len * shorten,
            y2: y2 - dy / len * shorten,
        }
    }
}

/// Overlay asking which piece a pawn should become. Calls `on_pick(None)` when dismissed.
#[component]
fn PromotionPicker(color: Color, on_pick: EventHandler<Option<Role>>) -> Element {
    rsx! {
        div { class: "promotion", onclick: move |_| on_pick.call(None),
            for role in [Role::Queen, Role::Rook, Role::Bishop, Role::Knight] {
                button {
                    r#type: "button",
                    title: "{role_name(role)}",
                    onclick: move |event| {
                        event.stop_propagation();
                        on_pick.call(Some(role));
                    },
                    img { src: piece_asset(Piece { color, role }), alt: "{role_name(role)}" }
                }
            }
        }
    }
}

fn piece_asset(piece: Piece) -> Asset {
    match (piece.color, piece.role) {
        (Color::White, Role::Pawn) => asset!("/assets/pieces/cburnett/wP.svg"),
        (Color::White, Role::Knight) => asset!("/assets/pieces/cburnett/wN.svg"),
        (Color::White, Role::Bishop) => asset!("/assets/pieces/cburnett/wB.svg"),
        (Color::White, Role::Rook) => asset!("/assets/pieces/cburnett/wR.svg"),
        (Color::White, Role::Queen) => asset!("/assets/pieces/cburnett/wQ.svg"),
        (Color::White, Role::King) => asset!("/assets/pieces/cburnett/wK.svg"),
        (Color::Black, Role::Pawn) => asset!("/assets/pieces/cburnett/bP.svg"),
        (Color::Black, Role::Knight) => asset!("/assets/pieces/cburnett/bN.svg"),
        (Color::Black, Role::Bishop) => asset!("/assets/pieces/cburnett/bB.svg"),
        (Color::Black, Role::Rook) => asset!("/assets/pieces/cburnett/bR.svg"),
        (Color::Black, Role::Queen) => asset!("/assets/pieces/cburnett/bQ.svg"),
        (Color::Black, Role::King) => asset!("/assets/pieces/cburnett/bK.svg"),
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

fn piece_name(piece: Piece) -> String {
    format!(
        "{} {}",
        super::side_name(piece.color).to_lowercase(),
        role_name(piece.role)
    )
}
