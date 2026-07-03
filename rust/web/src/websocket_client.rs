use leptos::prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct WebSocketTrigger {
    pub last_update: ReadSignal<u64>,
    pub set_last_update: WriteSignal<u64>,
}

#[cfg(feature = "hydrate")]
pub fn use_websocket() {
    use crate::websocket::{BrdgmeGameUpdate, WebSocketMessage};
    use futures_util::StreamExt;
    use gloo_net::websocket::futures::WebSocket;
    use leptos::logging::log;
    use leptos::task::spawn_local;

    let trigger = expect_context::<WebSocketTrigger>();
    let ws_game = expect_context::<RwSignal<Option<BrdgmeGameUpdate>>>();

    spawn_local(async move {
        let loc = web_sys::window()
            .expect("window should be available")
            .location();
        let protocol = match loc.protocol() {
            Ok(p) if p == "https:" => "wss:",
            Ok(_) => "ws:",
            Err(e) => {
                log!(
                    "WebSocket: failed to read location.protocol: {:?}, defaulting to ws:",
                    e
                );
                "ws:"
            }
        };
        let host = match loc.host() {
            Ok(h) => h,
            Err(e) => {
                log!(
                    "WebSocket: failed to read location.host: {:?}, cannot connect",
                    e
                );
                return;
            }
        };
        let url = format!("{}//{}/ws", protocol, host);

        loop {
            match WebSocket::open(&url) {
                Ok(mut ws) => {
                    while let Some(msg) = ws.next().await {
                        match msg {
                            Ok(gloo_net::websocket::Message::Text(text)) => {
                                if let Ok(WebSocketMessage::BrdgmeUpdate(update)) =
                                    serde_json::from_str::<WebSocketMessage>(&text)
                                {
                                    ws_game.set(Some(update));
                                    trigger.set_last_update.update(|n| *n += 1);
                                }
                            }
                            Err(e) => {
                                log!("WebSocket error: {:?}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                    log!("WebSocket disconnected, reconnecting...");
                }
                Err(e) => {
                    log!("WebSocket connect failed: {:?}", e);
                }
            }

            // Brief pause before reconnect to avoid tight loop on persistent failures.
            gloo_timers::future::TimeoutFuture::new(2_000).await;
        }
    });
}

#[cfg(not(feature = "hydrate"))]
pub fn use_websocket() {}
