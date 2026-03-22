use leptos::prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct WebSocketTrigger {
    pub last_update: ReadSignal<u64>,
    pub set_last_update: WriteSignal<u64>,
}

#[cfg(feature = "hydrate")]
pub fn use_websocket() {
    use gloo_net::websocket::futures::WebSocket;
    use futures_util::StreamExt;
    use crate::websocket::WebSocketMessage;
    use leptos::task::spawn_local;
    use leptos::logging::log;

    let trigger = expect_context::<WebSocketTrigger>();

    spawn_local(async move {
        let loc = web_sys::window().expect("window should be available").location();
        let protocol = if loc.protocol().expect("protocol should be available") == "https:" { "wss:" } else { "ws:" };
        let host = loc.host().expect("host should be available");
        let url = format!("{}//{}/ws", protocol, host);

        loop {
            match WebSocket::open(&url) {
                Ok(mut ws) => {
                    while let Some(msg) = ws.next().await {
                        match msg {
                            Ok(gloo_net::websocket::Message::Text(text)) => {
                                if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&text) {
                                    match ws_msg {
                                        WebSocketMessage::GameUpdate { .. } => {
                                            trigger.set_last_update.update(|n| *n += 1);
                                        }
                                        WebSocketMessage::GameRestarted { .. } => {
                                            trigger.set_last_update.update(|n| *n += 1);
                                        }
                                    }
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
pub fn use_websocket() {
    // No-op on server
}
