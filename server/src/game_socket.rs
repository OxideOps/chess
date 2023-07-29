use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::Response,
};
use futures::{
    executor::block_on,
    stream::{SplitSink, SplitStream},
    {SinkExt, StreamExt},
};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

pub type WriteStream = Arc<Mutex<Option<SplitSink<WebSocket, Message>>>>;
pub type ReadStream = Arc<Mutex<Option<SplitStream<WebSocket>>>>;
pub type PlayerConnections = Arc<[(WriteStream, ReadStream); 2]>;

pub async fn handler(
    ws: WebSocketUpgrade,
    connections: PlayerConnections,
    connected: Arc<RwLock<bool>>,
) -> Response {
    ws.on_upgrade(move |socket| {
        if let Some(index) = block_on(store_socket(socket, connections.clone())) {
            let other_index = (index + 1) % 2;
            let sink = connections[other_index].0.clone();
            let stream = connections[index].1.clone();
            handle_socket(Some((sink, stream, connected.clone())))
        } else {
            log::warn!("Cannot connect client to socket. Already 2 clients.");
            handle_socket(None)
        }
    })
}

async fn store_socket(socket: WebSocket, connections: PlayerConnections) -> Option<usize> {
    let index = if connections[0].0.lock().await.is_none() {
        Some(0)
    } else if connections[1].0.lock().await.is_none() {
        Some(1)
    } else {
        None
    };
    if let Some(index) = index {
        let (sink, stream) = socket.split();
        *connections[index].0.lock().await = Some(sink);
        *connections[index].1.lock().await = Some(stream);
    }
    index
}

async fn handle_socket(params: Option<(WriteStream, ReadStream, Arc<RwLock<bool>>)>) {
    if let Some((write_stream, read_stream, connected)) = params {
        *connected.write().await = true;
        if let Some(read_stream) = read_stream.lock().await.as_mut() {
            while let Some(msg) = read_stream.next().await {
                if !*connected.read().await {
                    log::info!("A player has disconnected from the socket, closing socket.");
                    break;
                }
                if let Ok(msg) = msg {
                    if let Some(write_stream) = write_stream.lock().await.as_mut() {
                        if let Err(err) = write_stream.send(msg).await {
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
        if let Some(write_stream) = write_stream.lock().await.as_mut() {
            if let Err(err) = write_stream.close().await {
                log::error!("Error closing socket: {err:?}");
            }
        }
        *connected.write().await = false;
        *write_stream.lock().await = None;
        *read_stream.lock().await = None;
    }
}
