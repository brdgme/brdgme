# WP-48: export/import

**Findings:** wd F7 (minor), wd F10, wd F11, wd F12, wd F13 (nits).
**Decision:** D-7 **OVERRULED** - no redacted export, no user-facing export.
The only export path is the full bundle, **admin-only**.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose. Any line number below
> is approximate - verify before trusting it.

## 1. Problem

- **wd F7** - the export bundle carries private log bodies (`is_public = false`
  plus their `target_positions`) and the raw `game_state` blob (hidden hands),
  while the module doc in `rust/web/src/game/export.rs` says the bundle "may get
  pasted into issues" and only claims to exclude email addresses.
- **wd F10** - `import.rs::placeholder_user` does check-then-insert on the
  username; a concurrent insert of the same name aborts the whole import tx.
- **wd F11** - `export.rs::BundlePlayer.bot_name` holds `game_bots.name` (the
  per-game seat name), not `game_bots.bot_name` (the bot type).
- **wd F12** - `BundleGame.created_at`/`updated_at` and `BundleLog.created_at`
  are exported but never inserted by `import.rs::import_bundle`.
- **wd F13** - `import_bundle` sets `is_turn_at` and `last_turn_at` to `NOW()`
  for every imported player regardless of turn state.

## 2. Why it's wrong

- **wd F7's factual claim is correct** (verified live: `build_export_bundle`
  selects `game_logs` with no `is_public` filter and copies `ge.game.game_state`
  verbatim). **But the access-control half is already fixed.** See 3a - the only
  entrypoint is already admin-gated. What remains is the misleading module doc.
- **wd F10 is correct as written.** Verified live: `placeholder_user` runs a
  `SELECT EXISTS(...)` then a separate `INSERT INTO users`.
- **wd F11 is correct as written**, and its second option ("leave as-is
  deliberately") is the one this WP takes - see Riders.
- **wd F12 is correct as written.** Verified live: the `games` insert lists only
  `(game_version_id, is_finished, finished_at, game_state)` and the `game_logs`
  insert only `(game_id, body, is_public, logged_at)`. The `update_*_updated_at`
  triggers are all `BEFORE UPDATE`, so explicit values on INSERT survive.
- **wd F13's diagnosis is correct but its recommended fix is WRONG.**
  `game_players.is_turn_at` and `last_turn_at` are `timestamp NOT NULL` with
  **no default** (migration `001_initial_schema.sql`). "Leave `last_turn_at` at
  the column default/NULL" is not possible. Use the substitute in the Riders row
  instead. The `update_is_turn_at`/`update_last_turn_at` triggers are
  `BEFORE UPDATE` only, so INSERT values are not rewritten.

## 3. Required end state

### 3a. The admin gate already exists - do not build a second one

**Answer to "is there a user-facing export path to remove?": NO. There is
nothing to delete.** Every entrypoint into `export.rs` was traced:

- Route: `GET /admin/games/{id}/export`, registered in
  `rust/web/src/router.rs::build_router`, handled by
  `rust/web/src/game/export.rs::admin_export_game`.
- That handler already does, in order:
  `crate::auth::session::get_user_from_session` -> 401,
  `crate::auth::session::validate_session_token` -> 401,
  **`crate::db::is_user_admin`** -> 403. This is the existing mechanism and the
  one to keep; it is the same `crate::db::is_user_admin` that
  `rust/web/src/admin.rs::require_admin` and
  `rust/web/src/game/server_fns.rs` (`viewer_is_admin`) use. `require_admin`
  itself is a **leptos server-fn** helper and cannot be called from a plain axum
  handler - do not try to swap it in.
- UI: one link, in `rust/web/src/components/game.rs`, inside the
  `<Show when=move || viewer_is_admin>` block ("Export JSON (admin)").
  `viewer_is_admin` comes from `GameViewData`, populated in
  `game/server_fns.rs::get_game_details` via `is_user_admin`.
- No leptos server fn, no CLI binary and no other component references
  `build_export_bundle` or `ExportBundle`. The only other consumers are
  `rust/web/src/game/import.rs` and `rust/web/src/bin/import_game.rs` (import
  side, dev-only).

So the whole of the F7 code change is **3b**.

### 3b. `rust/web/src/game/export.rs` module doc

Rewrite the module-level `//!` header so it no longer invites pasting bundles
into issues. It must state: the bundle contains **private log bodies, their
target positions, and the raw `game_state` blob (hidden information)**; it is
admin-only and must not be posted publicly; email addresses are still the one
thing excluded. Do not add a redaction mode, a flag, or a filter.

## 4. Non-goals

- **CANCELLED (D-7 overruled):** the `--redact-private` flag / redacted bundle
  variant, and any user-facing (non-admin) export path. Do **not** build either.
  Bug reporting is by **game ID**; the user explicitly accepts that game state
  may move on after a report and render it useless. That is not a defect.
- Do not filter private logs, targets or `game_state` out of the bundle.
- Do not change `BUNDLE_SCHEMA_VERSION` or any bundle field name (see F11).
- Do not touch `is_game_visible_to_user` or `rust/web/src/players.rs` - WP-47
  owns those. The admin gate here is `is_user_admin` and is unrelated.
- Do not add an export CLI binary; do not make `import_bundle` reachable in prod.

## 5. Regression test cases

`rust/web/src/game/export.rs` has **no** `#[cfg(test)]` module today; the router
harness in `rust/web/tests/ssr_pages.rs` does (`build_router`, `make_user`,
`login_cookie`, `get`). Add there:

- `GET /admin/games/{id}/export` with **no cookie** -> `401`.
- Same path with a logged-in **non-admin** cookie -> `403`, and the body must
  not contain the private log body.
- Same path with a cookie for a user whose `users.is_admin = true` -> `200` and
  a `content-disposition: attachment` header. (`make_user` inserts a non-admin;
  flip with `UPDATE users SET is_admin = true`, as
  `game/server_fns.rs` tests already do.)

In `rust/web/src/game/import.rs` `#[cfg(test)] mod tests`, extending the
existing `import_bundle_round_trips_a_game` or as new `#[sqlx::test]` cases:

- F12: imported `games.created_at`/`updated_at` and `game_logs.created_at`
  equal the bundle's values, not "now".
- F13: for a bundle player with `is_turn = false`, the imported row's
  `is_turn_at` is not "now"; for the player with `is_turn = true` it matches the
  rule chosen in the Riders row.
- F10: no test (racy, dev-only) - see the Riders table.

## 6. Riders

| finding | file | one-line fix | test needed |
| --- | --- | --- | --- |
| wd F10 | `web/src/game/import.rs::placeholder_user` | Drop the pre-check race window: keep the `taken` check, but if the `INSERT INTO users` returns a unique-violation (`sqlx::Error::Database` with `is_unique_violation()`), fall back to `crate::db::generate_unique_username` and insert again once. | n |
| wd F11 | `web/src/game/export.rs::BundlePlayer` | **No rename** - keeping `bot_name` avoids a `BUNDLE_SCHEMA_VERSION` bump for a dev-only tool. Take the finding's "leave as-is deliberately" branch: strengthen the existing doc comment to say explicitly that this is `game_bots.name` (seat name) and **not** `game_bots.bot_name` (the bot type, carried in `BundleBot.bot_name`). | n |
| wd F12 | `web/src/game/import.rs::import_bundle` | Add `created_at, updated_at` to the `games` INSERT column list bound from `bundle.game.created_at`/`.updated_at`, and `created_at` to the `game_logs` INSERT bound from `log.created_at`. Triggers are BEFORE UPDATE, so these stick. | y |
| wd F13 | `web/src/game/import.rs::import_bundle` | Both columns are `NOT NULL` with no default, so they must be given values: bind `is_turn_at` and `last_turn_at` to `bundle.game.updated_at` instead of `NOW()`. Do **not** follow the finding's "leave at default/NULL" suggestion. | y |
