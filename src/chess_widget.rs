use crate::game::Game;
use crate::pieces::{Piece, Player, Position};

use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use std::sync::RwLock;

const WIDGET_SIZE: u32 = 800;
static GAME: Lazy<RwLock<Game>> = Lazy::new(|| RwLock::new(Game::new()));

impl From<&ClientPoint> for Position {
    fn from(point: &ClientPoint) -> Position {
        Position {
            x: (8.0 * point.x / WIDGET_SIZE as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / WIDGET_SIZE as f64)).floor() as usize,
        }
    }
}

impl From<&Position> for ClientPoint {
    fn from(position: &Position) -> ClientPoint {
        ClientPoint {
            x: WIDGET_SIZE as f64 * position.x as f64 / 8.0,
            y: WIDGET_SIZE as f64 * (7.0 - position.y as f64) / 8.0,
            ..Default::default()
        }
    }
}

// We want the square a dragged piece is considered to be on to be based on the center of
// the piece, not the location of the mouse. This requires offsetting based on the original
// mouse down location
fn get_dragged_piece_position(mouse_down: &ClientPoint, mouse_up: &ClientPoint) -> Position {
    let top_left = ClientPoint::from(&mouse_down.into());
    (&ClientPoint::new(
        top_left.x + mouse_up.x - mouse_down.x + WIDGET_SIZE as f64 / 16.0,
        top_left.y + mouse_up.y - mouse_down.y + WIDGET_SIZE as f64 / 16.0,
    ))
        .into()
}

fn get_piece_image_file(piece: Piece) -> &'static str {
    match piece {
        Piece::Rook(Player::White) => "images/whiteRook.png",
        Piece::Bishop(Player::White) => "images/whiteBishop.png",
        Piece::Pawn(Player::White) => "images/whitePawn.png",
        Piece::Knight(Player::White) => "images/whiteKnight.png",
        Piece::King(Player::White) => "images/whiteKing.png",
        Piece::Queen(Player::White) => "images/whiteQueen.png",
        Piece::Rook(Player::Black) => "images/blackRook.png",
        Piece::Bishop(Player::Black) => "images/blackBishop.png",
        Piece::Pawn(Player::Black) => "images/blackPawn.png",
        Piece::Knight(Player::Black) => "images/blackKnight.png",
        Piece::King(Player::Black) => "images/blackKing.png",
        Piece::Queen(Player::Black) => "images/blackQueen.png",
    }
}

fn draw_piece<'a>(
    piece: Piece,
    pos: &Position,
    mouse_down_state: &Option<ClientPoint>,
    dragging_point_state: &Option<ClientPoint>,
) -> LazyNodes<'a, 'static> {
    let mut top_left = ClientPoint::from(pos);
    if let Some(mouse_down) = mouse_down_state {
        if let Some(dragging_point) = dragging_point_state {
            if *pos == mouse_down.into() {
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

#[inline_props]
pub fn ChessWidget(cx: Scope) -> Element {
    let mouse_down_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let dragging_point_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let dragged_piece_position = mouse_down_state.get().as_ref().map(|p| p.into());
    let (pieces, dragged): (Vec<_>, Vec<_>) = (0..8)
        .flat_map(|x| (0..8).map(move |y| Position { x, y }))
        .filter_map(|pos| {
            GAME.read()
                .unwrap()
                .get_piece(&pos)
                .map(|piece| (pos, piece))
        })
        .partition(|(pos, _piece)| Some(*pos) != dragged_piece_position);

    render! {
        style { include_str!("../styles/chess_widget.css") }
        div {
            autofocus: true,
            tabindex: 0,

            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    let from = mouse_down.into();
                    let to = get_dragged_piece_position(mouse_down, &event.client_coordinates());
                    GAME.write().unwrap().move_piece(from, to).ok();
                    mouse_down_state.set(None);
                    dragging_point_state.set(None);
                }
            },
            onmousemove: |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    if GAME.read().unwrap().has_piece(&mouse_down.into()) {
                        dragging_point_state.set(Some(event.client_coordinates()));
                    }
                }
            },
            onkeydown: |event| {
                match event.key() {
                    Key::ArrowLeft => {
                        GAME.write().unwrap().go_back_a_turn();
                        cx.needs_update()
                    },
                    Key::ArrowRight => {
                        GAME.write().unwrap().go_forward_a_turn()
                    },
                    Key::ArrowUp => {
                        GAME.write().unwrap().resume()
                    },
                    Key::ArrowDown => {
                        GAME.write().unwrap().go_to_beginning();
                    }
                    _ => {
                        println!("Functionality not implemented for key: {:?}", event.key())
                    }
                };
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
                .chain(dragged.into_iter())
                .map(|(pos, piece)| {
                    draw_piece(piece, &pos, mouse_down_state.get(), dragging_point_state.get())
                })
        }
    }
}
