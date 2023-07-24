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
use std::ops::Deref;
use std::time::Duration;

type ScopedWidget<'cx> = &'cx Scoped<'cx, ChessWidgetProps>;

const BOARD_SIZE: u32 = 800;

fn to_position(point: &ClientPoint) -> Position {
    Position {
        x: (8.0 * point.x / BOARD_SIZE as f64).floor() as usize,
        y: (8.0 * (1.0 - point.y / BOARD_SIZE as f64)).floor() as usize,
    }
}

fn to_point(position: &Position) -> ClientPoint {
    ClientPoint {
        x: BOARD_SIZE as f64 * position.x as f64 / 8.0,
        y: BOARD_SIZE as f64 * (7.0 - position.y as f64) / 8.0,
        ..Default::default()
    }
}

fn get_current_player_kind(cx: ScopedWidget, game: &UseRef<Game>) -> PlayerKind {
    match game.with(|game| game.get_current_player()) {
        Color::White => cx.props.white_player.kind,
        Color::Black => cx.props.black_player.kind,
    }
}

fn has_remote_player(cx: ScopedWidget) -> bool {
    [cx.props.white_player.kind, cx.props.black_player.kind].contains(&PlayerKind::Remote)
}

// We want the square a dragged piece is considered to be on to be based on the center of
// the piece, not the location of the mouse. This requires offsetting based on the original
// mouse down location
fn get_dragged_piece_position(mouse_down: &ClientPoint, mouse_up: &ClientPoint) -> Position {
    let top_left = to_point(&to_position(mouse_down));
    to_position(&ClientPoint::new(
        top_left.x + mouse_up.x - mouse_down.x + BOARD_SIZE as f64 / 16.0,
        top_left.y + mouse_up.y - mouse_down.y + BOARD_SIZE as f64 / 16.0,
    ))
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

fn draw_piece<'a, 'b>(
    piece: Piece,
    pos: &Position,
    mouse_down_state: &Option<ClientPoint>,
    dragging_point_state: &Option<ClientPoint>,
) -> LazyNodes<'a, 'b> {
    let mut top_left = to_point(pos);
    let mut z_index = 0;
    if let Some(mouse_down) = mouse_down_state {
        if let Some(dragging_point) = dragging_point_state {
            if *pos == to_position(mouse_down) {
                z_index = 1;
                top_left.x += dragging_point.x - mouse_down.x;
                top_left.y += dragging_point.y - mouse_down.y;
            }
        }
    }
    rsx! {
        img {
            src: "{get_piece_image_file(piece)}",
            class: "images",
            style: "left: {top_left.x}px; top: {top_left.y}px; z-index: {z_index}",
            width: "{BOARD_SIZE / 8}",
            height: "{BOARD_SIZE / 8}",
        }
    }
}

#[derive(PartialEq, Props)]
pub struct ChessWidgetProps {
    white_player: Player,
    black_player: Player,
}

#[derive(Copy, Clone)]
pub struct GameContext<'cx> {
    game: &'cx UseRef<Game>,
    mouse_down_state: &'cx UseState<Option<ClientPoint>>,
    dragging_point_state: &'cx UseState<Option<ClientPoint>>,
    write_socket: Option<&'cx Coroutine<Move>>,
}

impl<'cx> Deref for GameContext<'cx> {
    type Target = UseRef<Game>;
    fn deref(&self) -> &'cx Self::Target {
        self.game
    }
}

pub fn ChessWidget(cx: ScopedWidget) -> Element {
    let game = use_ref(cx, || {
        Game::builder().duration(Duration::from_secs(3600)).build()
    });
    let write_socket = if has_remote_player(cx) {
        create_game_socket(cx, game)
    } else {
        None
    };

    let game_cx = GameContext {
        game,
        mouse_down_state: use_state(cx, || None),
        dragging_point_state: use_state(cx, || None),
        write_socket,
    };

    cx.render(rsx! {
        style { include_str!("../../styles/chess_widget.css") }
        div {
            autofocus: true,
            tabindex: 0,

            onmousedown: |event| game_cx.mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: move |event| handle_on_mouse_up_event(event, cx, &game_cx),
            onmousemove: move |event| handle_on_mouse_move_event(event, &game_cx),
            onkeydown: |event| game.with_mut(|game| handle_key_event(game, event.key())),
            img {
                src: "images/board.png",
                class: "images",
                style: "left: 0; top: 0;",
                width: "{BOARD_SIZE}",
                height: "{BOARD_SIZE}",
            },
            for y in 0..8 {
                for x in 0..8 {
                    if let Some(piece) = game.with(|game| game.get_piece(&Position::new(x, y))) {
                        draw_piece(piece, &Position::new(x, y), game_cx.mouse_down_state.get(), game_cx.dragging_point_state.get())
                    }
                }
            },
            div {
                class: "time-container",
                style: "position: absolute; left: {BOARD_SIZE}px; top: 0px",
                p {
                    "White time: {game_cx.with(|game| game.get_timer(Color::White)):?}\n",
                },
                p {
                    "Black time: {game_cx.with(|game| game.get_timer(Color::Black)):?}",
                }
            }
        }
    })
}

fn handle_on_mouse_up_event(event: Event<MouseData>, cx: ScopedWidget, game_cx: &GameContext) {
    if let Some(mouse_down) = game_cx.mouse_down_state.get() {
        let from = to_position(mouse_down);
        let to = get_dragged_piece_position(mouse_down, &event.client_coordinates());
        if get_current_player_kind(cx, game_cx) == PlayerKind::Local
            && game_cx.with(|game| game.status) != GameStatus::Replay
            && game_cx.with_mut(|game| game.move_piece(from, to).is_ok())
        {
            if let Some(write_socket) = &game_cx.write_socket {
                write_socket.send(Move::new(from, to));
            }
        }
        game_cx.mouse_down_state.set(None);
        game_cx.dragging_point_state.set(None);
    }
}

fn handle_on_mouse_move_event(event: Event<MouseData>, game_cx: &GameContext) {
    if let Some(mouse_down) = game_cx.mouse_down_state.get() {
        if game_cx.with(|game| game.has_piece(&to_position(mouse_down))) {
            game_cx
                .dragging_point_state
                .set(Some(event.client_coordinates()));
        }
    }
}

fn handle_key_event(game: &mut Game, key: Key) {
    match key {
        Key::ArrowLeft => game.go_back_a_move(),
        Key::ArrowRight => game.go_forward_a_move(),
        Key::ArrowUp => game.resume(),
        Key::ArrowDown => game.go_to_start(),
        _ => log::debug!("{:?} key pressed", key),
    };
}
