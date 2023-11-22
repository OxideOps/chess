use std::sync::Arc;

use axum::{extract::WebSocketUpgrade, response::Response};
use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;

use crate::server::server_functions::games::*;

pub async fn handler(game_id: u32, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| async move {
        if let Some(connections) = GAMES.read().await.get(&game_id) {
            let mut connections = connections.lock().await;
            if connections.len() < 2 {
                let (send, recv) = socket.split();
                connections.push((Arc::new(Mutex::new(send)), Arc::new(Mutex::new(recv))));
                if connections.len() == 2 {
                    let (send1, recv1) = connections[0].clone();
                    let (send2, recv2) = connections[1].clone();
                    tokio::spawn(forward_messages(game_id, send1, recv2));
                    tokio::spawn(forward_messages(game_id, send2, recv1));
                }
            } else {
                log::warn!("Cannot connect client to socket. Already 2 clients.");
            }
        } else {
            log::warn!("Cannot connect client to socket. Game id does not exist.");
        }
    })
}

async fn close_socket(game_id: u32, send: WebSocketSender) {
    if let Err(err) = send.lock().await.close().await {
        log::error!("Error closing socket: {err:?}");
    }
    let mut pending_game = PENDING_GAME.lock().await;
    if *pending_game == Some(game_id) {
        *pending_game = None;
    }
    GAMES.write().await.remove(&game_id);
}

async fn game_exists(game_id: u32) -> bool {
    GAMES.read().await.contains_key(&game_id)
}

async fn forward_messages(game_id: u32, send: WebSocketSender, recv: WebSocketReceiver) {
    // forward messages
    while let Some(msg) = recv.lock().await.next().await {
        if !game_exists(game_id).await {
            log::info!("Game has ended. Closing socket.");
            break;
        }
        if let Ok(msg) = msg {
            if let Err(err) = send.lock().await.send(msg).await {
                log::error!("Error forwarding message to other client: {err:?}");
            }
        } else {
            log::error!("Closing socket due to error from one of the clients: {msg:?}");
            break;
        }
    }
    // if we have been disconnected, clean up our connections
    close_socket(game_id, send).await;
}
