use crate::game::Game;
use crate::pieces::{Piece, Player, Position};
use dioxus::html::geometry::ClientPoint;
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use std::sync::RwLock;

const WIDGET_SIZE: u32 = 800;
static GAME: Lazy<RwLock<Game>> = Lazy::new(|| RwLock::new(Game::default()));

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
    let top_left = ClientPoint::from(&Position::from(mouse_down));
    let middle = ClientPoint {
        x: top_left.x + WIDGET_SIZE as f64 / 16.0,
        y: top_left.y + WIDGET_SIZE as f64 / 16.0,
        ..Default::default()
    };
    let x_offset = mouse_down.x - middle.x;
    let y_offset = mouse_down.y - middle.y;
    Position::from(&ClientPoint {
        x: mouse_up.x - x_offset,
        y: mouse_up.y - y_offset,
        ..Default::default()
    })
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
            if *pos == Position::from(mouse_down) {
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
    let dragged_piece_position = mouse_down_state.get().as_ref().map(|m| Position::from(m));
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
            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: move |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    let from = Position::from(mouse_down);
                    let to = get_dragged_piece_position(mouse_down, &event.client_coordinates());
                    GAME.write().unwrap().move_piece(from, to).ok();
                    mouse_down_state.set(None);
                    dragging_point_state.set(None);
                }
            },
            onmousemove: |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    if GAME.read().unwrap().get_piece(&Position::from(mouse_down)).is_some() {
                        dragging_point_state.set(Some(event.client_coordinates()));
                    }
                }
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
