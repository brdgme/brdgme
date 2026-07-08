//! Minimal NATS/JetStream constants and event types mirroring
//! `rust/web/src/nats.rs`. The bot only ever consumes `bot.turn` and
//! publishes `bot.command`; the monolith owns creating the stream and both
//! durable consumers on its own startup (see docs/superpowers/plans/2026-07-05-13-nats-bot-eventing.md).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const STREAM_NAME: &str = "BOT";
pub const SUBJECT_COMMAND: &str = "bot.command";
pub const CONSUMER_TURN: &str = "bot-turn";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotTurnEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub difficulty: String,
    pub attempt: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCommandEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub command: String,
    pub attempt: i32,
}

pub async fn connect(nats_url: &str) -> Result<async_nats::jetstream::Context> {
    let client = async_nats::connect(nats_url)
        .await
        .with_context(|| format!("Failed to connect to NATS at {}", nats_url))?;
    Ok(async_nats::jetstream::new(client))
}
