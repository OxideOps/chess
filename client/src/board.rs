use crate::arrow::Arrow;
use crate::arrows::{ArrowData, Arrows};
use crate::game_socket::create_game_socket;
use crate::mouse_click::MouseClick;
use crate::stockfish_client::{run_stockfish, update_analysis_arrows, update_position};
use async_process::Child;
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
use thread_priority::{set_current_thread_priority, ThreadPriority};

fn get_piece_image_file(piece: Piece) -> String {
    format!("images/{piece}.png")
}

#[derive(Props, PartialEq)]
pub struct BoardProps<'a> {
    pub(crate) size: u32,
    game: &'a UseRef<Game>,
    #[props(!optional)]
    game_id: Option<u32>,
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
    perspective: Color,
    analyze: &'a UseState<bool>,
}

impl BoardProps<'_> {
    // We want the square a dragged piece is considered to be on to be based on the center of
    // the piece, not the location of the mouse. This requires offsetting based on the original
    // mouse down location
    fn get_dragged_piece_position(
        &self,
        mouse_down: &ClientPoint,
        mouse_up: &ClientPoint,
    ) -> Position {
        let center = self.get_center(&self.to_position(mouse_down));
        self.to_position(&ClientPoint::new(
            center.x + mouse_up.x - mouse_down.x,
            center.y + mouse_up.y - mouse_down.y,
        ))
    }

    fn get_positions(
        &self,
        pos: &Position,
        mouse_down_state: &Option<MouseClick>,
        dragging_point_state: &Option<ClientPoint>,
    ) -> (ClientPoint, usize) {
        let mut top_left = self.to_point(pos);
        let mut z_index = 1;
        if let Some(mouse_down) = mouse_down_state {
            if mouse_down.kind == MouseButton::Primary {
                if let Some(dragging_point) = dragging_point_state {
                    if *pos == self.to_position(&mouse_down.point) {
                        z_index = 2;
                        top_left.x += dragging_point.x - mouse_down.point.x;
                        top_left.y += dragging_point.y - mouse_down.point.y;
                    }
                }
            }
        }
        (top_left, z_index)
    }

    fn to_position(&self, point: &ClientPoint) -> Position {
        match self.perspective {
            Color::White => Position {
                x: (8.0 * point.x / self.size as f64).floor() as usize,
                y: (8.0 * (1.0 - point.y / self.size as f64)).floor() as usize,
            },
            Color::Black => Position {
                x: (8.0 * (1.0 - point.x / self.size as f64)).floor() as usize,
                y: (8.0 * point.y / self.size as f64).floor() as usize,
            },
        }
    }

    pub(crate) fn to_point(&self, position: &Position) -> ClientPoint {
        match self.perspective {
            Color::White => ClientPoint {
                x: self.size as f64 * position.x as f64 / 8.0,
                y: self.size as f64 * (7.0 - position.y as f64) / 8.0,
                ..Default::default()
            },
            Color::Black => ClientPoint {
                x: self.size as f64 * (7.0 - position.x as f64) / 8.0,
                y: self.size as f64 * position.y as f64 / 8.0,
                ..Default::default()
            },
        }
    }

    fn handle_on_key_down(&self, event: Event<KeyboardData>, arrows: &UseRef<Arrows>) {
        match event.key() {
            Key::ArrowLeft => self.game.with_mut(|game| game.go_back_a_move()),
            Key::ArrowRight => self.game.with_mut(|game| game.go_forward_a_move()),
            Key::ArrowUp => self.game.with_mut(|game| game.resume()),
            Key::ArrowDown => self.game.with_mut(|game| game.go_to_start()),
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
        &self,
        event: &Event<MouseData>,
        point: &ClientPoint,
        game_socket: Option<&Coroutine<Move>>,
    ) {
        let from = self.to_position(point);
        let to = self.get_dragged_piece_position(point, &event.client_coordinates());
        let current_player_kind = match self.game.with(|game| game.get_current_player()) {
            Color::White => self.white_player_kind,
            Color::Black => self.black_player_kind,
        };
        if current_player_kind == PlayerKind::Local
            && self.game.with(|game| {
                game.status != GameStatus::Replay
                    && game.is_move_valid(&Move::new(from, to)).is_ok()
            })
        {
            self.game.with_mut(|game| game.move_piece(from, to).ok());
            if let Some(game_socket) = game_socket {
                game_socket.send(Move::new(from, to));
            }
        }
    }

    fn complete_arrow(
        &self,
        event: &Event<MouseData>,
        mouse_down: &ClientPoint,
        arrows: &UseRef<Arrows>,
    ) {
        let from = self.to_position(mouse_down);
        let to = self.to_position(&event.client_coordinates());
        if to != from {
            arrows.with_mut(|arrows| arrows.push(ArrowData::with_move(Move { from, to })));
        }
    }

    fn handle_on_mouse_down_event(
        &self,
        event: Event<MouseData>,
        mouse_down_state: &UseState<Option<MouseClick>>,
        arrows: &UseRef<Arrows>,
    ) {
        let mouse_down = MouseClick::from(event);
        if mouse_down.kind.contains(MouseButton::Primary) && !*self.analyze.get() {
            arrows.with_mut(|arrows| arrows.clear());
        }
        mouse_down_state.set(Some(mouse_down));
    }

    fn handle_on_mouse_up_event(
        &self,
        event: Event<MouseData>,
        mouse_down_state: &UseState<Option<MouseClick>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
        game_socket: Option<&Coroutine<Move>>,
        arrows: &UseRef<Arrows>,
    ) {
        if let Some(mouse_down) = mouse_down_state.get() {
            if mouse_down.kind.contains(MouseButton::Primary) {
                self.drop_piece(&event, &mouse_down.point, game_socket);
            }
            if mouse_down.kind.contains(MouseButton::Secondary) {
                self.complete_arrow(&event, &mouse_down.point, arrows);
            }
            mouse_down_state.set(None);
        }
        dragging_point_state.set(None);
    }

    fn handle_on_mouse_move_event(
        &self,
        event: Event<MouseData>,
        mouse_down_state: &UseState<Option<MouseClick>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
    ) {
        if mouse_down_state.get().is_some() {
            dragging_point_state.set(Some(event.client_coordinates()));
        }
    }

    pub fn has_remote_player(&self) -> bool {
        [self.white_player_kind, self.black_player_kind].contains(&PlayerKind::Remote)
    }

    pub fn get_center(&self, pos: &Position) -> ClientPoint {
        let mut point = self.to_point(pos);
        point.x += self.size as f64 / 16.0;
        point.y += self.size as f64 / 16.0;
        point
    }

    pub fn get_move_for_arrow(
        &self,
        mouse_down_state: &UseState<Option<MouseClick>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
    ) -> Option<Move> {
        if let Some(mouse_down) = mouse_down_state.get() {
            if mouse_down.kind.contains(MouseButton::Secondary) {
                if let Some(dragging_point) = *dragging_point_state.get() {
                    let from = self.to_position(&mouse_down.point);
                    let to = self.to_position(&dragging_point);
                    return Some(Move { to, from });
                }
            }
        }
        None
    }
}

async fn toggle_stockfish(
    analyze: UseState<bool>,
    stockfish_process: UseRef<Option<Child>>,
    arrows: UseRef<Arrows>,
    player: Color,
) {
    if *analyze.get() {
        match run_stockfish().await {
            Ok(child) => {
                stockfish_process.set(Some(child));
                update_analysis_arrows(
                    &arrows,
                    stockfish_process
                        .with_mut(|process| process.as_mut().unwrap().stdout.take().unwrap()),
                    player,
                )
                .await;
            }
            Err(err) => log::error!("Failed to start stockfish: {err:?}"),
        }
    } else {
        stockfish_process.with_mut(|option| {
            if let Some(process) = option {
                log::info!("Stopping Stockfish");
                process.kill().expect("Failed to kill stockfish process");
                *option = None;
            }
        })
    }
}

pub fn Board<'a>(cx: Scope<'a, BoardProps<'a>>) -> Element<'a> {
    set_current_thread_priority(ThreadPriority::Max).ok();
    let mouse_down_state = use_state::<Option<MouseClick>>(cx, || None);
    let dragging_point_state = use_state::<Option<ClientPoint>>(cx, || None);
    let arrows = use_ref(cx, Arrows::default);
    let stockfish_process = use_ref::<Option<Child>>(cx, || None);
    let game_socket = cx.props.game_id.map(|game_id| {
        use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
            create_game_socket(cx.props.game.to_owned(), game_id, rx)
        })
    });
    use_effect(cx, cx.props.analyze, |analyze| {
        toggle_stockfish(
            analyze.to_owned(),
            stockfish_process.to_owned(),
            arrows.to_owned(),
            cx.props.game.with(|game| game.get_current_player()),
        )
    });
    use_effect(cx, (cx.props.game, cx.props.analyze), |(game, _)| {
        update_position(
            game.with(|game| game.get_fen_str()),
            stockfish_process.to_owned(),
        )
    });

    cx.render(rsx! {
        // div for widget
        div {
            class: "relative z-0",
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: |event| cx.props.handle_on_mouse_down_event(event, mouse_down_state, arrows),
            onmouseup: move |event| cx.props.handle_on_mouse_up_event(event, mouse_down_state, dragging_point_state, game_socket, arrows),
            onmousemove: move |event| cx.props.handle_on_mouse_move_event(event, mouse_down_state, dragging_point_state),
            onkeydown: move |event| cx.props.handle_on_key_down(event, arrows),
            // board
            img {
                src: "images/board.png",
                class: "images inset-0 z-0",
                width: "{cx.props.size}",
                height: "{cx.props.size}",
            },
            // pieces
            cx.props.game.with(|game| game.get_pieces()).into_iter().map(|(piece, pos)| {
                let (top_left, z_index) = cx.props.get_positions(&pos, mouse_down_state, dragging_point_state);
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
            arrows.with(|arrows| arrows.get()).into_iter().map(|data| {
                rsx! {
                    Arrow { data: data, board_props: cx.props }
                }
            }),
            if let Some(current_mv) = cx.props.get_move_for_arrow(mouse_down_state, dragging_point_state) {
                rsx! {
                    Arrow { data: ArrowData::with_move(current_mv), board_props: cx.props }
                }
            }
        },
    })
}
