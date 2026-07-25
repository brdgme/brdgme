# Findings: web-domain (web crate domain logic)

Scope: `game/mod.rs`, `game/export.rs`, `game/import.rs`, `game/server_fns.rs`,
`proposals.rs`, `stats/`, `players.rs`, `friends.rs`, `new_game.rs`,
`game_info/`, `models/`, `rules.rs`, `settings.rs`, `index.rs` - ~14.2k LOC,
19 files. Snapshot `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. Raw worker
dumps and the review log are in `findings/raw/web-domain-*.md`; every finding
below was verified or spot-checked by the Lead against the snapshot. Two
cross-file duplicates were merged during curation (unvalidated bot slots;
client page-number overflow), noted inline.

## game/mod.rs, game/export.rs, game/import.rs (bot-command pipeline)

NATS handoff from the web-server unit was resolved here: `term()`/`nak()`/
`in_progress()` are never called anywhere in web/src; the consumer acks
exactly once after all work; worst-case processing (10s timeout x 3 attempts)
is far under the 5-min ack_wait. The web-server "stranded messages" finding
is CONFIRMED and stays minor; the ack_wait concern is closed (bounded
processing, though unguarded by in_progress pings).

### Bot command permanently rejected (UserError) wedges the game with the bot on turn
- severity: major
- category: correctness
- location: web/src/game/mod.rs:304-314 (consumer ack) + web/src/game/mod.rs:402-410 (handler)
- finding: When the game service rejects a bot's command as a user error (buggy bot, or bot computed against subtly different state), the consumer acks the message and nothing re-publishes `bot.turn`. The bot is still `is_turn = true` in the DB; the turn-reminder sweep only targets human users (web/src/email/sweep.rs:34-91), and no other path calls `trigger_bot_turns` for that game (it only runs as the epilogue of a successful `execute_command`, mod.rs:168 - which no human can perform while it is the bot's turn). The game is permanently stuck. Re-publishing `bot.turn` (bounded by the attempt counter) would let the bot recompute a fresh command from current state.
- recommendation: On `UserError` from a bot command, re-publish `bot.turn` with `attempt + 1` (same bounded path as Conflict) instead of acking into the void; only give up (ack) after `MAX_TURN_ATTEMPTS`, and emit a distinct error/metric for the wedged game.

### Turn-retry exhaustion silently abandons the bot's turn
- severity: major
- category: correctness
- location: web/src/game/mod.rs:372-383
- finding: After `MAX_TURN_ATTEMPTS` stale-state conflicts, `handle_bot_command_event` logs an error and returns `Ok(())`, which the consumer acks. Same wedge as above: bot still on turn, no re-drive mechanism, no alertable signal beyond a log line. Conflicts should be vanishingly rare (they need a concurrent write between the bot's read and commit), so exhaustion implies something is systematically wrong - exactly the case that deserves a loud, durable signal.
- recommendation: Treat exhaustion as a real failure: emit a metric/Sentry event, or park the message (term + DLQ subject) so stuck games are discoverable; consider a periodic sweeper that re-publishes `bot.turn` for games where a bot has been on turn longer than a threshold.

### Failed `bot.turn` publish after DB commit loses the bot turn
- severity: major
- category: correctness
- location: web/src/game/mod.rs:227-242 (publish failures warn-only), web/src/game/mod.rs:390-397 (conflict re-publish query failure warn-only)
- finding: `publish_bot_turns` awaits the JetStream persistence ack (good), but on failure only logs a warn. Since the preceding `update_game_command_success` has already committed, the game now sits with a bot on turn and no event in the stream - permanently wedged (same absence of recovery as above). Same for the conflict path: if `find_bot_turns` fails, the re-publish is skipped silently and the original `bot.command` is then acked (mod.rs:400).
- recommendation: A reconciliation sweep ("bot on turn for > N minutes -> re-publish bot.turn") fixes all three wedge modes at once; short of that, surface publish failures as Err so the `bot.command` message stays unacked and redelivery retries the publish.

### bot.command consumer is spawned once and never restarted if it exits
- severity: major
- category: correctness
- location: web/src/main.rs:55-74 + web/src/game/mod.rs:263-325
- finding: `run_bot_command_consumer` returns `Ok(())` when the `messages()` stream ends (mod.rs:322-325) and `Err` on setup/stream failures; the spawn site (main.rs:62-73) only logs the error - no supervisor loop, no retry. If the consumer's message stream ever terminates (consumer deleted/recreated by another replica racing at startup, NATS-side error), every replica that experiences it silently stops driving bot turns until the pod restarts. UNCERTAIN how often `consumer.messages()` terminates in practice; the structural spawn-and-forget fragility is certain.
- recommendation: Wrap the consumer in a reconnect loop with backoff (the `Err` branch at main.rs:71 should restart, and the `Ok(())` stream-end path should also restart rather than exit).

### Permanently failing bot.command messages strand in the stream after max_deliver (no term/DLQ)
- severity: minor
- category: correctness
- location: web/src/game/mod.rs:315-320 (leave-unacked path); consumer config web/src/nats.rs:63-94
- finding: `term()` is never called anywhere in web/src. A message that fails with `Other` on all 3 deliveries (e.g. game service down longer than the redelivery window, or a duplicate-delivery "Not your turn" case) just stops being delivered; it sits in the WorkQueue stream forever with no advisory handling, no metric, and no way to enumerate stranded messages operationally. Cross-ref: same conclusion as the web-server unit's nats finding.
- recommendation: On `Other`, track deliveries (or use `message.info()` num_delivered) and `term()` at the ceiling with an error log + metric; or add a DLQ subject. At minimum, count/metric the stranded case.

### Finished games wipe is_eliminated for previously eliminated players
- severity: minor
- category: correctness
- location: web/src/game/mod.rs:36-41 (status_fields Finished arm) + web/src/db.rs:1744
- finding: When a game transitions to Finished, `status_fields` emits `eliminated: vec![]` (the `Status::Finished` variant carries no eliminated list), and `update_game_command_success` unconditionally rewrites `is_eliminated` from it - so a player eliminated mid-game flips back to `is_eliminated = false` when the game finishes. Likely harmless today (finished games have `place` set and no turn reminders fire), but it silently rewrites historical per-player data. UNCERTAIN whether any UI/stats consumer reads `is_eliminated` for finished games.
- recommendation: Preserve the Active-arm eliminated list on finish (carry it in StatusUpdate), or make `update_game_command_success` not touch `is_eliminated` when `status.is_finished`.

### Export bundle includes private log bodies despite "may get pasted into issues"
- severity: minor
- category: quality
- location: web/src/game/export.rs:1-4, 105-134
- finding: The module doc says the bundle "may get pasted into issues" and only excludes email addresses. But `game_logs` rows include private logs (`is_public = false`) with their full bodies (and the target positions), and the game_state blob itself may encode hidden information (other players' hands). An admin pasting a bundle into a public issue leaks in-game private communication/hidden state. UNCERTAIN whether spec D4 accepted this deliberately - flagged for a conscious decision.
- recommendation: Either document in the module header that private logs/hidden state are included and bundles must not be posted publicly, or add a `--redact-private` mode.

### Stale `before` snapshot errors swallowed silently in handle_bot_command_event
- severity: nit
- category: quality
- location: web/src/game/mod.rs:344-347
- finding: `find_game_extended(...).await.ok().flatten()` discards a DB error without logging it. If the read fails, email notifications go out with `before = None` (notify_game_emails then treats `was_finished` as false and diffs against nothing), and the failure is invisible. Same pattern in submit_command (server_fns.rs:492-495) - consistent, but still silent.
- recommendation: Log a warn on the Err branch before falling back to `None`.

### Conflict re-publish fans out to ALL bots currently on turn, not just the conflicting one
- severity: nit
- category: correctness
- location: web/src/game/mod.rs:390-392
- finding: On a stale-state conflict, `find_bot_turns` returns every bot on turn and all get a fresh `bot.turn` with `attempt + 1` - including bots whose turns are already in flight from an earlier publish (possible in simultaneous-turn games). The duplicates are caught by the is_turn/updated_at guards, so the blast radius is noise + redelivery burn, not corruption. The shared attempt counter also advances for the bystander bots.
- recommendation: Re-publish only for the event's `player_position` (filtered from `find_bot_turns`), keeping the attempt semantics per-bot.

### BundlePlayer.bot_name actually holds game_bots.name (the seat display name), not the bot type
- severity: nit
- category: consistency
- location: web/src/game/export.rs:53-54, 142 vs web/src/game/import.rs:90-99
- finding: The field is named `bot_name` (matching `game_bots.bot_name`, the bot type, everywhere else in the schema) but is populated with `game_bots.name` (the per-game seat name) - the field comment documents this, and import.rs:83,93 correctly keys `bot_ids` by `bot.name`, so it round-trips correctly. Purely a naming trap for future readers/maintainers of the bundle format.
- recommendation: Rename the bundle field to `game_bot_name`/`seat_name` at the next schema_version bump, or leave as-is deliberately.

### Bundle timestamps captured on export but dropped on import
- severity: nit
- category: consistency
- location: web/src/game/export.rs:44-45,77 vs web/src/game/import.rs:59-68,140-150
- finding: `BundleGame.created_at`/`updated_at` and `BundleLog.created_at` are exported but never inserted (games insert omits created_at; game_logs insert only sets logged_at). Imported games all show "created just now". Dev-only tool, so impact is cosmetic fidelity loss; also odd that the schema carries fields the importer ignores.
- recommendation: Insert the bundle timestamps where columns allow, or drop them from the bundle to keep the schema honest.

### placeholder_user check-then-insert can race on username uniqueness
- severity: nit
- category: correctness
- location: web/src/game/import.rs:171-193
- finding: `taken` is checked, then `INSERT INTO users (name ...)` runs separately; a concurrent insert of the same name fails the whole import transaction with a unique-violation error rather than falling back to a generated name. Single-threaded dev CLI in practice, so low priority.
- recommendation: Catch the unique-violation on insert and retry with `generate_unique_username`, or rely on it unconditionally.

### Imported players get is_turn_at/last_turn_at = NOW() regardless of turn state
- severity: nit
- category: correctness
- location: web/src/game/import.rs:106-108
- finding: Both timestamp columns are set to NOW() for every imported player. For the player on turn this resets the turn-age clock (turn-reminder sweep eligibility, turn-duration stats); for others `last_turn_at` claims they just played. Dev-only; no user-visible harm beyond odd local stats/reminder timing.
- recommendation: Set `is_turn_at` = NOW() only when `player.is_turn`, and leave `last_turn_at` at the column default/NULL.

## game/server_fns.rs

### undo_game allows undoing a finished game, permanently corrupting ratings
- severity: critical
- category: correctness
- location: web/src/game/server_fns.rs:731
- finding: `undo_game` never checks `ge.game.is_finished`. When a player makes the game-finishing move, `update_game_command_success` (db.rs:1749) keeps that player's `undo_game_state` (is_played && can_undo), and `apply_rating_changes` runs immediately at finish. The finisher can then call undo: `db::undo_game` (db.rs:1407) reverts `game_state`, sets `is_finished` per status and `finished_at = NULL`, but does NOT clear `rating_change`/`rating_before` on game_players and does not rewind `game_type_users` ratings. Result: (a) ratings from the undone outcome stick; (b) when the game finishes again (possibly with different placings), the idempotency guard in `apply_rating_changes` (db.rs:1554, "any player already has a rating_change") trips and the real outcome is never rated. The UI exposes this path normally (`can_undo` in GameViewData is just `undo_game_state.is_some()`).
- recommendation: Reject undo when `ge.game.is_finished` (or when `rating_change` is set for any player), or make `db::undo_game` rewind rating changes atomically. Simplest: `if ge.game.is_finished { return Err("Game is already finished") }` in the server fn plus a matching guard inside `db::undo_game`.

### undo_game has no stale-state guard - can clobber a concurrent move
- severity: major
- category: correctness
- location: web/src/game/server_fns.rs:784
- finding: The command path is protected by optimistic locking (`update_game_command_success` matches `WHERE updated_at = $expected`), but the undo path is not. `undo_game` reads the game via `find_game_extended` (line 748), makes an HTTP round-trip to the game service, then calls `db::undo_game` which unconditionally does `UPDATE games SET game_state = $1 ... WHERE id = $3` (db.rs:1416-1417) with no `updated_at` check. If another player (or a bot, which moves quickly after `broadcast_and_trigger`) plays between the read and the write, their move is silently destroyed and every player's `undo_game_state` is wiped (db.rs:1440).
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
- finding: `get_game_details` requires only "any authenticated user"; it never checks whether the viewer may see this game. Any logged-in user who has (or guesses/obtains) a game UUID gets the spectator render, player names, ratings, rating changes, and recent form - even when every player has `game_visibility = 'private'`. The anonymous index path carefully gates on `is_game_publicly_visible` (line 370), and db.rs even defines the exact helper needed, `is_game_visible_to_user` (db.rs:2228, viewer-is-player OR public OR friends), but it has almost no callers (only index.rs's friend-activity path) - the intended gate was never wired up here.
- recommendation: In `get_game_details`, when the viewer is not a player of the game, require `crate::db::is_game_visible_to_user(&pool, game_id, user.id)` and return "Game not found" otherwise; or document that `game_visibility` deliberately only affects the logged-out index and friend feeds.

### Game-service HTTP call made inside an open transaction holding FOR UPDATE lock
- severity: minor
- category: quality
- location: web/src/game/server_fns.rs:947
- finding: In the solo-restart branch, `create_game_from_service` performs the game-service HTTP request (line 583) while the transaction begun at line 888 holds the `FOR UPDATE` lock on the old game row and a pool connection. A slow or hung game service pins the DB connection and blocks every other operation touching that game row for the duration of the reqwest timeout. The atomicity rationale in the doc comment (line 568) only requires the DB writes to be atomic, not the HTTP call.
- recommendation: Call the game service for the new game state BEFORE `pool.begin()`, then do the lock + checks + inserts + link + commit purely with the already-fetched response.

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
- finding: `should_hide_add_friend` is awaited sequentially in a loop, once per human opponent (each itself a query, db.rs:1998), on the hottest read path of the app (every game page load, re-fetched on every websocket-driven refresh). For a 6-player game that is 5 extra round trips on top of the already numerous per-request queries.
- recommendation: Add a batched `should_hide_add_friend_many(pool, viewer, &[Uuid]) -> HashSet<Uuid>` using `= ANY($1)`, or fold it into the recent-form query which already takes `&human_user_ids`.

### get_game_logs is_new compares created_at against last_turn_at but displays logged_at
- severity: nit
- category: correctness
- location: web/src/game/server_fns.rs:704
- finding: `let is_new = log.created_at >= last_turn_at;` while the entry sorts/displays by `logged_at`. For normally-inserted logs the two are near-identical, but any log whose `logged_at` differs from `created_at` (backfill, undo/concede logs) can be highlighted inconsistently with its position in the list.
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
- recommendation: Use `filter_map` yielding the unwrapped id.

### restart_core does not itself verify the caller is a player of the old game
- severity: nit
- category: quality
- location: web/src/game/server_fns.rs:868
- finding: Membership in the old game is enforced only by the two callers (`restart_game_with_roster` line 1075, email commands.rs:994), not by `restart_core` itself, even though restart_core is the security-relevant choke point that writes the restart link and creates proposals. A third caller that forgets the check lets any user restart any finished game.
- recommendation: Move (or duplicate) the "You are not a player in this game" check into `restart_core`, which already re-reads the game row under lock.

## proposals.rs

### get_proposal serializes every invitee's email_token to any authenticated user
- severity: major
- category: correctness
- location: web/src/proposals.rs:78
- finding: `ProposalPlayerView` includes `pub email_token: Option<String>` (line 78), populated by `find_proposal_roster` (lines 511-523) and returned by the `get_proposal` server fn (lines 1744-1764) to ANY authenticated user - the viewer role is computed (Owner/Invitee/Other, lines 1748-1754) but never used to gate data; a `ViewerRole::Other` caller who knows/obtains the proposal id gets the full roster including all invitees' email tokens. The email token is the credential the inbound email handler uses to accept/decline on an invitee's behalf (email/inbound.rs:594). Inbound does additionally verify the From address matches the invitee's verified email (inbound.rs:611), so exploitation requires From spoofing past the Resend webhook, but the token is a secret and there is zero reason to ship it to any browser - not even the invitee's own.
- recommendation: Drop `email_token` from `ProposalPlayerView` (and the roster SELECT). Nothing in the client UI uses it.

### Client-supplied bot slots stored and used without validation (three entry points)
- severity: major
- category: correctness
- location: web/src/proposals.rs:1163 (create_proposal), web/src/proposals.rs:1469 (add_proposal_player), web/src/game/server_fns.rs:868 (restart_core)
- finding: All three entry points take `BotSlot { name, bot_name }` straight from the client and insert it as an auto-`"accepted"` slot; `start_proposal_tx` (lines 977-984) then feeds those strings into game creation, which also stores them unvalidated (db.rs:1093-1098). None of the paths check the bot type against `find_enabled_bots` (the list `get_available_bots` exposes). The email command path DOES validate (email/commands.rs:354), so the web paths are strictly weaker. A bogus bot name/difficulty produces a game whose bot player can never take a turn, wedging the game for the humans in it (the bot-turn wedge class above has no recovery path). Merged during curation: originally flagged independently by W2 (restart_core, minor) and W3 (proposals, major); rated major because the outcome is an unrecoverable wedged game reachable from normal client input.
- recommendation: Validate `bot.bot_name` against the enabled-bots list (and non-empty `bot.name`) at each entry point (or in one shared choke point) before insert; reject with a user-facing error.

### Auto-decline keyed on proposal created_at, not the player's invite time
- severity: major
- category: correctness
- location: web/src/proposals.rs:819
- finding: `fetch_auto_decline_candidates` selects pending human slots where `gp.created_at < NOW() - interval` (line 819) - the PROPOSAL's age, not the player row's. Two concrete failure modes: (1) the owner adds a new invitee to a proposal older than the threshold via `add_proposal_player`; the next sweep (email/sweep.rs:359) instantly auto-declines them before they ever see the invite; (2) a roster change resets accepted humans back to `pending` with fresh tokens (`reset_accepted_humans_for_roster_change`, lines 658-685) - on an old proposal those players are auto-declined on the next sweep tick, and `declined` is terminal (respond_proposal lines 1235-1246), killing the proposal. The nudge query (line 725) has the same proposal-age keying, which is only cosmetic there.
- recommendation: Key the auto-decline window on `pp.created_at` (or `pp.updated_at`, which the reset bumps), not `gp.created_at`.

### Owner can decline their own proposal and permanently wedge it
- severity: minor
- category: correctness
- location: web/src/proposals.rs:1229
- finding: `respond_proposal` looks the caller up in the roster (lines 1229-1232) with no owner exclusion. The owner's row is `"accepted"`, so `accepted -> declined` is allowed. Once declined: declined is terminal (cannot re-accept), `remove_proposal_slot` refuses to remove the owner (lines 1619-1623), and `start_proposal` rejects any declined slot (lines 1331-1336). The only exits are cancel or transfer-then-remove; the natural repair paths are all blocked.
- recommendation: Reject `respond_proposal` when `user.id == proposal.owner_user_id` ("Cancel the invite instead"), or treat owner-decline as cancellation.

### Ownership can be transferred to a declined (or pending) invitee
- severity: minor
- category: correctness
- location: web/src/proposals.rs:1696
- finding: `transfer_proposal_ownership` only checks the target is in the roster, not their response. Transferring to a declined player creates a proposal whose owner has a terminal `declined` response: it can never start, the owner slot cannot be removed, and the response cannot change. Transfer to a pending player is odd but recoverable.
- recommendation: Require the target's response to be `"accepted"` (or at least not `"declined"`).

### cancel_proposal notifies from a roster snapshot taken before the lock
- severity: minor
- category: correctness
- location: web/src/proposals.rs:1532
- finding: `players` is fetched from the pool BEFORE the transaction begins and the proposal is locked (lines 1536-1544); the accepted-invitee list for `notify_cancelled` (lines 1563-1569) is derived from that stale snapshot after commit. An invitee whose accept commits between the fetch and the lock gets no cancellation email. Every other mutating fn here reads players via `find_proposal_players_tx` inside the lock; cancel is the odd one out. Same read-then-lock TOCTOU shape as concede_game above.
- recommendation: Move the `find_proposal_players` call inside the transaction (use `find_proposal_players_tx` after `lock_proposal_for_update`).

### notify_owner_decline bypasses the invite-email gates every other mailer applies
- severity: minor
- category: consistency
- location: web/src/proposals.rs:286
- finding: `notify_owner_decline` (lines 286-328) checks only that the owner has an email. Every other recipient-facing mailer method applies `invite_recipient_should_send` (verified primary + `invite_emails_enabled` + web-presence suppression). An owner who disabled invite emails, or who is actively on the site watching the proposal page, still gets the decline email.
- recommendation: Apply the same `suppress_for_web_presence` + `invite_recipient_should_send` gate in `notify_owner_decline`.

### Notification emails carry a dead Reply-To and a footer inviting replies
- severity: minor
- category: correctness
- location: web/src/proposals.rs:324
- finding: `notify_owner_decline`, `notify_cancelled`, `notify_started`, and `notify_owner_ready` set the reply address to `i-{proposal_id}@brdg.me` (lines 324, 365, 410, 460). `proposal_id` renders as a hyphenated UUID, which is not a player `email_token` (tokens are `Uuid::new_v4().simple()`), so `handle_invite_reply`'s token lookup (inbound.rs:594) always misses and the reply is silently dropped. Meanwhile every one of these emails ends with the footer "Reply to this email to respond, or unsubscribe anytime." (lines 315, 356, 401, 451) - a reply the system cannot process.
- recommendation: Use a no-reply address (or the recipient's own token where one exists) and drop the "Reply to this email to respond" footer from pure notification emails.

### Mailer tasks swallow DB errors silently; empty names produce broken subjects
- severity: minor
- category: quality
- location: web/src/proposals.rs:170
- finding: All six `RealInviteMailer` methods use `let Ok(Some(..)) = ... else { return }` on `find_proposal` / `fetch_invite_recipient` inside spawned tasks, so a DB error at send time is indistinguishable from "recipient opted out" and leaves no trace in logs. `proposal_game_type_name` (lines 170-178) likewise collapses errors into `String::new()`, and owner/invitee name lookups fall back to `unwrap_or_default()` (lines 201-206, 300-305), yielding subjects like " invite from " when a lookup fails.
- recommendation: Log (`tracing::warn!`) on the error arms before returning; consider skipping the send rather than sending an email with blank substitutions.

### Pre-transaction authz block duplicated verbatim in four server fns
- severity: minor
- category: simplicity
- location: web/src/proposals.rs:1396
- finding: `add_proposal_player` (1396-1421), `cancel_proposal` (1519-1552), `remove_proposal_slot` (1585-1610), and `transfer_proposal_ownership` (1666-1691) each run the identical find -> owner-check -> open-check sequence twice: once against the pool before `begin()`, then again after `lock_proposal_for_update`. The in-lock check is the authoritative one; the pre-check only changes which error a racing caller sees, and `respond_proposal`/`start_proposal` get by fine with the in-lock check alone. Roughly 60 lines of copy-paste that must be kept in sync.
- recommendation: Drop the pre-transaction checks (or extract a `lock_owned_open_proposal(tx, id, user_id)` helper) so each fn checks once, inside the lock.

### RespondOutcome.started/game_id are always false/None; client nav path is dead
- severity: minor
- category: simplicity
- location: web/src/proposals.rs:62
- finding: `respond_proposal` always returns `RespondOutcome { accepted, started: false, game_id: None }` (lines 1277-1281) - a leftover from an auto-start design (the `respond_accept_does_not_auto_start` test confirms auto-start was removed). The client effect (lines 1836-1845) still branches on `outcome.game_id` and navigates to `/games/{gid}`, code that can never execute.
- recommendation: Shrink `RespondOutcome` to `accepted` (or unit) and delete the dead navigation branch.

### Invite emails are never trimmed or case-normalized
- severity: minor
- category: correctness
- location: web/src/proposals.rs:891
- finding: `find_or_create_user_by_email_tx` looks up `user_emails` by exact string (line 891) and inserts the raw client string (lines 912-919); `check_invite_policy_tx` (db.rs:2383) is also exact-match, and the UI submits the field untrimmed. `user_emails.email` has a case-sensitive UNIQUE constraint (migrations/001_initial_schema.sql:275), so inviting `Foo@x.com` when `foo@x.com` is registered silently creates a second account for the same mailbox, and the invite-policy check (blocks, friends-only) is bypassed for the real user. A trailing space does the same. See also the settings.rs and new_game.rs instances below - one canonicalization policy should fix all three.
- recommendation: Trim and lowercase invite emails at the server-fn boundary before lookup/insert (and ideally enforce lower-cased storage globally).

### Nudge sweep re-sends invites without re-checking proposal/player state
- severity: minor
- category: correctness
- location: web/src/proposals.rs:719
- finding: `fetch_nudge_candidates` snapshots (proposal, user, token) rows; the sweep (email/sweep.rs:288-305) then fire-and-forgets `send_invite` and marks the proposal nudged. `send_invite` (lines 182-232) re-fetches the proposal but never re-checks `status == 'open'` nor that the player is still `pending` with that token. Between snapshot and send the invitee may have responded, the roster may have rotated the token, or the proposal may have been cancelled/started - the stale "reply accept to join" email still goes out, and its reply-to token may no longer match.
- recommendation: In `send_invite`, verify the proposal is still open and the token still matches a pending row before sending.

### cancel_proposal_for_expiry swallows follow-up query errors into "no notifications"
- severity: minor
- category: quality
- location: web/src/proposals.rs:788
- finding: After successfully flipping the proposal to `cancelled`, the owner lookup errors are discarded with `.ok().flatten()` and `owner?` (lines 788-795), and the accepted-players query with `.unwrap_or_default()` (lines 796-802). A transient DB error after the UPDATE means the cancellation is committed but every accepted invitee's "invite was cancelled" email is silently skipped, with no log line (unlike the UPDATE error arm at line 783, which logs).
- recommendation: Log the error before returning None/empty so dropped notifications are observable.

### Interval built by string concatenation instead of a typed bind
- severity: nit
- category: quality
- location: web/src/proposals.rs:725
- finding: Three sweep queries bind `threshold_secs.to_string()` and build the interval as `($1 || ' seconds')::interval` (lines 725, 755, 819). It is parameterized (no injection), but binding text to synthesize an interval is roundabout.
- recommendation: `make_interval(secs => $1)` with a numeric bind, or `NOW() - $1 * interval '1 second'`.

### reset_accepted_humans_for_roster_change issues one UPDATE per player
- severity: nit
- category: simplicity
- location: web/src/proposals.rs:672
- finding: SELECT then a per-row UPDATE loop (lines 672-684) to assign fresh tokens. Rosters are small so this is harmless, but a single `UPDATE ... RETURNING user_id, email_token` would do it in one round trip.
- recommendation: Optional: collapse to a single UPDATE ... RETURNING.

### count_pending_human_invitees (pool variant) is dead code
- severity: nit
- category: quality
- location: web/src/proposals.rs:701
- finding: Only the `_tx` variant (line 873) has a caller (email/inbound.rs:735); the pool variant at line 701 is unused.
- recommendation: Delete it.

### Missing player_counts row degrades to a garbled error message
- severity: nit
- category: quality
- location: web/src/proposals.rs:1341
- finding: `start_proposal` (line 1341) and `respond_proposal` (line 1261) use `.unwrap_or_default()` when `find_game_type_player_counts` returns None (game type row missing), so `roster_error` renders "This game supports  players, but ...". `create_proposal` (line 1066) instead errors with "Game type not found".
- recommendation: Treat None as an internal error, matching create_proposal.

### Error-context labels say "create_proposal" from other call sites
- severity: nit
- category: consistency
- location: web/src/proposals.rs:896
- finding: `find_or_create_user_by_email_tx` hardcodes `internal("create_proposal: resolve email")` (lines 896, 911, 919) but is also called from `add_proposal_player` (line 1430), so failures there log under the wrong fn. Also `cancel_proposal`, `remove_proposal_slot`, and `get_pending_invites` lack the `tracing::instrument` attribute the other server fns carry.
- recommendation: Neutral context strings ("resolve invite email"); add the instrument attribute to the three uninstrumented server fns.

## stats/

### Stats endpoints bypass game_visibility privacy settings
- severity: major
- category: correctness
- location: web/src/stats/mod.rs:174
- finding: All three server fns (get_player_profile mod.rs:174, get_player_game_type_stats mod.rs:231, get_player_history mod.rs:297) are anonymous-accessible and return per-game data with no visibility check: game ids, opponent identities (user_id + name + place via opponents_by_game queries.rs:206), active/unfinished games (mod.rs:206), and head_to_head aggregates naming every human opponent (queries.rs:476). The project has a game_visibility model ('public'/'friends'/private, db.rs:2228 is_game_visible_to_user) but none of the stats queries consult it. A user who sets game_visibility to friends-only or private still appears by name, user id and placement on every other participant's public profile, game-type page and history, and their shared games' ids are enumerable. Same class as the get_game_details gate above, but separate endpoints with their own leak surface.
- recommendation: Decide the intended semantics: either filter games whose participants' visibility excludes the viewer (extend the queries with the all-participants-visible predicate used by is_game_visible_to_user), or at minimum anonymize opponents whose game_visibility is not public (drop user_id, mask name) in opponents_by_game and head_to_head.

### Client-controlled page can overflow offset computation
- severity: minor
- category: correctness
- location: web/src/stats/mod.rs:318 (also web/src/players.rs:771 client side)
- finding: `let offset = (page - 1) * page_size;` with `page: i64` taken directly from the client and only clamped by `page.max(1)`. `page = i64::MAX` overflows the multiplication: panic in debug builds, wrap to a negative offset in release, which Postgres rejects ("OFFSET must not be negative") and surfaces as a 500. The client side (players.rs:771) parses and forwards any i64 with no ceiling, and `Some(d.page + 1)` at players.rs:845 would also overflow at i64::MAX. Merged during curation with the client-side instance.
- recommendation: Clamp page to a sane upper bound (e.g. `page.clamp(1, 1_000_000)`) or use checked_mul and treat overflow as page 1; optionally clamp client-side too.

### Base rating 1200 hardcoded in rating_series reconstruction
- severity: minor
- category: correctness
- location: web/src/stats/queries.rs:183
- finding: `let mut rating = 1200;` reconstructs the rating series by summing rating_change from a hardcoded base. The same 1200 constant necessarily exists wherever ratings are initialized (game_type_users default). If the starting rating ever changes, or a player's rating was ever adjusted outside rating_change rows, the whole series (and its final point, which the profile implies equals current rating) silently drifts.
- recommendation: Pull the base into a shared `const INITIAL_RATING: i32` used by both rating logic and this reconstruction, or reconstruct from game_players.rating_before where available.

### get_player_game_type_stats computes stats for every game type to use one row
- severity: minor
- category: quality
- location: web/src/stats/mod.rs:256
- finding: The game-type page calls `game_type_stats(&pool, user.user_id, include_single_human)` (the full per-type aggregate over all of the user's game types, each row involving correlated player-count subqueries) and then `.find(|s| s.game_type_name == canonical)` to keep a single row, discarding the rest.
- recommendation: Add a game_type filter parameter to game_type_stats (`AND gt.name = $n`, nullable like finished_games' $3) and pass the canonical name.

### finished_games unbounded on game-type page
- severity: minor
- category: quality
- location: web/src/stats/mod.rs:274
- finding: get_player_game_type_stats calls `finished_games(..., None)` with `limit: None`, so `LIMIT $4::bigint` binds NULL (no limit) and the endpoint returns every finished game of the user for that type plus a full opponents map, serialized into one server-fn response. rating_series (mod.rs:271) and head_to_head (mod.rs:283) are likewise unbounded. For a long-lived account this is an unbounded payload on a public, anonymous endpoint.
- recommendation: Cap the game-type page list (e.g. Some(100)) and point to the paginated history page for the rest; consider capping rating_series by sampling or a LIMIT on most recent N.

### Single-human eligibility predicate duplicated across seven queries
- severity: minor
- category: simplicity
- location: web/src/stats/queries.rs:57
- finding: The correlated subquery `(SELECT count(*) FROM game_players ... user_id IS NOT NULL) >= CASE WHEN $n THEN 1 ELSE 2 END` is copy-pasted in overall_totals (57), game_type_stats (100), finished_games (267), game_history (398), game_history_count (463), head_to_head (493) and recent_form (571), with recent_form_for_game_type (queries.rs:648) hardcoding `>= 2` instead of taking the flag. A future change to the eligibility rule must be made in eight places, and the hardcoded variant already diverges in shape.
- recommendation: Factor into a SQL helper (inline SQL fragment constant, or a view / generated column) so the rule lives in one place; give recent_form_for_game_type the same parameterized form or a comment stating single-human is deliberately always excluded.

### game_history runs four correlated subqueries per row
- severity: minor
- category: quality
- location: web/src/stats/queries.rs:387
- finding: Each history row computes player_count plus match_min/match_max/match_avg as four separate correlated subqueries over game_players (lines 387-390), i.e. four extra scans of the same rows per game, 200 per 50-row page, on the hottest stats page.
- recommendation: Collapse into one `LEFT JOIN LATERAL (SELECT count(*), min(rating_before), max(rating_before), avg(rating_before)::int ...) agg ON true`.

### game_type history filter is exact-match while everything else is case-insensitive
- severity: minor
- category: consistency
- location: web/src/stats/mod.rs:297
- finding: get_player_history passes the client-supplied `game_type` string straight through to `gt.name = $3` (queries.rs:397, 462) without canonicalizing via find_game_type_name, unlike get_player_game_type_stats which resolves case-insensitively (mod.rs:248). A history link built from a lowercased URL segment silently filters to zero rows instead of matching.
- recommendation: Resolve the filter through find_game_type_name first, matching the game-type page behavior.

### Checked query macros and runtime query_as mixed without cause
- severity: nit
- category: consistency
- location: web/src/stats/queries.rs:211
- finding: opponents_by_game, game_history and game_history_count use runtime-checked `sqlx::query_as` with hand-written FromRow structs, while every other query in the module uses the compile-time-checked `sqlx::query!` macro. The binds are all static, so these forfeit compile-time SQL checking for no benefit.
- recommendation: Convert to `sqlx::query!` or note why runtime checking was needed.

### SVG viewBox literals duplicate the chart dimension constants
- severity: nit
- category: consistency
- location: web/src/stats/viz.rs:128
- finding: `viewBox="0 0 320 120"` is hardcoded in RatingChart (viz.rs:128) and Histogram (viz.rs:210) while all coordinate math uses CHART_WIDTH/CHART_HEIGHT/HIST_WIDTH/HIST_HEIGHT constants. Changing a constant silently clips or letterboxes the chart.
- recommendation: Build the viewBox string from the constants.

### finished_at DESC ordering puts NULLs first for legacy finished games
- severity: nit
- category: correctness
- location: web/src/stats/queries.rs:271
- finding: finished_games orders by `g.finished_at DESC, g.id`; finished_at is nullable. Postgres sorts NULLs first under DESC, so any legacy is_finished=true row with NULL finished_at pins to the top of "recent" lists and eats the LIMIT budget.
- recommendation: `ORDER BY g.finished_at DESC NULLS LAST, g.id` (recent_form's window ORDER BY at queries.rs:563 deserves the same treatment).

## players.rs, friends.rs, new_game.rs

### Concurrent cross requests hit friends_pair_key instead of auto-accepting
- severity: minor
- category: correctness
- location: web/src/db.rs:1889 (surfaced via web/src/friends.rs:181)
- finding: `send_friend_request` does SELECT-then-INSERT inside a transaction with no row lock. If A->B and B->A are sent concurrently, both see `row: None` and both INSERT; one commit fails the `friends_pair_key` unique index (migrations/010_friends.sql:7) and the user gets a generic internal error instead of the intended mutual-intent auto-accept. Data stays consistent (the index holds), but the D1 "requester cannot distinguish outcomes" behavior is violated by an error on a legitimate request.
- recommendation: Catch the 23505 unique violation and retry the transaction once (the retry will take the `Some(r)` branch), or use `INSERT ... ON CONFLICT` on the pair expression.

### Friends page mutation errors are silently swallowed
- severity: minor
- category: quality
- location: web/src/friends.rs:426
- finding: Only `add_action` errors are rendered. `respond_action`, `unfriend_action`, `unblock_action`, `policy_action`, and `visibility_action` errors are never displayed anywhere: if Decline, Decline-and-block, Unfriend, Unblock, or a policy/visibility change fails server-side, the UI gives no feedback and (for the selects) keeps showing the value the user picked even though it was not saved.
- recommendation: Render `.value().get().and_then(|r| r.err())` for each action (a shared error slot is fine), and re-sync the selects from the refetched overview on failure.

### SetInvitePolicy success does not refetch the overview
- severity: minor
- category: consistency
- location: web/src/friends.rs:367
- finding: Every other mutation action has an `Effect` that bumps `set_refresh` on success (lines 373-398), including `visibility_action`, which mirrors the same select pattern. `policy_action` has no such effect, so after changing the invite policy the overview is stale, inconsistent with its sibling.
- recommendation: Add the same success `Effect` for `policy_action` as `visibility_action` has (or drop the refetch for both).

### Restart prefill failure is silently swallowed
- severity: minor
- category: quality
- location: web/src/new_game.rs:271
- finding: `let Some(Some(Ok(pf))) = prefill.get() else { return; };` discards `Err` results from `get_restart_prefill` (game not found, not finished, "You are not a player in this game"). A user following a `?restart=<id>` link they are not entitled to, or with a stale id, gets a blank default setup form titled "Restarting X" with no indication the prefill failed.
- recommendation: Match the `Err` case and surface it via `set_form_error` (or render it near the heading).

### Email slots submitted unvalidated and untrimmed
- severity: minor
- category: correctness
- location: web/src/new_game.rs:374
- finding: `OpponentSlot::Email(email) => emails.push(email)` - Player slots are validated (lines 367-372) but Email slots are pushed as-is: empty strings, whitespace, and untrimmed/uncanonicalized addresses all go to the server. This is the client end of the proposals.rs email-canonicalization finding above; the form neither trims nor rejects obviously empty email slots.
- recommendation: Trim in `on_submit`, treat empty as a form error like the unselected-Player case, and lowercase to match whatever canonicalization the server adopts.

### block_user does not check the target exists
- severity: nit
- category: quality
- location: web/src/friends.rs:230
- finding: `block_user(user_id)` inserts straight into `blocks`; an unknown UUID trips the FK and surfaces as a generic internal error, unlike `send_friend_request`, which resolves the target first and returns "User not found".
- recommendation: Look up the user first or map the FK violation to a "User not found" error.

### get_friends_overview issues six sequential queries
- severity: nit
- category: quality
- location: web/src/friends.rs:99
- finding: friends, incoming, outgoing, blocked, invite_policy, and game_visibility are awaited one after another - six round trips on every friends-page load and refetch.
- recommendation: `tokio::try_join!` the six calls, or fold invite_policy + game_visibility into one query.

### Submit is a silent no-op when no version is selected
- severity: nit
- category: quality
- location: web/src/new_game.rs:355
- finding: `let Some(version_id) = selected_version_id.get_untracked() else { return; };` - with an empty `gt.versions` (or a version select parse failure at line 429 setting it to `None`), clicking Start game does nothing with no error, unlike every other validation path which sets `form_error`.
- recommendation: Set a form error ("No version available/selected") instead of returning silently.

### Create/restart outcome with no ids navigates nowhere
- severity: nit
- category: quality
- location: web/src/new_game.rs:316
- finding: The success effects only navigate when `outcome.game_id` or `outcome.proposal_id` is `Some`; a `ProposalOutcome` with both `None` (and `RestartOutcome::AlreadyRestarted { .. }` with both `None`, line 348) leaves the user on the form with the button re-enabled and no feedback, indistinguishable from nothing having happened even though the mutation succeeded.
- recommendation: Treat the both-None case as an error message, or make the server type make it unrepresentable.

### Bespoke percent-encoder instead of the percent-encoding crate
- severity: nit
- category: dependencies
- location: web/src/players.rs:35
- finding: `encode_path_segment` hand-rolls RFC 3986 unreserved-set percent-encoding (correctly, per its tests) and is used across players.rs, friends.rs, and new_game.rs. The `percent-encoding` crate (already in the dependency tree via url/reqwest ecosystems) provides exactly this.
- recommendation: Replace with `percent_encoding::utf8_percent_encode`, or keep the helper but delegate to the crate.

### Restart prefill can select a player count not offered by the game type
- severity: nit
- category: correctness
- location: web/src/new_game.rs:274
- finding: `let count = (pf.opponents.len() + 1) as i32; ... set_player_count.set(count);` - if the game type's `player_counts` no longer includes the original game's count, no radio renders as checked (line 478 compares against the offered counts only) while the form still submits with the stale count; the server then re-validates, but the UI state is misleading.
- recommendation: Clamp the prefill count to the nearest offered count, or render the radios from the union including the prefill value.

## game_info/, models/, rules.rs, settings.rs, index.rs

### Rules-page version picked by ORDER BY name, not latest
- severity: major
- category: correctness
- location: web/src/game_info/queries.rs:18
- finding: `game_info_rules_version_id` picks the linked rules version with `ORDER BY name LIMIT 1` over public non-deprecated versions. Version names are semver-like strings ("1.0.0"), so ascending name order returns the OLDEST version, and lexicographic ordering is wrong anyway ("10.0.0" sorts before "2.0.0"). The project convention for "current version" is `ORDER BY created_at DESC` (db.rs:219-231 `find_latest_non_deprecated_game_version`). The game info page therefore links to the rules of the oldest public version once a second version exists.
- recommendation: Use `ORDER BY created_at DESC LIMIT 1` to match `find_latest_non_deprecated_game_version`, or reuse that fn and filter is_public.

### Anonymous game-info page links to auth-gated rules endpoint
- severity: minor
- category: consistency
- location: web/src/rules.rs:308-310 (and web/src/game_info/mod.rs:180-182)
- finding: `get_game_info` is intentionally anonymous and renders a "Rules & strategy" link, but `get_rendered_rules` rejects anonymous callers with "Not authenticated". A logged-out visitor browsing /game-info clicks through to a bare error page. Rules content is less sensitive than the ratings the info page already exposes, so the gate is inconsistent both ways.
- recommendation: Decide the public-content posture once: either drop the auth requirement on get_rendered_rules (it exposes nothing user-specific) or hide the link for anonymous visitors.

### get_rendered_rules ignores is_public/is_deprecated on the version
- severity: minor
- category: correctness
- location: web/src/rules.rs:312-321 (queries at db.rs:258-279)
- finding: `find_game_version_rules` and `find_game_version_render_meta` select by id with no `is_public = true` filter, so any authenticated user who obtains or guesses a version UUID can render rules and trigger live strategy fetches for non-public (unreleased) game versions. Every other listing path filters `is_public = true AND is_deprecated = false` (db.rs:300).
- recommendation: Add `AND is_public = true` to the two lookups (or check the flag in get_rendered_rules and return "not found").

### Unterminated brdgme fence silently dropped by render_doc
- severity: minor
- category: correctness
- location: web/src/rules.rs:201-204
- finding: `render_doc` line-scans for fences; if a doc ends while `in_fence` is still true (author forgot the closing fence), the accumulated `fence` buffer is discarded without error - the tail of the document silently vanishes. This contradicts the module's stated "fail loudly on authoring errors" policy (rules.rs:86, 163).
- recommendation: After the loop, if `in_fence` is true return a new `RenderError::UnterminatedFence` (or render the remainder as prose) instead of dropping it.

### Two sequential live HTTP strategy fetches per rules page view, no caching
- severity: minor
- category: quality
- location: web/src/rules.rs:261-297
- finding: `fetch_strategy` makes two round trips to the game service (BasicStrategy then AdvancedStrategy) on every /rules page load, sequentially, and any failure fails the whole page including the DB-sourced rules section (rules.rs:333-335). The strategy content is static `include_str!` data on the game side.
- recommendation: At minimum degrade gracefully (render rules, omit strategy on fetch error). Consider caching per (uri, name, interface_version) since content is immutable per version, and issuing the two requests concurrently.

### Email address not trimmed/normalized before add (settings path)
- severity: minor
- category: correctness
- location: web/src/settings.rs:341 (server side auth/server.rs:789-800)
- finding: The add-email form dispatches `el.value()` raw; `add_email_address` does no trim or lowercase before `find_email_owner`/`insert_unverified_email` (only the domain is lowercased for the blocklist check at auth/server.rs:800). "User@x.com " (case variant or stray whitespace) passes the `contains('@')` check and inserts a distinct row against the case-sensitive UNIQUE on user_emails, and later confirmation-code lookups are exact-match on the stored string. Same root cause as the proposals.rs invite-email finding; this is the settings-page instance.
- recommendation: Trim and lowercase once at the server-fn boundary before ownership check, insert, and confirmation; optionally trim client-side too.

### Fire-and-forget settings mutations swallow server errors
- severity: minor
- category: quality
- location: web/src/settings.rs:145-148 (colors), web/src/settings.rs:210-237 (email-pref toggles), web/src/settings.rs:496-498 (theme sync)
- finding: ColorsSection, EmailPreferencesSection, and ThemeSection dispatch ServerActions and never observe `action.value()`. The UI optimistically updates local signals first (e.g. `turn.set(val)` at settings.rs:212-213 before dispatch), so if the server call fails (session expired, transport error) the page shows a saved state that was never persisted, with no feedback and no revert. Same pattern as the friends-page finding above.
- recommendation: Watch each action's value and on Err revert the local signal and surface a small error message, mirroring UsernameSection's pattern (settings.rs:62-69).

### Index page issues O(friends x scan_limit) sequential queries
- severity: minor
- category: quality
- location: web/src/index.rs:51-61 (helpers db.rs:2012-2024, db.rs:2316-2342)
- finding: `get_logged_in_index` loops over `list_friends` (unbounded) and awaits `friend_recent_visible_game` per friend; that helper fetches up to 10 candidate games and calls `is_game_visible_to_user` per candidate, each its own query. The logged-in landing page can therefore run 1 + F x (1 + up to 10) sequential DB round trips and grows linearly with friend count.
- recommendation: Fold visibility into one SQL query (LATERAL join per friend with the visibility predicate inlined), or at least bound the friend list and run the per-friend lookups concurrently.

### game_info server fn runs six sequential queries
- severity: nit
- category: quality
- location: web/src/game_info/mod.rs:39-66
- finding: header, rules_version_id, total_games, active_today, distinct_players, top_ranking, and form are seven awaits in sequence; the three count queries each re-join games->game_versions independently.
- recommendation: Fine at current scale; if it shows up in latency, merge the three counts into one query with FILTER clauses or run them concurrently.

### Redundant glob re-export of ssr queries
- severity: nit
- category: simplicity
- location: web/src/game_info/mod.rs:31-32
- finding: `pub use queries::*;` re-exports the six query fns at `crate::game_info::*`, but every call site (including this module) uses the `queries::` path and nothing else imports them via the glob.
- recommendation: Drop the re-export, or drop the `queries::` prefixes; not both.

### Stale module doc: "email placeholder"
- severity: nit
- category: quality
- location: web/src/settings.rs:1-2
- finding: The module doc still says "email placeholder" but EmailSection is a full add/confirm/make-active/remove implementation.
- recommendation: Update the doc comment.

### GameBot lacks FromRow unlike sibling models
- severity: nit
- category: consistency
- location: web/src/models/game.rs:42-48
- finding: Every model struct in the file derives `FromRow` except `GameBot`, which is instead constructed field-by-field in db.rs:43-47. `GameBot` also omits created_at/updated_at while the other structs carry them.
- recommendation: Derive FromRow and align fields, or leave as-is with a one-line comment noting it is a projection.

### Rules markdown allows raw HTML pass-through into inner_html
- severity: nit
- category: correctness
- location: web/src/rules.rs:150-157 (rendered at rules.rs:52-64)
- finding: pulldown-cmark passes raw inline HTML through by default and the result is injected via `inner_html`. Rules/strategy sources are trusted authored content (DB rules column populated at deploy, include_str! on the game side), so this is not currently exploitable, but nothing documents that trust boundary, and the same render_doc would become an XSS sink if ever fed user-supplied markdown.
- recommendation: Add a comment stating the trust assumption, or pass the parser events through a filter that escapes `Event::Html`/`Event::InlineHtml`.

### Mixed resource strategies between the two content pages
- severity: nit
- category: consistency
- location: web/src/rules.rs:32-33 (vs web/src/game_info/mod.rs:100)
- finding: GameInfoPage uses `Resource::new_blocking` (SSR-rendered), RulesPage uses `LocalResource` (client-only, spinner on first paint) for similar content-page loads. The rules page is the more content-heavy of the two and gets no SSR.
- recommendation: If the auth gate on get_rendered_rules stays, LocalResource is forced; if it goes (see finding above), switch RulesPage to a blocking Resource for parity.

## Areas reviewed and found clean

### game/mod.rs, export.rs, import.rs
- No panics/unwraps in request paths; the one fallible conversion (`HeaderValue::from_str`, export.rs:210-211) has a static fallback.
- Transaction boundaries: `update_game_command_success` wraps all updates in one tx (verified by the concurrent-conflict test); `import_bundle` runs in one tx and rolls back cleanly.
- Optimistic concurrency: the `updated_at` CAS (db.rs:1715-1728) correctly closes the read-modify-write race in `execute_command`.
- Authorization: submit_command maps the authenticated user to their seat; email path derives position from token context; bot commands only from NATS; admin_export_game chains session -> token -> is_user_admin.
- NATS publish durability (JetStream persistence ack awaited); all retry budgets bounded (MAX_TURN_ATTEMPTS=3, max_deliver=3, game_client max_attempts=3).
- Export auth/headers: admin-gated, no emails in bundle, Uuid-built filename (no header injection). Bot-vs-human rating skip covered by tests.

### game/server_fns.rs
- Authentication on every server fn except the two documented/flagged anonymous ones; authorization (game membership, admin checks against the DB) on all mutating fns.
- restart_core race handling: FOR UPDATE, AlreadyRestarted for linked-game and open-proposal cases, in-tx invite-policy and email resolution, dedup - solid and well tested.
- Transaction boundaries in restart_core/create_game_from_service (implicit rollback on early return; no-orphan behavior pinned by test).
- Public index privacy: is_game_publicly_visible re-checked at render time; private log lines excluded; spectator render at position None.
- Wire types cfg-gated correctly; Leptos DI idioms consistent; test coverage thorough (restart races, prefill, force delete, public index).

### proposals.rs
- Authz: every mutating server fn requires auth and enforces owner/invitee-ship inside a FOR UPDATE transaction.
- Race handling: accept/accept, accept/start, start/cancel serialized by lock_proposal_for_update + in-tx re-reads.
- Response state machine (pending->accepted/declined, accepted->declined, declined terminal) correct and well tested; auto-decline sweep race-safe against concurrent accepts (timing basis flagged above).
- start_proposal_tx atomicity; solo-vs-bots direct-create path; duplicate-player and invite-policy checks in-tx in both create and add paths; normalize_proposal_positions SQL correct and tested.
- No panics/unwraps in request paths; broadcast payloads carry only the proposal id.

### stats/
- viz.rs numeric edge cases (empty/flat/NaN series, division guards) panic-free and unit tested; hand-rolled SVG appropriate given the lean-dependency stance.
- SQL division guards correct; no injection (all bound params); no silent error swallowing in this module; opponents_by_game batched (no N+1); tied-first-place semantics consistent across totals/game_type_stats/head_to_head; rating_series reconstruction verified against game_type_users by test.

### players.rs, friends.rs, new_game.rs
- Authz clean on all 12 friends server fns and the new-game/restart surface; respond/unfriend/unblock scoped to the caller.
- D1/D7 silent-shield semantics correct (blocked-source silent rollback, declined indistinguishable from pending, block severs friendship atomically).
- Input validation: policy/visibility whitelists, search min-length + LIKE escaping + block exclusion + LIMIT 10, self-friend/self-block rejected with DB CHECK backing.
- No panics/unwraps in request paths; pure helpers well unit-tested (ordinals, histograms, path encoding, NaN-safe sort); XSS-safe via Leptos text nodes.

### game_info/, models/, rules.rs, settings.rs, index.rs
- game_info SQL (joins, filters, DISTINCT, LIMITs) correct and tested; no N+1 in the form lookup; case-insensitive header lookup by design.
- rules.rs validate_player_indices covers every markup node variant; synthetic_players clamps prevent panics.
- settings.rs authz: every mutation scoped by the authenticated user id server-side; set_pref_colors and set_theme validate against whitelists; email actions correctly scope by owner (make-active rejects unverified/foreign, remove rejects primary/foreign, confirm requires a pending row for this user).
- index.rs: require_user gate; friend games checked via is_game_visible_to_user; caller-scoped history/stats with LIMIT 10.
- models/*.rs plain data structs; GamePlayer (with undo_game_state) not handed raw to anonymous clients.
- No panics/unwraps in any reviewed request path.

## Severity tally

| severity | count |
|---|---:|
| critical | 1 |
| major | 12 |
| minor | 35 |
| nit | 30 |
| total | 78 |

Raw findings: 80 across W1-W6; 2 merged during curation (unvalidated bot
slots: W2 minor + W3 major -> one major; page-number overflow: W4 minor +
W5 nit -> one minor). No findings rejected; the bot-slot merge effectively
upgraded W2's restart_core instance from minor to major.
