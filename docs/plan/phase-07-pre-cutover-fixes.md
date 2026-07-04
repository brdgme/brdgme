# Phase 7: Pre-Cutover Fixes

**Status:** Complete

**Goal:** Resolve all blockers and close critical gaps found in the parity
review before the `leptos` branch replaces production. (The review document
`docs/REVIEW.md` has since been deleted; its findings are the task lists below.)

### Blockers (must fix before cutover)

- [x] **Auth in Axum handlers** (`game/server.rs`): Replace `Uuid::nil()` in
      `create_game` and `play_command` with the authenticated user ID from the
      session.
- [x] **Login UI wired to server functions** (`app.rs`): Connect
      `on_email_submit` and `on_code_submit` to the `Login` and `ConfirmLogin`
      server functions via `Action`. Navigates to `/dashboard` on success.
      Error messages shown on failure.
- [x] **Confirmation token not exposed in response** (`auth/server.rs`): Token
      removed from response message.
- [x] **Persistent session store** (`auth/session.rs`):
      `tower-sessions-sqlx-store 0.15.0` + `PostgresStore`, table created via
      `store.migrate()` in `create_session_layer`.
- [x] **`with_secure` env-driven** (`auth/session.rs`): Reads `SECURE_COOKIE`
      env var (`"true"` = secure).
- [x] **Graceful SIGTERM shutdown** (`main.rs`): Added `shutdown_signal()`
      listening for SIGTERM and Ctrl+C.
- [x] **Turn enforcement** (`game/server.rs`): Rejects commands when
      `!player.game_player.is_turn`.
- [x] **Authenticate `GET /api/game/{id}`** (`game/server.rs`): Returns 401
      when no valid session.
- [x] **`GamePlayer` model missing fields** (`models/game.rs`): Added
      `last_turn_at`, `is_eliminated`, `is_read`, `points`, `undo_game_state`,
      `rating_change`. Migration `002_game_player_fields.sql` applied.
- [x] **`update_game_command_success` writes all fields** (`db.rs`): Persists
      `is_turn_at`, `last_turn_at`, `is_eliminated`, `undo_game_state`, `points`,
      and `finished_at` on every command.
- [x] **`find_game_extended` handles missing `game_type_users` row** (`db.rs`):
      Returns default `GameTypeUser` with rating 1500 instead of erroring.
- [x] **Token expiry check in `validate_session_token`** (`auth/session.rs`):
      SQL query updated to `AND created_at > NOW() - INTERVAL '30 days'`.
      SQLx offline metadata regenerated.
- [x] **Email sending** (`auth/server.rs`): `send_login_email` originally
      implemented using `lettre 0.11` with `AsyncSmtpTransport` (plain
      SMTP, no TLS); replaced by Phase 22a with `resend-rs` over the
      Resend HTTP API. Reads `RESEND_API_KEY`, `EMAIL_FROM` from env.
      Logs the code instead of sending if `RESEND_API_KEY` unset (dev
      fallback).

### Missing endpoints (non-blocking, needed for feature parity)

- [x] **`POST /game/{id}/undo`**: Restore `undo_game_state`, call `Status` on
      the game service, clear all players' undo state, write a log entry,
      broadcast.
- [x] **`POST /game/{id}/mark_read`**: Set `is_read = true` on the calling
      player's `game_players` row.
- [x] **`POST /game/{id}/concede`**: Limited to 2-player games. Mark game
      finished, write log entry, broadcast.
- [x] **`POST /game/{id}/restart`**: Create new game with same players, link
      via `restarted_game_id`. Broadcasts `BrdgmeUpdate` for both the new game
      and the old game (with `restarted_game_id` now set), so all players see
      the "Go to new game" link. Restarting player navigates via server fn response.

### Operator (`rust/operator`) [Complete]

- [x] **`GameVersion` CRD** (`k8s/base/operator/crd.yaml`): Namespaced CRD
      with `typeName`, `playerCounts`, `weight`, `isDeprecated` fields.
- [x] **kube-rs operator**: Reconciles `GameVersion` CRs by upserting
      `game_types` and `game_versions` rows in PostgreSQL. Finalizer ensures
      `is_public = false` is written before CR deletion.
- [x] **`is_deprecated` support**: `lost-cities-1` CR has `isDeprecated: true`
      - kept running for in-progress games, excluded from new game creation.
- [x] **Migration 003**: Unique constraints on `game_types(name)` and
      `game_versions(game_type_id, name)` enabling `ON CONFLICT` upserts.
- [x] **20 `GameVersion` CR files** colocated with each game in
      `k8s/base/game/{name}/game-version.yaml`.
- [x] **Tilt integration**: `crd-ready` resource gates operator startup on CRD
      establishment; operator runs as `local_resource` with `RUST_LOG=info`.
- [x] **mirrord**: Added to `devenv.nix`; Tiltfile wraps `cargo leptos watch`
      with `mirrord exec --target pod/postgres-0 --target-namespace brdgme`
      so the local web server resolves `*.svc.cluster.local` without application
      changes or `/etc/hosts` modification.

### Frontend gaps (non-blocking)

- [x] **New-game creation UI** (`app.rs`, `GamesPage`): Game type selector,
      optional version selector, player count selector, opponent email inputs,
      submit → redirect to new game.
- [x] **Game log rendering** (`components/game.rs`): `GameLogs` and
      `RecentGameLogs` components fetch logs, render markup to HTML, group by
      10-minute windows, filter to logs since `last_turn_at`.
- [x] **Undo/concede/restart actions in `GameMeta`**: `UndoGame`,
      `ConcedeGame`, `RestartGame` server functions wired; visibility
      conditions enforced (`can_undo`, `is_finished`, `restarted_game_id`).
- [x] **"Whose turn" display** (`app.rs`): Shows specific player names
      from `players` filtered by `is_turn`.
- [x] **Mark-read on game page load** (`app.rs`): `mark_read` called via
      `Effect` on mount and game ID change.
- [x] **`GameRestarted` WebSocket navigation**: `GameRestarted` WS message
      triggers a refetch (same as `GameUpdate`). Player who clicked Restart
      navigates via `restart_action` effect. Other players see the finished
      game and a "Go to new game" link once `restarted_game_id` is populated.
- [x] **Command input: clear after server confirms** (`components/game.rs`):
      `Effect` runs `set_command("")` after `submit_action` succeeds.
- [x] **Command errors surfaced to user** (`components/game.rs`): `error_msg`
      memo observes `submit_action` result and displays errors.
- [x] **Clickable command suggestions** (`components/game.rs`): Click handler
      appends suggestion value to command input.
- [x] **Autocomplete prefix filtering** (`rust/lib/game`): `CommandSpec::suggest`
      called from `GameCommandInput`.

### Code quality (non-blocking)

- [x] **Dead code removed**: `New*` model structs, `chat.rs`, `friends.rs`,
      `PublicGameType` alias, `db::AppState` deleted or removed.
- [x] **`reqwest::Client` shared** (`game/client.rs`): Created once in `main.rs`,
      stored in `AppState`, provided as Leptos context; all `client::` fns take `&Client`.
- [x] **N+1 in `find_active_games_for_user`** (`db.rs`): Replaced loop with a
      single joined query across all required tables; SQLx cache updated.
- [x] **Duplicate command logic** (`game/server.rs` vs `server_fns.rs`):
      Extracted into `game::execute_command`; both callers delegate to it.
- [x] **`chrono` → `time`** (`models/`, `lib/game`, `lib/cmd`): Replaced
      `chrono::NaiveDateTime` with `time::PrimitiveDateTime` throughout all
      model structs, `brdgme_game::Log`, and `brdgme_cmd::CliLog`. Required
      because `tower-sessions-sqlx-store` enables `sqlx/time` which takes
      precedence over `sqlx/chrono` in type inference.
- [x] **Points persisted** (`db.rs`): `_points` suppression removed;
      points written per-player in `update_game_command_success`.
- [x] **Logout redirect/feedback** (`components/layout.rs`): Navigate to `/login` after logout action succeeds.
- [x] **WebSocket reconnection** (`websocket_client.rs`): Reconnects with a
      2-second delay after disconnect; `use_websocket` now runs a `spawn_local` loop.
- [x] **`finished_at` set when `is_finished = true`** (`db.rs`): Set via
      `COALESCE($arg, finished_at)` in `update_game_command_success`.
- [x] **`active_games` ownership moved to `SidebarMenu`** (formerly tracked as
      Phase 5.6.1): the `LocalResource` was hoisted to `App()` via context,
      breaking SSR resource tracking and causing a hydration crash. Now created
      directly in `SidebarMenu` (`components/layout.rs`), driven by
      `WebSocketTrigger`. Verified complete 2026-07-02.

