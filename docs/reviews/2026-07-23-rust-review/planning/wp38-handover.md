# WP-38 handover (verified against live code by survey Worker)

Work package: bot-turn wedge recovery. Decisions: D-05 (C-lite MODIFIED),
N-1 (15-min threshold, 60s Progress), D-08 (tolerate-on-read dangling no-op).
WP-37 has landed (admin.rs #[server] fns are thin wrappers over plain helpers
gated by `require_admin`). WP-39 has landed (nats.rs advisory listener +
`bot_stream_max_deliveries_total` already present - DO NOT touch those).

ONE commit at the end naming WP-38. Do NOT commit per-task. Do NOT push.
Commit style from log: `fix(web): <summary> (WP-38)`.

## Verified symbol locations (navigate by name, not line number)

### rust/web/src/game/mod.rs
- `handle_bot_command_event(pool, http_client, broadcaster, jetstream, resend, event) -> Result<(), ExecuteCommandError>`.
  - UserError arm currently: `tracing::warn!(...); Err(ExecuteCommandError::UserError(msg))`.
  - Conflict arm: `if attempt >= crate::nats::MAX_TURN_ATTEMPTS { tracing::error!(... "Bot turn exhausted state-conflict retries, giving up"); return Ok(()); }` then re-publishes via `find_bot_turns` + filter position + `publish_bot_turns(jetstream, game_id, &conflicting, attempt + 1)`.
- `run_bot_command_consumer(pool, http_client, broadcaster, jetstream, resend) -> anyhow::Result<()>`.
  - Arms: `Ok(()) | Err(Conflict) => message.ack()`; `Err(UserError(_)) => warn!; message.ack()`; `Err(Other(_)) => warn!("Leaving bot.command message unacked for redelivery...")`.
- `publish_bot_turns(jetstream, game_id, turns: &[crate::db::BotTurn], attempt: i32)` - PRIVATE (no pub). Two warn failure arms (publish-not-acked; failed-to-publish). A third arm (serialize fail) already uses error! + continue - leave it.
- `trigger_bot_turns(pool, jetstream, game_id)` - pub. `find_bot_turns` Err arm: `tracing::warn!(%game_id, "Failed to query bot turns: {}", e)`.

### rust/web/src/nats.rs
- `pub const MAX_TURN_ATTEMPTS: i32 = 3;` (exists).
- `ensure_stream_and_consumers(js) -> Result<()>`: local `let ack_wait = Duration::from_secs(5 * 60);` and `let max_deliver = 3;`. NO `MAX_DELIVER` const yet.
- Comment near ack_wait: "Do NOT lower this, and revisit alongside any ack-cadence change (WP-38 / D-5)". Do NOT change ack_wait or max_deliver VALUES.

### rust/web/src/email/sweep.rs
- `pub const DEFAULT_REMINDER_THRESHOLD: Duration = from_secs(86400);` and `pub const DEFAULT_SWEEP_INTERVAL: Duration = from_secs(900);` (pattern to copy).
- `pub fn spawn_turn_reminder_sweep(pool: PgPool, resend, http_client)` - shape to copy: reads interval, `tokio::spawn`, `tokio::time::interval`, `set_missed_tick_behavior(MissedTickBehavior::Skip)`, loop tick + sweep_once.
- `pub fn spawn_periodic_sweeps(pool: PgPool, resend, http_client, broadcaster)` - spawns 5 sweeps. ADD a `jetstream: async_nats::jetstream::Context` param and spawn the new bot-turn sweep.
- Tests: `#[cfg(all(test, feature = "ssr"))] mod tests`. Existing `#[sqlx::test]` fns use the pool fixture pattern.
- Production `fetch_candidates` gates on `u.reminder_emails_enabled` and excludes bots (`gp.game_bot_id IS NULL`). The NEW bot sweep must NOT consult `turn_emails_enabled` (D-11) - it is a separate query.

### rust/web/src/email/outbound.rs
- Counter pattern: `axum_prometheus::metrics::counter!("game_emails_sent_total").increment(1);`
- `pub fn parse_duration(raw: &str) -> Option<std::time::Duration>` (pub).

### rust/web/src/main.rs
- Calls `web::email::sweep::spawn_periodic_sweeps(pool.clone(), resend.clone(), http_client.clone(), broadcaster.clone());`. `jetstream` is in scope (a `async_nats::jetstream::Context`). Pass `jetstream.clone()`.

### rust/bot/src/main.rs
- Per-message `tokio::spawn(async move { let _permit = permit; ... let result = run_bot_turn(&state, event, trace_id).await; ... match result { Ok(()) => message.ack()..., Err(e) => error!(...) } })`. `message` moved by value, type `async_nats::jetstream::consumer::pull::Message`.
- `run_bot_turn(state: &AppState, req: BotTurnEvent, trace_id: Uuid) -> Result<()>`.
- Dangling skip: `tracing::info!(elapsed_ms=..., outcome="skipped", reason="bot not found or disabled", "bot_turn_end"); return Ok(());` - raise info! -> warn!.
- Tests: plain `#[cfg(test)] mod tests` (NO ssr feature on bot crate).

### rust/web/src/admin.rs
- `pub async fn list_bots(pool: &sqlx::PgPool) -> Result<Vec<BotRow>, ServerFnError>` - stratum-2 helper pattern to copy.
- `async fn require_admin(pool, context: &'static str) -> Result<(), ServerFnError>` - every #[server] fn calls `require_admin(&pool, "<fn>: ...").await?;`.
- `BotsSection(bots: Vec<BotRow>, version: RwSignal<u32>) -> impl IntoView` - renders `<h2>"Bots"</h2>` then `<table class="admin-table">`. Banner goes above the table.
- **CRITICAL guard test:** `every_admin_server_fn_calls_require_admin` hardcodes `assert_eq!(server_fns, 15, ...)`. Adding one #[server] fn REQUIRES bumping 15 -> 16, and the new fn MUST call require_admin (the test also asserts server_fns == gate count).
- `test_admin_list_bots_rejects_non_admin` is THIN: inserts a non-admin user, asserts `crate::db::is_user_admin` is false. Match this shape for the new rejection test.

### rust/web/tests/nats_bot_eventing.rs
- Helpers: `make_game_with_human_and_bot(pool, uri) -> Uuid` (integration variant), `drain_bot_turn_events(consumer, game_id, max, timeout) -> Vec<BotTurnEvent>`, `spawn_mock_game_service(handler) -> String`.
- Existing tests: `stale_conflict_republishes_bot_turn_with_incremented_attempt`, `attempt_limit_exhaustion_gives_up`. Tests are `#[serial]` (serial_test), each asserts only on own game_id.

## DB schema (verified)
- `game_players.is_turn` bool NOT NULL; `game_players.is_turn_at` timestamp NOT NULL.
- `game_players.game_bot_id` UUID FK->game_bots (bot/human discriminator; NULL = human).
- `game_bots.bot_name` TEXT = bot TYPE (NOT `game_bots.name`, which is per-game display name).
- `bots.name` TEXT UNIQUE; `bots.enabled` BOOLEAN DEFAULT true.
- `games.is_finished` bool NOT NULL (indexed). Unfinished = `is_finished = false`.
- Bot sweep join: `game_players gp JOIN game_bots gb ON gp.game_bot_id = gb.id JOIN bots b ON gb.bot_name = b.name`, filter `gp.is_turn = true`, `g.is_finished = false`, `gp.is_turn_at < now() - threshold`.
- Mirror existing `find_bot_turns` in db/bots.rs (`SELECT gp.position, gb.bot_name ... JOIN game_bots gb ON gp.game_bot_id = gb.id WHERE gp.game_id = $1 AND gp.is_turn = true`).

## Counters to add (axum_prometheus pattern)
- `bot_turn_wedge_total` (game/mod.rs: UserError-at-cap + Conflict exhaustion).
- `bot_turn_publish_failures_total` (game/mod.rs: publish_bot_turns both arms + find_bot_turns Err arms in trigger_bot_turns and Conflict arm).
- `bot_command_terminated_total` (game/mod.rs: Other arm at MAX_DELIVER ceiling, with AckKind::Term).
- `bot_turn_sweep_republished_total` (sweep.rs: re-publish for enabled bot).
- `bot_turn_dangling_bot_names` GAUGE (sweep.rs: count of dangling names; use `axum_prometheus::metrics::gauge!`).

## STOP-AND-REPORT triggers (do not improvise)
- If a named symbol is missing or shaped differently than above.
- If `every_admin_server_fn_calls_require_admin` is not the literal-15 guard described.
- If making `publish_bot_turns` pub(crate) breaks an unexpected caller.
