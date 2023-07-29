use anyhow;
use async_std::sync::RwLock;
use chess::game::Game;
use chess::moves::Move;
use dioxus::prelude::*;
use futures::executor;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio_tungstenite_wasm::{connect, Message, Message::Text, Result, WebSocketStream};
use url::Url;

use crate::widget::WidgetProps;

type WriteStream = SplitSink<WebSocketStream, Message>;
type ReadStream = SplitStream<WebSocketStream>;

const GAME_ID: u32 = 1234;
static SOCKET_CREATED: RwLock<bool> = RwLock::new(false);

async fn init_streams() -> anyhow::Result<(Option<WriteStream>, Option<ReadStream>)> {
    if *SOCKET_CREATED.read().await {
        return Ok((None, None));
    }
    let url = format!("ws://muddy-fog-684.fly.dev/game/{GAME_ID}");
    let (write, read) = connect(Url::parse(&url)?).await?.split();
    *SOCKET_CREATED.write().await = true;
    Ok((Some(write), Some(read)))
}

async fn send_move(mv: &Move, socket: &mut WriteStream) -> anyhow::Result<()> {
    log::info!("Sending move {mv:?}");
    socket.send(Text(serde_json::to_string(mv)?)).await?;
    Ok(())
}

async fn write_to_socket(mut rx: UnboundedReceiver<Move>, write_stream: Option<WriteStream>) {
    if let Some(mut socket) = write_stream {
        while let Some(mv) = rx.next().await {
            if let Err(err) = send_move(&mv, &mut socket).await {
                log::error!("write_to_socket: {err:?}");
            }
        }
    }
}

fn handle_message(message: Result<Message>, game: &UseRef<Game>) -> anyhow::Result<()> {
    let mv = serde_json::from_str::<Move>(&message?.into_text()?)?;
    log::info!("Got move {mv:?}");
    game.with_mut(|game| game.move_piece(mv.from, mv.to))?;
    Ok(())
}

async fn read_from_socket(read_stream: Option<ReadStream>, game: UseRef<Game>) {
    if let Some(mut stream) = read_stream {
        while let Some(message) = stream.next().await {
            if let Err(err) = handle_message(message, &game) {
                log::error!("read_from_socket: {err:?}");
            }
        }
    }
}

pub fn create_game_socket<'a>(
    cx: Scope<'a, WidgetProps>,
    game: &UseRef<Game>,
) -> &'a Coroutine<Move> {
    let (write_stream, read_stream) = match executor::block_on(init_streams()) {
        Ok(streams) => streams,
        Err(err) => {
            log::error!("create_game_socket: {err:?}");
            (None, None)
        }
    };
    use_coroutine(cx, |_rx: UnboundedReceiver<()>| {
        read_from_socket(read_stream, game.to_owned())
    });
    use_coroutine(cx, |rx: UnboundedReceiver<Move>| {
        write_to_socket(rx, write_stream)
    })
}
