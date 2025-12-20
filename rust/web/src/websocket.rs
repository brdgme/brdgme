use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;
use crate::db::GameExtended;
use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade, Message}, State},
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketMessage {
    GameUpdate {
        game_id: Uuid,
        game: Option<GameExtended>,
    },
    GameRestarted {
        game_id: Uuid,
        restarted_game_id: Uuid,
    },
}

#[derive(Clone)]
pub struct GameBroadcaster {
    sender: broadcast::Sender<WebSocketMessage>,
}

impl GameBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WebSocketMessage> {
        self.sender.subscribe()
    }

    pub fn broadcast(&self, message: WebSocketMessage) {
        let _ = self.sender.send(message);
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(broadcaster): State<GameBroadcaster>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, broadcaster))
}

async fn handle_socket(socket: WebSocket, broadcaster: GameBroadcaster) {
    let (mut sender, mut _receiver) = socket.split();
    let mut rx = broadcaster.subscribe();

    // Loop through broadcast messages and send to client
    while let Ok(msg) = rx.recv().await {
        // In a real app, we would filter based on the user's active games
        // For now, we'll send everything.
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(_) => continue,
        };

        if sender.send(Message::Text(json.into())).await.is_err() {
            // Client disconnected
            break;
        }
    }
}
