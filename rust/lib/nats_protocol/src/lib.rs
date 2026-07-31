//! Shared NATS/JetStream wire protocol for the bot <-> web eventing channel.
//!
//! The wire types (`BotTurnEvent`, `BotCommandEvent`) and the stream/subject/
//! consumer constants live here so the bot and the monolith cannot drift on the
//! wire. The golden-fixture integration test under `tests/` pins the exact JSON
//! encoding. Infra helpers (`connect`, stream/consumer setup, supervision) stay
//! in each consumer.

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

/// JetStream `max_deliver` for the BOT stream's pull consumers: the maximum
/// times a single message is redelivered before JetStream gives up. Shared
/// by the consumer config and the term ceiling (the `bot.command` consumer
/// Terms a message once `info.delivered >= MAX_DELIVER`) so the two cannot
/// drift (WP-38).
pub const MAX_DELIVER: i64 = 3;

/// JetStream `ack_wait` for the BOT stream's pull consumers. Must comfortably
/// exceed the worst-case handler duration, or JetStream redelivers
/// mid-processing and a message runs twice. Do NOT lower this; revisit
/// alongside any ack-cadence change (WP-38 / decision D-5).
pub const ACK_WAIT: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotTurnEvent {
    pub game_id: Uuid,
    pub player_position: i32,
    pub bot_name: String,
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
