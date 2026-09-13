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
/// The board is only interactive when viewing the latest position of a game
/// that isn't over. Arrow keys step through the move history when the board
/// has focus.
#[component]
pub fn Board(game: Signal<Game>, orientation: Color) -> Element {
    let mut selected: Signal<Option<Square>> = use_signal(|| None);
    // A move that needs a promotion piece before it can be played.
    let mut promotion: Signal<Option<(Square, Square)>> = use_signal(|| None);

    let g = game.read();
    let position = g.position();
    let turn = position.turn();
    let interactive = !g.is_viewing_history() && !g.status().is_game_over();
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
                let outcome = game.write().play_from_to(from, square, None).map(|_| ());
                match outcome {
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
            if let Some((from, to)) = promotion() {
                PromotionPicker {
                    color: turn,
                    on_pick: move |role: Option<Role>| {
                        if let Some(role) = role {
                            let mut game = game;
                            if let Err(err) = game.write().play_from_to(from, to, Some(role)) {
                                dioxus::logger::tracing::warn!("promotion failed: {err}");
                            }
                        }
                        promotion.set(None);
                        selected.set(None);
                    },
                }
            }
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
