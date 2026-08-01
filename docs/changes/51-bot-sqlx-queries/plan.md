# Bug Fixes - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.
>
> Extracted 2026-07-08 from `docs/plan/bugs.md`. This is a catch-all bug log,
> not a single cohesive design - each entry below is an independent bug with
> its own fix (and, where applicable, its own design decision embedded in the
> entry). Task granularity is per-bug; there is no shared spec because the
> bugs are unrelated to one another.

**Status:** Partially resolved

- [x] **Restart 500 error**: `restart_game` returns "Game service error: error
      parsing JSON response". Closed 2026-07-04 as could-not-reproduce.
      Diagnostics were improved: `client::request` now includes the raw
      response body in the error, and games are now restarted onto the latest
      non-deprecated game version (commit f21a136). If it recurs, the error
      message will contain the raw payload needed to diagnose it.
- [ ] **Bot restart limitation**: when a game is restarted, bots from the
      original game are not carried over to the new game. The `restart_game`
      handler (`game/server_fns.rs`) copies players but does not check
      `game_players.game_bot_id` and create corresponding `game_bots` rows in
      the new game.
      **Spec (decided 2026-07-05):** `create_game_with_users_tx` already
      accepts `bot_slots: &[BotSlot]` (`{ name, difficulty }`) and creates
      `game_bots` rows from it - `restart_game` just passes `&[]`. Fix:
      alongside the existing `opponent_ids` collection, collect
      `Vec<BotSlot>` from the original `ge.game_players` rows that have a
      `game_bot_id`, copying the bot's `name` and `difficulty` (join
      `game_bots` - add a small `db` lookup if `GameExtended` doesn't carry
      them), and pass it as `bot_slots`. Expected behaviour: new `game_bots`
      rows (fresh ids) with the original name + difficulty; positions are
      assigned by the existing slot logic exactly as for humans (reshuffled
      - same treatment as human players, no seat pinning); player-count
      constraints cannot differ (the roster is copied 1:1, `player_count`
      already includes bot seats). After creation the existing
      `broadcast_and_trigger` call already publishes bot turns, so a bot
      that opens the new game acts without a manual bump.
      **Tests (define done):** restart of a finished 1-human + 1-bot game →
      the new game has one `game_bots` row with the original name and
      difficulty and a `game_players` row referencing it; a `bot.turn`
      event is published when the bot is on turn (extend the
      `nats_bot_eventing.rs` pattern); restart of an all-human game is
      unchanged (existing tests still pass).
- [x] **3-player Lost Cities render**: Resolved 2026-07-04. Generated a
      scripted 3-player game via the engine API to a mid-round-2 state (round 1
      scored) and extracted a real render via `lost_cities_2_cli`; replaced the
      RULES.md placeholder and corrected the "most recent card" bullet, which
      had the top/bottom orientation backwards.
- [x] **Optimistic locking missing in `execute_command`**: two concurrent
      requests (e.g. two players submitting at the same instant, or a bot and a
      player) can both read the same game state, both call the game service, and
      both attempt to write back. The second write silently overwrites the first.
      Fix using `games.updated_at` (microsecond precision, set by trigger on
      every UPDATE) - no migration needed:
      1. Read `game.updated_at` in `execute_command` alongside `game_state`.
      2. Pass `expected_updated_at` to `update_game_command_success`.
      3. Change the UPDATE to
         `UPDATE games SET ... WHERE id = $1 AND updated_at = $expected`.
      4. `rows_affected == 0` → return a conflict error: a human player gets
         a "please retry" error; the bot treats it like a validation error and
         re-fetches fresh state via its existing post-LLM state-change
         detection.
      Changes: `execute_command` in `game/mod.rs`; signature and UPDATE query
      in `update_game_command_success` in `db.rs`.
      Resolved: implemented as specified above; no bot changes needed since
      the conflict error propagates like any other command error and the bot
      already retries with fresh state on failure. Conflict test added in
      11.3 (`concurrent_write_conflict_returns_err_and_preserves_first_write`).
- [x] **Concede confirmation**: Added `window.confirm("Are you sure you want to
      concede?")` in the click handler before dispatching `ConcedeGame`.
      `"Window"` added to web-sys features.
- [x] **Recent logs `is_new` always false**: `logged_at` (game service time) is
      set before `last_turn_at` is written to DB, so `logged_at > last_turn_at`
      was always false. Fixed: `log.created_at >= last_turn_at` (DB insert time,
      set after `last_turn_at` commits). Matches web-legacy.
- [x] **Suggestions/command input too narrow**: `game-command-input-container`
      wrapper div had no explicit width; as a centered flex child its children's
      `width: 63%` resolved against an unsized parent. Fixed: return a fragment
      `<>` from `GameCommandInput` so both elements are direct children of
      `.game-main` and correctly receive 63% of its width.
- [x] **Timestamp shown in recent logs**: `render_log_entries` now takes
      `show_timestamp: bool`; recent logs pass `false`, sidebar logs pass `true`.
      Also fixed: empty `log-time` divs (block elements adding blank lines) now
      only rendered when a label exists.
- [x] **Scroll to bottom**: `NodeRef` + `Effect::new` + `request_animation_frame`
      in `RecentGameLogs` (scrolls `.recent-logs`) and `GameLogs` (scrolls
      `.game-meta-logs-content` via `parent_element()`).
- [x] **Page flash on command submit**: Outer `Suspense` → `Transition` for the
      game data resource. `Transition` keeps previous content visible during
      re-fetches; `Suspense` was blanking the screen on every WebSocket update.
- [x] **Undo log plain text**: Was inserting `'Game undone.'` directly. Fixed:
      `db::undo_game` takes `player_position: usize` and inserts
      `{{player N}} used an undo` markup, rendered as the player name in color.
- [x] **No UI update after command/undo/concede**: rust/web relied solely on the
      WebSocket round-trip for re-fetches. Fixed: increment `trigger.last_update`
      immediately in the client-side `Effect` when any server action returns
      `Ok(())`. WebSocket still fires for other players as before.
- [ ] **Bot: dynamic sqlx queries mask schema drift** (`rust/bot/src/main.rs`):
      several `row.try_get(..).unwrap_or(..)` calls turn a query bug into
      silent wrong behaviour instead of an error - e.g. `is_turn: bool =
      row.try_get("is_turn").unwrap_or(false)` (line 82) means a broken
      column/alias makes the bot silently never play, with no log or panic.
      Flagged in the 2026-07-04 review as "acceptable short-term; the Phase
      13 rewrite should move to checked queries or explicit errors" - Phase
      13 (NATS bot eventing) shipped without addressing this, and it wasn't
      re-captured anywhere at the time. Fix: switch to checked `sqlx::query!`
      macros where feasible, or replace `unwrap_or` defaults with
      `.context(..)?` so a schema mismatch surfaces as an error rather than
      a silently wrong default.
- [ ] **Operator: no reconcile tests** (`rust/operator/src/controller.rs`):
      `upsert_game_type_and_version` (line 139) and the rest of the
      reconcile logic have zero test coverage. Flagged in the 2026-07-04
      review and missed when the rest of that review's findings were
      captured. Fix: a `#[sqlx::test]` against `upsert_game_type_and_version`
      covers the SQL half without needing a live kube test env (per the
      original review's suggestion).
