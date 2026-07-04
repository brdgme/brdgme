# 10: Eliminate runtime panics in rust/web

**Status:** Complete

**Goal:** Replace all panic-prone code in `rust/web/src` that could crash the
server process or the WASM frontend at runtime with proper error handling.
Startup panics (`main.rs`, `auth/session.rs`, `db.rs` env var) are
intentional and excluded.

**Background:** An audit of `rust/web/src` found 47 instances of `.unwrap()`,
`.expect()`, `unreachable!()`, and `panic!()`. Most are either in tests or in
`#[cfg(not(feature = "ssr"))]` stubs (correct). The following are runtime risks:

### Cases to fix

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

### Excluded (intentional)

- `main.rs`, `auth/session.rs`, `db.rs:55`: startup failures where the
  process cannot run without the resource. Panicking at boot is correct.
- `game/client.rs`: all within `#[cfg(test)]`. Panics in tests are fine.
- `server_fns.rs` `unreachable!()` in `#[cfg(not(feature = "ssr"))]` stubs:
  these paths are never compiled into the server and are never reachable on
  the client, so they are correct as-is.
- `components/game.rs:373` - `values.into_iter().next().unwrap()`: guarded by
  `values.len() == 1` on the preceding line; `unwrap()` is provably safe.

