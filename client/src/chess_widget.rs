#[cfg(not(target_arch = "wasm32"))]
use crate::desktop::game_socket::create_game_socket;
#[cfg(target_arch = "wasm32")]
use crate::web::game_socket::create_game_socket;
use chess::game::{Game, GameStatus};
use chess::moves::Move;
use chess::pieces::{Color, Piece, Position};
use chess::player::{Player, PlayerKind};

use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use std::sync::RwLock;

const WIDGET_SIZE: u32 = 800;
static GAME: Lazy<RwLock<Game>> = Lazy::new(|| RwLock::new(Game::new()));

#[derive(PartialEq)]
pub struct BoardPosition(Position);

impl From<&ClientPoint> for BoardPosition {
    fn from(point: &ClientPoint) -> Self {
        Self(Position {
            x: (8.0 * point.x / WIDGET_SIZE as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / WIDGET_SIZE as f64)).floor() as usize,
        })
    }
}

impl From<&BoardPosition> for ClientPoint {
    fn from(position: &BoardPosition) -> Self {
        Self {
            x: WIDGET_SIZE as f64 * position.0.x as f64 / 8.0,
            y: WIDGET_SIZE as f64 * (7.0 - position.0.y as f64) / 8.0,
            ..Default::default()
        }
    }
}

fn get_current_player_kind(cx: Scope<ChessWidgetProps>) -> PlayerKind {
    match GAME.read().unwrap().get_current_player() {
        Color::White => cx.props.white_player.kind,
        Color::Black => cx.props.black_player.kind,
    }
}

fn has_remote_player(cx: Scope<ChessWidgetProps>) -> bool {
    [cx.props.white_player.kind, cx.props.black_player.kind].contains(&PlayerKind::Remote)
}

// We want the square a dragged piece is considered to be on to be based on the center of
// the piece, not the location of the mouse. This requires offsetting based on the original
// mouse down location
fn get_dragged_piece_position(mouse_down: &ClientPoint, mouse_up: &ClientPoint) -> Position {
    let top_left = ClientPoint::from(&mouse_down.into());
    BoardPosition::from(&ClientPoint::new(
        top_left.x + mouse_up.x - mouse_down.x + WIDGET_SIZE as f64 / 16.0,
        top_left.y + mouse_up.y - mouse_down.y + WIDGET_SIZE as f64 / 16.0,
    ))
    .0
}

fn get_piece_image_file(piece: Piece) -> &'static str {
    match piece {
        Piece::Rook(Color::White) => "images/whiteRook.png",
        Piece::Bishop(Color::White) => "images/whiteBishop.png",
        Piece::Pawn(Color::White) => "images/whitePawn.png",
        Piece::Knight(Color::White) => "images/whiteKnight.png",
        Piece::King(Color::White) => "images/whiteKing.png",
        Piece::Queen(Color::White) => "images/whiteQueen.png",
        Piece::Rook(Color::Black) => "images/blackRook.png",
        Piece::Bishop(Color::Black) => "images/blackBishop.png",
        Piece::Pawn(Color::Black) => "images/blackPawn.png",
        Piece::Knight(Color::Black) => "images/blackKnight.png",
        Piece::King(Color::Black) => "images/blackKing.png",
        Piece::Queen(Color::Black) => "images/blackQueen.png",
    }
}

fn draw_piece<'a>(
    piece: Piece,
    pos: &Position,
    mouse_down_state: &Option<ClientPoint>,
    dragging_point_state: &Option<ClientPoint>,
) -> LazyNodes<'a, 'static> {
    let mut top_left = ClientPoint::from(&BoardPosition(*pos));
    if let Some(mouse_down) = mouse_down_state {
        if let Some(dragging_point) = dragging_point_state {
            if BoardPosition(*pos) == mouse_down.into() {
                top_left.x += dragging_point.x - mouse_down.x;
                top_left.y += dragging_point.y - mouse_down.y;
            }
        }
    }
    rsx! {
        img {
            src: "{get_piece_image_file(piece)}",
            class: "images",
            style: "left: {top_left.x}px; top: {top_left.y}px;",
            width: "{WIDGET_SIZE / 8}",
            height: "{WIDGET_SIZE / 8}",
        }
    }
}

#[derive(PartialEq, Props)]
pub struct ChessWidgetProps {
    white_player: Player,
    black_player: Player,
}

pub fn ChessWidget(cx: Scope<ChessWidgetProps>) -> Element {
    let mouse_down_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let dragging_point_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let board_state_hash = use_state(cx, || GAME.read().unwrap().get_real_state_hash());
    let dragged_piece_position = mouse_down_state
        .get()
        .as_ref()
        .map(|p| BoardPosition::from(p).0);
    let (pieces, dragged): (Vec<_>, Vec<_>) = (0..8)
        .flat_map(|x| (0..8).map(move |y| Position::new(x, y)))
        .filter_map(|pos| {
            GAME.read()
                .unwrap()
                .get_piece(&pos)
                .map(|piece| (pos, piece))
        })
        .partition(|(pos, _piece)| Some(*pos) != dragged_piece_position);
    let write_socket = if has_remote_player(cx) {
        create_game_socket(cx, board_state_hash, &GAME)
    } else {
        None
    };

    render! {
        style { include_str!("../styles/chess_widget.css") }
        div {
            autofocus: true,
            tabindex: 0,

            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: move |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    let from = BoardPosition::from(mouse_down).0;
                    let to = get_dragged_piece_position(mouse_down, &event.client_coordinates());
                    if get_current_player_kind(cx) == PlayerKind::Local
                        && GAME.read().unwrap().status != GameStatus::Replay
                        && GAME.write().unwrap().move_piece(from, to).is_ok()
                    {
                        if let Some(write_socket) = write_socket {
                            write_socket.send(Move::new(from, to));
                        }
                        board_state_hash.set(GAME.read().unwrap().get_real_state_hash());
                    }
                    mouse_down_state.set(None);
                    dragging_point_state.set(None);
                }
            },
            onmousemove: |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    if GAME.read().unwrap().has_piece(&BoardPosition::from(mouse_down).0) {
                        dragging_point_state.set(Some(event.client_coordinates()));
                    }
                }
            },
            onkeydown: |event| {
                match event.key() {
                    Key::ArrowLeft => {
                        GAME.write().unwrap().go_back_a_move();
                    },
                    Key::ArrowRight => {
                        GAME.write().unwrap().go_forward_a_move()
                    },
                    Key::ArrowUp => {
                        GAME.write().unwrap().resume()
                    },
                    Key::ArrowDown => {
                        GAME.write().unwrap().go_to_start();
                    }
                    _ => {
                        log::info!("Functionality not implemented for key: {:?}", event.key())
                    }
                };
                cx.needs_update()
            },
            img {
                src: "images/board.png",
                class: "images",
                style: "left: 0; top: 0;",
                width: "{WIDGET_SIZE}",
                height: "{WIDGET_SIZE}",
            },

            pieces
                .into_iter()
                .chain(dragged)
                .map(|(pos, piece)| {
                    draw_piece(piece, &pos, mouse_down_state.get(), dragging_point_state.get())
                })
        }
    }
}
