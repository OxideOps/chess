use crate::game_socket::create_game_socket;
use chess::color::Color;
use chess::game::Game;
use chess::game_status::GameStatus;
use chess::moves::Move;
use chess::piece::Piece;
use chess::player::PlayerKind;
use chess::position::Position;
use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;

fn get_piece_image_file(piece: Piece) -> String {
    format!("images/{piece}.png")
}

#[derive(Props, PartialEq)]
pub struct BoardProps<'a> {
    size: u32,
    game: &'a UseRef<Game>,
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
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
        let top_left = self.to_point(&self.to_position(mouse_down));
        self.to_position(&ClientPoint::new(
            top_left.x + mouse_up.x - mouse_down.x + self.size as f64 / 16.0,
            top_left.y + mouse_up.y - mouse_down.y + self.size as f64 / 16.0,
        ))
    }

    fn get_positions(
        &self,
        pos: &Position,
        mouse_down_state: &Option<ClientPoint>,
        dragging_point_state: &Option<ClientPoint>,
    ) -> (ClientPoint, usize) {
        let mut top_left = self.to_point(pos);
        let mut z_index = 0;
        if let Some(mouse_down) = mouse_down_state {
            if let Some(dragging_point) = dragging_point_state {
                if *pos == self.to_position(mouse_down) {
                    z_index = 1;
                    top_left.x += dragging_point.x - mouse_down.x;
                    top_left.y += dragging_point.y - mouse_down.y;
                }
            }
        }
        (top_left, z_index)
    }

    fn to_position(&self, point: &ClientPoint) -> Position {
        Position {
            x: (8.0 * point.x / self.size as f64).floor() as usize,
            y: (8.0 * (1.0 - point.y / self.size as f64)).floor() as usize,
        }
    }

    fn to_point(&self, position: &Position) -> ClientPoint {
        ClientPoint {
            x: self.size as f64 * position.x as f64 / 8.0,
            y: self.size as f64 * (7.0 - position.y as f64) / 8.0,
            ..Default::default()
        }
    }

    fn handle_on_key_down(&self, key: &Key) {
        self.game.with_mut(|game| {
            match key {
                Key::ArrowLeft => game.go_back_a_move(),
                Key::ArrowRight => game.go_forward_a_move(),
                Key::ArrowUp => game.resume(),
                Key::ArrowDown => game.go_to_start(),
                _ => log::debug!("{:?} key pressed", key),
            };
        })
    }

    fn handle_on_mouse_up_event(
        &self,
        event: Event<MouseData>,
        mouse_down_state: &UseState<Option<ClientPoint>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
        game_socket: Option<&Coroutine<Move>>,
    ) {
        if let Some(mouse_down) = mouse_down_state.get() {
            let from = self.to_position(mouse_down);
            let to = self.get_dragged_piece_position(mouse_down, &event.client_coordinates());
            let current_player_kind = match self.game.with(|game| game.get_current_player()) {
                Color::White => self.white_player_kind,
                Color::Black => self.black_player_kind,
            };
            if current_player_kind == PlayerKind::Local
                && self.game.with(|game| game.status) != GameStatus::Replay
                && self.game.with_mut(|game| game.move_piece(from, to).is_ok())
            {
                if let Some(game_socket) = game_socket {
                    game_socket.send(Move::new(from, to));
                }
            }
            mouse_down_state.set(None);
            dragging_point_state.set(None);
        }
    }

    fn handle_on_mouse_move_event(
        &self,
        event: Event<MouseData>,
        mouse_down_state: &UseState<Option<ClientPoint>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
    ) {
        if let Some(mouse_down) = mouse_down_state.get() {
            if self
                .game
                .with(|game| game.has_piece(&self.to_position(mouse_down)))
            {
                dragging_point_state.set(Some(event.client_coordinates()));
            }
        }
    }

    pub fn has_remote_player(&self) -> bool {
        [self.white_player_kind, self.black_player_kind].contains(&PlayerKind::Remote)
    }
}

pub fn Board<'a>(cx: Scope<'a, BoardProps<'a>>) -> Element<'a> {
    let mouse_down_state = use_state::<Option<ClientPoint>>(cx, || None);
    let dragging_point_state = use_state::<Option<ClientPoint>>(cx, || None);
    let game_socket = if cx.props.has_remote_player() {
        Some(use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
            create_game_socket(cx.props.game.to_owned(), rx)
        }))
    } else {
        None
    };

    cx.render(rsx! {
        style { include_str!("../../styles/widget.css") }
        // div for widget
        div {
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: move |event| cx.props.handle_on_mouse_up_event(event, mouse_down_state, dragging_point_state, game_socket),
            onmousemove: move |event| cx.props.handle_on_mouse_move_event(event, mouse_down_state, dragging_point_state),
            onkeydown: move |event| cx.props.handle_on_key_down(&event.key()),
            //board
            img {
                src: "images/board.png",
                class: "images",
                style: "left: 0; top: 0;",
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
        }
    })
}
