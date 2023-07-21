use crate::chess_widget::ChessWidgetProps;

use chess::game::Game;
use chess::moves::Move;
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

const GAME_ID: u32 = 1234;
static SOCKET_CREATED: RwLock<bool> = RwLock::new(false);

fn init_streams() -> (Option<WriteStream>, Option<ReadStream>) {
    if !*SOCKET_CREATED.read().unwrap() {
        let (write, read) = executor::block_on(connect_async(
            Url::parse(&format!("ws://muddy-fog-684.fly.dev/game/{GAME_ID}")).unwrap(),
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
            log::info!("Sending move {mv:?}");
            socket
                .send(Message::Text(serde_json::to_string(&mv).unwrap()))
                .await
                .unwrap();
        }
    }
}

async fn read_from_socket(
    read_stream: Option<ReadStream>,
    board_state_hash: UseState<u64>,
    game: &'static Lazy<RwLock<Game>>,
) {
    if let Some(mut stream) = read_stream {
        while let Some(message) = stream.next().await {
            let data = message.unwrap().into_text().unwrap();
            let mv: Move =
                serde_json::from_str(&data).expect("Failed to read move from remote player.");
            log::info!("Got move {mv:?}");
            if game.write().unwrap().move_piece(mv.from, mv.to).is_ok() {
                board_state_hash.set(game.read().unwrap().get_real_state_hash());
            }
        }
    }
}

pub fn create_game_socket<'a>(
    cx: &'a Scoped<'a, ChessWidgetProps>,
    board_state_hash: &UseState<u64>,
    game: &'static Lazy<RwLock<Game>>,
) -> Option<&'a Coroutine<Move>> {
    let (write_stream, read_stream) = init_streams();
    use_coroutine(cx, |_rx: UnboundedReceiver<()>| {
        let board_state_hash = board_state_hash.to_owned();
        read_from_socket(read_stream, board_state_hash, game)
    });
    Some(use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
        write_to_socket(rx, write_stream)
    }))
}
