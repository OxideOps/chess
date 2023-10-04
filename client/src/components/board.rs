use async_std::channel::{unbounded, Receiver, Sender};
use chess::color::Color;
use chess::game::Game;
use chess::moves::Move;
use chess::piece::Piece;
use chess::player::PlayerKind;
use chess::position::Position;
use dioxus::html::input_data::keyboard_types::Modifiers;
use dioxus::html::input_data::MouseButton;
use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;
use futures::executor::block_on;
use once_cell::sync::Lazy;

use crate::arrows::{ArrowData, Arrows};
use crate::components::{Arrow, BoardSquare, Piece};
use crate::game_socket::create_game_socket;
use crate::mouse_click::MouseClick;
use crate::shared_states::GameId;
use crate::stockfish::core::{on_game_changed, toggle_stockfish};
use crate::stockfish::interface::Process;

pub(crate) type Channel<T> = (Sender<T>, Receiver<T>);

// Channel for sending moves to `game_socket` to be sent to a remote player
static MOVE_CHANNEL: Lazy<Channel<Move>> = Lazy::new(unbounded);
// Channel for telling dragged pieces how far they have been dragged
static DRAG_CHANNEL: Lazy<Channel<ClientPoint>> = Lazy::new(unbounded);

fn get_board_image(theme: &str) -> String {
    format!("images/boards/{theme}/{theme}.png")
}

fn get_piece_image_file(theme: &str, piece: Piece) -> String {
    let piece_img = match piece {
        Piece::Pawn(Color::White) => "pw",
        Piece::Knight(Color::White) => "nw",
        Piece::Bishop(Color::White) => "bw",
        Piece::Rook(Color::White) => "rw",
        Piece::Queen(Color::White) => "qw",
        Piece::King(Color::White) => "kw",
        Piece::Pawn(Color::Black) => "pb",
        Piece::Knight(Color::Black) => "nb",
        Piece::Bishop(Color::Black) => "bb",
        Piece::Rook(Color::Black) => "rb",
        Piece::Queen(Color::Black) => "qb",
        Piece::King(Color::Black) => "kb",
    };

    format!("images/pieces/{theme}/{piece_img}.svg")
}

#[derive(Props, PartialEq)]
pub(crate) struct BoardProps {
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
    perspective: Color,
    size: u32,
    analyze: UseState<bool>,
}

// We want the square a dragged piece is considered to be on to be based on the center of
// the piece, not the location of the mouse. This requires offsetting based on the original
// mouse down location
fn get_dragged_piece_position(
    cx: Scope<BoardProps>,
    mouse_down: &ClientPoint,
    mouse_up: &ClientPoint,
) -> Position {
    let center = get_center(
        &to_position(cx, mouse_down),
        cx.props.size,
        cx.props.perspective,
    );
    to_position(
        cx,
        &ClientPoint::new(
            center.x + mouse_up.x - mouse_down.x,
            center.y + mouse_up.y - mouse_down.y,
        ),
    )
}

fn to_position(cx: Scope<BoardProps>, point: &ClientPoint) -> Position {
    match cx.props.perspective {
        Color::White => Position {
            x: (8.0 * point.x / cx.props.size as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / cx.props.size as f64)).floor() as usize,
        },
        Color::Black => Position {
            x: (8.0 * (1.0 - point.x / cx.props.size as f64)).floor() as usize,
            y: (8.0 * point.y / cx.props.size as f64).floor() as usize,
        },
    }
}

pub(crate) fn to_point(position: &Position, board_size: u32, perspective: Color) -> ClientPoint {
    match perspective {
        Color::White => ClientPoint::new(
            board_size as f64 * position.x as f64 / 8.0,
            board_size as f64 * (7.0 - position.y as f64) / 8.0,
        ),
        Color::Black => ClientPoint::new(
            board_size as f64 * (7.0 - position.x as f64) / 8.0,
            board_size as f64 * position.y as f64 / 8.0,
        ),
    }
}

fn handle_on_key_down(cx: Scope<BoardProps>, event: Event<KeyboardData>, arrows: &UseRef<Arrows>) {
    let game = use_shared_state::<Game>(cx).unwrap();

    match event.key() {
        Key::ArrowLeft => game.write().go_back_a_move(),
        Key::ArrowRight => game.write().go_forward_a_move(),
        Key::ArrowUp => game.write().resume(),
        Key::ArrowDown => game.write().go_to_start(),
        Key::Character(c) => match c.as_str() {
            "z" => {
                if event.modifiers() == Modifiers::CONTROL {
                    arrows.write().undo();
                }
            }
            "y" => {
                if event.modifiers() == Modifiers::CONTROL {
                    arrows.write().redo();
                }
            }
            _ => log::debug!("{:?} key pressed", c),
        },
        _ => log::debug!("{:?} key pressed", event.key()),
    };
}

fn drop_piece(cx: Scope<BoardProps>, event: &Event<MouseData>, point: &ClientPoint) {
    let game = use_shared_state::<Game>(cx).unwrap();
    let from = to_position(cx, point);
    let to = get_dragged_piece_position(cx, point, &event.client_coordinates());
    let (current_player_kind, opponent_player_kind) = match game.read().get_current_player() {
        Color::White => (cx.props.white_player_kind, cx.props.black_player_kind),
        Color::Black => (cx.props.black_player_kind, cx.props.white_player_kind),
    };
    let mv = Move::new(from, to);
    if current_player_kind == PlayerKind::Local
        && !game.read().is_replaying()
        && game.read().is_move_valid(&mv).is_ok()
    {
        game.write().move_piece(from, to).ok();
        if opponent_player_kind == PlayerKind::Remote {
            cx.spawn(async move {
                MOVE_CHANNEL.0.send(mv).await.expect("Failed to send move!");
            });
        }
    }
}

fn complete_arrow(arrows: &UseRef<Arrows>, drawing_arrow: &UseRef<Option<ArrowData>>) {
    if let Some(arrow_data) = *drawing_arrow.read() {
        if arrow_data.has_length() {
            arrows.write().push(arrow_data);
        }
    }
    *drawing_arrow.write() = None;
}

fn handle_on_mouse_down_event(
    cx: Scope<BoardProps>,
    event: Event<MouseData>,
    mouse_down_state: &UseState<Option<MouseClick>>,
    arrows: &UseRef<Arrows>,
    drawing_arrow: &UseRef<Option<ArrowData>>,
    selected_piece: &UseRef<Option<Position>>,
) {
    let mouse_down = MouseClick::from(event);
    if mouse_down.kind.contains(MouseButton::Primary) {
        selected_piece.set(Some(to_position(cx, &mouse_down.point)));
        arrows.write().clear();
    } else if mouse_down.kind.contains(MouseButton::Secondary) {
        let pos = to_position(cx, &mouse_down.point);
        drawing_arrow.set(Some(ArrowData::with_move(Move::new(pos, pos))));
    }
    mouse_down_state.set(Some(mouse_down));
}

fn handle_on_mouse_up_event(
    cx: Scope<BoardProps>,
    event: Event<MouseData>,
    mouse_down_state: &UseState<Option<MouseClick>>,
    arrows: &UseRef<Arrows>,
    drawing_arrow: &UseRef<Option<ArrowData>>,
    selected_piece: &UseRef<Option<Position>>,
) {
    if let Some(mouse_down) = mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            drop_piece(cx, &event, &mouse_down.point);
        } else if mouse_down.kind.contains(MouseButton::Secondary) {
            complete_arrow(arrows, drawing_arrow);
        }
        mouse_down_state.set(None);
        selected_piece.set(None);
    }
}

fn handle_on_mouse_move_event(
    cx: Scope<BoardProps>,
    event: Event<MouseData>,
    mouse_down_state: &UseState<Option<MouseClick>>,
    drawing_arrow: &UseRef<Option<ArrowData>>,
) {
    if let Some(mouse_down) = mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            block_on(DRAG_CHANNEL.0.send(ClientPoint::new(
                event.client_coordinates().x - mouse_down.point.x,
                event.client_coordinates().y - mouse_down.point.y,
            )))
            .expect("Failed to send drag offset");
        } else if mouse_down.kind.contains(MouseButton::Secondary) {
            let pos = to_position(cx, &event.client_coordinates());
            if drawing_arrow.read().unwrap().mv.to != pos {
                drawing_arrow.write().as_mut().unwrap().mv.to = pos;
            }
        }
    }
}

pub(crate) fn get_center(pos: &Position, board_size: u32, perspective: Color) -> ClientPoint {
    let mut point = to_point(pos, board_size, perspective);
    point.x += board_size as f64 / 16.0;
    point.y += board_size as f64 / 16.0;
    point
}

pub(crate) fn Board(cx: Scope<BoardProps>) -> Element {
    cx.provide_context(DRAG_CHANNEL.1.clone());

    // hooks
    let game = use_shared_state::<Game>(cx).unwrap();
    let mouse_down_state = use_state::<Option<MouseClick>>(cx, || None);
    let selected_piece = use_ref::<Option<Position>>(cx, || None);
    let arrows = use_ref(cx, Arrows::default);
    let analysis_arrows = use_lock(cx, Arrows::default);
    let drawing_arrow = use_ref::<Option<ArrowData>>(cx, || None);
    let stockfish_process = use_async_lock::<Option<Process>>(cx, || None);

    use_effect(cx, &cx.props.analyze, |analyze| {
        toggle_stockfish(
            analyze.to_owned(),
            stockfish_process.to_owned(),
            game.to_owned(),
            analysis_arrows.to_owned(),
        )
    });
    use_effect(cx, game, |game| {
        on_game_changed(
            game.read().get_fen_str(),
            stockfish_process.to_owned(),
            analysis_arrows.to_owned(),
        )
    });
    use_future(cx, use_shared_state::<GameId>(cx).unwrap(), |game_id| {
        create_game_socket(game.to_owned(), game_id, &MOVE_CHANNEL.1)
    });

    let board_img = get_board_image("qootee");
    let piece_theme = "maestro";
    cx.render(rsx! {
        // div for widget
        div {
            class: "relative z-0",
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: |event| handle_on_mouse_down_event(
                cx,
                event,
                mouse_down_state,
                arrows,
                drawing_arrow,
                selected_piece,
            ),
            onmouseup: |event| handle_on_mouse_up_event(
                cx,
                event,
                mouse_down_state,
                arrows,
                drawing_arrow,
                selected_piece,
            ),
            onmousemove: |event| handle_on_mouse_move_event(cx, event, mouse_down_state, drawing_arrow),
            onkeydown: |event| handle_on_key_down(cx, event, arrows),
            // board
            img {
                src: "{board_img}",
                class: "images inset-0 z-0",
                width: "{cx.props.size}",
                height: "{cx.props.size}"
            }
            // highlight squares
            game.read().get_highlighted_squares_info().into_iter().map(|(pos, class)| {
                rsx! {
                    BoardSquare {
                        class: class,
                        pos: pos,
                        board_size: cx.props.size,
                        perspective: cx.props.perspective,
                    }
                }
            }),
            if !game.read().is_replaying() && selected_piece.read().is_some() {
                rsx! {
                    game.read().get_valid_destinations_for_piece(&selected_piece.read().unwrap()).into_iter().map(|pos| {
                        rsx! {
                            BoardSquare {
                                class: "destination-square".into(),
                                pos: pos,
                                board_size: cx.props.size,
                                perspective: cx.props.perspective,
                            }
                        }
                    })
                }
            }
            // pieces
            game.read().get_pieces().into_iter().map(|(piece, pos)| {
                rsx! {
                    Piece {
                        image: get_piece_image_file(piece_theme, piece),
                        top_left_starting: to_point(&pos, cx.props.size, cx.props.perspective),
                        size: cx.props.size / 8,
                        is_dragging: mouse_down_state.as_ref().map_or(false, |mouse_down| {
                            mouse_down.kind.contains(MouseButton::Primary)
                                && pos == to_position(cx, &mouse_down.point)
                        }),
                    }
                }
            }),
            // arrows
            arrows.read().get().into_iter()
                .chain(analysis_arrows.read().get().into_iter())
                .chain(drawing_arrow.read().into_iter())
                .map(|data| rsx! {
                    Arrow {
                        data: data,
                        board_size: cx.props.size,
                        perspective: cx.props.perspective,
                    }
                })
        }
    })
}
