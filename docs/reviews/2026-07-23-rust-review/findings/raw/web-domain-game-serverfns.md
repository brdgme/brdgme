# Raw findings: web domain - game/server_fns.rs

Scope: rust/web/src/game/server_fns.rs (full file, 2480 lines incl. tests), with targeted cross-referencing of web/src/db.rs, web/src/game/mod.rs, web/src/email/commands.rs, web/src/proposals.rs. Reviewer: W2.

### undo_game allows undoing a finished game, permanently corrupting ratings
- severity: critical
- category: correctness
- location: web/src/game/server_fns.rs:731
- finding: `undo_game` never checks `ge.game.is_finished`. When a player makes the game-finishing move, `update_game_command_success` (db.rs:1749) keeps that player's `undo_game_state` (is_played && can_undo), and `apply_rating_changes` runs immediately at finish. The finisher can then call undo: `db::undo_game` (db.rs:1407) reverts `game_state`, sets `is_finished` per status and `finished_at = NULL`, but does NOT clear `rating_change`/`rating_before` on game_players and does not rewind `game_type_users` ratings. Result: (a) ratings from the undone outcome stick; (b) when the game finishes again (possibly with different placings), the idempotency guard in `apply_rating_changes` (db.rs:1554, "any player already has a rating_change") trips and the real outcome is never rated. The UI exposes this path normally (`can_undo` in GameViewData is just `undo_game_state.is_some()`, line 319-321).
- recommendation: Reject undo when `ge.game.is_finished` (or when `rating_change` is set for any player), or make `db::undo_game` rewind rating changes atomically. Simplest: `if ge.game.is_finished { return Err("Game is already finished") }` in the server fn plus a matching guard inside `db::undo_game`.

### undo_game has no stale-state guard - can clobber a concurrent move
- severity: major
- category: correctness
- location: web/src/game/server_fns.rs:784
- finding: The command path is protected by optimistic locking (`update_game_command_success` matches `WHERE updated_at = $expected` and returns `StaleStateConflict`), but the undo path is not. `undo_game` reads the game via `find_game_extended` (line 748), makes an HTTP round-trip to the game service, then calls `db::undo_game` which unconditionally does `UPDATE games SET game_state = $1 ... WHERE id = $3` (db.rs:1416) with no `updated_at` check. If another player (or a bot, which moves quickly after `broadcast_and_trigger`) plays between the read and the write, their move is silently destroyed and every player's `undo_game_state` is wiped (db.rs:1440).
- recommendation: Pass `ge.game.updated_at` into `db::undo_game` and add `AND updated_at = $n` to the games UPDATE, returning StaleStateConflict on 0 rows, mirroring `update_game_command_success`. Additionally verify the player's `undo_game_state` is still non-NULL inside the transaction.

### concede_game TOCTOU: is_finished checked on a snapshot, no lock or guard
- severity: major
- category: correctness
- location: web/src/game/server_fns.rs:827
- finding: `concede_game` checks `ge.game.is_finished` on a pool snapshot, then calls `db::concede_game` which unconditionally sets `is_finished = true` and writes places 1/2 (db.rs:1292-1321) with no `WHERE NOT is_finished` and no row lock. Races: (a) both players of a 2p game concede concurrently - both pass the check, the second write flips the places the first wrote, while `apply_rating_changes`'s idempotency guard keeps the FIRST outcome's ratings, leaving places and ratings contradicting each other; (b) a concede racing the opponent's game-finishing move overwrites the real placings with the concede outcome. Contrast with `restart_core` (line 893) which correctly serializes on the game row with `FOR UPDATE`.
- recommendation: Inside `db::concede_game`'s transaction, lock the game row (`SELECT is_finished FROM games WHERE id = $1 FOR UPDATE`) and bail if already finished; or make the games UPDATE `WHERE id = $1 AND NOT is_finished` and abort on 0 rows.

### get_game_details ignores game visibility settings - is_game_visible_to_user is dead code
- severity: major
- category: correctness
- location: web/src/game/server_fns.rs:231
- finding: `get_game_details` requires only "any authenticated user"; it never checks whether the viewer may see this game. Any logged-in user who has (or guesses/obtains) a game UUID gets the spectator render, player names, ratings, rating changes, and recent form - even when every player has `game_visibility = 'private'`. The anonymous index path carefully gates on `is_game_publicly_visible` (line 370, "refused rather than leaked"), and db.rs even defines the exact helper needed, `is_game_visible_to_user` (db.rs:2228, viewer-is-player OR public OR friends), but it has zero callers anywhere in web/src - the intended gate was never wired up.
- recommendation: In `get_game_details`, when the viewer is not a player of the game, require `crate::db::is_game_visible_to_user(&pool, game_id, user.id)` and return "Game not found" otherwise. Delete the helper if spectating by logged-in users is instead meant to be unconditionally allowed (and then document that `game_visibility` only affects the logged-out index).

### restart_core / restart_game_with_roster accept arbitrary bot_name
- severity: minor
- category: correctness
- location: web/src/game/server_fns.rs:868
- finding: `bot_slots` come straight from the client (`restart_game_with_roster` params, line 1048) and `restart_core` inserts `bot.bot_name` into proposal players (line 1023) or creates the game directly, with no validation against `find_enabled_bots`. The email command path DOES validate ("email/commands.rs:354 uses find_enabled_bots"), so the web path is strictly weaker. A bogus bot_name creates a game whose bot player can never take a turn - a permanently wedged game (feeds the W1 bot-turn wedge).
- recommendation: In `restart_core` (it is the shared choke point), reject any `bot_slots` entry whose `bot_name` is not in `find_enabled_bots`, matching the email command path.

### Game-service HTTP call made inside an open transaction holding FOR UPDATE lock
- severity: minor
- category: quality
- location: web/src/game/server_fns.rs:947
- finding: In the solo-restart branch, `create_game_from_service` performs the game-service HTTP request (line 583) while the transaction begun at line 888 holds the `FOR UPDATE` lock on the old game row and a pool connection. A slow or hung game service pins the DB connection and blocks every other operation touching that game row for the duration of the reqwest timeout. The atomicity rationale in the doc comment (line 568) only requires the DB writes to be atomic, not the HTTP call.
- recommendation: Call the game service for the new game state BEFORE `pool.begin()` (the `Request::New` call has no dependency on transaction state), then do the lock + checks + inserts + link + commit purely with the already-fetched response.

### Invite-mailer path silently swallows find_proposal_players errors
- severity: minor
- category: quality
- location: web/src/game/server_fns.rs:1119
- finding: `if let Ok(players) = crate::proposals::find_proposal_players(&pool, pid).await` drops the Err case on the floor with no log: if the query fails, no invite emails are sent and nothing records why. The restart itself succeeded, so nobody notices until invitees complain they were never emailed.
- recommendation: `match`/`inspect_err` and `tracing::warn!` the error (the fire-and-forget email pattern is fine, the silent DB-error drop is not).

### Markup parse failures silently degrade log lines to empty
- severity: minor
- category: quality
- location: web/src/game/server_fns.rs:699
- finding: `brdgme_markup::from_string(&log.body).unwrap_or_else(|_| (vec![], ""))` (also at line 411 in `render_game_public`) turns any malformed stored log body into an empty entry with no logging - the user sees a blank timestamped log row and operators get no signal that stored data is unparsable. Contrast with the board render at lines 265/392 where the same parse failure is a proper internal error.
- recommendation: Log a `tracing::warn!` with the game_id/log id on parse failure (keeping the graceful blank-line fallback is reasonable for logs), or fall back to the raw body as escaped text.

### N+1 queries for friend status in get_game_details
- severity: minor
- category: quality
- location: web/src/game/server_fns.rs:293
- finding: `should_hide_add_friend` is awaited sequentially in a loop, once per human opponent (each itself a query, db.rs:1998), on the hottest read path of the app (every game page load, re-fetched on every websocket-driven refresh). For a 6-player game that is 5 extra round trips on top of the already numerous per-request queries (find_game_extended, is_user_admin, recent_form, predecessor, restart proposal).
- recommendation: Add a batched `should_hide_add_friend_many(pool, viewer, &[Uuid]) -> HashSet<Uuid>` using `= ANY($1)`, or fold it into the recent-form query which already takes `&human_user_ids`.

### get_game_logs is_new compares created_at against last_turn_at but displays logged_at
- severity: nit
- category: correctness
- location: web/src/game/server_fns.rs:704
- finding: `let is_new = log.created_at >= last_turn_at;` while the entry sorts/displays by `logged_at`. For normally-inserted logs the two are near-identical, but any log whose `logged_at` differs from `created_at` (backfill, undo/concede logs inserted with `logged_at = NOW()` vs row `created_at`) can be highlighted inconsistently with its position in the list.
- recommendation: Use `log.logged_at >= last_turn_at` for consistency with the ordering field, or document why created_at is intentional.

### generate_bot_name is the only unauthenticated non-public server fn
- severity: nit
- category: consistency
- location: web/src/game/server_fns.rs:644
- finding: Every other server fn in this file either authenticates or is documented as deliberately anonymous (`get_public_index`). `generate_bot_name` does neither - it is harmless (random pet name) but is an anonymous public endpoint by omission rather than by decision.
- recommendation: Add the standard `get_current_user` guard or a one-line comment stating it is intentionally anonymous.

### p.user_id.unwrap() in request path
- severity: nit
- category: quality
- location: web/src/game/server_fns.rs:1126
- finding: `p.user_id.unwrap()` is guarded by the `filter(|p| p.user_id.is_some() ...)` two lines above, so it cannot panic today, but the guard and the unwrap are far enough apart that a future edit to the filter re-introduces a panic path in a request handler.
- recommendation: Use `filter_map` yielding the unwrapped id, e.g. `players.iter().filter(|p| p.response == "pending").filter_map(|p| p.user_id.map(|uid| (uid, &p.email_token)))`.

### restart_core does not itself verify the caller is a player of the old game
- severity: nit
- category: quality
- location: web/src/game/server_fns.rs:868
- finding: Membership in the old game is enforced only by the two callers (`restart_game_with_roster` line 1075, email commands.rs:994), not by `restart_core` itself, even though restart_core is the security-relevant choke point that writes the restart link and creates proposals. A third caller that forgets the check lets any user restart any finished game.
- recommendation: Move (or duplicate) the "You are not a player in this game" check into `restart_core`, which already re-reads the game row under lock.

## Cross-references to W1 (not duplicated here)
- Confirmed: lines 492-495 `find_game_extended(...).await.ok().flatten()` silently swallows DB errors, so `notify_game_emails` receives `before = None` and the email diffing degrades without any log. Same pattern W1 flagged elsewhere.
- The unvalidated bot_name finding above feeds W1's bot-turn wedge (a wedged bot has no recovery path).

## Checked and found CLEAN
- Authentication: every server fn except get_public_index (documented anonymous) and generate_bot_name (nit above) calls get_current_user and rejects None.
- Authorization: get_game_logs, submit_command, undo_game, concede_game, get_restart_prefill, restart_game_with_roster, bump_bot_turns all verify game membership; force_delete_game and bump_bot_turns verify is_admin against the DB (not client data); force_delete_game_impl tested for non-admin rejection.
- restart_core race handling: FOR UPDATE on the old game row, AlreadyRestarted for both the linked-game and open-proposal cases, invite-policy check and email-to-user resolution inside the transaction, duplicate-player dedup after email resolution - solid, and well covered by the sqlx tests in this file.
- Transaction boundaries in restart_core/create_game_from_service: create_game_from_service deliberately neither begins nor commits; early returns drop the tx (implicit rollback); the failed-service-call test (line 1716) pins no-orphan behavior. (Lock-scope quality issue noted above is about duration, not correctness.)
- Panic-freedom in request paths: no unwrap/expect/panic in non-test code except the guarded unwrap at line 1126 (nit) and petname's unwrap_or_else fallback (safe). Test-module unwraps are fine.
- concede_game preconditions (finished, 2-player, membership) all checked before the db call; db::concede_game's 2-player assumption is documented and matches the server fn guard (the TOCTOU is flagged above; the logic itself is correct).
- submit_command delegates turn/finished/position validation to execute_command (game/mod.rs:93-105), which checks is_finished, valid position, and is_turn, and maps UserError to inline feedback rather than a 500 - correct pattern.
- Public index privacy: is_game_publicly_visible is re-checked at render time in render_game_public (selection/render race handled), private log lines excluded (public-only query, tested), pub render (position None) used for spectators.
- roster_error: correct inclusion check with clear message; unit tested including gaps and solo.
- Wire types: SSR-only types correctly cfg-gated; serde derives consistent; GameViewData/PlayerViewData field docs accurate against the code.
- force_delete_game: delete order respects FKs, proposal links nulled, broadcast after delete; regression-tested including proposal references.
- Leptos idioms: expect_context for PgPool/reqwest/jetstream/broadcaster/Resend matches the rest of the project; server fn error style (internal(...) wrapper vs ServerFnError::new for user-facing) applied consistently.
- Test coverage of this file's logic (restart races, prefill, force delete, public index, sidebar ordering) is thorough and behavior-pinning.

Severity tally: 1 critical, 3 major, 4 minor, 4 nit.
