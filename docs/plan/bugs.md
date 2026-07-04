# Bug fixes

**Status:** Partially resolved

- [ ] **Restart 500 error**: `restart_game` returns "Game service error: error
      parsing JSON response". Diagnostics improved: `client::request` now reads
      response body as text first and includes it in the error message. Root
      cause still unknown - needs a live restart attempt to capture the raw
      game service response.
      **Delegation gap:** this is a diagnosis task with no procedure. Before
      delegating, write: exact repro steps (game type, state, who restarts),
      how to capture the raw response (RUST_LOG settings, which Tilt resource
      logs to read, or a curl replay of the restart request), and what to do
      with the captured payload (fix criteria vs report back).
- [ ] **Bot restart limitation**: when a game is restarted, bots from the
      original game are not carried over to the new game. The `restart_game`
      handler (`game/server.rs`) copies players but does not check
      `game_players.game_bot_id` and create corresponding `game_bots` rows in
      the new game.
      **Delegation gap:** no expected-behaviour spec. Decide and document:
      are new `game_bots` rows created copying name + difficulty; do bots keep
      their positions or are positions reshuffled like humans; behaviour when
      the restarted game has different player-count constraints; and the test
      cases that define done.
- [ ] **3-player Lost Cities render**: `lost-cities-2/RULES.md` has a
      placeholder for the 3-player "Reading the Display" section.
      **Delegation gap:** no task body. Needs: how to obtain a representative
      3-player game state, the render extraction procedure (per `docs/RULES.md`
      and the game crate conventions), and which RULES.md sections must change.
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

