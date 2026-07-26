# WP-38: bot-turn wedge recovery

**Findings:** ws F27 (verification ADJUSTED minor -> major), wd F1, wd F2, wd F3
(major), wd F5 (minor), bo F2 (major).

**Decision D-5: C-lite, MODIFIED** - reconciliation sweep + retry-exhaustion
alert + `AckKind::Progress` heartbeat, but **bots stay referenced BY NAME**. A
bot player name resolving to nothing (deleted, renamed away, disabled) is an
explicitly **SUPPORTED no-op state**: the game must not wedge, the message is
acked, the condition is surfaced on the admin page, and it is never retried
forever. "All bots disabled" is a valid intentional configuration and must not
trip alerts or blocking validation. **No bot-id migration; no migration at all.**

**Landing order: WP-37 must land first** (both touch `rust/web/src/admin.rs`,
and WP-37 Task 1 reshapes every `#[server]` fn there). If WP-38 lands first,
WP-37 Tasks 6-7 must be re-derived. **WP-46 also owns
`rust/web/src/email/sweep.rs`** (blocked on D-11): WP-38's edit there is purely
additive (a new sweep + one extra `spawn_periodic_sweeps` param), so either
order works, but whichever lands second must rebase, not fork, the sweep
scaffolding.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This tree is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

In `rust/web/src/game/mod.rs` unless stated - four ways a game wedges silently:

- **wd F1** - `handle_bot_command_event` returns `UserError` bare and
  `run_bot_command_consumer` acks it. The bot stays `is_turn = true` and nothing
  re-drives it.
- **wd F2** - the `Conflict` arm's `MAX_TURN_ATTEMPTS` exhaustion branch logs an
  error and returns `Ok(())` (acked). Same wedge, no alertable signal.
- **wd F3** - `publish_bot_turns` only `warn!`s when the publish or its
  persistence ack fails; `trigger_bot_turns` and the `Conflict` arm only `warn!`
  on `db::find_bot_turns` failure. The commit already happened, so the game sits
  with a bot on turn and no event in the stream.
- **wd F5** - `term()` is never called anywhere in `rust/web/src`; a
  `bot.command` failing with `Other` on every delivery stops being delivered and,
  under WorkQueue retention, sits in the stream forever.
- **ws F27** - games reference bots by `game_bots.bot_name` -> `bots.name`
  (`db::find_bot_turns` joins on it); admin rename/delete/disable breaks
  resolution for in-flight games.
- **bo F2** - `rust/bot/src/main.rs`'s loop acks only after `run_bot_turn`
  returns; a turn can exceed `ack_wait`, so JetStream redelivers while the first
  task runs and two copies can publish `bot.command`.

## 2. Why it's wrong

- **wd F1, F2, F3, F5 are correct as written.** Verified live: the three ack arms
  are exactly as described, the exhaustion `return Ok(())` is present,
  `publish_bot_turns` has two `warn!` failure arms and no error propagation, and
  `grep -rn "\.term()" web/src` returns nothing.
- **ws F27 is correct and its UNCERTAIN is resolved:** `run_bot_turn` logs
  `outcome = "skipped", reason = "bot not found or disabled"` and returns
  `Ok(())` when `config::load_bot_config` yields `None` with a non-empty `bots`
  table - acked, turn never taken. **Its structural recommendation (bot ids via
  migration) is REJECTED by D-5**; implement the "warn" half only.
- **bo F2 is correct**, UNCERTAIN resolved: `ack_wait` is
  `Duration::from_secs(5 * 60)` in `nats::ensure_stream_and_consumers`, below the
  bot's worst-case turn (per-attempt LLM timeout x attempt budget).
- **wd F3's "surface publish failures as Err" alternative is DECLINED:**
  `execute_command` has already committed, so leaving the message unacked re-runs
  the whole command on redelivery against advanced state. The sweep is the
  recovery mechanism; publish failure gets a loud signal, not a retry.

## 3. Required end state

Counters follow `axum_prometheus::metrics::counter!("name").increment(1)`
(cf. `rust/web/src/email/outbound.rs`).

### 3a. `rust/web/src/email/sweep.rs` - reconciliation sweep (safety net)

A new sweep in the exact shape of `spawn_turn_reminder_sweep` / `sweep_once`
(`pub const DEFAULT_*` threshold + interval, env override via
`crate::email::outbound::parse_duration`, `tokio::time::interval` with
`MissedTickBehavior::Skip`), registered in `spawn_periodic_sweeps`. That fn gains
a `jetstream` param; `rust/web/src/main.rs` passes `jetstream.clone()`.

Each tick, find every **unfinished** game with `game_players.is_turn = true` on a
bot player whose `is_turn_at` is older than the threshold (default 15 min),
joining `game_bots` -> `bots` on name:

- enabled `bots` row -> re-publish `bot.turn` at `attempt = 0` for that position
  via `game::publish_bot_turns` (make it `pub(crate)` or add a thin wrapper; do
  **not** duplicate the publish body); count `bot_turn_sweep_republished_total`.
- missing or disabled (**dangling**) -> **no re-publish, no alert**; report only
  via gauge `bot_turn_dangling_bot_names`.

Re-publishing is safe: a duplicate `bot.turn` for an in-flight turn is caught by
the bot's own DB re-check and the `is_turn`/`updated_at` guards.

### 3b. `rust/web/src/game/mod.rs` - what is acked, when, with which AckKind

- `handle_bot_command_event` `UserError` arm: take the **same bounded path as
  `Conflict`** - below `nats::MAX_TURN_ATTEMPTS`, re-publish `bot.turn` for
  `event.player_position` at `attempt + 1` and return `Ok(())`; at/past the cap,
  `error!` + `bot_turn_wedge_total`, return `Ok(())`. The consumer's `UserError`
  ack arm becomes unreachable for this path - keep it, update its comment.
- `Conflict` exhaustion branch: keep the `Ok(())` ack; add the same
  `bot_turn_wedge_total` increment beside the existing `error!`.
- `publish_bot_turns`: raise both failure arms to `error!` and increment
  `bot_turn_publish_failures_total`; same for the `find_bot_turns` `Err` arms in
  `trigger_bot_turns` and the `Conflict` arm. Signatures unchanged.
- `run_bot_command_consumer` `Other` arm: read `message.info()`; at the ceiling,
  `message.ack_with(AckKind::Term)` + `error!` naming the game id +
  `bot_command_terminated_total`. Below it, leave unacked as today. Export
  `pub const MAX_DELIVER` from `rust/web/src/nats.rs` and use it in
  `ensure_stream_and_consumers` too so the two cannot drift.

### 3c. `rust/bot/src/main.rs` - ack heartbeat (bo F2)

In the per-message `tokio::spawn`, before awaiting `run_bot_turn`, spawn a
heartbeat calling `message.ack_with(AckKind::Progress)` every 60s (comfortably
inside `ack_wait`); abort it as soon as `run_bot_turn` resolves, before the final
ack decision. Share the message (e.g. `Arc`); a failed `Progress` ack is
warn-logged and the heartbeat continues. Do **not** change
`ack_wait`/`max_deliver`, and do **not** change the dangling-bot skip path beyond
raising its `info!` to `warn!` - it stays the D-5-supported no-op.

### 3d. `rust/web/src/admin.rs` - dangling bot names warning (ws F27 + D-5)

New stratum-2 helper beside `list_bots` (`pub async fn`, `&sqlx::PgPool`): one
query returning the distinct `game_bots.bot_name` values used by an
**unfinished** game with no **enabled** `bots` row, each with an affected-game
count. A `#[server]` wrapper uses WP-37's `require_admin` gate (do not
re-introduce the boilerplate WP-37 removed). `BotsSection` renders a warning
banner above the bot list; **empty list renders nothing**, and an empty `bots`
table must never warn (the bot service falls back to a synthetic config then).

## 4. Non-goals

- **Everything WP-39 shipped:** supervised restart loop (ws F53 / wd F4),
  MAX_DELIVERIES advisory listener + `bot_stream_max_deliveries_total` (ws F56),
  ws F57 drift warning, ws F58 doc comment, wd F9 conflict re-publish filter,
  bo F1 / bo F3 / bo F5. Touch `nats.rs` only to add `MAX_DELIVER`.
- **WP-37's admin.rs work** (validation, `rows_affected`, `mask_api_key`,
  `ApiKeyUpdate`, Effect rewrites). WP-38 adds one query and one banner.
- No bot-id migration, schema change, or DLQ subject; no change to
  `ack_wait`/`max_deliver` values; no bot-slot validation (WP-45), undo/concede
  (WP-40) or `/ws` work (WP-42).
- Do not couple `/healthz` to the consumer or sweep, and do not wire the sweep
  into WP-36's shutdown token.

## 5. Regression test cases

- `rust/web/tests/nats_bot_eventing.rs` (existing; copy
  `stale_conflict_republishes_bot_turn_with_incremented_attempt` and
  `attempt_limit_exhaustion_gives_up`, reuse `make_game_with_human_and_bot`,
  `drain_bot_turn_events`, `spawn_mock_game_service`): a user-error response
  yields exactly one `bot.turn` at `attempt + 1` for the bot's own position; at
  the cap it publishes nothing and still acks; the sweep's candidate/publish fn
  re-publishes `attempt = 0` for an enabled bot past the threshold and publishes
  **nothing** for a missing/disabled one.
- `rust/web/src/email/sweep.rs` `#[cfg(all(test, feature = "ssr"))] mod tests`
  (existing): `#[sqlx::test]` on the candidate query - humans never candidates,
  finished games excluded, within-threshold excluded, dangling vs live names
  partitioned correctly.
- `rust/web/src/admin.rs` `mod tests` (existing): the helper returns nothing when
  every referenced bot is present and enabled, nothing on an empty `bots` table,
  and the right name+count when a bot is renamed away or disabled while an
  unfinished game references it; plus a non-admin rejection test matching
  `test_admin_list_bots_rejects_non_admin`.
- `rust/bot/src/main.rs` `mod tests` (existing, unit-only): if the heartbeat
  cadence is a `const`, assert it is strictly below the 5-minute `ack_wait`. No
  NATS-backed heartbeat test.

## 6. Riders

| file | one-line fix | test |
|---|---|---|
| `rust/web/src/game/mod.rs` | wd F5: `term()` at the `max_deliver` ceiling in the `Other` arm instead of silent stranding | y |
| `rust/web/src/nats.rs` | add `pub const MAX_DELIVER`, use it in `ensure_stream_and_consumers` so config and term ceiling cannot drift | n |
| `rust/bot/src/main.rs` | ws F27: raise the "bot not found or disabled" skip log from `info!` to `warn!` (still a no-op) | n |
