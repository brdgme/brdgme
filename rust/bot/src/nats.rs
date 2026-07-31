//! Minimal NATS/JetStream constants and event types for the bot, re-exported
//! from the shared `brdgme_nats` wire-protocol crate (R-14). The bot only ever
//! consumes `bot.turn` and publishes `bot.command`; the monolith owns creating
//! the stream and both durable consumers on its own startup (see
//! docs/superpowers/plans/2026-07-05-13-nats-bot-eventing.md).

use anyhow::{Context, Result};

pub use brdgme_nats::*;

pub async fn connect(nats_url: &str) -> Result<async_nats::jetstream::Context> {
    let client = async_nats::connect(nats_url)
        .await
        .with_context(|| format!("Failed to connect to NATS at {}", nats_url))?;
    Ok(async_nats::jetstream::new(client))
}
