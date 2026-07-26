# WP-39: bot consumer supervision (mechanical)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Close the "silent permanent bot outage" class mechanically, without touching ack semantics or wedge-recovery design (that is WP-38 / decision D-5). Concretely: the web monolith's `bot.command` consumer gets a supervised restart loop so its death no longer silently stops all bot play until a pod restart (ws F53 + wd F4, both major); messages that exhaust `max_deliver=3` become operationally visible via a JetStream MAX_DELIVERIES advisory listener with an error log and metric (ws F56, minor — visibility only, NO recovery); NATS stream/consumer config drift between code and server is warned about at startup (ws F57, minor); the `ack_wait` invariant is documented (ws F58, nit — document-only); the stale-conflict re-publish stops fanning out to bystander bots (wd F9, nit); the bot service's reachable `unreachable!()` becomes a proper error (bo F1, major); the bot service gets an in-process concurrency bound (bo F3, minor) and SIGTERM graceful shutdown that drains in-flight turns (bo F5, minor); and the bot `/healthz` DB-check recommendation is DECLINED with rationale recorded in code (bo F8, nit — it is a liveness probe; see Task 7).

**Architecture — how the pieces fit today (verified against live source 2026-07-25):**

- **Web consumer spawn (the outage):** `rust/web/src/main.rs:55-74` spawns `web::game::run_bot_command_consumer` exactly once, discarding the `JoinHandle`. The consumer (`rust/web/src/game/mod.rs:252-326`) returns `Err` on setup failure (`get_consumer_from_stream` at :261-263 or `consumer.messages()` at :264) — logged once at `main.rs:71` and never restarted — and returns `Ok(())` when the `messages` stream ends (`while let` exits at :323, `Ok(())` at :325) — **not even logged**. If the task panics, the dropped `JoinHandle` swallows it silently. `/healthz` is a static "OK" (deliberately dependency-free), so k8s keeps the pod. Result: bots stop moving in every game on every replica that hits this, forever, with zero signal. In-flight messages are safe (un-acked → redelivered after `ack_wait`), but nothing consumes the redelivery.
- **Contrast — the bot service does NOT have this bug:** `rust/bot/src/main.rs:812-869` runs its consumer loop directly in `main`; if the stream ends, `main` returns and the process exits, and k8s `restartPolicy: Always` restarts the container. The web monolith can't use that shape (the same process serves HTTP), hence the in-process supervisor.
- **Metrics infra:** web already uses the `metrics` facade (0.24) via `axum_prometheus` (0.10) — e.g. `axum_prometheus::metrics::counter!("game_emails_sent_total").increment(1)` in `rust/web/src/email/outbound.rs:65`, gauges in `websocket.rs:97/104` — exported by `serve_metrics` (`main.rs:171-195`) on the private `:9090` port. New counters in this WP follow that exact pattern.
- **max_deliver stranding (visibility target):** both durable pull consumers (`rust/web/src/nats.rs:63-94`) use `AckPolicy::Explicit`, `ack_wait = 5 min`, `max_deliver = 3` on a WorkQueue-retention stream. The consumer acks every poison class (parse error `game/mod.rs:277-280`, `UserError` :305-315, `Conflict` :300-304); only transient `Other` failures are left unacked (:316-321). A message failing transiently on all 3 deliveries is never delivered again and — WorkQueue retention deletes only on ack — sits in the stream forever. The NATS server emits a per-event advisory on subject `$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.<stream>.<consumer>` (JSON including `stream`, `consumer`, `stream_seq`, `deliveries`); nothing subscribes today. Visibility = subscribe + error-log + counter. **Recovery (term/DLQ/re-publish/admin surface) is WP-38/D-5 — explicitly out of scope.**
- **Config drift:** `ensure_stream_and_consumers` (`nats.rs:52-97`) uses `get_or_create_stream`/`get_or_create_consumer`, which return the existing server object untouched when it exists — editing `ack_wait`/`max_deliver`/retention/subjects in code is a silent no-op against a live NATS deployment. async-nats 0.49.1 exposes the server's actual config via `Stream::cached_info().config` and `Consumer::cached_info().config` (both `Config` types derive `PartialEq`), so a field-by-field comparison at startup is cheap and warn-able.
- **Conflict fan-out:** on a stale-state conflict, `handle_bot_command_event` (`game/mod.rs:391-398`) re-publishes `bot.turn` for **every** row from `db::find_bot_turns` (all bots currently on turn), not just `event.player_position`. In simultaneous-turn games the bystander bots get duplicate `bot.turn` events with an advanced `attempt` counter; the duplicates are caught downstream by is_turn/updated_at guards, so the cost is noise + redelivery burn + wrongly-advanced attempt budget for bystanders, not corruption.
- **Bot retry loop panic:** `rust/bot/src/main.rs:242` is `for attempt in 0..MAX_ATTEMPTS` (20) with `unreachable!()` at :454 after the loop. The exhaustion check at :420 (`if attempt + 1 == MAX_ATTEMPTS`) only runs on the command-rejected path. Two paths `continue` without it: LLM call error (:311, after `router.mark_failed()`) and game-state-changed refresh (:372). If iteration 19 takes either, the loop exits normally and `unreachable!()` panics the task spawned at :832 — the `JoinHandle` is dropped, so the panic is swallowed, `transaction.finish()` (:853) never runs, the "Bot turn hard failed" error log (:863) never fires, and the message is redelivered up to `max_deliver` with the same silent outcome each time. Concrete trigger: state changes mid-LLM-call on the final attempt.
- **Bot concurrency/shutdown:** the loop at :812-867 does `tokio::spawn` per message with no bound — a burst of `bot.turn` events means that many concurrent 300s-timeout LLM calls from one 128Mi pod. There is no signal handling: SIGTERM (every deploy) kills tasks mid-LLM-call; work redelivers after `ack_wait` (5 min) so nothing is lost, but LLM spend is wasted and games stall for the redelivery window. The tokio `signal` feature is already enabled in `rust/bot/Cargo.toml:12` but unused.
- **Bot healthz:** `rust/bot/src/main.rs:685-690` checks only NATS connection state. `k8s/base/bot/deployment.yaml` uses `/healthz` as a **livenessProbe only** — there is no readinessProbe and no Service (the comment in the manifest says exactly this). That fact drives the Task 7 disposition.

**Tech Stack:** Rust 1.97.0 workspace at `/home/beefsack/Development/brdgme/rust`. `web` crate (server code behind `--features ssr`): async-nats 0.49.1, axum_prometheus 0.10 / metrics 0.24, anyhow, tokio (ssr features `rt-multi-thread, macros, signal`). `bot` crate: async-nats 0.49.1, tokio features `rt-multi-thread, macros, signal` (already includes `signal`), anyhow. New deps: only `tokio` dev-dependency features for web tests (`test-util`, see Task 1); NO new crates.

**Global Constraints:**

- All cargo commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only; web needs `--features ssr`. NEVER workspace-wide builds (AGENTS.md resource constraints).
- Changes to `rust/web/src/game/mod.rs` MUST land with tests (docs/CODING.md mandatory-tests rule).
- NATS/DB-backed web integration tests (`tests/nats_bot_eventing.rs`) fail in a bare local run without the throwaway containers — pre-existing (backlog #40). The authoritative gate is `/home/beefsack/Development/brdgme/scripts/rust-test.sh`; it MUST pass before the final commit.
- `cargo fmt --all -- --check` clean after every task; clippy at `-D warnings` (`cargo clippy -p web --all-targets --features ssr -- -D warnings`, `cargo clippy -p bot --all-targets -- -D warnings`).
- **WP-38 boundary (scrupulous):** do not change WHAT gets acked, when, or with which `AckKind` in either consumer's message-outcome handling; do not add `term()`, NAK-with-backoff, DLQ subjects, re-publish-on-exhaustion, ack-heartbeats (`AckKind::Progress`), or any wedge-recovery mechanism; do not change `ack_wait`/`max_deliver` values. The only `AckKind` use this WP introduces is `Nak` inside ONE integration test to force redeliveries — test-only, not production code.
- **WP-36 coordination (main.rs):** WP-36 Task 5 edits `rust/web/src/main.rs` at the `with_graceful_shutdown` call (line 108) and appends a WS-drain block after `axum::serve(...).await`. This WP edits `main.rs:38-46` (clone the NATS client) and replaces the spawn block at :55-74. The regions are disjoint; whichever WP lands second must re-resolve line numbers (cited lines here are pre-WP-36 live lines). Do NOT wire the new supervisor/advisory tasks into WP-36's shutdown token: WP-36 explicitly decided background tasks are not signaled on shutdown (un-acked messages redeliver; safe). WP-36's spec text attributes consumer supervision to "WP-38" — stale label; it is this WP. No design conflict either way.
- k8s: no manifest changes in this WP, and the implementer never applies anything to prod.
- Migrations: none; do not create any.

**Non-Goals:**

- **WP-38 / D-5 wedge RECOVERY:** ws F27, wd F1 (UserError ack semantics), wd F2 (MAX_TURN_ATTEMPTS exhaustion recovery), wd F3 (publish-after-commit loss), wd F5 (term/DLQ for stranded messages — this WP only makes stranding VISIBLE), bo F2 (ack-heartbeat / ack_wait raise).
- **WP-36 Task 5:** web WebSocket close-frame shutdown and any other `main.rs` graceful-shutdown work for the web binary. This WP's graceful-shutdown work is the BOT binary only (bo F5).
- **WP-61 bot quality items:** bo F4 (merge_json_patch RFC), bo F6 ("(you)" marker), bo F7 (prompt trace log), bo F9+ (config error masking etc.). Also bo F16 (unused deps sweep) — though Task 6 incidentally gives the already-enabled tokio `signal` feature its intended consumer.
- ws F59-F62 (websocket pass, WP-42); wd F8 (`before`-snapshot swallow, error-swallowing package).
- Deeper health checks for the WEB pod (`/healthz` covering the consumer) — the supervisor + metric make the outage alertable without coupling liveness to NATS; revisit only if D-5 wants it.

**Snapshot drift:** Checked 2026-07-25 by `diff` against `/home/beefsack/Development/brdgme-review-snapshot/rust` (f8763a5). `web/src/main.rs`, `web/src/nats.rs`, `bot/src/main.rs`: **byte-identical** — snapshot line citations are live. `web/src/game/mod.rs`: ONE added line at :7 (`pub mod placing;`, #47 work) — every snapshot line ≥7 shifts **+1**; all `game/mod.rs` lines cited in this spec are LIVE lines (e.g. finding's ":322-324" stream-end is live :323-325; ":315-320" Other-unacked arm is live :316-321; ":390-392" fan-out is live :391-398). `web/tests/nats_bot_eventing.rs` also checked: unchanged.

---

### Task 1: ws F53 + wd F4 — supervised restart loop for the `bot.command` consumer

**Problem (restated):** `main.rs:55-74` is spawn-and-forget. Three death modes, all currently unrecoverable until pod restart: (a) `run_bot_command_consumer` returns `Err` (consumer lookup or `messages()` subscription fails — e.g. NATS-side error, consumer deleted/recreated by a racing replica) → one error log, task gone; (b) the message stream ends and the function returns `Ok(())` (live `game/mod.rs:323-325`) → **silent**, task gone; (c) the task panics → `JoinHandle` dropped, panic swallowed, task gone. `/healthz` stays green throughout. Bots never move again on that replica.

**Fix (re-derived):** In-process supervisor with exponential backoff, restart on ALL three death modes, error log + restart counter so the condition is alertable from the existing `/metrics` endpoint. Restart-in-process (not crash-the-pod): crashing would also kill HTTP serving for a consumer-only fault, and a persistent NATS outage would crashloop the web deployment; the backoff loop retries harmlessly instead (`get_consumer_from_stream` fails fast while NATS is down, each failure logged + counted). Panics are caught by wrapping each run in its own `tokio::spawn` and inspecting the `JoinError`. The supervisor is a generic helper in `nats.rs` taking a run-factory closure, so (1) it is unit-testable without NATS by injecting failing closures, and (2) Task 2 reuses it for the advisory listener. Backoff: 1s doubling to 30s cap, reset to 1s after any run that stayed alive ≥60s (so a healthy consumer that dies once after days restarts immediately, while a tight crash loop settles at one attempt per 30s — cheap enough forever, fast enough to matter).

**Files:**
- `rust/web/Cargo.toml` (dev-dependency: tokio `test-util`)
- `rust/web/src/nats.rs`
- `rust/web/src/main.rs`

**Steps:**

- [ ] In `rust/web/Cargo.toml` `[dev-dependencies]` (line 150), add:
  ```toml
  tokio = { version = "1", features = ["test-util", "rt-multi-thread", "macros", "signal"] }
  ```
  (Dev-dep features merge with the ssr dep's; `test-util` enables `tokio::test(start_paused = true)` so the backoff sleeps run in virtual time.)
- [ ] Write the failing tests. In `rust/web/src/nats.rs`, append at the bottom:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use std::sync::Arc;
      use std::sync::atomic::{AtomicUsize, Ordering};
      use std::time::Duration;

      /// review ws F53/wd F4: the supervisor must restart the consumer after
      /// an Err exit, after a clean Ok(()) stream-end exit, and after a panic.
      #[tokio::test(start_paused = true)]
      async fn supervisor_restarts_on_err_ok_and_panic() {
          let calls = Arc::new(AtomicUsize::new(0));
          let calls_clone = calls.clone();
          let sup = tokio::spawn(supervise_consumer("test-consumer", move || {
              let calls = calls_clone.clone();
              async move {
                  match calls.fetch_add(1, Ordering::SeqCst) {
                      0 => Err(anyhow::anyhow!("boom")),           // Err exit
                      1 => Ok(()),                                  // stream-end exit
                      2 => panic!("consumer task panic"),           // panic exit
                      _ => std::future::pending().await,            // stays alive
                  }
              }
          }));
          // Paused clock: sleeps auto-advance. Poll until the 4th run starts.
          tokio::time::timeout(Duration::from_secs(600), async {
              while calls.load(Ordering::SeqCst) < 4 {
                  tokio::time::sleep(Duration::from_millis(50)).await;
              }
          })
          .await
          .expect("supervisor did not restart through all three death modes");
          sup.abort();
      }

      /// The backoff must cap (not grow unboundedly) so a persistent failure
      /// keeps retrying at a bounded interval.
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
          // 20 restarts x worst-case 30s cap = 600s virtual time upper bound.
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
  ```
- [ ] Run: `cargo test -p web --features ssr nats::tests -- --nocapture` — expected: **compile error** (`supervise_consumer` does not exist). This is the red state.
- [ ] Implement in `rust/web/src/nats.rs` (below `ensure_stream_and_consumers`):
  ```rust
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
                  tracing::error!(consumer = name, "consumer exited with error: {:#}; restarting", e);
              }
              Err(join_err) => {
                  tracing::error!(consumer = name, "consumer task panicked: {}; restarting", join_err);
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
  ```
  Notes for the implementer: `Duration` is already imported in nats.rs (`use std::time::Duration;`). `tokio::time::Instant` (not `std::time::Instant`) so paused-clock tests measure virtual time. The labeled `counter!` form is metrics-0.24 syntax already in the dependency graph via `axum_prometheus`; if clippy or the compiler rejects the label form for any reason, fall back to the unlabeled `counter!("nats_consumer_restarts_total")` — the label is a nicety, not load-bearing.
- [ ] Run: `cargo test -p web --features ssr nats::tests` — expected: **2 tests PASS** (no NATS/DB needed — these are pure-tokio tests).
- [ ] In `rust/web/src/main.rs`, replace the spawn block at lines 55-74 with:
  ```rust
  tokio::spawn({
      let pool = pool.clone();
      let http_client = http_client.clone();
      let broadcaster = broadcaster.clone();
      let jetstream = jetstream.clone();
      let resend = resend.clone();
      async move {
          web::nats::supervise_consumer("bot-command", move || {
              web::game::run_bot_command_consumer(
                  pool.clone(),
                  http_client.clone(),
                  broadcaster.clone(),
                  jetstream.clone(),
                  resend.clone(),
              )
          })
          .await;
      }
  });
  ```
  (Same clones as today, moved one level up so the factory closure can re-clone per restart. `run_bot_command_consumer` itself is UNCHANGED — its `Ok(())`/`Err` exits become restart triggers instead of silent death.)
- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` — clean.
- [ ] Run: `cargo fmt --all -- --check` — clean.
- [ ] Commit: `fix(web): supervise bot.command consumer with backoff restarts + restart metric (review ws F53, wd F4, WP-39)`

### Task 2: ws F56 — max_deliver stranding visibility (advisory listener)

**Problem (restated):** a `bot.command` (or `bot.turn`) message that fails transiently on all 3 deliveries is never delivered again; WorkQueue retention only deletes on ack, so it sits in the stream forever and the bot never moves in that game — no log, no metric, no operational way to notice. The NATS server DOES emit an advisory event per exhaustion on `$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.<stream>.<consumer>` (JSON body carrying `stream`, `consumer`, `stream_seq`, `deliveries`); nothing subscribes.

**Fix (re-derived):** subscribe to `$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.BOT.*` with the plain (core NATS) client — the same `async_nats::Client::subscribe` API `websocket.rs` already uses — and on each advisory: `tracing::error!` with the parsed fields plus increment counter `bot_stream_max_deliveries_total`. That makes stranding alertable (metric) and diagnosable (`stream_seq` in the log identifies the exact stranded message for manual inspection via NATS CLI). Parsing is a separate pure function so it is unit-testable. The listener runs under Task 1's `supervise_consumer` (subscription streams can end on client close; supervision costs nothing). **Deliberately NOT here:** `term()`, DLQ, re-publish, UI surface, stream `max_age` — all WP-38/D-5 recovery design.

**Files:**
- `rust/web/src/nats.rs`
- `rust/web/src/main.rs`
- `rust/web/tests/nats_bot_eventing.rs`

**Steps:**

- [ ] Write the failing unit test. In the `nats.rs` `mod tests` from Task 1, add:
  ```rust
  /// review ws F56: the advisory payload shape emitted by nats-server for
  /// max-deliveries exhaustion (type io.nats.jetstream.advisory.v1.max_deliver).
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
  ```
- [ ] Run: `cargo test -p web --features ssr parses_max_deliveries` — expected: **compile error** (function/struct do not exist).
- [ ] Implement in `rust/web/src/nats.rs`:
  ```rust
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
  pub async fn run_max_deliveries_advisory_listener(
      client: async_nats::Client,
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
      while let Some(msg) = sub.next().await {
          match parse_max_deliveries_advisory(&msg.payload) {
              Some(adv) => {
                  axum_prometheus::metrics::counter!("bot_stream_max_deliveries_total")
                      .increment(1);
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
  ```
  (`futures_util` is already a web dependency — `run_bot_command_consumer` imports it the same way. Add `use serde::Deserialize;` only if not already imported — nats.rs line 7 already has `use serde::{Deserialize, Serialize};`.)
- [ ] Run: `cargo test -p web --features ssr parses_max_deliveries` — expected: **PASS**.
- [ ] Wire into `rust/web/src/main.rs`: at line 46 the NATS client is currently moved into the broadcaster (`let broadcaster = GameBroadcaster::new(nats_client);`). Change to keep a copy and spawn the supervised listener (place immediately after the Task 1 supervisor spawn):
  ```rust
  let advisory_client = nats_client.clone();
  let broadcaster = GameBroadcaster::new(nats_client);
  ```
  ```rust
  tokio::spawn(async move {
      web::nats::supervise_consumer("max-deliveries-advisory", move || {
          web::nats::run_max_deliveries_advisory_listener(advisory_client.clone())
      })
      .await;
  });
  ```
- [ ] Write the integration test (proves the advisory subject/shape claims against a real nats-server, and that our constant matches what the server emits). In `rust/web/tests/nats_bot_eventing.rs`, append:
  ```rust
  /// review ws F56: when a message exhausts max_deliver, the server emits a
  /// MAX_DELIVERIES advisory on the subject our listener subscribes to, with
  /// a payload our parser understands. Forces redeliveries with Nak (test-only;
  /// production code never naks — WP-38 boundary).
  #[sqlx::test]
  #[serial]
  async fn max_deliver_exhaustion_emits_parseable_advisory(pool: PgPool) {
      let _pool = pool; // JetStream-only test.
      let jetstream = make_jetstream().await;
      let nats_url =
          std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
      let core_client = async_nats::connect(&nats_url).await.unwrap();
      let mut advisories = core_client
          .subscribe(nats::MAX_DELIVERIES_ADVISORY_SUBJECT)
          .await
          .unwrap();

      // Publish a marker bot.command and remember its stream sequence.
      let marker_game_id = Uuid::new_v4();
      let event = BotCommandEvent {
          game_id: marker_game_id,
          player_position: 0,
          command: "advisory-test".to_string(),
          attempt: 0,
      };
      let ack = jetstream
          .publish(nats::SUBJECT_COMMAND, serde_json::to_vec(&event).unwrap().into())
          .await
          .unwrap()
          .await
          .unwrap();
      let our_seq = ack.sequence;

      // Pull and Nak until max_deliver (3) is exhausted. Other tests'
      // leftovers are acked and discarded, per this file's convention.
      let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
      let consumer = stream
          .get_or_create_consumer(
              nats::CONSUMER_COMMAND,
              async_nats::jetstream::consumer::pull::Config {
                  durable_name: Some(nats::CONSUMER_COMMAND.to_string()),
                  filter_subject: nats::SUBJECT_COMMAND.to_string(),
                  ..Default::default()
              },
          )
          .await
          .unwrap();
      let mut naks = 0;
      'outer: for _ in 0..10 {
          let mut messages = consumer
              .batch()
              .max_messages(20)
              .expires(Duration::from_millis(500))
              .messages()
              .await
              .unwrap();
          while let Some(Ok(message)) = messages.next().await {
              let ev: BotCommandEvent = serde_json::from_slice(&message.payload).unwrap();
              if ev.game_id == marker_game_id {
                  message
                      .ack_with(async_nats::jetstream::AckKind::Nak(None))
                      .await
                      .unwrap();
                  naks += 1;
                  if naks >= 3 {
                      break 'outer;
                  }
              } else {
                  message.ack().await.unwrap();
              }
          }
      }
      assert_eq!(naks, 3, "expected to nak the marker message max_deliver times");

      // The advisory for OUR stream_seq must arrive and parse.
      let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
      let adv = loop {
          let remaining = deadline - tokio::time::Instant::now();
          let msg = tokio::time::timeout(remaining, advisories.next())
              .await
              .expect("timed out waiting for MAX_DELIVERIES advisory")
              .expect("advisory subscription ended");
          if let Some(adv) = nats::parse_max_deliveries_advisory(&msg.payload) {
              if adv.stream_seq == our_seq {
                  break adv;
              }
          }
      };
      assert_eq!(adv.stream, nats::STREAM_NAME);
      assert_eq!(adv.consumer, nats::CONSUMER_COMMAND);
      assert_eq!(adv.deliveries, 3);

      // Cleanup: delete the deliberately-stranded message so it doesn't
      // accumulate in the shared throwaway stream across runs.
      let _ = stream.delete_message(our_seq).await;
  }
  ```
  API notes (verified against async-nats 0.49.1 source): `Message::ack_with(AckKind::Nak(None))` → immediate redelivery; `PublishAckFuture` resolves to an ack with `pub sequence: u64`; `Stream::delete_message(sequence)` exists. If `AckKind`'s import path gives trouble, it is `async_nats::jetstream::AckKind` (re-exported from `jetstream::message`).
- [ ] Run: `cargo test -p web --features ssr max_deliver_exhaustion --no-run` — expected: **compiles**. (Execution needs the NATS container; it runs under the final `rust-test.sh` gate.)
- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `feat(web): log + count JetStream MAX_DELIVERIES advisories for stranded bot messages (review ws F56 visibility half, WP-39; recovery deferred to WP-38/D-5)`

### Task 3: ws F57 + ws F58 — startup warning on NATS config drift; document the ack_wait invariant

**Problem (restated):** F57 — `get_or_create_stream`/`get_or_create_consumer` return the existing server object untouched, so editing `ack_wait`/`max_deliver`/retention/subjects in `nats.rs` is silently a no-op against any NATS deployment that already has them; code and server config diverge with no warning. F58 — `ack_wait = 5 min` must exceed the `bot.command` handler's worst case or JetStream redelivers mid-processing and the command runs twice; verification resolved this as theoretical today (full-process-then-ack with a hard 10s shared-client HTTP timeout and bounded retries — verification annotation downgraded it to nit), but the invariant lives in nobody's head.

**Fix (re-derived):** F57 — after each get-or-create, compare the server's actual config (`cached_info().config`, populated by async-nats from the server response; both config types derive `PartialEq` on their fields) against the desired config **field-by-field for exactly the fields the code sets** (whole-struct equality would false-positive on server-populated defaults like `num_replicas`/`deliver_policy`), and `tracing::warn!` each mismatch with a remediation hint. Warn, don't fail startup: failing would brick every deploy on a benign drift, which is a worse outage than the drift. Pure comparison functions keep it unit-testable without a server. F58 — document-only (any cadence/ack_wait change is WP-38/D-5 territory): a comment on the `ack_wait` binding stating the invariant and where the bound comes from.

**Files:**
- `rust/web/src/nats.rs`

**Steps:**

- [ ] Write the failing tests. In the `nats.rs` `mod tests`, add:
  ```rust
  /// review ws F57: drift between code-desired and server-actual config must
  /// be detected field-by-field (only fields the code sets).
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
      // Identical in the fields we set -> no drift, even if
      // server-populated fields we do NOT set differ.
      let mut actual = desired.clone();
      actual.num_replicas = 3; // server-populated field we do not set
      assert!(consumer_config_drift(&desired, &actual).is_empty());
      // Changed ack_wait and max_deliver → two drift entries.
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
  ```
- [ ] Run: `cargo test -p web --features ssr config_drift` — expected: **compile error** (functions do not exist).
- [ ] Implement in `rust/web/src/nats.rs`:
  ```rust
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
  pub fn consumer_config_drift(desired: &pull::Config, actual: &pull::Config) -> Vec<String> {
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
  ```
- [ ] Rework `ensure_stream_and_consumers` (lines 52-97) to bind the desired configs, pass clones to get-or-create, and warn on drift. Shape:
  ```rust
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
      let max_deliver = 3;

      for (name, subject) in [(CONSUMER_TURN, SUBJECT_TURN), (CONSUMER_COMMAND, SUBJECT_COMMAND)] {
          let desired = pull::Config {
              durable_name: Some(name.to_string()),
              filter_subject: subject.to_string(),
              ack_policy: async_nats::jetstream::consumer::AckPolicy::Explicit,
              ack_wait,
              max_deliver,
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
  ```
  Behavior is identical for the create path (same configs, same order); the loop replaces the two copy-pasted consumer blocks. `get_or_create_consumer` returns `Consumer<pull::Config>`; `cached_info()` is populated from the server's response (verified in async-nats 0.49.1 source), so on the "get existing" path it reflects the SERVER's config — exactly what drift must be measured against.
- [ ] Run: `cargo test -p web --features ssr config_drift` — expected: **2 tests PASS**.
- [ ] Run: `cargo test -p web --features ssr --no-run` (the `nats_bot_eventing.rs` tests call `ensure_stream_and_consumers` via `make_jetstream`; they must still compile) and `cargo clippy -p web --all-targets --features ssr -- -D warnings` — clean.
- [ ] Commit: `feat(web): warn on NATS stream/consumer config drift at startup; document ack_wait invariant (review ws F57, ws F58, WP-39)`

### Task 4: wd F9 — conflict re-publish targets only the conflicting bot

**Problem (restated):** in `handle_bot_command_event`'s `Conflict` arm (live `game/mod.rs:391-398`), `find_bot_turns` returns EVERY bot currently on turn and all of them get a fresh `bot.turn` with `attempt + 1`. In simultaneous-turn games the bystander bots (whose own turns may already be in flight) receive duplicate events with a wrongly-advanced attempt counter — noise, redelivery burn, and attempt-budget consumption for bots that never conflicted. Downstream guards prevent corruption, but the fan-out is wrong.

**Fix (re-derived):** filter the `find_bot_turns` result to `event.player_position` before publishing. `db::BotTurn` (db.rs:517-520) has `position: i32`; `event.player_position` is `i32` — direct comparison. Edge cases: (a) the conflicting bot is no longer on turn after the winning write → filter yields empty → no re-publish, correct: the winning writer's own `broadcast_and_trigger` → `trigger_bot_turns` publishes attempt-0 events for whoever IS now on turn; (b) single-bot games → filter is a no-op, existing behavior and the existing `stale_conflict_republishes_bot_turn_with_incremented_attempt` test still pass; (c) exhaustion path (`attempt >= MAX_TURN_ATTEMPTS`, live :374-384) is untouched — WP-38 territory.

**Files:**
- `rust/web/src/game/mod.rs`
- `rust/web/tests/nats_bot_eventing.rs`

**Steps:**

- [ ] Write the failing test. In `rust/web/tests/nats_bot_eventing.rs`, add a two-bot game helper and the test:
  ```rust
  /// One human (creator) plus two bot players, pointed at `uri`.
  async fn make_game_with_two_bots(pool: &PgPool, uri: &str) -> Uuid {
      let p0 = make_user(pool, "p0").await;
      let game_version_id = make_game_version(pool, uri).await;
      let game = db::create_game_with_users(
          pool,
          CreateGameOpts {
              game_version_id,
              whose_turn: &[0],
              eliminated: &[],
              placings: &[],
              points: &[],
              creator_id: p0.id,
              opponent_ids: &[],
              opponent_emails: &[],
              bot_slots: &[
                  db::BotSlot {
                      name: "Bot A".to_string(),
                      bot_name: "easy".to_string(),
                  },
                  db::BotSlot {
                      name: "Bot B".to_string(),
                      bot_name: "easy".to_string(),
                  },
              ],
              chat_id: None,
              game_state: "initial_state",
              all_accepted: false,
          },
      )
      .await
      .unwrap();
      game.id
  }

  /// review wd F9: a stale-state conflict must re-publish bot.turn only for
  /// the conflicting event's position, not fan out to every bot on turn.
  #[sqlx::test]
  #[serial]
  async fn conflict_republish_targets_only_the_conflicting_bot(pool: PgPool) {
      let jetstream = make_jetstream().await;
      let http_client = reqwest::Client::new();
      let broadcaster = make_broadcaster().await;

      let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
      let addr = listener.local_addr().unwrap();
      let uri = format!("http://{}", addr);
      let game_id = make_game_with_two_bots(&pool, &uri).await;
      // Both bots simultaneously on turn (seating is shuffled; look them up).
      let bot_positions: Vec<i32> = sqlx::query_scalar!(
          "SELECT position FROM game_players WHERE game_id = $1 AND game_bot_id IS NOT NULL ORDER BY position",
          game_id
      )
      .fetch_all(&pool)
      .await
      .unwrap();
      assert_eq!(bot_positions.len(), 2);
      sqlx::query!(
          "UPDATE game_players SET is_turn = (position = ANY($2)) WHERE game_id = $1",
          game_id,
          &bot_positions
      )
      .execute(&pool)
      .await
      .unwrap();

      // Mock game service that always induces a stale-state conflict.
      let pool_for_handler = pool.clone();
      let app = Router::new().route(
          "/",
          post(move |Json(_req): Json<Request>| {
              let pool = pool_for_handler.clone();
              async move {
                  sqlx::query!("UPDATE games SET updated_at = NOW() WHERE id = $1", game_id)
                      .execute(&pool)
                      .await
                      .unwrap();
                  Json(play_response("new_state", vec![0], true))
              }
          }),
      );
      tokio::spawn(async move {
          axum::serve(listener, app).await.unwrap();
      });

      let conflicting_pos = bot_positions[0];
      let event = BotCommandEvent {
          game_id,
          player_position: conflicting_pos,
          command: "abc".to_string(),
          attempt: 0,
      };
      let _ = handle_bot_command_event(&pool, &http_client, &broadcaster, &jetstream, &None, &event)
          .await;

      let stream = jetstream.get_stream(nats::STREAM_NAME).await.unwrap();
      let consumer = stream
          .get_or_create_consumer(
              nats::CONSUMER_TURN,
              async_nats::jetstream::consumer::pull::Config {
                  durable_name: Some(nats::CONSUMER_TURN.to_string()),
                  filter_subject: nats::SUBJECT_TURN.to_string(),
                  ..Default::default()
              },
          )
          .await
          .unwrap();
      let events = drain_bot_turn_events(&consumer, game_id, 20, Duration::from_secs(5)).await;
      assert_eq!(
          events.len(),
          1,
          "conflict must re-publish only the conflicting bot's turn, got {:?}",
          events
      );
      assert_eq!(events[0].player_position, conflicting_pos);
      assert_eq!(events[0].attempt, 1);
  }
  ```
- [ ] Run: `cargo test -p web --features ssr conflict_republish_targets --no-run` — expected: **compiles** (nothing new referenced). Then note: executed under containers this test FAILS today with `events.len() == 2` (both bots re-published) — that failure is the red state; if you have the containers running (`scripts/rust-test.sh` environment), run `cargo test -p web --features ssr conflict_republish_targets` and confirm the 2-events failure before implementing.
- [ ] Implement in `rust/web/src/game/mod.rs`, `Conflict` arm (live lines 391-398). Replace:
  ```rust
  match crate::db::find_bot_turns(pool, event.game_id).await {
      Ok(turns) => {
          publish_bot_turns(jetstream, event.game_id, &turns, attempt + 1).await;
      }
  ```
  with:
  ```rust
  match crate::db::find_bot_turns(pool, event.game_id).await {
      Ok(turns) => {
          // Re-publish only for the bot whose command conflicted; bystander
          // bots on turn (simultaneous-turn games) keep their own in-flight
          // turns and their own attempt budgets (review wd F9). If the
          // conflicting bot is no longer on turn, the winning write's own
          // trigger_bot_turns has already published fresh attempt-0 events.
          let conflicting: Vec<crate::db::BotTurn> = turns
              .into_iter()
              .filter(|t| t.position == event.player_position)
              .collect();
          publish_bot_turns(jetstream, event.game_id, &conflicting, attempt + 1).await;
      }
  ```
  (Err arm at live :395-397 unchanged.)
- [ ] Run (containers): `cargo test -p web --features ssr conflict_republish_targets` — expected: **PASS**; also `cargo test -p web --features ssr stale_conflict_republishes` and `attempt_limit_exhaustion_gives_up` — still **PASS** (single-bot filter is a no-op; exhaustion path untouched). Without containers, defer execution to the final gate and verify compile + clippy now.
- [ ] Run: `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `fix(web): re-publish bot.turn only for the conflicting position on stale-state conflict (review wd F9, WP-39)`

### Task 5: bo F1 — replace the reachable `unreachable!()` with an error

**Problem (restated):** `rust/bot/src/main.rs:242-455`. The retry loop's exhaustion check (:420) only guards the command-rejected path; the LLM-error `continue` (:311) and the state-refresh `continue` (:372) can consume the final iteration (attempt 19), after which the `for` loop exits normally and hits `unreachable!()` (:454). The panic kills the task spawned at :832 whose `JoinHandle` is dropped — so it is swallowed: no "bot_turn_end" log, no "Bot turn hard failed" log, the Sentry transaction at :853 never finishes, and the message silently redelivers (up to `max_deliver`) into the same panic. Concrete trigger: 19 rejected commands, then the game state changes during attempt 19's LLM call.

**Fix (re-derived):** exhausting the loop without returning is a legitimate outcome (all-attempts-consumed via mixed continue paths), so make it a normal error return. `run_bot_turn` already returns `anyhow::Result<()>`; the consumer's existing `Err` arm (:860-864) then logs "Bot turn hard failed" and leaves the message unacked for redelivery — the same, already-designed failure path the :430-435 exhaustion return uses. No control-flow restructure (the finding's "move the check to the top" alternative is more invasive for zero behavioral gain — every non-final iteration behaves identically either way). What happens after redelivery ALSO exhausts is WP-38/D-5's wedge-recovery problem, not this fix's.

**Files:**
- `rust/bot/src/main.rs`

**Steps:**

- [ ] In `rust/bot/src/main.rs`, replace line 454:
  ```rust
  unreachable!()
  ```
  with:
  ```rust
  // Reachable: the final attempt can be consumed by an LLM-call failure or
  // a mid-turn game-state refresh (`continue` paths above), not only by a
  // rejected command. Return the same controlled failure the rejected-command
  // exhaustion path uses: the consumer logs it and leaves the message
  // unacked for redelivery (review bo F1).
  Err(anyhow!(
      "Bot turn gave up after {} attempts without submitting a command \
       (final attempt consumed by an LLM failure or a game-state refresh)",
      MAX_ATTEMPTS
  ))
  ```
- [ ] Run: `cargo test -p bot` — expected: **PASS** (existing `merge_json_patch` unit tests; nothing else is unit-testable here — `run_bot_turn` needs DB + game service + LLM, and the bot crate has no integration harness; exercising a 20-attempt loop end-to-end is out of proportion for this fix. Honest status: this change is verified by review + compile + the type system, not by an automated test).
- [ ] Run: `cargo clippy -p bot --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `fix(bot): replace reachable unreachable!() in turn retry loop with a proper error (review bo F1, WP-39)`

### Task 6: bo F3 + bo F5 — bound in-process concurrency; drain in-flight turns on SIGTERM

**Problem (restated):** F3 — the consumer loop (`bot/src/main.rs:812-867`) does `tokio::spawn` per message with no local bound; the only backpressure is the consumer's server-side `max_ack_pending` (not set by the monolith's config, so server default), so a burst of `bot.turn` events means that many concurrent 300s-timeout LLM calls + DB + game-service requests from one 20m-CPU/128Mi pod. F5 — no signal handling at all: SIGTERM (every deploy/reschedule) hard-kills tasks minutes into an LLM call; the un-acked message redelivers after `ack_wait` so nothing is lost, but the LLM spend is wasted and the game stalls for the redelivery window. The tokio `signal` feature is already enabled (`bot/Cargo.toml:12`) — this was intended and never finished.

**Fix (re-derived):** one `Arc<Semaphore>` serves both findings, no new dependencies. Concurrency bound: acquire an owned permit BEFORE pulling the next message is processed (`acquire_owned` awaited in the loop → when saturated, the loop stops spawning and stops pulling; un-pulled messages simply wait in the stream — natural backpressure, no local queue). Size from `MAX_CONCURRENT_TURNS` env (default 8 — generous for a pod this size while capping a stampede; operators tune via the deployment env). Graceful shutdown: a `shutdown_signal()` future (SIGTERM + ctrl-c, same shape as web's `main.rs:198-218`); `tokio::select!` it against both await points (next-message and permit-acquire); on signal, break out — any message already pulled but not yet spawned stays un-acked and redelivers — then drain by `acquire_many(max_concurrent)` on the semaphore, which resolves exactly when every in-flight turn has finished (each task holds its permit until it completes and acks/leaves-unacked). No hard timeout: kubelet's SIGKILL at the end of the termination grace period is the backstop (k8s default 30s; in-flight LLM calls longer than the grace period are killed exactly as today, so this is a strict improvement; raising `terminationGracePeriodSeconds` is an operator decision outside this WP). Cancel-safety: `futures_util::StreamExt::next` (already the loop's await) is cancel-safe — dropping the `Next` future cannot lose a yielded item; `acquire_owned` is likewise cancel-safe (dropping it does not consume a permit). WP-38 boundary: ack/nack behavior inside the per-message task is UNTOUCHED.

**Files:**
- `rust/bot/src/main.rs`

**Steps:**

- [ ] Add imports at the top of `rust/bot/src/main.rs` (near the existing `use std::time::Instant;`): `use std::sync::Arc;` and `use tokio::sync::Semaphore;` (tokio `sync` is in the default feature set pulled by `rt-multi-thread`; if the compiler disagrees, add `"sync"` to the tokio features in `bot/Cargo.toml`).
- [ ] Add the signal helper (place near `serve_health`):
  ```rust
  /// Resolves on SIGTERM or ctrl-c. Same shape as web's shutdown_signal;
  /// this is what the long-enabled tokio "signal" feature was for.
  async fn shutdown_signal() {
      use tokio::signal;

      let ctrl_c = async {
          signal::ctrl_c()
              .await
              .expect("failed to install Ctrl+C handler");
      };
      let terminate = async {
          signal::unix::signal(signal::unix::SignalKind::terminate())
              .expect("failed to install SIGTERM handler")
              .recv()
              .await;
      };
      tokio::select! {
          _ = ctrl_c => {},
          _ = terminate => {},
      }
  }
  ```
- [ ] Rework the consumer loop in `main` (lines 807-869). Before the loop (after `let mut messages = consumer.messages().await?;` at :808):
  ```rust
  const DEFAULT_MAX_CONCURRENT_TURNS: usize = 8;
  let max_concurrent: usize = std::env::var("MAX_CONCURRENT_TURNS")
      .ok()
      .and_then(|v| v.parse().ok())
      .filter(|&n| n > 0)
      .unwrap_or(DEFAULT_MAX_CONCURRENT_TURNS);
  // One permit per in-flight turn: bounds concurrent LLM calls from this
  // pod (review bo F3) and doubles as the shutdown drain — acquiring all
  // permits back means every in-flight turn has finished (review bo F5).
  let turn_permits = Arc::new(Semaphore::new(max_concurrent));
  let shutdown = shutdown_signal();
  tokio::pin!(shutdown);

  tracing::info!(max_concurrent, "Bot subscribed to bot.turn, waiting for messages");
  ```
  (This replaces the existing plain "Bot subscribed..." log at :810.) Then replace the `while let Some(message) = messages.next().await { ... }` loop (:812-867) with:
  ```rust
  loop {
      let message = tokio::select! {
          _ = &mut shutdown => break,
          maybe = messages.next() => match maybe {
              Some(m) => m,
              None => {
                  // Stream ended: fall through to the drain, then let main
                  // return — the container restarts (restartPolicy: Always),
                  // which is this binary's supervision story.
                  tracing::error!("bot.turn message stream ended");
                  break;
              }
          },
      };
      let message = match message {
          Ok(m) => m,
          Err(e) => {
              tracing::warn!("Failed to pull bot.turn message: {}", e);
              continue;
          }
      };
      let event: BotTurnEvent = match serde_json::from_slice(&message.payload) {
          Ok(e) => e,
          Err(e) => {
              tracing::error!("Failed to parse bot.turn payload: {}", e);
              if let Err(e) = message.ack().await {
                  tracing::warn!("Failed to ack unparseable bot.turn message: {}", e);
              }
              continue;
          }
      };

      let permit = tokio::select! {
          _ = &mut shutdown => {
              // Not spawned and not acked: redelivered after ack_wait.
              break;
          }
          permit = turn_permits.clone().acquire_owned() => {
              permit.expect("turn semaphore is never closed")
          }
      };

      let state = state.clone();
      tokio::spawn(async move {
          let _permit = permit; // held for the turn's full duration
          // ... EXISTING task body from the current :833-865 UNCHANGED:
          // trace_id, header_pairs, sentry transaction, run_bot_turn,
          // ack-on-Ok / leave-unacked-on-Err ...
      });
  }

  // Shutdown/stream-end drain: wait for every in-flight turn to finish so
  // completed work is acked instead of redelivered and re-billed. Bounded
  // externally by the pod's termination grace period (SIGKILL backstop).
  tracing::info!(
      in_flight = max_concurrent - turn_permits.available_permits(),
      "draining in-flight bot turns before exit"
  );
  let _ = turn_permits.acquire_many(max_concurrent as u32).await;
  tracing::info!("all in-flight bot turns complete; exiting");
  ```
  Implementer notes: (1) the `&mut shutdown` arms — after `shutdown` completes once, both selects `break` immediately, so the completed future is never polled again (no re-poll panic). (2) `acquire_many` takes `u32`; `max_concurrent` is operator-supplied small — a plain `as u32` cast is fine, but clamp in the env parse (`.filter(|&n| n > 0 && n <= 1024)`) if clippy complains about the cast. (3) Do NOT move or edit the spawned task body — the WP-38 boundary lives inside it.
- [ ] Run: `cargo test -p bot` — expected: **PASS** (unit tests unaffected). Honest status: the select/drain flow itself has no automated test — the bot crate has no NATS/DB harness, and standing one up is out of scope for this WP. Manual verification (optional, needs Tilt/kind which is NOT allowed on <32GB machines per AGENTS.md — only do this if the environment supports it): `kubectl rollout restart deployment/bot` while a turn is in flight and confirm the "draining in-flight bot turns" / "all in-flight bot turns complete" log pair.
- [ ] Run: `cargo clippy -p bot --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `feat(bot): bound turn concurrency with a semaphore and drain in-flight turns on SIGTERM (review bo F3, bo F5, WP-39)`

### Task 7: bo F8 — healthz DB-check recommendation: DECLINE, record rationale in code

**Problem (restated):** `/healthz` (`bot/src/main.rs:685-690`) reports only NATS connection state; the finding recommends adding a DB pool probe "if the deployment uses /healthz for readiness".

**Disposition (re-derived — recommendation OVERTURNED):** the deployment (`k8s/base/bot/deployment.yaml`) uses `/healthz` as a **livenessProbe only** — there is no readinessProbe and no Service (the manifest's own comment says so; the bot takes no traffic to gate). A liveness probe must not depend on external dependencies: failing liveness restarts the pod, and restarting cannot heal a DB outage — it would just crashloop the bot (and mask the actual DB alert) while sqlx's `PgPool` already reconnects by itself once the DB returns. The NATS-state check is the correct liveness content (a wedged NATS client is exactly what a restart fixes). Adding a `/readyz` would gate nothing (no Service). So: no behavioral change; document the decision at the probe so the next reader doesn't "fix" it.

**Files:**
- `rust/bot/src/main.rs`

**Steps:**

- [ ] Add a doc comment on `healthz` (:685):
  ```rust
  /// Liveness content only (k8s/base/bot/deployment.yaml wires /healthz as a
  /// livenessProbe; there is no readinessProbe and no Service). NATS state is
  /// deliberately the ONLY check: a wedged NATS client is fixed by a restart,
  /// whereas a DB outage is not — probing the pool here would crashloop the
  /// pod against a down database while sqlx's PgPool reconnects on its own
  /// (review bo F8: DB-check recommendation declined for a liveness probe).
  ```
- [ ] Run: `cargo clippy -p bot --all-targets -- -D warnings` and `cargo fmt --all -- --check` — clean.
- [ ] Commit: `docs(bot): record why /healthz deliberately excludes the DB pool (review bo F8 declined, WP-39)`

### Task 8: Final gate

- [ ] Run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — expected: full pass (spins up Postgres/NATS containers; runs migrations, fmt check, both clippy splits, sqlx prepare check, `cargo test --workspace --exclude web` — which includes `-p bot` — and `cargo test -p web --features ssr` — which executes the new `nats::tests` unit tests, `max_deliver_exhaustion_emits_parseable_advisory`, `conflict_republish_targets_only_the_conflicting_bot`, and the pre-existing `nats_bot_eventing` suite). Mandatory before the package is considered done (AGENTS.md).
- [ ] If WP-36 has landed by execution time: re-resolve `main.rs` line numbers in Tasks 1-2 (its Task 5 edits lines 104-110ff) and confirm the supervisor/advisory spawns sit BEFORE the `axum::serve` call and are NOT wired into `begin_shutdown`/`drain_ws_tasks` (deliberate — see Global Constraints).
- [ ] Report to the user: prod picks these up on the next web + bot image deploys; new alertable metrics are `nats_consumer_restarts_total` and `bot_stream_max_deliveries_total` on web's private `:9090` `/metrics`; `MAX_CONCURRENT_TURNS` env (default 8) is now honored by the bot pod. No k8s manifest changes shipped or required.

---

## Findings disposition

| Finding | Severity | Disposition | Notes |
|---|---|---|---|
| ws F53 — bot.command consumer unsupervised; silent permanent bot outage | major | FIX (Task 1) | Verification strengthened it: `Ok(())` stream-end exit (live `game/mod.rs:323-325`) is not even logged; panic swallowed via dropped JoinHandle. Chose in-process supervisor (restart loop + backoff + `nats_consumer_restarts_total` metric) over the finding's crash-the-pod alternative: crashing kills HTTP serving for a consumer-only fault and would crashloop the whole web deployment during a NATS outage. All three death modes (Err/Ok/panic) restart. |
| wd F4 — consumer spawned once, never restarted on stream end or error | major | FIX (Task 1) | Same defect as ws F53, one fix. Both the `Err` branch and the `Ok(())` stream-end restart, exactly as the finding recommends. |
| ws F56 — max_deliver=3 exhaustion strands silently | minor | FIX visibility half (Task 2); recovery half OUT OF SCOPE (WP-38/D-5) | Verification resolved the UNCERTAIN: consumer acks all poison classes; stranding is limited to 3x-transient failures — minor stands. Shipped: MAX_DELIVERIES advisory listener (server-emitted `$JS.EVENT.ADVISORY.CONSUMER.MAX_DELIVERIES.BOT.*`) → error log with `stream_seq` + `bot_stream_max_deliveries_total` counter, supervised by Task 1's loop, advisory shape proven by integration test against real nats-server. NOT shipped (boundary): `term()`, DLQ, re-publish, `max_age`, admin surface. |
| ws F57 — get_or_create never reconciles config drift | minor | FIX (Task 3) | Warn-at-startup via field-by-field compare of `cached_info().config` against desired (whole-struct equality would false-positive on server-populated defaults — refinement over the finding's wording). Warn, not fail-startup: bricking deploys on benign drift is a worse outage than drift. Manual delete/recreate remediation stated in the warning itself. |
| ws F58 — ack_wait=5min may be shorter than processing | nit (verification downgraded from minor) | DOCUMENT-ONLY (Task 3) | Verification: full-process-then-ack with hard 10s HTTP timeout and bounded retries — exceeding 5 min is implausible today. Invariant + WP-38/D-5 pointer recorded as a comment on the `ack_wait` binding. Any cadence/heartbeat change is D-5 territory (bo F2). |
| wd F9 — conflict re-publish fans out to all bots on turn | nit | FIX (Task 4) | Filter `find_bot_turns` to `event.player_position` exactly as recommended. Empty-after-filter case verified safe: the winning write's `trigger_bot_turns` publishes fresh attempt-0 events. New two-bot integration test; existing single-bot conflict tests unaffected. |
| bo F1 — reachable `unreachable!()` panics the spawned task on the final attempt | major | FIX (Task 5) | Minimal-fix variant of the recommendation: replace `unreachable!()` with `Err(...)` (finding's loop-restructure alternative declined — more churn, zero behavioral gain). Routes into the consumer's existing hard-fail arm: logged, Sentry transaction finished, message left unacked for redelivery. No automated test (no bot integration harness) — stated honestly. |
| bo F3 — unbounded tokio::spawn per message | minor | FIX (Task 6) | `Arc<Semaphore>` permit acquired before spawn; saturation stops pulling (backpressure into the stream, no local queue). `MAX_CONCURRENT_TURNS` env, default 8. |
| bo F5 — no graceful shutdown; SIGTERM aborts mid-LLM-call | minor | FIX (Task 6) | SIGTERM/ctrl-c select against both loop await points; drain via `acquire_many(max_concurrent)` on the same semaphore (no tokio-util dep needed, contra the finding's TaskTracker suggestion). No internal timeout — kubelet SIGKILL is the backstop. Also finally uses the long-enabled tokio `signal` feature (cross-ref bo F16/WP-61). |
| bo F8 — /healthz checks NATS only, not DB | nit | DECLINE, document (Task 7) | Recommendation OVERTURNED on live-manifest evidence: `k8s/base/bot/deployment.yaml` wires `/healthz` as livenessProbe ONLY (no readinessProbe, no Service — the finding's own "if used for readiness" condition does not hold). DB in a liveness probe = crashloop against a down DB that a restart cannot heal, while PgPool self-reconnects. Rationale recorded as a doc comment on `healthz`. |
