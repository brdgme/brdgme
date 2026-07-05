//! JetStream setup shared by the monolith's publish (`bot.turn`) and consume
//! (`bot.command`) sides. See docs/plan/13-nats-bot-eventing.md for the
//! resolved stream/consumer design.

use anyhow::{Context, Result};
use async_nats::jetstream::{consumer::pull, stream};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

pub const STREAM_NAME: &str = "BOT";
pub const SUBJECT_TURN: &str = "bot.turn";
pub const SUBJECT_COMMAND: &str = "bot.command";
pub const CONSUMER_TURN: &str = "bot-turn";
pub const CONSUMER_COMMAND: &str = "bot-command";

/// Overall cap on turn-level re-publishes after a stale-state conflict
/// (`BotTurnEvent::attempt`), on top of the original publish.
pub const MAX_TURN_ATTEMPTS: i32 = 3;

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
    /// Echoes `BotTurnEvent::attempt` from the `bot.turn` event this command
    /// resulted from, so the `bot.command` consumer knows how many
    /// turn-level retries have already happened before deciding whether a
    /// stale-state conflict should give up or re-publish `bot.turn` again.
    pub attempt: i32,
}

/// Connects to NATS and wraps the client in a JetStream context.
pub async fn connect(nats_url: &str) -> Result<async_nats::jetstream::Context> {
    let client = async_nats::connect(nats_url)
        .await
        .with_context(|| format!("Failed to connect to NATS at {}", nats_url))?;
    Ok(async_nats::jetstream::new(client))
}

/// Idempotently creates the `BOT` stream and its two durable pull consumers
/// (`bot-turn` filtered to `bot.turn`, `bot-command` filtered to
/// `bot.command`). Safe to call on every monolith startup.
pub async fn ensure_stream_and_consumers(js: &async_nats::jetstream::Context) -> Result<()> {
    let stream = js
        .get_or_create_stream(stream::Config {
            name: STREAM_NAME.to_string(),
            subjects: vec!["bot.>".to_string()],
            retention: stream::RetentionPolicy::WorkQueue,
            ..Default::default()
        })
        .await
        .context("Failed to create/get BOT stream")?;

    let ack_wait = Duration::from_secs(5 * 60);
    let max_deliver = 3;

    stream
        .get_or_create_consumer(
            CONSUMER_TURN,
            pull::Config {
                durable_name: Some(CONSUMER_TURN.to_string()),
                filter_subject: SUBJECT_TURN.to_string(),
                ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                ack_wait,
                max_deliver,
                ..Default::default()
            },
        )
        .await
        .context("Failed to create/get bot-turn consumer")?;

    stream
        .get_or_create_consumer(
            CONSUMER_COMMAND,
            pull::Config {
                durable_name: Some(CONSUMER_COMMAND.to_string()),
                filter_subject: SUBJECT_COMMAND.to_string(),
                ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
                ack_wait,
                max_deliver,
                ..Default::default()
            },
        )
        .await
        .context("Failed to create/get bot-command consumer")?;

    Ok(())
}
