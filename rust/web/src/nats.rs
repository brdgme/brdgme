//! JetStream setup shared by the monolith's publish (`bot.turn`) and consume
//! (`bot.command`) sides. See docs/superpowers/plans/2026-07-05-13-nats-bot-eventing.md for the
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

/// JetStream `max_deliver` for the BOT stream's pull consumers: the maximum
/// times a single message is redelivered before JetStream gives up. Shared
/// by the consumer config and the (future) term ceiling so the two cannot
/// drift (WP-38).
pub const MAX_DELIVER: i64 = 3;

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

/// Connects to NATS and wraps the client in a JetStream context.
pub async fn connect(nats_url: &str) -> Result<async_nats::jetstream::Context> {
    let client = async_nats::connect(nats_url)
        .await
        .with_context(|| format!("Failed to connect to NATS at {}", nats_url))?;
    Ok(async_nats::jetstream::new(client))
}

/// Field-by-field drift check for the fields `ensure_stream_and_consumers`
/// sets on the stream (whole-struct equality would false-positive on
/// server-populated defaults). Returns one human-readable line per
/// mismatch (review ws F57).
pub fn stream_config_drift(desired: &stream::Config, actual: &stream::Config) -> Vec<String> {
    let mut drift = Vec::new();
    if desired.subjects != actual.subjects {
        drift.push(format!(
            "subjects: code wants {:?}, server has {:?}",
            desired.subjects, actual.subjects
        ));
    }
    if desired.retention != actual.retention {
        drift.push(format!(
            "retention: code wants {:?}, server has {:?}",
            desired.retention, actual.retention
        ));
    }
    drift
}

/// Same as `stream_config_drift` for the fields set on the pull consumers.
/// Accepts the generic `consumer::Config` because `cached_info().config`
/// returns that type regardless of the pull/push specialization used to
/// create the consumer.
pub fn consumer_config_drift(
    desired: &pull::Config,
    actual: &async_nats::jetstream::consumer::Config,
) -> Vec<String> {
    let mut drift = Vec::new();
    if desired.durable_name != actual.durable_name {
        drift.push(format!(
            "durable_name: code wants {:?}, server has {:?}",
            desired.durable_name, actual.durable_name
        ));
    }
    if desired.filter_subject != actual.filter_subject {
        drift.push(format!(
            "filter_subject: code wants {:?}, server has {:?}",
            desired.filter_subject, actual.filter_subject
        ));
    }
    if desired.ack_policy != actual.ack_policy {
        drift.push(format!(
            "ack_policy: code wants {:?}, server has {:?}",
            desired.ack_policy, actual.ack_policy
        ));
    }
    if desired.ack_wait != actual.ack_wait {
        drift.push(format!(
            "ack_wait: code wants {:?}, server has {:?}",
            desired.ack_wait, actual.ack_wait
        ));
    }
    if desired.max_deliver != actual.max_deliver {
        drift.push(format!(
            "max_deliver: code wants {:?}, server has {:?}",
            desired.max_deliver, actual.max_deliver
        ));
    }
    drift
}

/// Idempotently creates the `BOT` stream and its two durable pull consumers
/// (`bot-turn` filtered to `bot.turn`, `bot-command` filtered to
/// `bot.command`). Safe to call on every monolith startup.
pub async fn ensure_stream_and_consumers(js: &async_nats::jetstream::Context) -> Result<()> {
    let desired_stream = stream::Config {
        name: STREAM_NAME.to_string(),
        subjects: vec!["bot.>".to_string()],
        retention: stream::RetentionPolicy::WorkQueue,
        ..Default::default()
    };
    let stream = js
        .get_or_create_stream(desired_stream.clone())
        .await
        .context("Failed to create/get BOT stream")?;
    let drift = stream_config_drift(&desired_stream, &stream.cached_info().config);
    if !drift.is_empty() {
        tracing::warn!(
            stream = STREAM_NAME,
            ?drift,
            "NATS stream config drift: code changes to stream config are NOT applied to an \
             existing stream; update it manually (e.g. nats CLI) or delete/recreate to apply"
        );
    }

    // `ack_wait` must comfortably exceed the worst-case `bot.command`
    // handler duration, or JetStream redelivers mid-processing and the
    // command runs twice (review ws F58). Today's worst case is bounded
    // well under 5 min: the consumer processes-then-acks with a 10s
    // overall HTTP client timeout (web::main http_client) and bounded
    // retries. Do NOT lower this, and revisit alongside any ack-cadence
    // change (WP-38 / decision D-5, which also owns the bot-turn
    // consumer's long-turn story).
    let ack_wait = Duration::from_secs(5 * 60);

    for (name, subject) in [
        (CONSUMER_TURN, SUBJECT_TURN),
        (CONSUMER_COMMAND, SUBJECT_COMMAND),
    ] {
        let desired = pull::Config {
            durable_name: Some(name.to_string()),
            filter_subject: subject.to_string(),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            ack_wait,
            max_deliver: MAX_DELIVER,
            ..Default::default()
        };
        let consumer = stream
            .get_or_create_consumer(name, desired.clone())
            .await
            .with_context(|| format!("Failed to create/get {} consumer", name))?;
        let drift = consumer_config_drift(&desired, &consumer.cached_info().config);
        if !drift.is_empty() {
            tracing::warn!(
                consumer = name,
                ?drift,
                "NATS consumer config drift: code changes to consumer config are NOT applied \
                 to an existing durable consumer; delete/recreate it manually to apply"
            );
        }
    }
    Ok(())
}

/// JetStream server advisory subject for messages that exhausted
/// `max_deliver` on any consumer of the BOT stream. These messages will
/// never be redelivered and (WorkQueue retention) never deleted — they are
/// stranded until an operator or a future recovery mechanism (WP-38/D-5)
/// intervenes.
pub const MAX_DELIVERIES_ADVISORY_SUBJECT: &str =
    "$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.BOT.*";

/// Body of an io.nats.jetstream.advisory.v1.max_deliver advisory (unknown
/// fields ignored; `deliveries` defaulted defensively).
#[derive(Debug, Deserialize)]
pub struct MaxDeliveriesAdvisory {
    pub stream: String,
    pub consumer: String,
    pub stream_seq: u64,
    #[serde(default)]
    pub deliveries: i64,
}

/// Lenient parse — a malformed advisory must never kill the listener.
pub fn parse_max_deliveries_advisory(payload: &[u8]) -> Option<MaxDeliveriesAdvisory> {
    serde_json::from_slice(payload).ok()
}

/// Subscribes to MAX_DELIVERIES advisories for the BOT stream and turns
/// each one into an error log + `bot_stream_max_deliveries_total` metric,
/// so stranded messages are alertable instead of silent (review ws F56).
/// Visibility only: recovery (term/DLQ/re-publish) is WP-38/D-5.
/// Returns when the subscription stream ends; run under
/// `supervise_consumer` so it is re-established.
pub async fn run_max_deliveries_advisory_listener(client: async_nats::Client) -> Result<()> {
    use futures_util::StreamExt;

    let mut sub = client
        .subscribe(MAX_DELIVERIES_ADVISORY_SUBJECT)
        .await
        .context("Failed to subscribe to JetStream max-deliveries advisories")?;
    tracing::info!(
        subject = MAX_DELIVERIES_ADVISORY_SUBJECT,
        "Listening for JetStream max-deliveries advisories"
    );
    while let Some(msg) = sub.next().await {
        match parse_max_deliveries_advisory(&msg.payload) {
            Some(adv) => {
                axum_prometheus::metrics::counter!("bot_stream_max_deliveries_total").increment(1);
                tracing::error!(
                    stream = %adv.stream,
                    consumer = %adv.consumer,
                    stream_seq = adv.stream_seq,
                    deliveries = adv.deliveries,
                    "message exhausted max_deliver and is stranded in the stream; \
                     the affected bot will not act again without manual intervention"
                );
            }
            None => {
                tracing::warn!(
                    subject = %msg.subject,
                    "unparseable max-deliveries advisory payload"
                );
            }
        }
    }
    Ok(())
}

/// Runs `make_run()` forever, restarting it whenever it exits — cleanly
/// (`Ok`: the message stream ended), with an error, or by panic (each run
/// is spawned so the `JoinError` is caught instead of being swallowed by a
/// dropped handle). Exponential backoff 1s..30s between restarts, reset
/// after any run that survived 60s. Every restart emits an error log and
/// increments `nats_consumer_restarts_total{consumer=<name>}` so a
/// crash-looping or dead consumer is alertable from /metrics instead of
/// silently stopping bot play until a pod restart (review ws F53, wd F4).
///
/// This supervises task LIVENESS only; message-level recovery semantics
/// (what gets acked/termed/re-published) are owned by the consumer body
/// and are out of scope here (WP-38 / D-5).
pub async fn supervise_consumer<F, Fut>(name: &'static str, mut make_run: F)
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);
    const STABLE_RESET: Duration = Duration::from_secs(60);

    let mut backoff = INITIAL_BACKOFF;
    loop {
        let started = tokio::time::Instant::now();
        match tokio::spawn(make_run()).await {
            Ok(Ok(())) => {
                tracing::error!(consumer = name, "consumer stream ended; restarting");
            }
            Ok(Err(e)) => {
                tracing::error!(
                    consumer = name,
                    "consumer exited with error: {:#}; restarting",
                    e
                );
            }
            Err(join_err) => {
                tracing::error!(
                    consumer = name,
                    "consumer task panicked: {}; restarting",
                    join_err
                );
            }
        }
        axum_prometheus::metrics::counter!("nats_consumer_restarts_total", "consumer" => name)
            .increment(1);
        if started.elapsed() >= STABLE_RESET {
            backoff = INITIAL_BACKOFF;
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn parses_max_deliveries_advisory_payload() {
        let payload = br#"{
            "type": "io.nats.jetstream.advisory.v1.max_deliver",
            "id": "abc123",
            "timestamp": "2026-07-25T00:00:00Z",
            "stream": "BOT",
            "consumer": "bot-command",
            "stream_seq": 42,
            "deliveries": 3
        }"#;
        let adv = parse_max_deliveries_advisory(payload).expect("should parse");
        assert_eq!(adv.stream, "BOT");
        assert_eq!(adv.consumer, "bot-command");
        assert_eq!(adv.stream_seq, 42);
        assert_eq!(adv.deliveries, 3);
        assert!(parse_max_deliveries_advisory(b"not json").is_none());
    }

    #[test]
    fn detects_consumer_config_drift() {
        let desired = pull::Config {
            durable_name: Some(CONSUMER_COMMAND.to_string()),
            filter_subject: SUBJECT_COMMAND.to_string(),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(300),
            max_deliver: 3,
            ..Default::default()
        };
        let mut actual = async_nats::jetstream::consumer::Config {
            durable_name: Some(CONSUMER_COMMAND.to_string()),
            filter_subject: SUBJECT_COMMAND.to_string(),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(300),
            max_deliver: 3,
            num_replicas: 3,
            ..Default::default()
        };
        assert!(consumer_config_drift(&desired, &actual).is_empty());
        actual.ack_wait = Duration::from_secs(30);
        actual.max_deliver = 5;
        let drift = consumer_config_drift(&desired, &actual);
        assert_eq!(drift.len(), 2);
        assert!(drift.iter().any(|d| d.contains("ack_wait")));
        assert!(drift.iter().any(|d| d.contains("max_deliver")));
    }

    #[test]
    fn detects_stream_config_drift() {
        let desired = stream::Config {
            name: STREAM_NAME.to_string(),
            subjects: vec!["bot.>".to_string()],
            retention: stream::RetentionPolicy::WorkQueue,
            ..Default::default()
        };
        let mut actual = desired.clone();
        assert!(stream_config_drift(&desired, &actual).is_empty());
        actual.retention = stream::RetentionPolicy::Limits;
        actual.subjects = vec!["bot.*".to_string()];
        let drift = stream_config_drift(&desired, &actual);
        assert_eq!(drift.len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_restarts_on_err_ok_and_panic() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let sup = tokio::spawn(supervise_consumer("test-consumer", move || {
            let calls = calls_clone.clone();
            async move {
                match calls.fetch_add(1, Ordering::SeqCst) {
                    0 => Err(anyhow::anyhow!("boom")),
                    1 => Ok(()),
                    2 => panic!("consumer task panic"),
                    _ => std::future::pending().await,
                }
            }
        }));
        tokio::time::timeout(Duration::from_secs(600), async {
            while calls.load(Ordering::SeqCst) < 4 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("supervisor did not restart through all three death modes");
        sup.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_keeps_retrying_under_persistent_failure() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let sup = tokio::spawn(supervise_consumer("always-fails", move || {
            let calls = calls_clone.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Err(anyhow::anyhow!("still down"))
            }
        }));
        tokio::time::timeout(Duration::from_secs(700), async {
            while calls.load(Ordering::SeqCst) < 20 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("supervisor stopped retrying under persistent failure");
        sup.abort();
    }
}
