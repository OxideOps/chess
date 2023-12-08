use async_std::channel::{unbounded, Receiver, Sender};
use chess::{Color, Game, Move, Piece, PlayerKind, Position};
use dioxus::{
    html::{
        geometry::ElementPoint,
        input_data::{
            keyboard_types::{Key, Modifiers},
            MouseButton,
        },
    },
    prelude::*,
};
use futures::executor::block_on;
use once_cell::sync::Lazy;

use super::super::{
    arrows::{ArrowData, Arrows},
    components::{Arrow, BoardSquare, Piece},
    game_socket::create_game_socket,
    mouse_click::MouseClick,
    shared_states::{Analyze, BoardSize, GameId, Perspective},
    stockfish::{
        core::{on_game_changed, toggle_stockfish},
        interface::Process,
        Eval,
    },
};

pub(crate) type Channel<T> = (Sender<T>, Receiver<T>);

// Channel for sending moves to `game_socket` to be sent to a remote player
static MOVE_CHANNEL: Lazy<Channel<Move>> = Lazy::new(unbounded);
// Channel for telling dragged pieces how far they have been dragged
static DRAG_CHANNEL: Lazy<Channel<ElementPoint>> = Lazy::new(unbounded);

#[derive(Props, PartialEq)]
pub(crate) struct BoardProps {
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
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
    pub(crate) hovered_position: &'a UseState<Option<Position>>,
    pub(crate) board_size: u32,
    pub(crate) perspective: Color,
    pub(crate) selected_squares: &'a UseRef<Vec<Position>>,
}

pub(crate) fn Board(cx: Scope<BoardProps>) -> Element {
    cx.provide_context(DRAG_CHANNEL.1.clone());

    // hooks
    let hooks = BoardHooks {
        eval: use_shared_state::<Eval>(cx)?,
        game: use_shared_state::<Game>(cx)?,
        mouse_down_state: use_state::<Option<MouseClick>>(cx, || None),
        selected_piece: use_ref::<Option<Position>>(cx, || None),
        arrows: use_ref(cx, Arrows::default),
        analysis_arrows: use_lock(cx, Arrows::default),
        drawing_arrow: use_ref::<Option<ArrowData>>(cx, || None),
        stockfish_process: use_async_lock::<Option<Process>>(cx, || None),
        hovered_position: use_state::<Option<Position>>(cx, || None),
        board_size: **use_shared_state::<BoardSize>(cx)?.read(),
        perspective: **use_shared_state::<Perspective>(cx)?.read(),
        selected_squares: use_ref::<Vec<Position>>(cx, Vec::new),
    };

    use_effect(cx, use_shared_state::<Analyze>(cx).unwrap(), |analyze| {
        toggle_stockfish(
            **analyze.read(),
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
            class: "board-container",
            style: "height: {hooks.board_size}px; width: {hooks.board_size}px;",
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: move |event| handle_on_mouse_down_event(&hooks, event),
            onmouseup: move |event| handle_on_mouse_up_event(cx.props, &hooks, event),
            onmousemove: move |event| handle_on_mouse_move_event(&hooks, event),
            onkeydown: move |event| handle_on_key_down(cx, &hooks, event),
            // board
            img {
                src: "{get_board_image(&cx.props.board_theme)}",
                class: "board",
                width: "{hooks.board_size}",
                height: "{hooks.board_size}"
            }
            // highlight squares
            for (pos, class) in get_highlighted_squares_info(cx.props, &hooks) {
                BoardSquare {
                    class: class,
                    position: pos,
                    hovered: *hooks.hovered_position == Some(pos) && is_valid_destination(&hooks, pos),
                    selected: hooks.selected_squares.read().contains(&pos)
                }
            }
            // pieces
            for (piece, pos) in hooks.game.read().get_pieces() {
                Piece {
                    image: get_piece_image_file(&cx.props.piece_theme, piece),
                    top_left_starting: _to_point(&hooks, &pos),
                    is_dragging: hooks.mouse_down_state.as_ref().map_or(false, |mouse_down| {
                        mouse_down.kind.contains(MouseButton::Primary)
                            && pos == _to_position(&hooks, &mouse_down.point)
                    }),
                }
            },
            // arrows
            for data in hooks.arrows.read().get().into_iter()
                .chain(hooks.drawing_arrow.read().into_iter())
            {
                Arrow { data: data }
            }
            // analysis arrows
            for data in hooks.analysis_arrows.read().get().into_iter() {
                Arrow { data: data }
            }
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

fn _to_position(hooks: &BoardHooks, point: &ElementPoint) -> Position {
    match hooks.perspective {
        Color::White => Position {
            x: (8.0 * point.x / hooks.board_size as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / hooks.board_size as f64)).floor() as usize,
        },
        Color::Black => Position {
            x: (8.0 * (1.0 - point.x / hooks.board_size as f64)).floor() as usize,
            y: (8.0 * point.y / hooks.board_size as f64).floor() as usize,
        },
    }
}

fn _to_point(hooks: &BoardHooks, position: &Position) -> ElementPoint {
    to_point(hooks.board_size, hooks.perspective, position)
}

pub(crate) fn to_point(board_size: u32, perspective: Color, position: &Position) -> ElementPoint {
    match perspective {
        Color::White => ElementPoint::new(
            board_size as f64 * position.x as f64 / 8.0,
            board_size as f64 * (7.0 - position.y as f64) / 8.0,
        ),
        Color::Black => ElementPoint::new(
            board_size as f64 * (7.0 - position.x as f64) / 8.0,
            board_size as f64 * position.y as f64 / 8.0,
        ),
    }
}

fn handle_on_key_down(cx: Scope<BoardProps>, hooks: &BoardHooks, event: Event<KeyboardData>) {
    #[cfg(not(feature = "desktop"))]
    let _ = &cx;
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
            #[cfg(feature = "desktop")]
            "q" if event.modifiers() == Modifiers::CONTROL => {
                log::info!("Quitting game..");
                dioxus_desktop::use_window(cx).close();
            }
            _ => log::debug!("{:?} key pressed", c),
        },
        _ => log::debug!("{:?} key pressed", event.key()),
    };
}

fn can_move(props: &BoardProps, hooks: &BoardHooks) -> bool {
    let (current_player_kind, opponent_player_kind) = match hooks.game.read().get_current_player() {
        Color::White => (props.white_player_kind, props.black_player_kind),
        Color::Black => (props.black_player_kind, props.white_player_kind),
    };
    current_player_kind == PlayerKind::Local
        && (!hooks.game.read().is_replaying() || opponent_player_kind == PlayerKind::Local)
}

fn drop_piece(
    props: &BoardProps,
    hooks: &BoardHooks,
    event: &Event<MouseData>,
    point: &ElementPoint,
) {
    let from = _to_position(hooks, point);
    let to = _to_position(hooks, &event.element_coordinates());
    let opponent_player_kind = match hooks.game.read().get_current_player() {
        Color::White => props.black_player_kind,
        Color::Black => props.white_player_kind,
    };
    let mv = Move::new(from, to);
    if can_move(props, hooks) && hooks.game.read().is_move_valid(&mv).is_ok() {
        hooks.game.write().move_piece(from, to).ok();
        if opponent_player_kind == PlayerKind::Remote {
            spawn(async move {
                if let Err(e) = MOVE_CHANNEL.0.send(mv).await {
                    log::error!("Failed to send move: {e}")
                }
            })
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

async fn send_dragging_point(hooks: &BoardHooks<'_>, event: Event<MouseData>) {
    DRAG_CHANNEL
        .0
        .send(ElementPoint::new(
            event.element_coordinates().x - hooks.board_size as f64 / 16.0,
            event.element_coordinates().y - hooks.board_size as f64 / 16.0,
        ))
        .await
        .expect("Failed to send dragging point");
}

fn handle_on_mouse_down_event(hooks: &BoardHooks, event: Event<MouseData>) {
    let mouse_down = MouseClick::from(event.clone());
    if mouse_down.kind.contains(MouseButton::Primary) {
        hooks
            .selected_piece
            .set(Some(_to_position(hooks, &mouse_down.point)));
        block_on(send_dragging_point(hooks, event));
        hooks.arrows.write().clear();
    } else if mouse_down.kind.contains(MouseButton::Secondary) {
        let pos = _to_position(hooks, &mouse_down.point);
        hooks
            .drawing_arrow
            .set(Some(ArrowData::user_arrow(Move::new(pos, pos))));
    }
    hooks.mouse_down_state.set(Some(mouse_down));
}

fn handle_on_mouse_up_event(props: &BoardProps, hooks: &BoardHooks, event: Event<MouseData>) {
    if let Some(mouse_down) = hooks.mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            drop_piece(props, hooks, &event, &mouse_down.point);
        } else if mouse_down.kind.contains(MouseButton::Secondary) {
            let from = _to_position(hooks, &mouse_down.point);
            let to = _to_position(hooks, &event.element_coordinates());
            if from == to {
                if let Some(index) = hooks
                    .selected_squares
                    .read()
                    .iter()
                    .position(|&pos| pos == from)
                {
                    hooks.selected_squares.write().remove(index);
                } else {
                    hooks.selected_squares.write().push(from)
                }
            } else {
                complete_arrow(hooks);
            }
        }
        hooks.mouse_down_state.set(None);
        hooks.selected_piece.set(None);
        hooks.hovered_position.set(None);
    }
}

fn handle_on_mouse_move_event(hooks: &BoardHooks, event: Event<MouseData>) {
    let pos = _to_position(hooks, &event.element_coordinates());
    if let Some(mouse_down) = hooks.mouse_down_state.get() {
        if mouse_down.kind.contains(MouseButton::Primary) {
            block_on(send_dragging_point(hooks, event));
            if hooks.hovered_position.is_none() || hooks.hovered_position.unwrap() != pos {
                hooks.hovered_position.set(Some(pos));
            }
        } else if mouse_down.kind.contains(MouseButton::Secondary)
            && hooks.drawing_arrow.read().unwrap().mv.to != pos
        {
            hooks.drawing_arrow.write().as_mut().unwrap().mv.to = pos;
        }
    }
}

fn is_valid_destination(hooks: &BoardHooks, pos: Position) -> bool {
    let from = hooks.selected_piece.read().unwrap();
    let destinations = hooks.game.read().get_valid_destinations_for_piece(&from);
    destinations.contains(&pos)
}

pub(crate) fn get_center(board_size: u32, perspective: Color, pos: &Position) -> ElementPoint {
    let mut point = to_point(board_size, perspective, pos);
    point.x += board_size as f64 / 16.0;
    point.y += board_size as f64 / 16.0;
    point
}

fn get_highlighted_squares_info(props: &BoardProps, hooks: &BoardHooks) -> Vec<(Position, String)> {
    let game = hooks.game.read();
    let mut info = game.get_highlighted_squares_info();
    if can_move(props, hooks)
        && let Some(pos) = &*hooks.selected_piece.read()
    {
        info.extend(
            game.get_valid_destinations_for_piece(pos)
                .into_iter()
                .map(|pos| (pos, "destination-square".to_string())),
        );
    }
    info
}
