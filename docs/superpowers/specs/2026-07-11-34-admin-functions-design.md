# 34: Admin Functions (force-delete, game export/import) - Design

**Status:** Decided 2026-07-11 - pre-beta (wanted in place before the #16
beta period). This is a point-in-time decision record, not a living
document.

**Problem:** no admin capability exists at all - the `users` table has no
role/flag column and no server function performs a permission check.
Two operator needs are already real: deleting junk games created during
testing, and pulling a bugged production game down to the local dev
environment to reproduce and debug it.

**Out of scope (parked, deliberately not backlogged):** bot model
configuration (multi-provider routing, failover, runtime model switching).
Discussed 2026-07-11 and parked for a future session - the current
sealed-secret + reseal workflow stands for now.

## Decisions (2026-07-11)

- **D1 - `is_admin` boolean column on `users`.** Migration adds
  `is_admin boolean NOT NULL DEFAULT false`; the operator sets it once via
  SQL (psql into prod). No roles table, no `ADMIN_EMAILS` env var - a
  per-user DB flag is the smallest thing that survives restarts and
  extends later. A `require_admin`-style guard is added for server
  functions; admin-only UI is rendered conditionally on the current
  user's flag (the flag ships in the current-user DTO). UI hiding is
  cosmetic - every admin server function and route enforces the check
  server-side.
- **D2 - contextual admin controls, no admin dashboard.** Admin actions
  appear on the game page (in `GameMeta`, alongside Undo/Concede/
  Restart/Bump bot) when the viewer is an admin. A dedicated admin page
  is not needed for two per-game actions.
- **D3 - force delete is a hard delete.** The `games` table has no
  soft-delete support (`is_finished`/`finished_at`/`restarted_game_id`
  only), and adding `deleted_at` would put a filter obligation on every
  game query - a wide, easy-to-miss change surface. Hard delete instead:
  remove `game_log_targets`, `game_logs`, `game_players`, and the game's
  `game_bots` rows, null out `restarted_game_id` on any game referencing
  the deleted one, then delete the `games` row, all in one transaction.
  A confirm dialog guards the button. **No rating rewind** - ELO effects
  of a deleted finished game stand; if rating adjustment is ever needed
  it will be a separate admin feature (Michael, 2026-07-11). After
  delete, navigate away and broadcast the usual game-update signal so
  open clients refresh.
- **D4 - export is a JSON bundle served from an admin-guarded Axum
  route.** A download button on the game page hits e.g.
  `GET /admin/games/:id/export` (session + `is_admin` checked), which
  streams a JSON file: `schema_version`, `exported_at`, the `games` row
  (including the opaque state blob), game type + `game_versions` info
  (name, URI), `game_players`, `game_bots` (name/difficulty/
  personality), `game_logs` + `game_log_targets`, and player display
  names. **No email addresses in the bundle** - names only; the file is
  for debugging and may get pasted into issues. A plain Axum route is
  used rather than a Leptos server fn because it downloads as a file.
- **D5 - import is a dev-side CLI.** A binary in `rust/web` (e.g.
  `cargo run --bin import-game -- bundle.json`) reads `DATABASE_URL`,
  ingests the bundle into local Postgres: creates placeholder local
  users for the named players, maps the game type/version to the
  locally-running game service URI (the bundle's prod URI will not
  resolve locally), and inserts the game, players, bots, and logs under
  fresh IDs. Fidelity caveat: the state blob is only meaningful to a
  compatible game service version - the CLI warns when the local
  game version differs from the bundle's. Dev-only tooling; it is never
  deployed or reachable in prod.

## Non-goals

Rating rewind on delete (D3), an admin dashboard page (D2), admin
management UI (the flag is set via SQL), bundling email addresses in
exports (D4), prod-side import (D5), bot model configuration (parked).
