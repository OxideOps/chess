use std::{collections::HashMap, sync::Arc};

use axum::extract::ws::{Message, WebSocket};
use futures::stream::{SplitSink, SplitStream};
use once_cell::sync::Lazy;
use tokio::sync::{Mutex, RwLock};

pub type WebSocketSender = Arc<Mutex<SplitSink<WebSocket, Message>>>;
pub type WebSocketReceiver = Arc<Mutex<SplitStream<WebSocket>>>;
pub type PlayerConnections = Arc<Mutex<Vec<(WebSocketSender, WebSocketReceiver)>>>;

pub static GAMES: Lazy<Arc<RwLock<HashMap<u32, PlayerConnections>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

pub static PENDING_GAME: Lazy<Arc<Mutex<Option<u32>>>> = Lazy::new(|| Arc::new(Mutex::new(None)));
