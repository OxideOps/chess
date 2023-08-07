use crate::arrow::Arrow;
use crate::game_socket::create_game_socket;
use crate::mouse_click::MouseClick;
use crate::ray::Ray;
use chess::color::Color;
use chess::game::Game;
use chess::game_status::GameStatus;
use chess::moves::Move;
use chess::piece::Piece;
use chess::player::PlayerKind;
use chess::position::Position;
use dioxus::html::input_data::MouseButton;
use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;

fn get_piece_image_file(piece: Piece) -> String {
    format!("images/{piece}.png")
}

#[derive(Props, PartialEq)]
pub struct BoardProps<'a> {
    size: u32,
    game: &'a UseRef<Game>,
    #[props(!optional)]
    game_id: Option<u32>,
    white_player_kind: PlayerKind,
    black_player_kind: PlayerKind,
    perspective: Color,
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
        let center = self.snap_center(mouse_down);
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

    fn to_point(&self, position: &Position) -> ClientPoint {
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
        arrows: &UseRef<Vec<Ray>>,
    ) {
        arrows.with_mut(|arrows| {
            arrows.push(Ray {
                from: self.snap_center(&mouse_down),
                to: self.snap_center(&event.client_coordinates()),
            })
        });
    }

    fn handle_on_mouse_up_event(
        &self,
        event: Event<MouseData>,
        mouse_down_state: &UseState<Option<MouseClick>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
        game_socket: Option<&Coroutine<Move>>,
        arrows: &UseRef<Vec<Ray>>,
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

    fn snap_center(&self, point: &ClientPoint) -> ClientPoint {
        let mut point = self.to_point(&self.to_position(point));
        point.x += self.size as f64 / 16.0;
        point.y += self.size as f64 / 16.0;
        point
    }

    pub fn get_current_ray(
        &self,
        mouse_down_state: &UseState<Option<MouseClick>>,
        dragging_point_state: &UseState<Option<ClientPoint>>,
    ) -> Option<Ray> {
        if let Some(mouse_down) = mouse_down_state.get() {
            if mouse_down.kind.contains(MouseButton::Secondary) {
                if let Some(to) = *dragging_point_state.get() {
                    let from = self.snap_center(&mouse_down.point);
                    return Some(Ray { to, from });
                }
            }
        }
        None
    }
}

pub fn Board<'a>(cx: Scope<'a, BoardProps<'a>>) -> Element<'a> {
    let mouse_down_state = use_state::<Option<MouseClick>>(cx, || None);
    let dragging_point_state = use_state::<Option<ClientPoint>>(cx, || None);
    let arrows = use_ref::<Vec<Ray>>(cx, || vec![]);
    let game_socket = cx.props.game_id.map(|game_id| {
        use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
            create_game_socket(cx.props.game.to_owned(), game_id, rx)
        })
    });

    cx.render(rsx! {
        // div for widget
        div {
            class: "relative z-0",
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: |event| mouse_down_state.set(Some(event.into())),
            onmouseup: move |event| cx.props.handle_on_mouse_up_event(event, mouse_down_state, dragging_point_state, game_socket, arrows),
            onmousemove: move |event| cx.props.handle_on_mouse_move_event(event, mouse_down_state, dragging_point_state),
            onkeydown: move |event| cx.props.handle_on_key_down(&event.key()),
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
        },
        // arrows
        arrows.with(|arrows| arrows.to_owned()).iter().map(|ray| {
            rsx! {
                Arrow { ray: *ray, board_size: cx.props.size }
            }
        }),
        if let Some(current_ray) = cx.props.get_current_ray(mouse_down_state, dragging_point_state) {
            rsx! {
                Arrow { ray: current_ray, board_size: cx.props.size }
            }
        }
    })
}
