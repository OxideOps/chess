use anyhow;
use chess::game::Game;
use chess::moves::Move;
use dioxus::prelude::*;
use futures_util::{
    future::join,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio_tungstenite_wasm::{connect, Message, Message::Text, Result, WebSocketStream};
use url::Url;

type WriteStream = SplitSink<WebSocketStream, Message>;
type ReadStream = SplitStream<WebSocketStream>;

const GAME_ID: u32 = 1234;

pub async fn create_game_socket(game: UseRef<Game>, rx: UnboundedReceiver<Move>) {
    if let Err(err) = handle_socket(&game, rx).await {
        log::error!("create_game_socket: {err:?}");
    }
}

async fn handle_socket(game: &UseRef<Game>, rx: UnboundedReceiver<Move>) -> anyhow::Result<()> {
    let url = format!("ws://muddy-fog-684.fly.dev/game/{GAME_ID}");
    let (write, read) = connect(Url::parse(&url)?).await?.split();
    join(read_from_socket(read, game), write_to_socket(rx, write)).await;
    Ok(())
}

async fn send_move(mv: &Move, socket: &mut WriteStream) -> anyhow::Result<()> {
    log::info!("Sending move {mv:?}");
    socket.send(Text(serde_json::to_string(mv)?)).await?;
    Ok(())
}

async fn write_to_socket(mut rx: UnboundedReceiver<Move>, mut socket: WriteStream) {
    while let Some(mv) = rx.next().await {
        if let Err(err) = send_move(&mv, &mut socket).await {
            log::error!("write_to_socket: {err:?}");
        }
    }
}

fn handle_message(message: Result<Message>, game: &UseRef<Game>) -> anyhow::Result<()> {
    let mv = serde_json::from_str::<Move>(&message?.into_text()?)?;
    log::info!("Got move {mv:?}");
    game.with_mut(|game| game.move_piece(mv.from, mv.to))?;
    Ok(())
}

async fn read_from_socket(mut stream: ReadStream, game: &UseRef<Game>) {
    while let Some(message) = stream.next().await {
        if let Err(err) = handle_message(message, &game) {
            log::error!("read_from_socket: {err:?}");
        }
    }
}
