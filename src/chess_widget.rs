use crate::game::{Game, GameStatus};
use crate::moves::Move;
use crate::pieces::{Color, Piece, Position};
use crate::player::{Player, PlayerKind};

use dioxus::html::{geometry::ClientPoint, input_data::keyboard_types::Key};
use dioxus::prelude::*;
use futures::executor;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use once_cell::sync::Lazy;
use std::sync::RwLock;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use url::Url;

type WriteStream = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
type ReadStream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

const WIDGET_SIZE: u32 = 800;
const GAME_ID: u32 = 1234;
static GAME: Lazy<RwLock<Game>> = Lazy::new(|| RwLock::new(Game::new()));
static SOCKET_CREATED: RwLock<bool> = RwLock::new(false);

fn get_current_player_kind(cx: Scope<ChessWidgetProps>) -> PlayerKind {
    match GAME.read().unwrap().get_current_player() {
        Color::White => cx.props.white_player.kind,
        Color::Black => cx.props.black_player.kind,
    }
}

fn has_remote_player(cx: Scope<ChessWidgetProps>) -> bool {
    [cx.props.white_player.kind, cx.props.black_player.kind].contains(&PlayerKind::Remote)
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

fn create_socket(cx: Scope<'_, ChessWidgetProps>) -> (Option<WriteStream>, Option<ReadStream>) {
    if !*SOCKET_CREATED.read().unwrap() && has_remote_player(cx) {
        let (write, read) = executor::block_on(connect_async(
            Url::parse(&format!("ws://localhost:3000/{GAME_ID}")).unwrap(),
        ))
        .unwrap()
        .0
        .split();
        *SOCKET_CREATED.write().unwrap() = true;
        (Some(write), Some(read))
    } else {
        (None, None)
    }
}

async fn write_to_socket(mut rx: UnboundedReceiver<Move>, write_stream: Option<WriteStream>) {
    if let Some(mut socket) = write_stream {
        while let Some(mv) = rx.next().await {
            println!("Sending move {mv:?}");
            socket
                .send(Message::Text(serde_json::to_string(&mv).unwrap()))
                .await
                .unwrap();
        }
    }
}

async fn read_from_socket(read_stream: Option<ReadStream>, board_state_hash: UseState<u64>) {
    if let Some(mut stream) = read_stream {
        while let Some(message) = stream.next().await {
            let data = message.unwrap().into_text().unwrap();
            let mv: Move = serde_json::from_str(&data).unwrap();
            println!("Got move {mv:?}");
            if GAME.write().unwrap().move_piece(mv.from, mv.to).is_ok() {
                board_state_hash.set(GAME.read().unwrap().get_real_state_hash());
            }
        }
    }
}

#[inline_props]
pub fn ChessWidget(cx: Scope, white_player: Player, black_player: Player) -> Element {
    let mouse_down_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let dragging_point_state: &UseState<Option<ClientPoint>> = use_state(cx, || None);
    let board_state_hash = use_state(cx, || GAME.read().unwrap().get_real_state_hash());
    let dragged_piece_position = mouse_down_state.get().as_ref().map(|p| p.into());
    let (write_stream, read_stream) = create_socket(cx);
    let (pieces, dragged): (Vec<_>, Vec<_>) = (0..8)
        .flat_map(|x| (0..8).map(move |y| Position { x, y }))
        .filter_map(|pos| {
            GAME.read()
                .unwrap()
                .get_piece(&pos)
                .map(|piece| (pos, piece))
        })
        .partition(|(pos, _piece)| Some(*pos) != dragged_piece_position);
    let read_socket = use_coroutine(cx, |_rx: UnboundedReceiver<()>| {
        let board_state_hash = board_state_hash.to_owned();
        read_from_socket(read_stream, board_state_hash)
    });
    let write_socket = use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
        write_to_socket(rx, write_stream)
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
                    if get_current_player_kind(cx) == PlayerKind::Local
                        && GAME.read().unwrap().status != GameStatus::Replay
                        && GAME.write().unwrap().move_piece(from, to).is_ok()
                    {
                        if has_remote_player(cx) {
                            write_socket.send(Move::new(from, to));
                        }
                        board_state_hash.set(GAME.read().unwrap().get_real_state_hash());
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
