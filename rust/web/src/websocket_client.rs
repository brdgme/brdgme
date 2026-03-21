use leptos::prelude::*;
use uuid::Uuid;

#[derive(Copy, Clone, Debug)]
pub struct WebSocketTrigger {
    pub last_update: ReadSignal<u64>,
    pub set_last_update: WriteSignal<u64>,
    pub game_restarted: ReadSignal<Option<(Uuid, Uuid)>>,
    pub set_game_restarted: WriteSignal<Option<(Uuid, Uuid)>>,
}

#[cfg(feature = "hydrate")]
pub fn use_websocket() {
    use gloo_net::websocket::futures::WebSocket;
    use futures_util::StreamExt;
    use crate::websocket::WebSocketMessage;
    use leptos::task::spawn_local;
    use leptos::logging::log;

    let trigger = expect_context::<WebSocketTrigger>();

    Effect::new(move |_| {
        let loc = web_sys::window().expect("window should be available").location();
        let protocol = if loc.protocol().expect("protocol should be available") == "https:" { "wss:" } else { "ws:" };
        let host = loc.host().expect("host should be available");
        let url = format!("{}//{}/ws", protocol, host);

        match WebSocket::open(&url) {
            Ok(mut ws) => {
                spawn_local(async move {
                    while let Some(msg) = ws.next().await {
                        if let Ok(gloo_net::websocket::Message::Text(text)) = msg {
                            if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                                match ws_msg {
                                    WebSocketMessage::GameUpdate { .. } => {
                                        trigger.set_last_update.update(|n| *n += 1);
                                    }
                                    WebSocketMessage::GameRestarted { game_id, restarted_game_id } => {
                                        trigger.set_game_restarted.set(Some((game_id, restarted_game_id)));
                                        trigger.set_last_update.update(|n| *n += 1);
                                    }
                                }
                            }
                        }
                    }
                });
            }
            Err(e) => {
                log!("Failed to connect to WebSocket: {:?}", e);
            }
        }
    });
}

#[cfg(not(feature = "hydrate"))]
pub fn use_websocket() {
    // No-op on server
}
