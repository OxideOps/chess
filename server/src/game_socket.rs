use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use server_functions::*;

pub async fn handler(game_id: u32, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| async move {
        if let Some(connections) = GAMES.read().await.get(&game_id) {
            if let Some(index) = store_socket(socket, connections.clone()).await {
                let other_index = (index + 1) % 2;
                let sink = connections[other_index].0.clone();
                let stream = connections[index].1.clone();
                tokio::spawn(forward_messages(game_id, sink, stream));
            } else {
                log::warn!("Cannot connect client to socket. Already 2 clients.");
            }
        } else {
            log::warn!("Cannot connect client to socket. Game id does not exist.");
        }
    })
}

async fn store_socket(socket: WebSocket, connections: PlayerConnections) -> Option<usize> {
    let index = if connections[0].0.read().await.is_none() {
        Some(0)
    } else if connections[1].0.read().await.is_none() {
        Some(1)
    } else {
        None
    };
    if let Some(index) = index {
        let (sink, stream) = socket.split();
        *connections[index].0.write().await = Some(sink);
        *connections[index].1.write().await = Some(stream);
    }
    index
}

async fn close_socket(game_id: u32, send: Send, recv: Recv) {
    if let Some(send) = send.write().await.as_mut() {
        if let Err(err) = send.close().await {
            log::error!("Error closing socket: {err:?}");
        }
    }
    let mut pending_game = PENDING_GAME.write().await;
    if *pending_game == Some(game_id) {
        *pending_game = None;
    }
    GAMES.write().await.remove(&game_id);
    *send.write().await = None;
    *recv.write().await = None;
}

async fn game_exists(game_id: u32) -> bool {
    GAMES.read().await.contains_key(&game_id)
}

async fn forward_messages(game_id: u32, send: Send, recv: Recv) {
    if let Some(recv) = recv.write().await.as_mut() {
        // forward messages
        while let Some(msg) = recv.next().await {
            if !game_exists(game_id).await {
                log::info!("Game has ended. Closing socket.");
                break;
            }
            if let Ok(msg) = msg {
                if let Some(send) = send.write().await.as_mut() {
                    if let Err(err) = send.send(msg).await {
                        log::error!("Error forwarding message to other client: {err:?}");
                    }
                } else {
                    log::warn!("No other client socket to forward messages to!");
                }
            } else {
                log::error!("Closing socket due to error from one of the clients: {msg:?}");
                break;
            }
        }
    } else {
        log::error!("Socket that we should have just created does not exist. If we get here there is a bug.");
    }
    // if we have been disconnected, clean up our connections
    close_socket(game_id, send, recv).await;
}
