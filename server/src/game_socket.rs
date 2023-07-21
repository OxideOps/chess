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
    if let Some((sink, stream, connected)) = params {
        *connected.write().await = true;
        while let Some(msg) = stream.lock().await.as_mut().unwrap().next().await {
            if !*connected.read().await {
                break;
            }
            if let Ok(msg) = msg {
                if let Some(sink) = sink.lock().await.as_mut() {
                    sink.send(msg).await.expect("Failed to send move!");
                }
            } else {
                *connected.write().await = false;
                break;
            }
        }
        // if we have been disconnected, clean up our connections
        sink.lock()
            .await
            .as_mut()
            .unwrap()
            .close()
            .await
            .expect("Failed to close socket");
        *sink.lock().await = None;
        *stream.lock().await = None;
    }
}
