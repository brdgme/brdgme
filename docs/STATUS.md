# Current Work Status

## Phase 5.5: COMPLETE

## Phase 5.6: In Progress - 9 of 13 blockers coded, build currently broken

---

## This session: Phase 5.6 Pre-Cutover Fixes

### Completed this session (code written)

**1. `scripts/setup-kind-cluster.sh` - local registry fix (Phase 5.5 completion)**
- Added `local-registry-hosting` ConfigMap (KEP-1755)
- Added `kubectl patch configmap/config-deployment` to add `kind-registry:5000`
  to `registries-skipping-tag-resolving` - this was the missing piece that would
  have caused Knative's controller to fail resolving image digests.

**2. `auth/session.rs` - rewrote session layer**
- Replaced `MemoryStore` with `PostgresStore` from `tower-sessions-sqlx-store`
- `create_session_layer(pool: &PgPool)` is now async; creates + migrates store internally
  via `PostgresStore::new(pool).migrate().await`
- `with_secure` now reads `SECURE_COOKIE` env var (`"true"` = secure, default false)
- Expiry changed from 24 hours to 30 days (matching old system)
- `validate_session_token` query updated:
  `WHERE id = $1 AND created_at > NOW() - INTERVAL '30 days'`
- Removed dead `SESSION_AUTH_TOKEN_KEY` constant
- Simplified `set_user_session` (no longer stores separate auth token key)
- Simplified `clear_user_session` (removes only `SESSION_USER_KEY`)

**3. `auth/server.rs` - removed token from response**
- `login()` response message changed from exposing the token to just "Login email sent"

**4. `main.rs` - async session layer + SIGTERM**
- `create_session_layer` call is now awaited
- Added `shutdown_signal()` function: listens for SIGTERM and Ctrl+C via
  `tokio::signal::unix` and `tokio::select!`
- `axum::serve(...).with_graceful_shutdown(shutdown_signal())` wired in

**5. `game/server.rs` - auth in all Axum handlers**
- `create_game`, `get_game`, `play_command` all extract `session: Session`
- `get_user_from_session(&session).await` checked; returns 401 if absent
- `Uuid::nil()` replaced with `user.id` from session
- Turn enforcement added to `play_command`:
  `if !player.game_player.is_turn { return 403 }`

**6. `app.rs` - login UI wired to server functions**
- `login_action: Action<String, LoginResponse>` calls `login(email)` server fn
- `confirm_action: Action<String, AuthUser>` calls `confirm_login(token)` server fn
- `on_email_submit` dispatches `login_action`; code input only shown after
  server confirms success (via `Effect` watching `login_action.value()`)
- `on_code_submit` dispatches `confirm_action`
- Navigates to `/dashboard` on successful confirm (via `use_navigate` + `Effect`)
- Error messages shown below each form on failure

**7. `Cargo.toml` - session store dependency**
- `tower-sessions = "0.14.0"` (kept at 0.14.0, see blocker below)
- Removed `tower-sessions-memory-store`
- Added `tower-sessions-sqlx-store = { version = "0.15.0", features = ["postgres"] }`

---

## BLOCKER: tower-sessions version conflict (build is broken)

### The problem
`tower-sessions-sqlx-store 0.15.0` depends on `tower-sessions-core 0.14.0`.
`tower-sessions 0.15.0` depends on `tower-sessions-core 0.15.0`.
Rust treats these as incompatible traits - `PostgresStore` does not implement
the `SessionStore` trait version that `SessionManagerLayer` (from tower-sessions 0.15.0)
requires.

`tower-sessions 0.15.0` is the LATEST (0.14.0 was before it).
`tower-sessions-sqlx-store 0.15.0` is the LATEST per crates.io.

User preference: stay on latest stable. Current Cargo.toml has:
- `tower-sessions = "0.14.0"` (temporarily reverted from 0.15.0 to attempt fix)
- `tower-sessions-sqlx-store = "0.15.0"`
This still causes the conflict because sqlx-store 0.15.0 uses core 0.14.0 and
tower-sessions 0.14.0 also uses core 0.14.0 - this SHOULD be compatible.

### What to try at start of next session

First, verify the actual resolved versions with a live dependency tree:
```bash
cargo tree -p web --features ssr 2>&1 | grep tower-sessions
```

Then try:
1. **`cargo update`** in `rust/` - the Cargo.lock may have stale entries
   locking to incompatible patch versions. This is the most likely fix.
2. If that doesn't work, check crates.io for a `tower-sessions-sqlx-store`
   version > 0.15.0 that explicitly targets `tower-sessions-core 0.15.0`:
   `cargo search tower-sessions-sqlx-store`
3. If no newer sqlx-store exists, the only options are:
   a. Keep `tower-sessions = "0.14.0"` and confirm that combination compiles
   b. Wait for sqlx-store to publish a 0.16.0 targeting tower-sessions-core 0.15.0

### After version conflict is resolved

The `validate_session_token` query changed (added 30-day expiry check). The
`.sqlx/` offline metadata file for the old query is now stale. Must regenerate:
```bash
# With tilt up (postgres running):
cd rust
cargo sqlx prepare --workspace -- --features ssr
```
Without this, `SQLX_OFFLINE=true` Docker builds will fail.

---

## Remaining Phase 5.6 blockers (not yet coded)

In order of recommended priority:

1. **`GamePlayer` model missing fields** (`models/game.rs`):
   Add `last_turn_at`, `is_eliminated`, `is_read`, `points`, `undo_game_state`,
   `rating_change`. Required before undo, mark_read, and points work.

2. **`update_game_command_success` writes all fields** (`db.rs`):
   Persist `is_turn_at`, `last_turn_at`, `is_eliminated`, `undo_game_state`,
   and points on every command. Also set `finished_at` when `is_finished = true`
   (verify no DB trigger does this).

3. **`find_game_extended` handles missing `game_type_users` row** (`db.rs`):
   Use LEFT JOIN with a default rating (1500) rather than erroring.

4. **Email sending** (`auth/server.rs`):
   Send confirmation token via in-cluster SMTP service. The SMTP pod is already
   deployed. Use the `email` crate (already in Cargo.toml) or `lettre` (more
   actively maintained - may be worth switching). Read SMTP host/port from env.

---

## Summary of all Phase 5.6 blocker status

| # | Blocker | Status |
|---|---------|--------|
| 1 | Persistent session store | Coded, blocked by version conflict |
| 2 | Login UI wired | Coded, pending compile |
| 3 | Token not in response | Done |
| 4 | `with_secure` env-driven | Coded, pending compile |
| 5 | Token expiry 30-day | Coded, needs sqlx prepare |
| 6 | Email sending | Not started |
| 7 | Auth in Axum handlers | Coded, pending compile |
| 8 | Authenticate GET /game/:id | Coded, pending compile |
| 9 | Turn enforcement | Coded, pending compile |
| 10 | GamePlayer missing fields | Not started |
| 11 | update_game_command_success all fields | Not started |
| 12 | find_game_extended LEFT JOIN | Not started |
| 13 | Graceful SIGTERM | Done |
