use async_std::channel::Receiver;
use chess::{Game, Move};
use dioxus::prelude::*;
use futures_util::{
    join,
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio_tungstenite_wasm::{connect, Message, Message::Text, Result, WebSocketStream};
use url::Url;

use super::shared_states::GameId;

type WriteStream = SplitSink<WebSocketStream, Message>;
type ReadStream = SplitStream<WebSocketStream>;

pub(super) async fn create_game_socket(
    game: UseSharedState<Game>,
    game_id: UseSharedState<GameId>,
    rx: &Receiver<Move>,
) {
    if let Some(game_id) = game_id.with(|id| **id) {
        match connect_to_socket(game_id).await {
            Ok((write, read)) => {
                join!(read_from_socket(read, &game), write_to_socket(rx, write));
            }
            Err(err) => log::error!("Error connecting game socket: {err:?}"),
        };
    }
}

async fn connect_to_socket(game_id: u32) -> anyhow::Result<(WriteStream, ReadStream)> {
    let url = format!("ws://localhost:8080/game/{game_id}");
    Ok(connect(Url::parse(&url)?).await?.split())
}

async fn send_move(mv: &Move, socket: &mut WriteStream) -> anyhow::Result<()> {
    log::info!("Sending move {mv}");
    Ok(socket.send(Text(serde_json::to_string(mv)?)).await?)
}

async fn write_to_socket(rx: &Receiver<Move>, mut socket: WriteStream) {
    while let Ok(mv) = rx.recv().await {
        if let Err(err) = send_move(&mv, &mut socket).await {
            log::error!("Error sending move: {err:?}");
        }
    }
}

fn handle_message(message: Result<Message>, game: &UseSharedState<Game>) -> anyhow::Result<()> {
    let mv = serde_json::from_str::<Move>(&message?.into_text()?)?;
    log::info!("Got move {mv}");
    game.write().move_piece(mv.from, mv.to)?;
    Ok(())
}

async fn read_from_socket(mut stream: ReadStream, game: &UseSharedState<Game>) {
    while let Some(message) = stream.next().await {
        if let Err(err) = handle_message(message, game) {
            log::error!("Error receiving move: {err:?}");
        }
    }
}
