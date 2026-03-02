# Current Work Status

## Phase 5.6: COMPLETE - All 13 blockers done

---

## This session: completed work

### GamePlayer model fields added (blockers 10, 11, 12)

**Migration `002_game_player_fields.sql`**
Added missing columns to `game_players`:
- `last_turn_at TIMESTAMP` - when the player last took their turn
- `is_eliminated BOOLEAN NOT NULL DEFAULT false`
- `is_read BOOLEAN NOT NULL DEFAULT false`
- `points REAL` - per-player points from game service
- `undo_game_state TEXT` - pre-command state stored for undo
- `rating_change INTEGER`

Migration applied via `sqlx migrate run` from `rust/web/`.

**`models/game.rs`**: `GamePlayer` struct updated with all six new fields.

**`db.rs` - `find_game_extended`** (blocker 12):
- Query extended to select all new `game_players` columns.
- Missing `game_type_users` row no longer errors; returns a default
  `GameTypeUser` with `rating = 1500`, `peak_rating = 1500`, `id = Uuid::nil()`.

**`db.rs` - `update_game_command_success`** (blocker 11):
- Added parameters: `played_player_id`, `prev_game_state`, `new_game_state`,
  `can_undo`, `eliminated`.
- Removed suppressed `_points` and `_game_player_id` prefixes.
- Games table: sets `finished_at = COALESCE($arg, finished_at)` on completion.
- Players loop: writes `is_eliminated`, `points`, `is_turn_at` (updated when
  turn becomes true), `last_turn_at` (set to NOW for the player who played),
  `undo_game_state` (set to prev state for played player if `can_undo`).

**`game/server.rs` and `game/server_fns.rs`**:
- Propagate `can_undo` and `eliminated` from game service response.
- Pass `prev_game_state` (pre-command) and `new_game_state` separately.

### Email sending implemented (blocker 6)

**`rust/web/Cargo.toml`**:
- Replaced `email = "0.0.21"` with
  `lettre = { version = "0.11", features = ["tokio1", "smtp-transport", "builder"], default-features = false }`.

**`auth/server.rs`**:
- `send_login_email(to, token)` added: builds a `Message`, connects to SMTP
  via `AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)` (plain
  SMTP, no TLS - matches in-cluster `namshi/smtp` relay on port 25).
- Reads `SMTP_HOST` (required), `SMTP_PORT` (default 25), `SMTP_FROM`
  (default `noreply@brdgme.com`) from env.
- If `SMTP_HOST` is unset, logs a warning with the token (dev fallback) and
  returns without error.
- `login()` server function calls `send_login_email` after writing the token.

### SQLx offline metadata regenerated

`cargo sqlx prepare -- --features ssr` run from `rust/web/` after migration.
`SQLX_OFFLINE=true cargo check --features ssr` passes.

---

## What to do next

All Phase 5.6 blockers are complete. Next phase: **Phase 6 (NATS integration)**
or tackle the non-blocking missing endpoints and frontend gaps listed in
`docs/PLAN.md` Phase 5.6.

Recommended order for remaining Phase 5.6 items before starting Phase 6:
1. **`POST /game/{id}/undo`** endpoint - now unblocked by `undo_game_state` field.
2. **`POST /game/{id}/mark_read`** endpoint - uses `is_read` field.
3. **`POST /game/{id}/concede`** endpoint.
4. **`POST /game/{id}/restart`** endpoint.
5. Frontend gaps (game log rendering, clickable suggestions, command errors,
   mark-read on load, whose-turn display, etc.).

To enable email in local dev (hybrid mode), add to `devenv.nix`:
```
env.SMTP_HOST = "localhost";
env.SMTP_PORT = "2525";  # or whatever port you forward from the cluster
```
Or port-forward the SMTP service: `kubectl port-forward -n brdgme svc/smtp 2525:25`.

---

## Summary of all Phase 5.6 blocker status

| # | Blocker | Status |
|---|---------|--------|
| 1 | Persistent session store | Done |
| 2 | Login UI wired | Done |
| 3 | Token not in response | Done |
| 4 | `with_secure` env-driven | Done |
| 5 | Token expiry 30-day | Done |
| 6 | Email sending | Done |
| 7 | Auth in Axum handlers | Done |
| 8 | Authenticate GET /game/:id | Done |
| 9 | Turn enforcement | Done |
| 10 | GamePlayer missing fields | Done |
| 11 | update_game_command_success all fields | Done |
| 12 | find_game_extended LEFT JOIN | Done |
| 13 | Graceful SIGTERM | Done |
