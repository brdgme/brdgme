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
    use tokio_util::sync::CancellationToken;

    #[derive(Clone)]
    pub struct GameBroadcaster {
        pub(crate) client: async_nats::Client,
        pub(crate) shutdown: CancellationToken,
    }

    impl GameBroadcaster {
        pub fn new(client: async_nats::Client) -> Self {
            Self {
                client,
                shutdown: CancellationToken::new(),
            }
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

        pub fn begin_shutdown(&self) {
            self.shutdown.cancel();
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use futures_util::StreamExt;
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
