# 10: Eliminate runtime panics in rust/web - Implementation Plan (historical)

> Extracted 2026-07-08 from `docs/plan/10-eliminate-runtime-panics.md`. This work is
> complete/closed; retained as an execution record.

**Status:** Complete

**Spec:** `docs/superpowers/specs/2026-07-08-10-eliminate-runtime-panics-design.md`

## Cases to fix

- [x] **`db.rs:407` - `games.last_mut().unwrap()`** (`find_active_games_for_user`):
  replaced with `.ok_or_else(...)` returning a descriptive `anyhow::Error`
  instead of panicking.

- [x] **`db.rs:250-261` and `db.rs:393-404` - co-nullable LEFT JOIN unwraps**:
  extracted `build_user_from_row` and `build_game_bot_from_row` helpers
  (`db.rs`) using `ok_or_else` + `?`; both call sites (`find_game_extended`,
  `find_active_games_for_user`) now propagate an `anyhow::Error` instead of
  panicking if a LEFT JOIN row is malformed.

- [x] **`app.rs:112` and `app.rs:119` - `NodeRef::get().unwrap()` in form
  submit handlers**: replaced with `.get().map(|el| el.value())` +
  `let...else` that logs a warning via `leptos::logging::warn!` and returns
  early on `None`.

- [x] **`websocket_client.rs:21-23` - `window()`, `protocol()`, `host()`
  `.expect()` calls**: `window()` kept as `.expect()` (guaranteed in WASM).
  `protocol()` falls back to `"ws:"` with a logged warning on `Err`; `host()`
  logs a warning and aborts the connect attempt on `Err` (no valid URL can be
  built without it).
