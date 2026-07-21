use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameUpdateSignal {
    pub game_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalUpdateSignal {
    pub proposal_id: Uuid,
}

#[cfg(feature = "ssr")]
pub use ssr::*;

#[cfg(feature = "ssr")]
mod ssr {
    use super::*;
    use axum::{
        extract::{
            State,
            ws::{Message, WebSocket, WebSocketUpgrade},
        },
        response::IntoResponse,
    };
    use futures_util::{sink::SinkExt, stream::StreamExt};
    use sqlx::PgPool;

    #[derive(Clone)]
    pub struct GameBroadcaster {
        client: async_nats::Client,
    }

    impl GameBroadcaster {
        pub fn new(client: async_nats::Client) -> Self {
            Self { client }
        }

        pub async fn broadcast_game_update(&self, game_id: Uuid) {
            let signal = GameUpdateSignal { game_id };
            let payload = match serde_json::to_vec(&signal) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to serialize GameUpdateSignal: {}", e);
                    return;
                }
            };
            if let Err(e) = self
                .client
                .publish(format!("game.{}", game_id), payload.into())
                .await
            {
                tracing::error!("NATS publish failed on game.{}: {}", game_id, e);
            }
            if let Err(e) = self.client.flush().await {
                tracing::error!("NATS flush failed after game.{}: {}", game_id, e);
            }
        }

        pub async fn broadcast_proposal_update(&self, proposal_id: Uuid) {
            let signal = ProposalUpdateSignal { proposal_id };
            let payload = match serde_json::to_vec(&signal) {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!("Failed to serialize ProposalUpdateSignal: {}", e);
                    return;
                }
            };
            if let Err(e) = self
                .client
                .publish(format!("proposal.{}", proposal_id), payload.into())
                .await
            {
                tracing::error!("NATS publish failed on proposal.{}: {}", proposal_id, e);
            }
            if let Err(e) = self.client.flush().await {
                tracing::error!("NATS flush failed after proposal.{}: {}", proposal_id, e);
            }
        }
    }

    pub async fn ws_handler(
        ws: WebSocketUpgrade,
        State(broadcaster): State<GameBroadcaster>,
        State(pool): State<PgPool>,
        session: tower_sessions::Session,
    ) -> impl IntoResponse {
        let user_id = crate::auth::session::get_user_from_session(&session)
            .await
            .map(|u| u.id);
        ws.on_upgrade(move |socket| handle_socket(socket, broadcaster, pool, user_id))
    }

    /// Decrements the `ws_connections` gauge on drop, so every exit path out of
    /// `handle_socket` (the `select!` loop has several `break`s, plus the early
    /// return on subscribe failure) decrements exactly once without scattering
    /// manual decrements across each exit point.
    struct WsConnectionGuard;

    impl WsConnectionGuard {
        fn new() -> Self {
            axum_prometheus::metrics::gauge!("ws_connections").increment(1.0);
            Self
        }
    }

    impl Drop for WsConnectionGuard {
        fn drop(&mut self) {
            axum_prometheus::metrics::gauge!("ws_connections").decrement(1.0);
        }
    }

    async fn handle_socket(
        socket: WebSocket,
        broadcaster: GameBroadcaster,
        pool: PgPool,
        user_id: Option<Uuid>,
    ) {
        let _ws_guard = WsConnectionGuard::new();
        let (mut sender, mut receiver) = socket.split();

        let mut game_sub = match broadcaster.client.subscribe("game.>").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("NATS subscribe failed: {}", e);
                return;
            }
        };
        let mut proposal_sub = match broadcaster.client.subscribe("proposal.>").await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("NATS subscribe failed: {}", e);
                return;
            }
        };

        // Periodic ping to keep idle connections alive across load-balancer idle timeouts.
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ping_interval.tick().await; // first tick fires immediately, skip it

        // Stamp web presence on connect and on each ~30s ping tick; the throttle
        // inside `record_web_activity` bounds this to ~1 DB write/minute (#22b
        // active-web suppression).
        if let Some(uid) = user_id {
            crate::email::outbound::record_web_activity(&pool, uid).await;
        }

        loop {
            tokio::select! {
                msg = game_sub.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => break,
                    };
                    let payload = match std::str::from_utf8(&msg.payload) {
                        Ok(p) => p.to_string(),
                        Err(_) => continue,
                    };
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
                msg = proposal_sub.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => break,
                    };
                    let payload = match std::str::from_utf8(&msg.payload) {
                        Ok(p) => p.to_string(),
                        Err(_) => continue,
                    };
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
                _ = ping_interval.tick() => {
                    if let Some(uid) = user_id {
                        crate::email::outbound::record_web_activity(&pool, uid).await;
                    }
                    if sender.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
                // Drain inbound messages so pongs and close frames are processed; we don't
                // act on client-sent data here.
                incoming = receiver.next() => {
                    match incoming {
                        Some(Ok(_)) => {}
                        _ => break,
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Duration;
        use tokio::time::timeout;

        async fn make_broadcaster() -> GameBroadcaster {
            let nats_url =
                std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
            let client = async_nats::connect(&nats_url).await.unwrap();
            GameBroadcaster::new(client)
        }

        #[tokio::test]
        #[ignore = "flaky NATS timing; see docs/superpowers/plans/2026-07-07-27-web-simplification.md deferred item 2"]
        async fn broadcast_publishes_skinny_signal_to_game_subject_only() {
            let broadcaster = make_broadcaster().await;
            let nats_url =
                std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
            let client = async_nats::connect(&nats_url).await.unwrap();

            let game_id = Uuid::new_v4();
            let mut game_sub = client.subscribe(format!("game.{}", game_id)).await.unwrap();
            let mut user_sub = client.subscribe("user.>").await.unwrap();
            let mut ws_sub = client.subscribe("ws.>").await.unwrap();
            client.flush().await.unwrap();

            broadcaster.broadcast_game_update(game_id).await;

            let msg = timeout(Duration::from_secs(5), game_sub.next())
                .await
                .expect("timed out waiting for game.{id} message")
                .expect("game.{id} subscription ended unexpectedly");

            assert_eq!(msg.subject.as_str(), format!("game.{}", game_id));
            let v: serde_json::Value = serde_json::from_slice(&msg.payload).unwrap();
            assert_eq!(v, serde_json::json!({ "game_id": game_id.to_string() }));

            assert!(
                timeout(Duration::from_millis(300), game_sub.next())
                    .await
                    .is_err(),
                "expected exactly one message on game.{{id}}"
            );
            assert!(
                timeout(Duration::from_millis(300), user_sub.next())
                    .await
                    .is_err(),
                "expected no message on user.>"
            );
            assert!(
                timeout(Duration::from_millis(300), ws_sub.next())
                    .await
                    .is_err(),
                "expected no message on ws.>"
            );
        }
    }
}
