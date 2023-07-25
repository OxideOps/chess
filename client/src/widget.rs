#[cfg(not(target_arch = "wasm32"))]
use crate::desktop::game_socket::create_game_socket;
#[cfg(target_arch = "wasm32")]
use crate::web::game_socket::create_game_socket;
use async_std::task::sleep;

use chess::game::{Game, GameStatus};
use chess::moves::Move;
use chess::pieces::{Color, Piece, Position};
use chess::player::{Player, PlayerKind};

use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;
use std::time::Duration;

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

fn get_piece_image_file(piece: Piece) -> String {
    format!("images/{piece}.png")
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

fn display_time(time: Duration) -> String {
    let total_secs = time.as_secs();
    let hours = total_secs / 3600;
    let minutes = total_secs % 3600 / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn use_timer_future(
    cx: Scope<WidgetProps>,
    game: &UseRef<Game>,
    active_time_state: &UseState<String>,
) {
    use_future(cx, (game,), |(game,)| {
        let active_time_state = active_time_state.to_owned();
        async move {
            if game.with(|game| game.is_timer_active()) {
                loop {
                    let active_time = game.with(|game| game.get_active_timer());
                    let sleep_time = active_time.subsec_micros();
                    sleep(Duration::from_micros(sleep_time as u64)).await;
                    active_time_state.set(display_time(active_time));
                }
            } else {
                sleep(Duration::from_secs(u64::MAX)).await;
            }
        }
    });
}
#[derive(Props, PartialEq)]
pub struct WidgetProps {
    white_player: Player,
    black_player: Player,
    time: Duration,
}

pub fn has_remote_player(props: &WidgetProps) -> bool {
    [props.white_player.kind, props.black_player.kind].contains(&PlayerKind::Remote)
}

pub fn Widget(cx: Scope<WidgetProps>) -> Element {
    let game = use_ref(cx, || Game::builder().duration(cx.props.time).build());

    let write_socket = if has_remote_player(cx.props) {
        create_game_socket(cx, game)
    } else {
        None
    };

    let mouse_down_state = use_state::<Option<ClientPoint>>(cx, || None);
    let dragging_point_state = use_state::<Option<ClientPoint>>(cx, || None);
    let white_time = use_state(cx, || display_time(cx.props.time));
    let black_time = use_state(cx, || display_time(cx.props.time));
    let active_time_state = match game.with(|game| game.get_current_player()) {
        Color::White => white_time,
        Color::Black => black_time,
    };
    use_timer_future(cx, game, active_time_state);

    cx.render(rsx! {
        style { include_str!("../../styles/widget.css") }
        div {
            autofocus: true,
            tabindex: 0,
            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: move |event| handle_on_mouse_up_event(&event, game, cx.props, mouse_down_state, dragging_point_state, write_socket),
            onmousemove: move |event| handle_on_mouse_move_event(&event, game, mouse_down_state, dragging_point_state),
            onkeydown: move |event| game.with_mut(|game| handle_key_event(game, event.key())),
            img {
                src: "images/board.png",
                class: "images",
                style: "left: 0; top: 0;",
                width: "{BOARD_SIZE}",
                height: "{BOARD_SIZE}",
            },
            div {
                class: "time-container",
                style: "position: absolute; left: {BOARD_SIZE}px; top: 0px",
                p {
                    "White time: {white_time}\n",
                },
                p {
                    "Black time: {black_time}",
                }
            }
            game.with(|game| game.get_pieces()).into_iter().map(|(piece, pos)| {
                draw_piece(piece, &pos, mouse_down_state.get(), dragging_point_state.get())
            }),
        }
    })
}

fn handle_on_mouse_up_event(
    event: &Event<MouseData>,
    game: &UseRef<Game>,
    props: &WidgetProps,
    mouse_down_state: &UseState<Option<ClientPoint>>,
    dragging_point_state: &UseState<Option<ClientPoint>>,
    write_socket: Option<&Coroutine<Move>>,
) {
    if let Some(mouse_down) = mouse_down_state.get() {
        let from = to_position(mouse_down);
        let to = get_dragged_piece_position(mouse_down, &event.client_coordinates());
        let current_player_kind = match game.with(|game| game.get_current_player()) {
            Color::White => props.white_player.kind,
            Color::Black => props.black_player.kind,
        };
        if current_player_kind == PlayerKind::Local
            && game.with(|game| game.status) != GameStatus::Replay
            && game.with_mut(|game| game.move_piece(from, to).is_ok())
        {
            if let Some(write_socket) = &write_socket {
                write_socket.send(Move::new(from, to));
            }
        }
        mouse_down_state.set(None);
        dragging_point_state.set(None);
    }
}

fn handle_on_mouse_move_event(
    event: &Event<MouseData>,
    game: &UseRef<Game>,
    mouse_down_state: &UseState<Option<ClientPoint>>,
    dragging_point_state: &UseState<Option<ClientPoint>>,
) {
    if let Some(mouse_down) = mouse_down_state.get() {
        if game.with(|game| game.has_piece(&to_position(mouse_down))) {
            dragging_point_state.set(Some(event.client_coordinates()));
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
