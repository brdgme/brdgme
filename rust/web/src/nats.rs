//! JetStream setup shared by the monolith's publish (`bot.turn`) and consume
//! (`bot.command`) sides. See docs/changes/archive/2026-07-05-13-nats-bot-eventing/plan.md for the
//! resolved stream/consumer design.

use anyhow::{Context, Result};
use async_nats::jetstream::{consumer::pull, stream};
use serde::Deserialize;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub use brdgme_nats::*;

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
        // JetStream dedup window for `Nats-Msg-Id` (set on `bot.turn`
        // publishes): collapses rapid re-publishes of the same turn state -
        // broadcast races, and the conflict/user-error re-publish that lands
        // within 120s (R-15 / F-105). The 15-minute reconciliation sweep is a
        // deliberate retry and is intentionally outside this window; a real
        // turn change bumps `updated_at` (and a retry bumps `attempt`), so
        // neither is suppressed by the window.
        duplicate_window: Duration::from_secs(120),
        ..Default::default()
    };
    // `create_stream` (not `get_or_create_stream`): on the deployed NATS 2.11
    // server it reconciles an existing stream's config in place, so code
    // changes (e.g. the duplicate window above) are applied at startup rather
    // than silently ignored on a pre-existing stream (R-15 / F-102).
    let stream = js
        .create_stream(desired_stream.clone())
        .await
        .context("Failed to create/reconcile BOT stream")?;
    let drift = stream_config_drift(&desired_stream, &stream.cached_info().config);
    if !drift.is_empty() {
        // Reconciliation is automatic now, so residual drift means the server
        // rejected part of the config - a genuine anomaly, not the old
        // "changes are ignored" steady state.
        tracing::warn!(
            stream = STREAM_NAME,
            ?drift,
            "NATS stream config still drifted after reconciliation; the server may be \
             rejecting part of the desired config"
        );
    }

    // `ACK_WAIT` (shared `brdgme_nats` const) must comfortably exceed the
    // worst-case `bot.command` handler duration, or JetStream redelivers
    // mid-processing and the command runs twice (review ws F58). Today's worst
    // case is bounded well under 5 min: the consumer processes-then-acks with a
    // 10s overall HTTP client timeout (web::main http_client) and bounded
    // retries. Do NOT lower it, and revisit alongside any ack-cadence change
    // (WP-38 / decision D-5, which also owns the bot-turn consumer's long-turn
    // story).
    for (name, subject) in [
        (CONSUMER_TURN, SUBJECT_TURN),
        (CONSUMER_COMMAND, SUBJECT_COMMAND),
    ] {
        let desired = pull::Config {
            durable_name: Some(name.to_string()),
            filter_subject: subject.to_string(),
            ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
            ack_wait: ACK_WAIT,
            max_deliver: MAX_DELIVER,
            ..Default::default()
        };
        // `create_consumer` (not `get_or_create_consumer`): on NATS 2.11 it
        // updates an existing durable's config in place, so `ack_wait` /
        // `max_deliver` changes are reconciled at startup instead of being
        // stranded on a pre-existing consumer (R-15 / F-102). The durable name
        // comes from `desired.durable_name`.
        let consumer = stream
            .create_consumer(desired.clone())
            .await
            .with_context(|| format!("Failed to create/reconcile {} consumer", name))?;
        let drift = consumer_config_drift(&desired, &consumer.cached_info().config);
        if !drift.is_empty() {
            tracing::warn!(
                consumer = name,
                ?drift,
                "NATS consumer config still drifted after reconciliation; the server may be \
                 rejecting part of the desired config"
            );
        }
    }
    Ok(())
}

/// JetStream server advisory subject for messages that exhausted
/// `max_deliver` on any consumer of the BOT stream. These messages will
/// never be redelivered and (WorkQueue retention) never deleted — they are
/// stranded until an operator intervenes. The advisory listener below makes
/// that stranding visible (error log + metric); automated recovery
/// (re-publish/DLQ) is not yet implemented (WP-38/D-5).
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
/// Visibility only: the `bot.command` consumer already Terms messages that
/// exhaust `max_deliver`, but DLQ/re-publish recovery for stranded messages
/// is WP-38/D-5.
/// Returns when the subscription stream ends or `shutdown` is cancelled; run
/// under `supervise_consumer` so it is re-established. The shutdown arm lets
/// the supervisor's bounded drain (R-11 / F-109) wind this listener down
/// instead of abandoning it at process exit.
pub async fn run_max_deliveries_advisory_listener(
    client: async_nats::Client,
    shutdown: CancellationToken,
) -> Result<()> {
    use futures_util::StreamExt;

    let mut sub = client
        .subscribe(MAX_DELIVERIES_ADVISORY_SUBJECT)
        .await
        .context("Failed to subscribe to JetStream max-deliveries advisories")?;
    tracing::info!(
        subject = MAX_DELIVERIES_ADVISORY_SUBJECT,
        "Listening for JetStream max-deliveries advisories"
    );
    loop {
        let msg = tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!("max-deliveries advisory listener shutdown signalled; stopping");
                return Ok(());
            }
            msg = sub.next() => msg,
        };
        let Some(msg) = msg else {
            return Ok(());
        };
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
///
/// `shutdown` bounds the supervisor's lifetime (R-11 / F-109): once it is
/// cancelled the supervisor stops restarting and returns. If a run is in
/// flight at that moment, the supervisor waits for it to wind down (the
/// consumer bodies observe the same token and exit cleanly) so `main`'s
/// bounded drain can await this task to completion instead of killing a
/// consumer mid-`execute_command`.
pub async fn supervise_consumer<F, Fut>(
    name: &'static str,
    shutdown: CancellationToken,
    mut make_run: F,
) where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    const INITIAL_BACKOFF: Duration = Duration::from_secs(1);
    const MAX_BACKOFF: Duration = Duration::from_secs(30);
    const STABLE_RESET: Duration = Duration::from_secs(60);

    let mut backoff = INITIAL_BACKOFF;
    loop {
        let started = tokio::time::Instant::now();
        let mut run = tokio::spawn(make_run());
        let result = tokio::select! {
            res = &mut run => Some(res),
            _ = shutdown.cancelled() => None,
        };
        match result {
            None => {
                tracing::info!(
                    consumer = name,
                    "shutdown signalled; waiting for consumer to wind down"
                );
                let _ = run.await;
                return;
            }
            Some(Ok(Ok(()))) => {
                tracing::error!(consumer = name, "consumer stream ended; restarting");
            }
            Some(Ok(Err(e))) => {
                tracing::error!(
                    consumer = name,
                    "consumer exited with error: {:#}; restarting",
                    e
                );
            }
            Some(Err(join_err)) => {
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
        tokio::select! {
            _ = shutdown.cancelled() => {
                tracing::info!(consumer = name, "shutdown signalled during backoff; stopping");
                return;
            }
            _ = tokio::time::sleep(backoff) => {}
        }
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
        let sup = tokio::spawn(supervise_consumer(
            "test-consumer",
            tokio_util::sync::CancellationToken::new(),
            move || {
                let calls = calls_clone.clone();
                async move {
                    match calls.fetch_add(1, Ordering::SeqCst) {
                        0 => Err(anyhow::anyhow!("boom")),
                        1 => Ok(()),
                        2 => panic!("consumer task panic"),
                        _ => std::future::pending().await,
                    }
                }
            },
        ));
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
        let sup = tokio::spawn(supervise_consumer(
            "always-fails",
            tokio_util::sync::CancellationToken::new(),
            move || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("still down"))
                }
            },
        ));
        tokio::time::timeout(Duration::from_secs(700), async {
            while calls.load(Ordering::SeqCst) < 20 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("supervisor stopped retrying under persistent failure");
        sup.abort();
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down() {
        let shutdown = tokio_util::sync::CancellationToken::new();
        let started = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicUsize::new(0));
        let started_clone = started.clone();
        let finished_clone = finished.clone();
        let run_token = shutdown.clone();
        let sup = tokio::spawn(supervise_consumer(
            "shutdown-test",
            shutdown.clone(),
            move || {
                let started = started_clone.clone();
                let finished = finished_clone.clone();
                let run_token = run_token.clone();
                async move {
                    started.fetch_add(1, Ordering::SeqCst);
                    // Mirrors a shutdown-aware consumer body (bot-command /
                    // max-deliveries-advisory): run until shutdown, then wind
                    // down cleanly.
                    run_token.cancelled().await;
                    finished.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
        ));
        tokio::time::timeout(Duration::from_secs(600), async {
            while started.load(Ordering::SeqCst) < 1 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("supervised run did not start");
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(600), sup)
            .await
            .expect("supervisor did not exit after shutdown")
            .unwrap();
        assert_eq!(
            started.load(Ordering::SeqCst),
            1,
            "supervisor must not restart the consumer after shutdown"
        );
        assert_eq!(
            finished.load(Ordering::SeqCst),
            1,
            "supervisor must wait for the running consumer to wind down before exiting"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn supervisor_backoff_sleep_is_interrupted_by_shutdown() {
        let shutdown = tokio_util::sync::CancellationToken::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();
        let sup = tokio::spawn(supervise_consumer(
            "backoff-shutdown",
            shutdown.clone(),
            move || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("down"))
                }
            },
        ));
        tokio::time::timeout(Duration::from_secs(600), async {
            while calls.load(Ordering::SeqCst) < 1 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("first run did not happen");
        // The supervisor is now in its (paused) backoff sleep; cancelling must
        // interrupt it rather than waiting out the full backoff.
        shutdown.cancel();
        tokio::time::timeout(Duration::from_secs(600), sup)
            .await
            .expect("supervisor did not exit promptly when shutdown interrupted backoff")
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "no restart after shutdown");
    }
}
