use crate::arrow::Arrow;
use crate::arrows::Arrows;
use crate::game_socket::create_game_socket;
use crate::mouse_click::MouseClick;
use chess::color::Color;
use chess::game::Game;
use chess::game_status::GameStatus;
use chess::moves::Move;
use chess::piece::Piece;
use chess::player::PlayerKind;
use chess::position::Position;
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::input_data::MouseButton;
use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;

fn get_piece_image_file(piece: Piece) -> String {
    format!("images/{piece}.png")
}

#[derive(Props, PartialEq)]
pub struct BoardProps {
    pub(crate) size: u32,
    #[props(!optional)]
    game_id: Option<u32>,
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
    perspective: Color,
}

// We want the square a dragged piece is considered to be on to be based on the center of
// the piece, not the location of the mouse. This requires offsetting based on the original
// mouse down location
fn get_dragged_piece_position(mouse_down: &ClientPoint, mouse_up: &ClientPoint) -> Position {
    let center = get_center(&to_position(mouse_down));
    to_position(&ClientPoint::new(
        center.x + mouse_up.x - mouse_down.x,
        center.y + mouse_up.y - mouse_down.y,
    ))
}

fn get_positions(
    pos: &Position,
    mouse_down_state: &Option<MouseClick>,
    dragging_point_state: &Option<ClientPoint>,
) -> (ClientPoint, usize) {
    let mut top_left = to_point(pos);
    let mut z_index = 1;
    if let Some(mouse_down) = mouse_down_state {
        if mouse_down.kind == MouseButton::Primary {
            if let Some(dragging_point) = dragging_point_state {
                if *pos == to_position(&mouse_down.point) {
                    z_index = 2;
                    top_left.x += dragging_point.x - mouse_down.point.x;
                    top_left.y += dragging_point.y - mouse_down.point.y;
                }
            }
        }
    }
    (top_left, z_index)
}

fn to_position(point: &ClientPoint, perspective: Color) -> Position {
    match perspective {
        Color::White => Position {
            x: (8.0 * point.x / size as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / size as f64)).floor() as usize,
        },
        Color::Black => Position {
            x: (8.0 * (1.0 - point.x / size as f64)).floor() as usize,
            y: (8.0 * point.y / size as f64).floor() as usize,
        },
    }
}

pub(crate) fn to_point(position: &Position) -> ClientPoint {
    match perspective {
        Color::White => ClientPoint {
            x: size as f64 * position.x as f64 / 8.0,
            y: size as f64 * (7.0 - position.y as f64) / 8.0,
            ..Default::default()
        },
        Color::Black => ClientPoint {
            x: size as f64 * (7.0 - position.x as f64) / 8.0,
            y: size as f64 * position.y as f64 / 8.0,
            ..Default::default()
        },
    }
}

fn handle_on_key_down(event: Event<KeyboardData>, arrows: &UseRef<Arrows>) {
    match event.key() {
        Key::ArrowLeft => game.with_mut(|game| game.go_back_a_move()),
        Key::ArrowRight => game.with_mut(|game| game.go_forward_a_move()),
        Key::ArrowUp => game.with_mut(|game| game.resume()),
        Key::ArrowDown => game.with_mut(|game| game.go_to_start()),
        Key::Character(c) => match c.as_str() {
            "z" => {
                if event.modifiers() == Modifiers::CONTROL {
                    arrows.with_mut(|arrows| arrows.undo());
                }
            }
            "y" => {
                if event.modifiers() == Modifiers::CONTROL {
                    arrows.with_mut(|arrows| arrows.redo());
                }
            }
            _ => log::debug!("{:?} key pressed", c),
        },
        _ => log::debug!("{:?} key pressed", event.key()),
    };
}

fn drop_piece(
    event: &Event<MouseData>,
    point: &ClientPoint,
    game_socket: Option<&Coroutine<Move>>,
) {
    let from = to_position(point);
    let to = get_dragged_piece_position(point, &event.client_coordinates());
    let current_player_kind = match game.with(|game| game.get_current_player()) {
        Color::White => white_player_kind,
        Color::Black => black_player_kind,
    };
    if current_player_kind == PlayerKind::Local
        && game.with(|game| {
            game.status != GameStatus::Replay && game.is_move_valid(&Move::new(from, to)).is_ok()
        })
    {
        game.with_mut(|game| game.move_piece(from, to).ok());
        if let Some(game_socket) = game_socket {
            game_socket.send(Move::new(from, to));
        }
    }
}

fn complete_arrow(event: &Event<MouseData>, mouse_down: &ClientPoint, arrows: &UseRef<Arrows>) {
    let from = to_position(mouse_down);
    let to = to_position(&event.client_coordinates());
    if to != from {
        arrows.with_mut(|arrows| arrows.push(Move { from, to }));
    }
}

fn handle_on_mouse_down_event(
    event: Event<MouseData>,
    mouse_down_state: &UseState<Option<MouseClick>>,
    arrows: &UseRef<Arrows>,
) {
    let mouse_down = MouseClick::from(event);
    if mouse_down.kind.contains(MouseButton::Primary) {
        arrows.with_mut(|arrows| arrows.clear());
    }
    mouse_down_state.set(Some(mouse_down));
}

fn handle_on_mouse_up_event(
    event: Event<MouseData>,
    mouse_down_state: &UseState<Option<MouseClick>>,
    dragging_point_state: &UseState<Option<ClientPoint>>,
    game_socket: Option<&Coroutine<Move>>,
    arrows: &UseRef<Arrows>,
) {
    if let Some(mouse_down) = mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            drop_piece(&event, &mouse_down.point, game_socket);
        }
        if mouse_down.kind.contains(MouseButton::Secondary) {
            complete_arrow(&event, &mouse_down.point, arrows);
        }
        mouse_down_state.set(None);
    }
    dragging_point_state.set(None);
}

fn handle_on_mouse_move_event(
    event: Event<MouseData>,
    mouse_down_state: &UseState<Option<MouseClick>>,
    dragging_point_state: &UseState<Option<ClientPoint>>,
) {
    if mouse_down_state.get().is_some() {
        dragging_point_state.set(Some(event.client_coordinates()));
    }
}

pub fn has_remote_player(white_player_kind: PlayerKind, black_player_kind: PlayerKind) -> bool {
    [white_player_kind, black_player_kind].contains(&PlayerKind::Remote)
}

pub fn get_center(pos: &Position) -> ClientPoint {
    let mut point = to_point(pos);
    point.x += size as f64 / 16.0;
    point.y += size as f64 / 16.0;
    point
}

pub fn get_move_for_arrow(
    mouse_down_state: &UseState<Option<MouseClick>>,
    dragging_point_state: &UseState<Option<ClientPoint>>,
) -> Option<Move> {
    if let Some(mouse_down) = mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Secondary) {
            if let Some(dragging_point) = *dragging_point_state.get() {
                let from = to_position(&mouse_down.point);
                let to = to_position(&dragging_point);
                return Some(Move::new(to, from));
            }
        }
    }
    None
}

pub fn Board(cx: Scope<BoardProps>) -> Element {
    // hooks
    let game = use_shared_state::<Game>(cx).unwrap();
    let mouse_down_state = use_state::<Option<MouseClick>>(cx, || None);
    let dragging_point_state = use_state::<Option<ClientPoint>>(cx, || None);
    let arrows = use_ref(cx, Arrows::default);
    let game_socket = cx.props.game_id.map(|game_id| {
        use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
            create_game_socket(game, game_id, rx)
        })
    });

    cx.render(rsx! {
        // div for widget
        div {
            class: "relative z-0",
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: |event| handle_on_mouse_down_event(event, mouse_down_state, arrows),
            onmouseup: move |event| handle_on_mouse_up_event(event, mouse_down_state, dragging_point_state, game_socket, arrows),
            onmousemove: |event| handle_on_mouse_move_event(event, mouse_down_state, dragging_point_state),
            onkeydown: |event| handle_on_key_down(event, arrows),
            // board
            img {
                src: "images/board.png",
                class: "images inset-0 z-0",
                width: "{cx.props.size}",
                height: "{cx.props.size}",
            },
            // pieces
            game.read().get_pieces().into_iter().map(|(piece, pos)| {
                let (top_left, z_index) = get_positions(&pos, mouse_down_state, dragging_point_state);
                rsx! {
                    img {
                        src: "{get_piece_image_file(piece)}",
                        class: "images",
                        style: "left: {top_left.x}px; top: {top_left.y}px; z-index: {z_index}",
                        width: "{cx.props.size / 8}",
                        height: "{cx.props.size / 8}",
                    }
                }
            }),
             // arrows
            arrows.with(|arrows| arrows.get()).iter().map(|mv| {
                rsx! {
                    Arrow { mv: *mv, board_props: cx.props }
                }
            }),
            if let Some(current_mv) = get_move_for_arrow(mouse_down_state, dragging_point_state) {
                rsx! {
                    Arrow { mv: current_mv, board_props: cx.props }
                }
            }
        },
    })
}
