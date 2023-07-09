use crate::game::Game;
use crate::moves::Move;
use crate::pieces::{Color, Piece, Position};
use crate::player::{Player, PlayerKind};

use anyhow::Result;
use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use std::sync::RwLock;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message::Text;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use url::Url;

type ReadStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const WIDGET_SIZE: u32 = 800;
const GAME_ID: u32 = 1234;
static GAME: Lazy<RwLock<Game>> = Lazy::new(|| RwLock::new(Game::new()));

fn get_current_player_kind(cx: Scope<ChessWidgetProps>) -> PlayerKind {
    match GAME.read().unwrap().get_current_player() {
        Color::White => cx.props.white_player.kind,
        Color::Black => cx.props.black_player.kind,
    }
}

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
    let top_left = ClientPoint::from(&mouse_down.into());
    (&ClientPoint::new(
        top_left.x + mouse_up.x - mouse_down.x + WIDGET_SIZE as f64 / 16.0,
        top_left.y + mouse_up.y - mouse_down.y + WIDGET_SIZE as f64 / 16.0,
    ))
        .into()
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

fn draw_piece<'a>(
    piece: Piece,
    pos: &Position,
    mouse_down_state: &Option<ClientPoint>,
    dragging_point_state: &Option<ClientPoint>,
) -> LazyNodes<'a, 'static> {
    let mut top_left = ClientPoint::from(pos);
    if let Some(mouse_down) = mouse_down_state {
        if let Some(dragging_point) = dragging_point_state {
            if *pos == mouse_down.into() {
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

async fn listen_for_remote_moves(mut read: ReadStream) {
    while let Some(message) = read.next().await {
        let data = message.unwrap().into_text().unwrap();
    }
}

#[inline_props]
pub fn ChessWidget(cx: Scope, white_player: Player, black_player: Player) -> Element {
    let mouse_down_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let dragging_point_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let dragged_piece_position = mouse_down_state.get().as_ref().map(|p| p.into());
    let (pieces, dragged): (Vec<_>, Vec<_>) = (0..8)
        .flat_map(|x| (0..8).map(move |y| Position { x, y }))
        .filter_map(|pos| {
            GAME.read()
                .unwrap()
                .get_piece(&pos)
                .map(|piece| (pos, piece))
        })
        .partition(|(pos, _piece)| Some(*pos) != dragged_piece_position);
    let game_socket = use_coroutine(cx, |mut rx: UnboundedReceiver<Move>| async move {
        let (ws_stream, _) = connect_async(Url::parse(&format!("ws://localhost:3000/ws")).unwrap())
            .await
            .unwrap();
        let (mut write, mut read) = ws_stream.split();

        tokio::spawn(listen_for_remote_moves(read));

        while let Some(mv) = rx.next().await {
            println!("{mv:?}");
            write
                .send(Text(serde_json::to_string(&mv).unwrap()))
                .await
                .unwrap();
        }
    });

    render! {
        style { include_str!("../styles/chess_widget.css") }
        div {
            autofocus: true,
            tabindex: 0,

            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    let from = mouse_down.into();
                    let to = get_dragged_piece_position(mouse_down, &event.client_coordinates());
                    if get_current_player_kind(cx) == PlayerKind::Local {
                        GAME.write().unwrap().move_piece(from, to).map(|_| {
                            game_socket.send(Move::new(from, to));
                        }).ok();
                    }
                    mouse_down_state.set(None);
                    dragging_point_state.set(None);
                }
            },
            onmousemove: |event| {
                if let Some(mouse_down) = mouse_down_state.get() {
                    if GAME.read().unwrap().has_piece(&mouse_down.into()) {
                        dragging_point_state.set(Some(event.client_coordinates()));
                    }
                }
            },
            onkeydown: |event| {
                match event.key() {
                    Key::ArrowLeft => {
                        GAME.write().unwrap().go_back_a_turn();
                    },
                    Key::ArrowRight => {
                        GAME.write().unwrap().go_forward_a_turn()
                    },
                    Key::ArrowUp => {
                        GAME.write().unwrap().resume()
                    },
                    Key::ArrowDown => {
                        GAME.write().unwrap().go_to_beginning();
                    }
                    _ => {
                        println!("Functionality not implemented for key: {:?}", event.key())
                    }
                };
                cx.needs_update()
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
