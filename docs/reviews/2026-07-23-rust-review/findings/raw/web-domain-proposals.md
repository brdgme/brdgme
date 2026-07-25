# Raw findings: web domain - proposals.rs

Scope: rust/web/src/proposals.rs (full file, 2961 LOC), with targeted cross-referencing of web/src/db.rs, web/src/game/server_fns.rs, web/src/email/inbound.rs, web/src/email/sweep.rs, web/src/websocket.rs.

### get_proposal serializes every invitee's email_token to any authenticated user
- severity: major
- category: correctness
- location: web/src/proposals.rs:78
- finding: `ProposalPlayerView` includes `pub email_token: Option<String>` (line 78), populated by `find_proposal_roster` (lines 511-523) and returned by the `get_proposal` server fn (lines 1744, 1761) to ANY authenticated user - the viewer role is computed (Owner/Invitee/Other, lines 1748-1754) but never used to gate data; a `ViewerRole::Other` caller who knows/obtains the proposal id gets the full roster including all invitees' email tokens. The email token is the credential the inbound email handler uses to accept/decline on an invitee's behalf (email/inbound.rs:594 `find_proposal_player_by_email_token`). Inbound does additionally verify the From address matches the invitee's verified email (inbound.rs:611), so exploitation requires From spoofing past the Resend webhook, but the token is a secret and there is zero reason to ship it to any browser - not even the invitee's own.
- recommendation: Drop `email_token` from `ProposalPlayerView` (and the roster SELECT). Nothing in the client UI (ProposalDetail, lines 1911-2185) uses it.

### Client-supplied BotSlot name/difficulty stored and used without validation
- severity: major
- category: correctness
- location: web/src/proposals.rs:1163
- finding: `create_proposal` (bot insert loop, lines 1163-1177) and `add_proposal_player` (lines 1469-1482) take `crate::game::server_fns::BotSlot { name, bot_name }` straight from the client and insert it as an auto-`"accepted"` slot; `start_proposal_tx` (lines 977-984) then feeds those strings into `create_game_from_service` -> `create_game_with_users_tx`, which also stores them unvalidated (db.rs:1093-1098). Neither path checks `bot_name`/`bot_difficulty` against `find_enabled_bots` (the list `get_available_bots` exposes, server_fns.rs:650). An arbitrary/bogus bot difficulty produces a game whose bot player can never take a turn, wedging the game for the humans in it (same bot-turn wedge class W1 found in game/mod.rs). This is the same unvalidated-bot_name pattern W2 flagged in `restart_core` (server_fns.rs); proposals adds two more entry points.
- recommendation: Validate `bot.bot_name` against the enabled-bots list (and non-empty `bot.name`) in `create_proposal` and `add_proposal_player` before insert; reject with a user-facing error.

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
- finding: `respond_proposal` looks the caller up in the roster (lines 1229-1232) with no owner exclusion. The owner's row is `"accepted"`, so `accepted -> declined` is allowed (line 1237). Once declined: declined is terminal (cannot re-accept), `remove_proposal_slot` refuses to remove the owner (lines 1619-1623), and `start_proposal` rejects any declined slot (lines 1331-1336). The only exits are cancel or transfer-then-remove; the natural repair paths are all blocked.
- recommendation: Reject `respond_proposal` when `user.id == proposal.owner_user_id` ("Cancel the invite instead"), or treat owner-decline as cancellation.

### Ownership can be transferred to a declined (or pending) invitee
- severity: minor
- category: correctness
- location: web/src/proposals.rs:1696
- finding: `transfer_proposal_ownership` only checks the target is in the roster (`players.iter().any(|p| p.user_id == Some(target_user_id))`, line 1696), not their response. Transferring to a declined player creates a proposal whose owner has a terminal `declined` response: it can never start (declined guard, lines 1331-1336), the owner slot cannot be removed, and the response cannot change. Transfer to a pending player is odd but recoverable (they can accept).
- recommendation: Require the target's response to be `"accepted"` (or at least not `"declined"`).

### cancel_proposal notifies from a roster snapshot taken before the lock
- severity: minor
- category: correctness
- location: web/src/proposals.rs:1532
- finding: `players` is fetched from the pool (line 1532) BEFORE the transaction begins and the proposal is locked (lines 1536-1544); the accepted-invitee list for `notify_cancelled` (lines 1563-1569) is derived from that stale snapshot after commit. An invitee whose accept commits between the fetch and the lock gets no cancellation email. Same read-then-lock TOCTOU shape W2 flagged in concede (server_fns.rs). Every other mutating fn here (`start_proposal`, `remove_proposal_slot`) reads players via `find_proposal_players_tx` inside the lock; cancel is the odd one out.
- recommendation: Move the `find_proposal_players` call inside the transaction (use `find_proposal_players_tx` after `lock_proposal_for_update`).

### notify_owner_decline bypasses the invite-email gates every other mailer applies
- severity: minor
- category: consistency
- location: web/src/proposals.rs:286
- finding: `notify_owner_decline` (lines 286-328) checks only that the owner has an email (line 297). Every other recipient-facing mailer method (`send_invite`, `notify_changed_reinvite`, `notify_cancelled`, `notify_started`, `notify_owner_ready`) applies `invite_recipient_should_send` (verified primary + `invite_emails_enabled` + web-presence suppression). An owner who disabled invite emails, or who is actively on the site watching the proposal page, still gets the decline email.
- recommendation: Apply the same `suppress_for_web_presence` + `invite_recipient_should_send` gate in `notify_owner_decline`.

### Notification emails carry a dead Reply-To and a footer inviting replies
- severity: minor
- category: correctness
- location: web/src/proposals.rs:324
- finding: `notify_owner_decline`, `notify_cancelled`, `notify_started`, and `notify_owner_ready` set the reply address to `i-{proposal_id}@brdg.me` (lines 324, 365, 410, 460). `proposal_id` renders as a hyphenated UUID, which is not a player `email_token` (tokens are `Uuid::new_v4().simple()`), so `handle_invite_reply`'s token lookup (inbound.rs:594) always misses and the reply is silently dropped ("unknown invite token"). Meanwhile every one of these emails ends with the footer "Reply to this email to respond, or unsubscribe anytime." (lines 315, 356, 401, 451) - a reply the system cannot process.
- recommendation: Use a no-reply address (or the recipient's own token where one exists) and drop the "Reply to this email to respond" footer from pure notification emails.

### Mailer tasks swallow DB errors silently; empty names produce broken subjects
- severity: minor
- category: quality
- location: web/src/proposals.rs:170
- finding: All six `RealInviteMailer` methods use `let Ok(Some(..)) = ... else { return }` on `find_proposal` / `fetch_invite_recipient` inside spawned tasks, so a DB error at send time is indistinguishable from "recipient opted out" and leaves no trace in logs. `proposal_game_type_name` (lines 170-178) likewise collapses errors into `String::new()`, and owner/invitee name lookups fall back to `unwrap_or_default()` (lines 201-206, 300-305), yielding subjects/headers like " invite from " when a lookup fails.
- recommendation: Log (`tracing::warn!`) on the error arms before returning; consider skipping the send rather than sending an email with blank substitutions.

### Pre-transaction authz block duplicated verbatim in four server fns
- severity: minor
- category: simplicity
- location: web/src/proposals.rs:1396
- finding: `add_proposal_player` (1396-1421), `cancel_proposal` (1519-1552), `remove_proposal_slot` (1585-1610), and `transfer_proposal_ownership` (1666-1691) each run the identical find -> owner-check -> open-check sequence twice: once against the pool before `begin()`, then again after `lock_proposal_for_update`. The in-lock check is the authoritative one; the pre-check only changes which error a racing caller sees, and `respond_proposal`/`start_proposal` get by fine with the in-lock check alone. Roughly 60 lines of copy-paste that must be kept in sync.
- recommendation: Drop the pre-transaction checks (or extract a `lock_owned_open_proposal(tx, id, user_id)` helper) so each fn checks once, inside the lock, matching respond/start.

### RespondOutcome.started/game_id are always false/None; client nav path is dead
- severity: minor
- category: simplicity
- location: web/src/proposals.rs:62
- finding: `respond_proposal` always returns `RespondOutcome { accepted, started: false, game_id: None }` (lines 1277-1281) - a leftover from an auto-start design (`respond_accept_does_not_auto_start` test, line 2660, confirms auto-start was removed). The client effect (lines 1836-1845) still branches on `outcome.game_id` and navigates to `/games/{gid}`, code that can never execute.
- recommendation: Shrink `RespondOutcome` to `accepted` (or unit) and delete the dead navigation branch.

### Invite emails are never trimmed or case-normalized
- severity: minor
- category: correctness
- location: web/src/proposals.rs:891
- finding: `find_or_create_user_by_email_tx` looks up `user_emails` by exact string (line 891) and inserts the raw client string (lines 912-919); `check_invite_policy_tx` (db.rs:2383) is also exact-match, and the UI submits the field untrimmed (only `!e.is_empty()`, line 2100). `user_emails.email` has a case-sensitive UNIQUE constraint (migrations/001_initial_schema.sql:275), so inviting `Foo@x.com` when `foo@x.com` is registered silently creates a second account for the same mailbox, and the invite-policy check (blocks, friends-only) is bypassed for the real user. A trailing space does the same.
- recommendation: Trim and lowercase invite emails at the server-fn boundary before lookup/insert (and ideally enforce lower-cased storage globally).

### Nudge sweep re-sends invites without re-checking proposal/player state
- severity: minor
- category: correctness
- location: web/src/proposals.rs:719
- finding: `fetch_nudge_candidates` snapshots (proposal, user, token) rows; the sweep (email/sweep.rs:288-305) then fire-and-forgets `send_invite` and marks the proposal nudged. `send_invite` (lines 182-232) re-fetches the proposal but never re-checks `status == 'open'` nor that the player is still `pending` with that token. Between snapshot and send the invitee may have responded, the roster may have rotated the token, or the proposal may have been cancelled/started - the stale "reply accept to join" email still goes out, and its reply-to token may no longer match.
- recommendation: In `send_invite`, verify the proposal is still open and the token still matches a pending row before sending (one extra SELECT in the spawned task).

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
- recommendation: `make_interval(secs => $1)` with an `f64`/`i32` bind, or `NOW() - $1 * interval '1 second'`.

### reset_accepted_humans_for_roster_change issues one UPDATE per player
- severity: nit
- category: simplicity
- location: web/src/proposals.rs:672
- finding: SELECT then a per-row UPDATE loop (lines 672-684) to assign fresh tokens. Rosters are small so this is harmless, but a single `UPDATE ... SET email_token = replace(gen_random_uuid()::text, '-', '') ... RETURNING user_id, email_token` would do it in one round trip.
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
- finding: `find_or_create_user_by_email_tx` hardcodes `internal("create_proposal: resolve email")` (lines 896, 911, 919) but is also called from `add_proposal_player` (line 1430), so failures there log under the wrong fn. Also `cancel_proposal`, `remove_proposal_slot`, and `get_pending_invites` lack the `tracing::instrument` attribute the other server fns carry (e.g. lines 1035, 1199, 1287, 1376, 1654, 1716).
- recommendation: Neutral context strings ("resolve invite email"); add the instrument attribute to the three uninstrumented server fns.

## Checked and found CLEAN

- Authz: every mutating server fn (`create_proposal`, `respond_proposal`, `start_proposal`, `add_proposal_player`, `cancel_proposal`, `remove_proposal_slot`, `transfer_proposal_ownership`) calls `require_user` and enforces owner/invitee-ship inside a `SELECT ... FOR UPDATE` transaction; `get_proposal`/`get_pending_invites` require auth (data-shape issue reported separately as the email_token finding).
- Race handling: accept/accept, accept/start, start/cancel, and edit races are correctly serialized by `lock_proposal_for_update` + in-tx re-reads; early `return Err` with an open transaction correctly rolls back on drop (sqlx semantics).
- Response state machine: pending->accepted, pending->declined, accepted->declined only; declined terminal - matches the matches! guard and is well tested (lines 2859-2960).
- `proposal_ready_to_start` logic and its unit tests; bot slots correctly never block readiness.
- `start_proposal_tx`: game creation, restarted_game_id linkage, and status flip are atomic in the caller's tx; broadcast/notify only after commit. Owner-transfer interactions with start (creator_id = current owner, prior owner becomes opponent) are correct.
- Solo-vs-bots direct-create path: `all_accepted: false` is correct because db.rs:1164 forces `accepted` for the creator and bots have no acceptance row.
- Duplicate-player checks in both create (sort/dedup, lines 1089-1098) and add (in-tx roster scan, line 1445); invite-policy (blocks/friends/none) checked in-tx in both create and add paths.
- `normalize_proposal_positions` window-function SQL is correct and tested (lines 2521-2603).
- `transfer_proposal_ownership` cannot target a bot (user_id None never equals Some).
- `auto_decline_proposal_player` guard `AND response = 'pending'` makes the sweep race-safe against concurrent accepts (the timing basis is the separate finding above).
- `track_proposal_seq` memo logic and websocket-driven refetch; `broadcast_proposal_update` payload carries only the proposal id (no data leak).
- No unwrap/expect/panic in request paths (`expect_context` for DI and `unwrap_or_default` fallbacks only); test-only unwraps confined to `#[cfg(test)]`.
- Bot column mapping (bot_name = display name, bot_difficulty = bot type) is consistently applied per the header comment, including in `start_proposal_tx`'s BotSlot construction.

Severity tally: 0 critical, 3 major, 10 minor, 5 nit.
