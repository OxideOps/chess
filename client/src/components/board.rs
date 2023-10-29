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
use crate::stockfish::Eval;

pub(crate) type Channel<T> = (Sender<T>, Receiver<T>);

// Channel for sending moves to `game_socket` to be sent to a remote player
static MOVE_CHANNEL: Lazy<Channel<Move>> = Lazy::new(unbounded);
// Channel for telling dragged pieces how far they have been dragged
static DRAG_CHANNEL: Lazy<Channel<ClientPoint>> = Lazy::new(unbounded);

#[derive(Props, PartialEq)]
pub(crate) struct BoardProps {
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
    perspective: Color,
    pub(crate) size: u32,
    analyze: UseState<bool>,
    board_theme: String,
    piece_theme: String,
}

#[derive(Clone, Copy)]
pub(crate) struct BoardHooks<'a> {
    pub(crate) eval: &'a UseSharedState<Eval>,
    pub(crate) game: &'a UseSharedState<Game>,
    pub(crate) mouse_down_state: &'a UseState<Option<MouseClick>>,
    pub(crate) selected_piece: &'a UseRef<Option<Position>>,
    pub(crate) arrows: &'a UseRef<Arrows>,
    pub(crate) analysis_arrows: &'a UseLock<Arrows>,
    pub(crate) drawing_arrow: &'a UseRef<Option<ArrowData>>,
    pub(crate) stockfish_process: &'a UseAsyncLock<Option<Process>>,
}

pub(crate) fn Board(cx: Scope<BoardProps>) -> Element {
    cx.provide_context(DRAG_CHANNEL.1.clone());

    // hooks
    let hooks = BoardHooks {
        eval: use_shared_state::<Eval>(cx).unwrap(),
        game: use_shared_state::<Game>(cx).unwrap(),
        mouse_down_state: use_state::<Option<MouseClick>>(cx, || None),
        selected_piece: use_ref::<Option<Position>>(cx, || None),
        arrows: use_ref(cx, Arrows::default),
        analysis_arrows: use_lock(cx, Arrows::default),
        drawing_arrow: use_ref::<Option<ArrowData>>(cx, || None),
        stockfish_process: use_async_lock::<Option<Process>>(cx, || None),
    };

    use_effect(cx, &cx.props.analyze, |analyze| {
        toggle_stockfish(
            analyze,
            hooks.stockfish_process.to_owned(),
            hooks.game.to_owned(),
            hooks.analysis_arrows.to_owned(),
            hooks.eval.to_owned(),
        )
    });
    use_effect(cx, hooks.game, |game| {
        on_game_changed(
            game.read().get_fen_str(),
            hooks.stockfish_process.to_owned(),
            hooks.analysis_arrows.to_owned(),
        )
    });
    use_future(cx, use_shared_state::<GameId>(cx).unwrap(), |game_id| {
        create_game_socket(hooks.game.to_owned(), game_id, &MOVE_CHANNEL.1)
    });

    cx.render(rsx! {
        // div for widget
        div {
            class: "relative z-0",
            style: "height: {cx.props.size}px; width: {cx.props.size}px;",
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: move |event| handle_on_mouse_down_event(cx.props, &hooks, event),
            onmouseup: move |event| handle_on_mouse_up_event(cx.props, &hooks, event),
            onmousemove: move |event| handle_on_mouse_move_event(cx.props, &hooks, event),
            onkeydown: move |event| handle_on_key_down(cx, &hooks, event),
            // board
            img {
                src: "{get_board_image(&cx.props.board_theme)}",
                class: "images inset-0 z-0",
                width: "{cx.props.size}",
                height: "{cx.props.size}"
            }
            // highlight squares
            hooks.game.read().get_highlighted_squares_info().into_iter().map(|(pos, class)| {
                rsx! {
                    BoardSquare {
                        class: class,
                        top_left: to_point(cx.props, &pos),
                        board_size: cx.props.size,
                    }
                }
            }),
            if (!hooks.game.read().is_replaying() || is_local_game(cx.props))
                && let Some(pos) = &*hooks.selected_piece.read()
            {
                rsx! {
                    hooks.game.read().get_valid_destinations_for_piece(pos).into_iter().map(|pos| {
                        rsx! {
                            BoardSquare {
                                class: "destination-square".into(),
                                top_left: to_point(cx.props, &pos),
                                board_size: cx.props.size,
                            }
                        }
                    })
                }
            }
            // pieces
            hooks.game.read().get_pieces().into_iter().map(|(piece, pos)| {
                rsx! {
                    Piece {
                        image: get_piece_image_file(&cx.props.piece_theme, piece),
                        top_left_starting: to_point(cx.props, &pos),
                        size: cx.props.size / 8,
                        is_dragging: hooks.mouse_down_state.as_ref().map_or(false, |mouse_down| {
                            mouse_down.kind.contains(MouseButton::Primary)
                                && pos == to_position(cx.props, &mouse_down.point)
                        }),
                    }
                }
            }),
            // arrows
            hooks.arrows.read().get().into_iter()
                .chain(hooks.analysis_arrows.read().get().into_iter())
                .chain(hooks.drawing_arrow.read().into_iter())
                .map(|data| rsx! {
                    Arrow {
                        data: data,
                        board_props: &cx.props,
                    }
                })
        }
    })
}

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

// We want the square a dragged piece is considered to be on to be based on the center of
// the piece, not the location of the mouse. This requires offsetting based on the original
// mouse down location
fn get_dragged_piece_position(
    props: &BoardProps,
    mouse_down: &ClientPoint,
    mouse_up: &ClientPoint,
) -> Position {
    let center = get_center(props, &to_position(props, mouse_down));
    to_position(
        props,
        &ClientPoint::new(
            center.x + mouse_up.x - mouse_down.x,
            center.y + mouse_up.y - mouse_down.y,
        ),
    )
}

fn to_position(props: &BoardProps, point: &ClientPoint) -> Position {
    match props.perspective {
        Color::White => Position {
            x: (8.0 * point.x / props.size as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / props.size as f64)).floor() as usize,
        },
        Color::Black => Position {
            x: (8.0 * (1.0 - point.x / props.size as f64)).floor() as usize,
            y: (8.0 * point.y / props.size as f64).floor() as usize,
        },
    }
}

pub(crate) fn to_point(props: &BoardProps, position: &Position) -> ClientPoint {
    match props.perspective {
        Color::White => ClientPoint::new(
            props.size as f64 * position.x as f64 / 8.0,
            props.size as f64 * (7.0 - position.y as f64) / 8.0,
        ),
        Color::Black => ClientPoint::new(
            props.size as f64 * (7.0 - position.x as f64) / 8.0,
            props.size as f64 * position.y as f64 / 8.0,
        ),
    }
}

fn handle_on_key_down(cx: Scope<BoardProps>, hooks: &BoardHooks, event: Event<KeyboardData>) {
    match event.key() {
        Key::ArrowLeft => hooks.game.write().go_back_a_move(),
        Key::ArrowRight => hooks.game.write().go_forward_a_move(),
        Key::ArrowUp => hooks.game.write().resume(),
        Key::ArrowDown => hooks.game.write().go_to_start(),
        Key::Character(c) => match c.as_str() {
            "z" if event.modifiers() == Modifiers::CONTROL => {
                hooks.arrows.write().undo();
            }
            "y" if event.modifiers() == Modifiers::CONTROL => {
                hooks.arrows.write().redo();
            }
            #[cfg(not(target_arch = "wasm32"))]
            "q" if event.modifiers() == Modifiers::CONTROL => {
                dioxus_desktop::use_window(cx).close();
            }
            _ => log::debug!("{:?} key pressed", c),
        },
        _ => log::debug!("{:?} key pressed", event.key()),
    };
}

fn drop_piece(
    props: &BoardProps,
    hooks: &BoardHooks,
    event: &Event<MouseData>,
    point: &ClientPoint,
) {
    let from = to_position(props, point);
    let to = get_dragged_piece_position(props, point, &event.client_coordinates());
    let (current_player_kind, opponent_player_kind) = match hooks.game.read().get_current_player() {
        Color::White => (props.white_player_kind, props.black_player_kind),
        Color::Black => (props.black_player_kind, props.white_player_kind),
    };
    let mv = Move::new(from, to);
    if current_player_kind == PlayerKind::Local
        && (!hooks.game.read().is_replaying() || opponent_player_kind == PlayerKind::Local)
        && hooks.game.read().is_move_valid(&mv).is_ok()
    {
        hooks.game.write().move_piece(from, to).ok();
        if opponent_player_kind == PlayerKind::Remote {
            block_on(MOVE_CHANNEL.0.send(mv)).expect("Failed to send move!");
        }
    }
}

fn complete_arrow(hooks: &BoardHooks) {
    if let Some(arrow_data) = *hooks.drawing_arrow.read() {
        if arrow_data.has_length() {
            hooks.arrows.write().push(arrow_data);
        }
    }
    *hooks.drawing_arrow.write() = None;
}

fn handle_on_mouse_down_event(props: &BoardProps, hooks: &BoardHooks, event: Event<MouseData>) {
    let mouse_down = MouseClick::from(event);
    if mouse_down.kind.contains(MouseButton::Primary) {
        hooks
            .selected_piece
            .set(Some(to_position(props, &mouse_down.point)));
        hooks.arrows.write().clear();
    } else if mouse_down.kind.contains(MouseButton::Secondary) {
        let pos = to_position(props, &mouse_down.point);
        hooks
            .drawing_arrow
            .set(Some(ArrowData::with_move(Move::new(pos, pos))));
    }
    hooks.mouse_down_state.set(Some(mouse_down));
}

fn handle_on_mouse_up_event(props: &BoardProps, hooks: &BoardHooks, event: Event<MouseData>) {
    if let Some(mouse_down) = hooks.mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            drop_piece(props, hooks, &event, &mouse_down.point);
        } else if mouse_down.kind.contains(MouseButton::Secondary) {
            complete_arrow(hooks);
        }
        hooks.mouse_down_state.set(None);
        hooks.selected_piece.set(None);
    }
}

fn handle_on_mouse_move_event(props: &BoardProps, hooks: &BoardHooks, event: Event<MouseData>) {
    if let Some(mouse_down) = hooks.mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            block_on(DRAG_CHANNEL.0.send(ClientPoint::new(
                event.client_coordinates().x - mouse_down.point.x,
                event.client_coordinates().y - mouse_down.point.y,
            )))
            .expect("Failed to send drag offset");
        } else if mouse_down.kind.contains(MouseButton::Secondary) {
            let pos = to_position(props, &event.client_coordinates());
            if hooks.drawing_arrow.read().unwrap().mv.to != pos {
                hooks.drawing_arrow.write().as_mut().unwrap().mv.to = pos;
            }
        }
    }
}

pub(crate) fn get_center(props: &BoardProps, pos: &Position) -> ClientPoint {
    let mut point = to_point(props, pos);
    point.x += props.size as f64 / 16.0;
    point.y += props.size as f64 / 16.0;
    point
}

fn is_local_game(props: &BoardProps) -> bool {
    props.white_player_kind == PlayerKind::Local && props.black_player_kind == PlayerKind::Local
}
