# Raw findings: web-frontend-email W2 (email/commands.rs)

Scope: rust/web/src/email/commands.rs (full read, 2189 LOC incl. tests). Supporting reads: email/inbound.rs (dispatch call sites), game/server_fns.rs (create_game_from_service, undo_game, concede_game, restart_core), db.rs (concede_game, undo_game, update_game_command_success, apply_rating_changes).

### Email-address management commands reachable via spoofable From-header path enable account takeover
- severity: critical
- category: correctness
- location: web/src/email/commands.rs:546
- finding: `run_settings_emails` (emails add/confirm/active/remove) is reachable through `dispatch_standalone_server_command` / `dispatch_settings_standalone`, where user identity comes only from a From-header lookup (W1 flagged that auth model as spoofable). This escalates spoofing into full account takeover: an attacker spoofing the victim's From address sends `emails add attacker@evil.com` (code is emailed to the attacker's own address, so the attacker receives it), then `emails confirm <code>`, then `emails active attacker@evil.com`. The victim's primary address is now the attacker's; all turn emails including per-game-player reply tokens are subsequently delivered to the attacker, giving them game-scoped command auth as the victim too. `emails remove` similarly lets a spoofer strip the victim's secondary addresses. No re-authentication (e.g. confirmation to the existing primary address) gates any of these mutations.
- recommendation: Exclude account-security-sensitive subcommands (emails add/confirm/active/remove, and arguably `name`) from the From-header-authenticated standalone path, or require a confirmation round-trip to the current primary address before switching/adding. At minimum require the game-token-authenticated path for these.

### `bot:<name>` opponents are not validated against enabled bots
- severity: major
- category: correctness
- location: web/src/email/commands.rs:59
- finding: `classify_opponent` returns `OpponentToken::Bot(inner)` for any `bot:`-prefixed token without checking `bot_names` (the check on line 62 only applies to bare tokens). `run_new_command` then passes the arbitrary string straight into `BotSlot { bot_name: difficulty }` and `create_game_from_service` (server_fns.rs:573), which inserts it into `game_bots` with no validation either. `new chess bot:garbage` creates a real game containing a bot player no bot runner will ever serve, wedging the game on that bot's turn forever. This is the same class as the prior-unit finding "unvalidated client-supplied bot slots", here reachable by any inbound email (including the spoofable standalone path).
- recommendation: After `classify_opponent`, reject `Bot(d)` where `d` is not in `bot_names` with a user error listing valid bot names.

### Concede TOCTOU can overwrite a finished game's results
- severity: major
- category: correctness
- location: web/src/email/commands.rs:891
- finding: `run_concede` checks `ge.game.is_finished` on a snapshot read, then calls `crate::db::concede_game` (db.rs:1284), whose transaction has no `WHERE is_finished = false` guard and no `updated_at` optimistic check (unlike `update_game_command_success`, db.rs:1716, which guards on `expected_updated_at`). If the opponent's finishing move commits between the read and the concede (email processing latency makes this window realistic), concede unconditionally rewrites `place` to 1/2 for both players, clobbering the real result and its log. Ratings are protected only by the `rating_change` idempotency guard in `apply_rating_changes` (db.rs:1554), so the stored placings and the stored ratings can end up describing different outcomes. The web server fn `concede_game` (server_fns.rs:808) shares the race; the root cause is the unguarded `db::concede_game`.
- recommendation: In `db::concede_game`, make the games UPDATE `... WHERE id = $1 AND is_finished = false` and abort with a conflict error when 0 rows are affected; surface that as a user-facing "game already finished" reply.

### Undo TOCTOU: `undo_game` applies a snapshot state with no concurrency guard
- severity: major
- category: correctness
- location: web/src/email/commands.rs:966
- finding: `run_undo` reads `undo_game_state` from a snapshot, round-trips to the game service, then calls `db::undo_game` (db.rs:1407), which overwrites `games.game_state` unconditionally - no `updated_at = expected` check like the move path uses (db.rs:1716-1727). If another player moves between the snapshot read and the undo commit, that move is silently reverted (the mover's `undo_game_state` was even NULLed by the interleaved move, but the stale in-memory copy is still applied). Also, `run_undo` does not check `ge.game.is_finished`: undoing a game-finishing move is permitted, `undo_game` sets `finished_at = NULL`, but `rating_change` values written at finish are never rewound, and the idempotency guard in `apply_rating_changes` then suppresses re-rating when the game finishes again with a possibly different result - permanently wrong ratings. Web `undo_game` (server_fns.rs:731) has the identical races.
- recommendation: Add an `expected_updated_at` guard to `db::undo_game` mirroring `update_game_command_success`, mapping 0-rows to the existing Conflict-style user error. Either reject undo on finished games or clear `rating_change` inside the undo transaction.

### run_restart leaks internal errors to email senders as user errors
- severity: major
- category: quality
- location: web/src/email/commands.rs:1044
- finding: `restart_core`'s error is mapped with `.map_err(|e| CommandError::User(e.to_string()))`. `restart_core` returns internal failures too (DB errors, game-service errors), so raw internal error strings are emailed back to the sender verbatim, and because they are classified `User` they bypass the `tracing::error!` logging that inbound.rs:333 applies only to `CommandError::Internal` - internal failures in restart are neither logged nor redacted. Every other command in this file maps internal failures to `CommandError::Internal`.
- recommendation: Distinguish user-facing restart refusals (roster errors, already-restarted) from internal failures; map only the former to `CommandError::User` and the rest to `CommandError::Internal`.

### run_concede/run_undo duplicate the web server fns near-verbatim
- severity: major
- category: simplicity
- location: web/src/email/commands.rs:886
- finding: `run_concede` (886-922) and `run_undo` (924-981) are line-for-line copies of `concede_game`/`undo_game` in game/server_fns.rs:807/731 (same snapshot read, same checks, same service round-trip, same notify/broadcast tail), differing only in how the player is resolved (token `game_player_id`/`position` vs `user.id` lookup) and in that email `run_undo` calls `broadcast_and_trigger` while web undo does too but web concede only broadcasts. Both copies already carry the same latent races (see above); a fix applied to one path will drift from the other. Contrast with `restart`, which was correctly factored into shared `restart_core`.
- recommendation: Extract `concede_core` and `undo_core` (pool + resolved game_player) shared by the server fns and the email commands, as was done for restart.

### `emails confirm` can only confirm the most recently added address
- severity: minor
- category: correctness
- location: web/src/email/commands.rs:731
- finding: `run_emails_confirm` selects the single newest unverified address (`ORDER BY created_at DESC LIMIT 1`) and validates the code against it. If a user has two pending addresses, the code for the older one always fails with "Invalid or expired confirmation code", with no way to confirm it by email short of removing the newer address.
- recommendation: Look up the pending address by matching the code across the user's unverified addresses (join login_confirmations), or validate the code against each unverified address.

### Internal DB errors from validate_confirmation_code masked as "invalid code"
- severity: minor
- category: quality
- location: web/src/email/commands.rs:744
- finding: `.map_err(|_| CommandError::User("Invalid or expired confirmation code."))` discards the error entirely, so a DB outage or query bug during confirmation is reported to the user as a wrong code and never logged (inbound.rs only logs `Internal` errors).
- recommendation: Only map the genuine validation-failure variant to a user error; propagate other errors as `CommandError::Internal`.

### Standalone path rejects subscribe/unsubscribe that help_text advertises
- severity: minor
- category: consistency
- location: web/src/email/commands.rs:305
- finding: `help_text` (served on the standalone path via `dispatch_settings_standalone`) advertises `subscribe`/`unsubscribe` as account-wide and `bump`, but `dispatch_standalone_server_command` only special-cases `new` and `bump`; `subscribe`/`unsubscribe` fall through to the rejection at line 291, whose "Available commands" list also omits `bump` (which is handled) while the user could instead use `emails on/off`. A user without a game replying "unsubscribe" - the most likely standalone reply - gets an error.
- recommendation: Handle `subscribe_toggle` in `dispatch_standalone_server_command` (it only needs pool + user_id), and make the rejection message match the actually-supported set.

### Inline SQL in commands.rs instead of db helpers
- severity: minor
- category: consistency
- location: web/src/email/commands.rs:731
- finding: This file mostly delegates to `crate::db` helpers, but four spots run raw SQL inline: unverified-email lookup (731), login_confirmations cleanup (750), users notification-flag fetch (826-833), games version fetch (1079-1080), plus the three `set_*_emails_enabled` UPDATE helpers (847-884) which duplicate what a single parameterised db helper (or the web settings server-fn path) would provide. Splits the data-access convention and risks drift with the web settings path that toggles the same columns.
- recommendation: Move these queries into db.rs alongside the existing user-email helpers, shared with the web settings server fns.

### Self-mention in `new` opponents is silently dropped
- severity: nit
- category: quality
- location: web/src/email/commands.rs:381
- finding: `if id == ctx.user_id { continue; }` silently ignores the sender naming themself as an opponent. `new chess me myuser` quietly creates a different roster than requested; combined with `roster_error` the resulting count error message ("the request has N (including you)") can confuse.
- recommendation: Return a user error ("you are included automatically") instead of silently skipping.

### `bump` reply does not mention the digest cap
- severity: nit
- category: quality
- location: web/src/email/commands.rs:454
- finding: `bump` caps at `SWITCH_DIGEST_CAP` games via `cap_digest` but the reply says "Re-sent {n} games" with no hint that more games were waiting when the cap was hit.
- recommendation: When capped, append "(capped at N; reply bump again for the rest)" or similar.

### Game-scoped dispatch reserves verbs that could collide with game moves
- severity: nit
- category: consistency
- location: web/src/email/commands.rs:1146
- finding: UNCERTAIN. `concede`, `undo`, `restart`, `rules`, `new`, `bump`, `list`, `help`, and all settings verbs are matched before falling through to `crate::game::execute_command`, so a game whose move grammar includes any of these words (e.g. a hypothetical "list" or "undo" game action) is unplayable by email for that move. No current game is known to collide, but nothing documents the reservation.
- recommendation: Document the reserved-verb set where game command grammars are defined, or support an escape prefix (e.g. leading `/` or `play `) to force game-move interpretation.

## Areas reviewed and found clean

- Settings verb parsing (`settings_verb`, 223-234): case-insensitive, alias-normalised, whitespace-tolerant; well tested.
- `split_new_args`/`resolve_game_type` (69-151): longest-match-first multi-word game-type resolution is correct on attacker-controlled text; empty/unknown inputs produce user errors, no panics.
- `run_new_command` roster validation: duplicate-human check, player-count validation via shared `roster_error`, transactional create + commit before notify; human opponents resolved server-side by username.
- `run_concede`/`run_undo`/`run_restart` membership checks: all verify the token's game_player/user is actually in the game before mutating (no cross-game authorization gap given the token routing).
- `run_restart` uses shared race-safe `restart_core` (FOR UPDATE serialisation, AlreadyRestarted outcome) - the right pattern the concede/undo paths should follow.
- Undo stale-state hazard from old undo snapshots is bounded by `update_game_command_success` (db.rs:1749-1753) NULLing `undo_game_state` for all non-moving players each move; only the TOCTOU window flagged above remains.
- Email add/remove/active state machine (ownership check, unavailable on other-owner, cannot remove primary, unverified cannot become primary) is correct per-command and thoroughly integration-tested.
- No unwrap/expect/panic on untrusted input anywhere in the non-test code; all fallible paths return CommandError.
- Internal errors are wrapped with per-command context strings (good log ergonomics), and inbound.rs logs Internal and replies generically (except the restart gap flagged above).
- Test coverage is strong for parsing, settings, and email management; bump tests cover presence-bypass semantics.

Severity tally: critical 1, major 4, minor 4, nit 3.
