# Findings: web-frontend-email (web crate frontend + email)

Scope: `app.rs`, `lib.rs`, `theme.rs`, `components/`, and the whole `email/`
subtree (`inbound.rs`, `commands.rs`, `sweep.rs`, `notify.rs`, `render.rs`,
`outbound.rs`, `mod.rs`) - ~9,740 LOC, 17 files. Snapshot
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. Raw worker dumps and the review
log are in `findings/raw/web-frontend-email-*.md`; every finding below was
verified or spot-checked by the Lead against the snapshot. Domain context read
before review: `docs/email.md` (mrml/MJML rendering, the Gmail
foster-parenting hazard) and `docs/hydration.md` (SSR hydration rules,
mounted-gate idiom). The deliberate `<mj-raw>`/`<tr><td>`/font-size structure,
the mounted-gate, and the nested `<Suspense fallback=|| ()>` remnants were
confirmed present and NOT flagged.

One cross-worker duplicate was merged during curation (the RFC 8058 /
unsubscribe header defect, seen in both inbound.rs and render.rs). Several
findings cross-reference the web-domain unit (undo/concede db.rs races,
proposal `email_token` leak, unvalidated bot slots, `before=None` diffing);
those are noted inline rather than re-scoped.

## email/inbound.rs (Resend inbound webhook -> reply-to-play)

### Settings route authenticated solely by a spoofable From header
- severity: critical
- category: correctness
- location: web/src/email/inbound.rs:484
- finding: `resend_webhook` routes both `Some(InboundRoute::Settings(_))` and `None` to `handle_settings_reply_route`, discarding the settings token entirely; `handle_settings_reply` (inbound.rs:1111) then resolves the acting user purely from the inbound `From` address via `resolve_user_by_verified_from`. No SPF/DKIM/authentication-results verdict is consulted, and From is trivially forgeable in SMTP. Anyone who knows a user's verified email address can forge a From and send to `s-anything@brdg.me` - or to ANY unprefixed address on the domain, since `None` also lands here - and execute standalone commands as that user. The svix signature only authenticates that Resend delivered the webhook, not the email's sender. Severity assumes Resend does not itself reject unauthenticated inbound mail before the webhook fires; if it enforces SPF/DMARC upstream the practical exposure narrows, but the code takes no defense of its own.
- recommendation: Make the `s-` token a real per-user secret (like game `email_token`s) and require it, additionally requiring the From match as `handle_game_reply` does. Drop the `None` fallthrough (unrouted addresses should be ignored/bounced, not treated as settings). Consult Resend's SPF/DKIM verdict if the payload exposes it.

### Idempotency marker inserted before processing: failures are permanently dropped
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:456
- finding: `mark_event_processed` inserts the dedupe row before any processing, and the handler returns 200 OK regardless of downstream outcome. If anything after the marker fails - JSON parse (inbound.rs:465), raw-email fetch from Resend (inbound.rs:529), invite-transaction DB errors, `start_proposal_tx` failure, outbound reply send - the event is already marked processed, svix sees 2xx and never retries, and each handler early-return just logs and sends the player nothing. A player's move or invite response silently vanishes with no retry and no error email.
- recommendation: Insert the marker only after successful processing (keep a pre-check read to short-circuit true duplicates) and return 5xx on transient failure so svix retries; or keep the early marker but delete/mark-failed on error paths and return 5xx. At minimum, reply "internal error, please retry" on post-auth failures instead of silence.

### Advertised mailto unsubscribe can never be honored; RFC 8058 one-click header is invalid
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:1070 (also web/src/email/render.rs:235)
- finding: Outbound emails advertise `List-Unsubscribe: <mailto:unsubscribe@brdg.me?subject=unsubscribe>` plus `List-Unsubscribe-Post: List-Unsubscribe=One-Click` (built at inbound.rs:1064-1075 and render.rs:235). The inbound path cannot honor it: `unsubscribe@brdg.me` has no g-/i-/s- prefix so it falls into the settings route, which reads only the body and never the subject - a client's unsubscribe action sends an empty-body, subject-"unsubscribe" email, so the user gets "I could not find a command" and stays subscribed. Separately, `List-Unsubscribe-Post: One-Click` alongside a mailto-only URI violates RFC 8058 (which requires an HTTPS one-click target and which Gmail/Yahoo bulk-sender rules reference), and may count against deliverability.
- recommendation: In the settings fallback, detect delivery to `unsubscribe@` (or an `unsubscribe` subject) and run the unsubscribe toggle for the resolved user. Either add an HTTPS one-click endpoint and URI, or drop the `List-Unsubscribe-Post` header. Fix both header sites together.

### From/recipient matching likely breaks on "Display Name <addr>" header forms
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:37
- finding: UNCERTAIN (depends on the exact shape Resend puts in `data.from`/`data.to`). `parse_reply_address` splits on the first `@`, so "Name <g-tok@brdg.me>" yields local part "Name <g-tok" and no route; `from_matches_verified_email`/`resolve_user_by_verified_from` compare the raw From string against stored bare addresses with `LOWER(email) = LOWER($2)`, so `"Alice <alice@x.com>"` never matches. Nearly all mail clients set a display name, so if Resend passes the raw header form the whole reply-to-play flow silently rejects mail ("no response" info logs), and the same raw `from` is passed straight to `send_rendered_email` as the recipient.
- recommendation: Parse addresses properly before matching - `mail_parser` is already a dependency - extracting the bare addr-spec from the From value and each recipient. Add tests with display-name forms.

### From verification is forgeable; email tokens are the only real auth (proposal token leak makes invite replies forgeable)
- severity: major
- category: correctness
- location: web/src/email/inbound.rs:378
- finding: `from_matches_verified_email` compares the webhook-supplied From against verified addresses; since From is spoofable and no DKIM/SPF result is consulted, the effective secret for game/invite routes is the `g-`/`i-` token in the recipient address alone. The web-domain unit already flagged that proposal views leak every invitee's `email_token` to any authenticated viewer (not re-flagged here): combined with this forgeable From check, any authenticated user who can view a proposal can forge accept/decline emails for other invitees, and `handle_invite_reply` will accept them (potentially starting the game). The From check adds no defense once a token is known.
- recommendation: Treat email tokens as bearer secrets (fix the web-domain view leak, then rotate tokens); validate Resend's authentication results for defense in depth; consider rotating a game/invite token after use.

### Quote-stripping heuristics misparse common client reply formats
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:8
- finding: `parse_reply_commands` only stops at a single-line `On ...wrote:` attribution or a `--` signature marker. Gmail wraps long attributions so `wrote:` lands on the next line (the attribution line is then parsed as a command); Outlook top-posts an unquoted `-----Original Message-----`/`From:`/`Sent:` block; localized clients use non-English attributions. Stray lines become commands, and since the loop stops at the first failure the player gets a confusing "command failed" report for text they never typed.
- recommendation: Harden the heuristics (multi-line attribution, `-----Original Message-----`, `From:`/`Sent:` blocks) or adopt a dedicated reply-parser; at minimum treat a trailing block of unmatched noise after valid commands forgivingly.

### Row lock held across outbound email send in invite early-exit paths
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:684
- finding: In `handle_invite_reply`, the "invite no longer open" (inbound.rs:684) and "already responded" (inbound.rs:710) paths call `send_invite_reply_response` - several DB reads plus an HTTP send to Resend - while `tx` (holding the `lock_proposal_for_update` FOR UPDATE row lock) is still alive; the lock releases only when the function returns and `tx` drops. A slow Resend call blocks all concurrent responders to that proposal.
- recommendation: Commit/rollback the transaction explicitly before sending the response email in the early-exit paths.

### Dead production code: run_commands_in_order, CommandLoopOutcome, error_reply_text
- severity: minor
- category: simplicity
- location: web/src/email/inbound.rs:184
- finding: `run_commands_in_order`/`CommandLoopOutcome` (inbound.rs:184-204) and `error_reply_text` (inbound.rs:227) have no callers in web/src outside this file's own tests - the real loop is `run_game_reply_commands`. They look like superseded scaffolding kept alive only by their tests.
- recommendation: Delete them and their tests, or wire `error_reply_text` into the actual error path if it was meant to be used.

### RESEND_API_KEY fetch + ResendInbound construction duplicated three times
- severity: minor
- category: simplicity
- location: web/src/email/inbound.rs:518
- finding: The identical env-read + empty-check + error-log + `ResendInbound` construction + `fetch_raw_email` block appears in `handle_game_reply` (518-535), `handle_invite_reply` (625-642), and `handle_settings_reply_route` (1088-1107). It also diverges from the rest of the app: main.rs reads `RESEND_API_KEY` once at startup into `AppState.resend`, while this file re-reads the env var per inbound email.
- recommendation: Store the inbound source (or the API key) on `AppState` and extract a `fetch_inbound_text(state, email_id) -> Option<String>` helper.

### All processing happens inline before the webhook responds
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:477
- finding: The handler fetches the raw email from Resend, runs game commands (each an HTTP round-trip to the game service), renders MJML, and sends the reply - all before returning 200. Svix has a ~15s delivery timeout; a slow game service or Resend hiccup makes svix record a failure and retry, and the early dedupe marker then absorbs the retry (returns OK doing nothing), so a timed-out-but-still-running first attempt is indistinguishable from success (compounds the marker finding above).
- recommendation: Verify + dedupe + enqueue (`tokio::spawn` or a job row) and return 200 immediately; do the fetch/dispatch/reply work in the background.

### No pruning of processed_webhook_events
- severity: minor
- category: quality
- location: web/src/email/inbound.rs:408
- finding: `mark_event_processed` inserts one row per webhook delivery forever. Migration 014 creates an index on `processed_at` (clearly intended for pruning) but no code in web/src (including sweep.rs) deletes old rows, so the table grows unbounded.
- recommendation: Add a periodic delete of rows older than the svix retry window (e.g. in the existing sweep job).

### Silent return when player row missing from roster
- severity: nit
- category: quality
- location: web/src/email/inbound.rs:704
- finding: In `handle_invite_reply`, if the token's player is not in `find_proposal_players_tx` results the function returns with no log line, unlike every sibling early-return which logs at info/warn/error.
- recommendation: Add a `tracing::warn!` before returning.

### Invite response subject degrades to " invite" on lookup failure
- severity: nit
- category: quality
- location: web/src/email/inbound.rs:841
- finding: `send_invite_reply_response` folds proposal/game-version/game-type lookup failures into an empty `game_type_name` via `.unwrap_or(None).unwrap_or_default()` and `_ => (String::new(), None)`, producing the subject " invite" (leading space) with errors swallowed unlogged.
- recommendation: Log the failure and use a neutral fallback subject like "Your brdg.me invite".

### Reply-address formats hardcoded and duplicated instead of shared helpers
- severity: nit
- category: consistency
- location: web/src/email/inbound.rs:882
- finding: The invite reply address is built inline as `format!("i-{}@brdg.me", ...)` (inbound.rs:882) and the settings one as `format!("s-{user_id}@brdg.me")` (inbound.rs:1191), while the game route uses `crate::email::notify::reply_address`. The domain is hardcoded in all three (and in render.rs) with no single source of truth; `parse_reply_address` meanwhile accepts any domain.
- recommendation: Add `invite_reply_address`/`settings_reply_address` helpers next to `reply_address`, with the domain from one constant/config.

### "accept" wins over "decline" regardless of order in the body
- severity: nit
- category: correctness
- location: web/src/email/inbound.rs:646
- finding: `handle_invite_reply` scans all command lines for both words; if a reply contains both, `accept` unconditionally wins (inbound.rs:722) rather than honoring the first/last stated intent.
- recommendation: Use the first line that matches either verb.

### verify_webhook can panic on non-header-safe input
- severity: nit
- category: quality
- location: web/src/email/inbound.rs:144
- finding: `HeaderValue::from_str(...).unwrap()` on three caller-supplied strings. Safe on the current call path (values came from a HeaderMap via `to_str()`), but `verify_webhook` is `pub`; called with a string containing control characters it panics instead of returning an error.
- recommendation: Map `from_str` errors to a `VerifyError` variant instead of unwrapping.

## email/commands.rs (email command dispatch)

### Email-address management commands reachable via the spoofable From path enable account takeover
- severity: critical
- category: correctness
- location: web/src/email/commands.rs:546
- finding: `run_settings_emails` (emails add/confirm/active/remove) is reachable through `dispatch_standalone_server_command`, where identity comes only from the From-header lookup (see the inbound critical). This escalates spoofing to full account takeover: an attacker spoofing the victim's From sends `emails add attacker@evil.com` (the code is emailed to the attacker's own address), then `emails confirm <code>`, then `emails active attacker@evil.com`. The victim's primary is now the attacker's; all turn emails (including per-game-player reply tokens) then flow to the attacker, giving game-scoped command auth too. `emails remove` similarly strips the victim's secondaries. No re-authentication to the existing primary gates any of these mutations.
- recommendation: Exclude account-security subcommands (emails add/confirm/active/remove, arguably `name`) from the From-authenticated standalone path, or require a confirmation round-trip to the current primary before switching/adding. At minimum require the game-token-authenticated path.

### `bot:<name>` opponents are not validated against enabled bots
- severity: major
- category: correctness
- location: web/src/email/commands.rs:59
- finding: `classify_opponent` returns `OpponentToken::Bot(inner)` for any `bot:`-prefixed token without checking `bot_names` (the check at commands.rs:62 only applies to bare tokens). `run_new_command` passes the arbitrary string into `BotSlot { bot_name }` and `create_game_from_service`, which inserts it into `game_bots` with no validation. `new chess bot:garbage` creates a real game with a bot no runner will serve, wedging it on that bot's turn forever. Same class as the web-domain "unvalidated client-supplied bot slots" finding, here reachable by any inbound email (including the spoofable standalone path).
- recommendation: After `classify_opponent`, reject `Bot(d)` where `d` is not in `bot_names`, with a user error listing valid bots.

### Concede TOCTOU can overwrite a finished game's results (email path of the web-domain db.rs race)
- severity: major
- category: correctness
- location: web/src/email/commands.rs:891
- finding: `run_concede` checks `ge.game.is_finished` on a snapshot read, then calls `crate::db::concede_game` (db.rs:1284), whose transaction has no `WHERE is_finished = false` guard and no `updated_at` optimistic check (unlike `update_game_command_success`, db.rs:1716). If the opponent's finishing move commits in the email-processing window, concede rewrites `place` to 1/2 for both players, clobbering the real result; ratings are protected only by the `apply_rating_changes` idempotency guard (db.rs:1554), so stored placings and ratings can describe different outcomes. The web server fn shares the race; the root cause is the unguarded `db::concede_game` (web-domain unit flagged the server-fn side).
- recommendation: In `db::concede_game`, make the games UPDATE `... WHERE id = $1 AND is_finished = false` and map 0 rows to a "game already finished" conflict surfaced to the user.

### Undo TOCTOU: undo_game applies a snapshot with no concurrency guard (email path of the web-domain critical)
- severity: major
- category: correctness
- location: web/src/email/commands.rs:966
- finding: `run_undo` reads `undo_game_state` from a snapshot, round-trips to the game service, then calls `db::undo_game` (db.rs:1407), which overwrites `games.game_state` unconditionally - no `updated_at = expected` check like the move path (db.rs:1716-1727). A concurrent move between snapshot and commit is silently reverted. `run_undo` also does not check `is_finished`: undoing a finishing move is permitted, `finished_at` is NULLed, but `rating_change` values written at finish are never rewound and the `apply_rating_changes` idempotency guard then suppresses re-rating - permanently wrong ratings. The web-domain unit flagged the server-fn `undo_game` as critical for this same db.rs root cause; this is the email entry point.
- recommendation: Add an `expected_updated_at` guard to `db::undo_game` mirroring the move path (map 0 rows to a Conflict user error); reject undo on finished games or clear `rating_change` inside the undo transaction.

### run_restart leaks internal errors to email senders as user errors
- severity: major
- category: quality
- location: web/src/email/commands.rs:1044
- finding: `restart_core`'s error is mapped with `.map_err(|e| CommandError::User(e.to_string()))`. `restart_core` also returns internal failures (DB/game-service errors), so raw internal error strings are emailed back verbatim, and because they are classified `User` they bypass the `tracing::error!` logging that inbound.rs applies only to `CommandError::Internal` - internal failures in restart are neither logged nor redacted. Every other command maps internal failures to `Internal`.
- recommendation: Distinguish user-facing restart refusals (roster errors, already-restarted) from internal failures; map only the former to `User`, the rest to `Internal`.

### run_concede/run_undo duplicate the web server fns near-verbatim
- severity: major
- category: simplicity
- location: web/src/email/commands.rs:886
- finding: `run_concede` (886-922) and `run_undo` (924-981) are near line-for-line copies of `concede_game`/`undo_game` in game/server_fns.rs (same snapshot read, checks, service round-trip, notify/broadcast tail), differing only in player resolution (token vs `user.id`). Both copies carry the same latent races above; a fix to one path will drift from the other. Contrast `restart`, which was correctly factored into shared `restart_core`.
- recommendation: Extract `concede_core`/`undo_core` (pool + resolved game_player) shared by the server fns and email commands, as done for restart.

### `emails confirm` can only confirm the most recently added address
- severity: minor
- category: correctness
- location: web/src/email/commands.rs:731
- finding: `run_emails_confirm` selects the single newest unverified address (`ORDER BY created_at DESC LIMIT 1`) and validates the code against it. With two pending addresses, the older one's code always fails with "Invalid or expired confirmation code" and cannot be confirmed by email short of removing the newer address.
- recommendation: Match the code across the user's unverified addresses (join login_confirmations) rather than assuming the newest.

### Internal DB errors from validate_confirmation_code masked as "invalid code"
- severity: minor
- category: quality
- location: web/src/email/commands.rs:744
- finding: `.map_err(|_| CommandError::User("Invalid or expired confirmation code."))` discards the error, so a DB outage or query bug during confirmation is reported to the user as a wrong code and never logged (inbound.rs logs only `Internal`).
- recommendation: Map only the genuine validation-failure variant to a user error; propagate others as `Internal`.

### Standalone path rejects subscribe/unsubscribe that help_text advertises
- severity: minor
- category: consistency
- location: web/src/email/commands.rs:305
- finding: `help_text` (served on the standalone path) advertises `subscribe`/`unsubscribe` and `bump`, but `dispatch_standalone_server_command` only special-cases `new` and `bump`; `subscribe`/`unsubscribe` fall through to the rejection at commands.rs:291, whose "Available commands" list also omits `bump`. A user without a game replying "unsubscribe" - the most likely standalone reply - gets an error. (Note this is the same broken unsubscribe UX as the inbound header finding, from the command-dispatch side.)
- recommendation: Handle `subscribe_toggle` in `dispatch_standalone_server_command` (needs only pool + user_id) and make the rejection message match the supported set.

### Inline SQL in commands.rs instead of db helpers
- severity: minor
- category: consistency
- location: web/src/email/commands.rs:731
- finding: The file mostly delegates to `crate::db`, but four spots run raw SQL inline (unverified-email lookup 731, login_confirmations cleanup 750, notification-flag fetch 826-833, games version fetch 1079-1080) plus three `set_*_emails_enabled` UPDATE helpers (847-884) that duplicate a single parameterised db helper. Splits the data-access convention and risks drift with the web settings server-fn path that toggles the same columns.
- recommendation: Move these into db.rs alongside the existing user-email helpers, shared with the web settings server fns.

### Self-mention in `new` opponents is silently dropped
- severity: nit
- category: quality
- location: web/src/email/commands.rs:381
- finding: `if id == ctx.user_id { continue; }` silently ignores the sender naming themself as an opponent, so `new chess me myuser` quietly builds a different roster than requested, and the resulting count error can confuse.
- recommendation: Return a user error ("you are included automatically") instead of silently skipping.

### `bump` reply does not mention the digest cap
- severity: nit
- category: quality
- location: web/src/email/commands.rs:454
- finding: `bump` caps at `SWITCH_DIGEST_CAP` games via `cap_digest`, but the reply says "Re-sent {n} games" with no hint that more were waiting when the cap hit.
- recommendation: When capped, append "(capped at N; reply bump again for the rest)".

### Game-scoped dispatch reserves verbs that could collide with game moves
- severity: nit
- category: consistency
- location: web/src/email/commands.rs:1146
- finding: UNCERTAIN. `concede`, `undo`, `restart`, `rules`, `new`, `bump`, `list`, `help`, and settings verbs are matched before falling through to `execute_command`, so a game whose move grammar includes any of these words is unplayable by email for that move. No current game is known to collide, but nothing documents the reservation.
- recommendation: Document the reserved-verb set where game grammars are defined, or support an escape prefix (leading `/` or `play `) to force game-move interpretation.

## email/sweep.rs (periodic sweeps)

### Suppressed turn reminder is permanently marked as sent
- severity: major
- category: correctness
- location: web/src/email/sweep.rs:132
- finding: `send_reminder` returns `true` (sweep.rs:132-137) when `should_email_recipient` is false or `suppress_for_web_presence` is true, and `sweep_once` (sweep.rs:207-209) treats `true` as "sent" and calls `mark_reminder_sent`, setting `turn_reminder_sent_at`. A player who has a page open (10-minute presence window) when the sweep fires is marked reminded WITHOUT any email and will never be reminded for this turn (the candidate query excludes `turn_reminder_sent_at IS NOT NULL`). The test at sweep.rs:1002-1044 documents intended "skip while active, send once idle" behavior, but it only exercises `send_reminder` directly; via `sweep_once` the second send can never happen.
- recommendation: Distinguish "skip, retry later" from "handled" in the return (e.g. an enum Sent/Skip/Retry); only `mark_reminder_sent` on Sent (and permanent skips like no address). Add a `sweep_once`-level suppress-then-idle test.

### FOR UPDATE SKIP LOCKED in the candidate query is a no-op (no transaction)
- severity: major
- category: correctness
- location: web/src/email/sweep.rs:68
- finding: `fetch_candidates` appends `FOR UPDATE SKIP LOCKED` but runs via `fetch_all(pool)` in autocommit, so the row locks release the instant the SELECT completes, long before `send_reminder`/`mark_reminder_sent`. The clause provides zero protection: concurrent sweeps (two web replicas each `spawn_periodic_sweeps` in main.rs) can both fetch the same candidates and double-send. The other sweeps (nudge, expiry, auto-decline) are plain SELECTs with no claim mechanism at all. UNCERTAIN whether prod runs >1 replica, but the code reads as protected when it is not.
- recommendation: Claim atomically (`UPDATE ... SET turn_reminder_sent_at = NOW() ... RETURNING` then send), or hold a transaction across fetch+send+mark, or serialize sweeps with `pg_try_advisory_lock`. At minimum delete the misleading clause.

### Reminder send gate uses turn_emails_enabled, not reminder_emails_enabled
- severity: minor
- category: correctness
- location: web/src/email/sweep.rs:132
- finding: Candidates are selected on `u.reminder_emails_enabled = true` (sweep.rs:67), but the send path gates on `should_email_recipient`, which checks `turn_emails_enabled` (outbound.rs:177-179). A user with reminders enabled but turn emails disabled is selected, silently skipped, and (per the marking bug) permanently marked reminded. The two preferences are conflated.
- recommendation: Decide which preference governs reminders and apply it consistently in the SQL filter and the send-time check.

### Invite nudge marked regardless of fire-and-forget send outcome
- severity: minor
- category: correctness
- location: web/src/email/sweep.rs:298
- finding: `sweep_invite_nudge_once` calls `mailer.send_invite(...)` - which `tokio::spawn`s the work and bails silently on missing token/recipient, presence suppression, or Resend failure - then unconditionally calls `mark_proposal_nudged`. `nudged_at` is set even if zero nudges went out, and there is no retry (candidate filter is `nudged_at IS NULL`), so a transient Resend outage loses the nudge forever.
- recommendation: Make the nudge send synchronous in the sweep (or have `send_invite` report an outcome) and only mark proposals for which a send succeeded or was permanently unsendable.

### Auto-decline does not notify the proposal owner, unlike manual decline
- severity: minor
- category: consistency
- location: web/src/email/sweep.rs:353
- finding: The user-driven decline path calls `mailer().notify_owner_decline(...)` (proposals.rs:1274). `sweep_invite_auto_decline_once` only flips the row and does a websocket `broadcast_proposal_update` - no email. Since a declined player blocks `start`, an owner who is not connected has a proposal silently become unstartable with no notification until the 14-day expiry sweep cancels it. (The web-domain unit already flagged auto-decline keying on proposal `created_at`; this is the adjacent notification gap.)
- recommendation: Fire `notify_owner_decline` (or a batched "N invites auto-declined" mail) from the auto-decline sweep.

### Expiry cancellation loses notifications if owner lookup fails after status update
- severity: minor
- category: correctness
- location: web/src/email/sweep.rs:331
- finding: `cancel_proposal_for_expiry` (proposals.rs:770-806, reached only from this sweep) first sets `status = 'cancelled'`, then fetches the owner with errors folded to `None` via `.ok().flatten()` and `owner?`. If that second query fails the function returns `None`, so `notify_cancelled` never fires - and never will, because the proposal is no longer `'open'` and drops out of future candidate sets. Accepted players are silently never told.
- recommendation: Fetch owner/accepted ids before (or atomically with, via `RETURNING`) the status update, or log-and-still-notify with recoverable ids.

### send_reminder duplicates ~90 lines of notify::send_one
- severity: minor
- category: simplicity
- location: web/src/email/sweep.rs:98
- finding: `send_reminder` re-implements the load-game / find-player / fetch-recipient / suppression / token / palette / render / send pipeline of `notify::send_one` (notify.rs:235-336) nearly line-for-line, differing only in header text, thread parameters, and returning a bool. The two copies of the recipient-gating logic have already drifted (see the turn_emails_enabled finding).
- recommendation: Add a `NotifyKind::Reminder` (and a result-returning variant) to `send_one` and delete `send_reminder`'s duplicated body.

### Rust candidate predicate is prod-dead and drifting from the SQL
- severity: minor
- category: quality
- location: web/src/email/sweep.rs:31
- finding: `is_reminder_candidate`/`should_reset_reminder` are public but referenced only by this file's tests; production filtering lives in the SQL of `fetch_candidates` and reset logic in db.rs. The Rust predicate already lacks the SQL's `game_bot_id IS NULL` and `reminder_emails_enabled` conditions, so the tests exercise a spec that differs from what runs. It also has a dead `unwrap_or(Duration::hours(24))` fallback (sweep.rs:42) for a conversion that cannot fail.
- recommendation: Either drive `fetch_candidates` filtering through the predicate, or delete it and test the SQL via the existing `#[sqlx::test]`s.

### Five copy-pasted spawn/interval loops
- severity: nit
- category: simplicity
- location: web/src/email/sweep.rs:214
- finding: `spawn_turn_reminder_sweep`, `spawn_unverified_email_sweep`, `spawn_invite_nudge_sweep`, `spawn_invite_expiry_sweep`, and `spawn_invite_auto_decline_sweep` each repeat identical tokio interval/`MissedTickBehavior::Skip`/loop boilerplate.
- recommendation: One `spawn_sweep(name, interval, closure)` helper.

### Reminder email threads into the game thread its turn email opted out of
- severity: nit
- category: consistency
- location: web/src/email/sweep.rs:183
- finding: Turn notifications deliberately de-thread via unique per-turn subjects with `thread_id = None` (notify.rs:309-313), but the reminder for the same turn uses `game_subject` plus `thread_id = Some("game-{id}")` - the thread reserved for eliminated/finished mails. The reminder will not thread with the turn email it nudges and will thread with game-over mails instead.
- recommendation: Use the same per-turn subject scheme for reminders, or document why the game thread is preferred.

### Sweep candidate queries are unbounded
- severity: nit
- category: quality
- location: web/src/email/sweep.rs:58
- finding: None of the sweep candidate queries (`fetch_candidates`, `fetch_nudge_candidates`, `fetch_expiry_candidates`, `fetch_auto_decline_candidates`) have a LIMIT. After downtime longer than the thresholds, a single sweep iteration fetches and serially emails every overdue row with no batching or pacing.
- recommendation: Add a per-sweep LIMIT (remaining rows are picked up next tick).

## email/notify.rs (game notification emails)

### game_log_count error collapses to 0, corrupting threading and first-message flag
- severity: minor
- category: correctness
- location: web/src/email/notify.rs:227
- finding: `game_log_count` returns `unwrap_or(0)` on any DB error. In `send_one` (notify.rs:307-311) that makes `is_first_message = true` (wrong greeting/threading in `render_game_email`) and gives every affected turn email the identical subject `"{type} {game_id}-0"` - the unique-subject-per-turn de-threading lever (documented at notify.rs:68-73) collapses, so clients thread unrelated turns together. The same count feeds `failure_report_content` (notify.rs:211).
- recommendation: Propagate the error and skip/degrade explicitly (e.g. a timestamp suffix), or at least log and use a sentinel that keeps subjects unique.

### notify_game_emails with before=None re-notifies every player already on turn
- severity: minor
- category: correctness
- location: web/src/email/notify.rs:480
- finding: With `before = None`, `was_turn` defaults to false for every player, so all players currently on turn are treated as newly-on-turn and emailed. `email/commands.rs:426` calls `notify_game_emails(..., None)` after inbound command handling; in simultaneous-turn games, players already on turn (and already notified) get a duplicate turn email on every such call. Same defaulting applies to `was_elim`. (The web-domain unit flagged the `was_finished`/`before=None` variant as a nit; this is the turn/elimination side, with a concrete duplicate-send path.)
- recommendation: Make `before` non-optional where call sites can capture it, or treat `None` as "unknown" and skip transition-based sends rather than defaulting to false.

### Per-recipient game reload and serial sends in notify_game_emails
- severity: minor
- category: quality
- location: web/src/email/notify.rs:445
- finding: `notify_game_emails` loads `after` via `find_game_extended`, then each `send_one` reloads the same `GameExtended` (notify.rs:244) plus a separate `game_log_count` per recipient - N+1 loads of an already-held snapshot - and sends serially, including the Resend round-trip, inside request paths. A 4-human game does 4 game loads, 4 log counts, 4 render calls, and 4 sequential mail API calls before the server fn returns.
- recommendation: Pass the loaded `GameExtended` (and log count) into `send_one`, and consider spawning the send loop (the module contract is already best-effort/log-only).

## email/render.rs, email/outbound.rs (rendering + send)

### ensure_email_token has a lost-update race that can invalidate an already-emailed reply address
- severity: minor
- category: correctness
- location: web/src/email/outbound.rs:110
- finding: `ensure_email_token` does SELECT-then-UPDATE with no atomicity. Two concurrent sends for the same `game_players` row (a turn notification and a reminder sweep firing together) can both observe `email_token IS NULL`, generate different tokens, and each send an email with its own `g-{token}@brdg.me` reply address. The second UPDATE overwrites the first token, so replies to the first email's address no longer resolve - the reply is silently dead.
- recommendation: Make it atomic: `UPDATE game_players SET email_token = COALESCE(email_token, $1), updated_at = NOW() WHERE id = $2 RETURNING email_token`, dropping the SELECT.

### ensure_email_token returns an unpersisted token for a nonexistent game_player_id
- severity: minor
- category: correctness
- location: web/src/email/outbound.rs:116
- finding: When the SELECT returns no row (slot does not exist), the function generates a token, runs an UPDATE matching 0 rows, and returns `Ok(token)` anyway. The caller then emails a reply address whose token exists nowhere in the DB, so inbound replies can never match. Existence (`fetch_optional` returning `None`) is not checked.
- recommendation: Return an error (or `Ok(None)`) when the row is `None`; the RETURNING form above also fixes this.

### game_emails_sent_total counts failed sends as sent
- severity: minor
- category: correctness
- location: web/src/email/outbound.rs:65
- finding: The `game_emails_sent_total` counter is incremented before the Resend call, so a send that fails (returns `false`, which the sweep's mark-as-sent decision keys off) still increments the "sent" metric. During a Resend outage the metric shows normal volume while nothing is delivered - masking exactly the incident the metric exists to surface.
- recommendation: Increment on the `Ok(_)` arm only, and add a `game_emails_failed_total` on the `Err` arm. If attempt-counting is intended, rename to `..._attempts_total`.

### mrml parse/render failure is silently swallowed
- severity: minor
- category: quality
- location: web/src/email/render.rs:181
- finding: `mrml::parse(&mjml).ok()` and `.render(...).ok()` discard the error before falling back to `fallback_html`. The fallback is deliberate, but there is no log/metric, so if body content ever breaks mrml parsing (all game emails degrading to the bare-`<pre>` fallback), nothing surfaces it - a real observability gap given this file's Gmail-rendering incident history.
- recommendation: Log at `tracing::warn!` (the error, not the body) via `.inspect_err(...)`/match before falling back, and/or increment a fallback counter.

### render_block silently renders malformed markup as empty
- severity: minor
- category: quality
- location: web/src/email/render.rs:72
- finding: `brdgme_markup::from_string(markup).unwrap_or_default()` maps a parse failure to zero nodes, so a malformed board/header/digest renders as an empty block in HTML and text with no diagnostic. Contrast rules.rs:139, which propagates the same error as `RenderError::Markup`.
- recommendation: Log a warning with the block kind on parse failure (keeping the empty-render fallback so the email still ships).

### URLs interpolated into href attributes without escaping
- severity: nit
- category: correctness
- location: web/src/email/render.rs:154
- finding: `browser_url`/`rules_url` are interpolated raw into `<a href="{url}">` and the MJML string. Today both are server-built from `public_base_url()` + a UUID (notify.rs:43-50), so there is no injection path, but the renderer's contract does not require that; a future caller passing a URL with `"` or `&` would break the attribute. UNCERTAIN whether any call site would ever carry user-influenced URLs.
- recommendation: Attribute-escape the URLs at interpolation, or document the trusted-URL precondition on `EmailContent`.

### parse_duration lives in outbound.rs but is sweep configuration parsing
- severity: nit
- category: consistency
- location: web/src/email/outbound.rs:13
- finding: `parse_duration` is a generic env-var duration parser used only by sweep.rs (5 sweep-config call sites). It has nothing to do with outbound send plumbing; its placement makes outbound.rs's "single send choke point" description inaccurate.
- recommendation: Move it to sweep.rs (or a config module) next to its consumers.

## theme.rs

### random_pref_colors hand-rolls Fisher-Yates with modulo over rand
- severity: nit
- category: simplicity
- location: web/src/theme.rs:72
- finding: The manual shuffle via `rand::random::<u32>() as usize % (i + 1)` re-implements `rand`'s `SliceRandom::shuffle`/`choose_multiple`. The modulo bias is negligible at n<=8, so this is a simplicity/intent issue, not correctness.
- recommendation: `use rand::seq::SliceRandom; colors.shuffle(&mut rand::rng());` then truncate (per the rand 0.9 API in use).

## components/, app.rs, lib.rs (Leptos frontend)

### GameMeta mutation actions swallow errors silently
- severity: major
- category: quality
- location: web/src/components/game.rs:56
- finding: The four ServerActions in GameMeta only handle success. The undo effect (game.rs:56-61) and concede effect (62-67) match `Some(Ok(()))` only; `bump_bot_action` has no value watcher at all, and the force-delete effect (72-77) ignores `Err`. If Concede or "Delete game (admin)" fails server-side (auth expiry, transient 500), the user gets zero feedback - the page does not change, and the failed mutation produces no WS bump so there is no refetch either. Same fire-and-forget pattern flagged in prior units, and it contrasts with `PlayerInfo add_friend` (game.rs:263-275) and `GameCommandInput error_msg` (game.rs:562-570) in the same file.
- recommendation: Render a generic per-action error line (mirroring GameCommandInput) when `action.value()` holds `Err`, at minimum for concede and force-delete which are destructive.

### Turnstile widget likely never renders after client-side navigation to /login
- severity: major
- category: correctness
- location: web/src/app.rs:595
- finding: UNCERTAIN. The `cf-turnstile` div relies on Turnstile's implicit rendering, which scans the DOM when `api.js` executes (loaded once in `shell()`, app.rs:88). On a client-side navigation to /login (the logout flow navigates to "/login", layout.rs:120-124), the div is inserted after the implicit scan ran, so no widget appears, `get_turnstile_response()` (app.rs:458-468) returns "", and `login()` rejects whenever `TURNSTILE_SECRET_KEY` is set (verified in auth/server.rs:234-237). The user is stuck until a hard refresh. The div is also added reactively only after the `site_key` resource resolves (app.rs:589-598), so even a hard load can insert it post-scan if the resource is not serialized into SSR HTML.
- recommendation: Call `turnstile.render()` explicitly from an Effect/NodeRef once the div mounts (Cloudflare's explicit-rendering mode), or force a full-page load for /login. Verify by logging out in a prod-configured build and attempting login without refresh.

### Presence-ping loop and profile-theme sync cannot restart after logout -> login in the same tab
- severity: minor
- category: correctness
- location: web/src/app.rs:179
- finding: `presence_started` is set true once and never reset; the ping loop breaks on logout (app.rs:185-187). If a user logs out (loop exits) then logs back in without a reload, the Effect at app.rs:180-193 sees `presence_started == true` and never spawns a new loop, so the re-authenticated session sends no pings until a full reload. `applied_profile_theme` (app.rs:154-173) has the same one-shot latch: logging in as a different user in the same tab never applies the second user's stored theme.
- recommendation: Reset both flags when `current_user` transitions to logged-out, or key the latches on the user id instead of a bool.

### GamePage error branch leaks raw ServerFnError text to the user
- severity: minor
- category: quality
- location: web/src/app.rs:762
- finding: `Err(e) => view! { <div class="error">"Error: " {e.to_string()}</div> }` renders the raw `ServerFnError` for any get_game_details failure. GameCommandInput in the same feature deliberately never does this ("never leak the raw ServerFnError text", game.rs:567) and shows a generic message. Raw text can include internal detail and is inconsistent UX.
- recommendation: Show a generic "Failed to load game" message (distinguishing only the invalid-ID case if useful) to match the established policy.

### GameMeta inlines confirm dialogs instead of using the shared confirm() helper
- severity: minor
- category: consistency
- location: web/src/components/game.rs:118
- finding: The concede handler (game.rs:118-120) and admin force-delete handler (game.rs:170-172) each inline `web_sys::window().and_then(|w| w.confirm_with_message(...).ok()).unwrap_or(false)`, exactly the body of `crate::components::confirm()` (confirm.rs:1-5) that proposals.rs uses in five places. Duplicated logic and an extra `web_sys` call site outside the sanctioned helper.
- recommendation: Replace both inline blocks with `crate::components::confirm(...)`.

### friend_request_count resource recreated on every route change, unlike its siblings
- severity: minor
- category: consistency
- location: web/src/components/layout.rs:135
- finding: `SidebarMenu` creates `friend_request_count` as a local LocalResource. Per the comment at layout.rs:126-129, every page wraps its own `<MainLayout>`, so the sidebar remounts on each navigation; `active_games` and `current_user` were hoisted into `App` specifically to avoid the reset-to-None flash and duplicate fetch. `friend_request_count` was not, so every navigation refires the request and the "(N new)" badge disappears until it resolves. It also never refetches on `last_update`, so an incoming friend request does not update the badge until navigation.
- recommendation: Hoist the resource into `App` and provide via context like `active_games`/`current_user` (optionally tracking the WS trigger for live updates).

### Bot difficulty select can desync from state when bot_names resolves after render
- severity: minor
- category: correctness
- location: web/src/components/opponent_slot.rs:316
- finding: UNCERTAIN. The `<select>`'s `prop:value` closure tracks only `slot()`, while the `<option>` list is a separate closure tracking `bot_names` (opponent_slot.rs:328-346). When the LocalResource resolves after the select first renders (fallback list shown), the options are re-created; replacing option nodes can reset the DOM selection to the first option without re-running `prop:value`, so the visible selection diverges from state until the user touches the control. Harmless when server list equals fallback; wrong difficulty submitted if it differs or reorders.
- recommendation: Track `bot_names` from the `prop:value` closure too (or render the select only once bot_names is available).

### Logout action failure gives no feedback
- severity: minor
- category: quality
- location: web/src/components/layout.rs:120
- finding: The logout effect only handles `is_ok()`; a failed Logout server call leaves the user apparently still logged in with no error. Same fire-and-forget pattern as GameMeta, in the layout so it affects every page.
- recommendation: Show a transient error (or retry) when `logout_action.value()` is `Err`.

### format_log_time hardcodes en-US locale despite "browser local" intent
- severity: nit
- category: quality
- location: web/src/components/game.rs:303
- finding: The comment says timestamps format "in the browser's local time zone via Date.toLocaleString", but the call is `date.to_locale_string("en-US", ...)`. Time zone is local, but date wording/order is forced to US English for all users.
- recommendation: Pass `undefined`/navigator.language if locale-following output is intended; otherwise fix the comment.

### Click-only anchors without href are keyboard-inaccessible
- severity: nit
- category: quality
- location: web/src/components/layout.rs:166
- finding: The "logout" link (layout.rs:166-171), "I already have a login code" link (app.rs:603), and "Logging in as <email>" link (app.rs:623) are `<a>` elements with `on:click` and `cursor:pointer` but no href/tabindex/role, so they cannot be focused or activated by keyboard.
- recommendation: Use `<button>` styled as a link, or add `href="#"` with prevent_default like the rest of the codebase.

### mod.rs placeholder comment is stale
- severity: nit
- category: quality
- location: web/src/components/mod.rs:1
- finding: "Components module - placeholder for UI components / This will be expanded in later milestones" - the module is fully populated; the comment is misleading.
- recommendation: Delete the two comment lines.

### sentry snippet escaping does not cover </script>
- severity: nit
- category: quality
- location: web/src/app.rs:55
- finding: `js_string_escape` escapes backslash and double-quote only. A DSN or `SENTRY_RELEASE` env value containing `</script>` (or a newline) would break out of the inline `<script inner_html=...>` block. Operator-controlled, so not a practical security issue, but the escaping stops short of the actual inline-script injection vector.
- recommendation: Also escape `<` (standard JSON-in-script hardening), or serialize via `serde_json::to_string`.

## Areas reviewed and found CLEAN

- **inbound.rs**: svix signature verification (constant-time HMAC, timestamp tolerance, empty-secret rejected, bad/missing headers -> 401); webhook dedupe `INSERT ... ON CONFLICT DO NOTHING` is race-free; all SQL parameterized, no injection from email-derived text; the invite accept flow's FOR UPDATE lock + pending-check + conditional start in one transaction is a sound double-accept guard (bot-slot NULL user_id tokens rejected); `run_game_reply_commands` outcome partitioning is correct and well tested; `extract_plain_text` delegates MIME parsing to `mail_parser`.
- **commands.rs**: settings-verb parsing (case-insensitive, alias-normalised) and `split_new_args`/`resolve_game_type` (longest-match-first) are correct on attacker text with no panics; concede/undo/restart all verify game membership before mutating; `run_restart` correctly uses race-safe `restart_core` (the pattern concede/undo should follow); the email add/remove/active state machine is correct and integration-tested; no unwrap/panic on untrusted input in non-test code.
- **sweep.rs**: `MissedTickBehavior::Skip` + awaited `sweep_once` = no self-overlap within a replica; env-threshold parsing/defaults consistent and tested; interval SQL (`($1 || ' seconds')::interval` with a bound i64) is injection-safe; transient-failure paths in `send_reminder` correctly leave `turn_reminder_sent_at` NULL for retry; `SendMode` ladder matches documented semantics.
- **notify.rs**: pure helpers (`reply_address`, `turn_header_text`, `game_subject`, `turn_subject`, URL builders) and threading semantics (Message-Id first, In-Reply-To+References after, none when de-threaded) are correct and tested; `digest_since_last_turn` filtering and render-failure degradation are sound.
- **render.rs/outbound.rs/theme.rs**: HTML escaping of all interpolated markup goes through `brdgme_markup::html` (escapes `&`/`<`/`>` on every text node), so user names cannot inject HTML or break out of `<mj-raw>`; no email-header injection (values are fixed strings or UUID-based thread ids; `reply_to` is a 32-char token; `to` is the verified address); palette resolution is test-pinned to match web themes; `suppress_for_web_presence` fail-open is documented and tested; theme.rs registry/CSS generation, contrast floor, and sample markup are exhaustively tested. The deliberate `<mj-raw>`/`<tr><td>`/font-size structure, concrete hex colours, unthemed plain part, and fallback_html path were confirmed correct and not flagged.
- **frontend (components/app.rs/lib.rs)**: hydration discipline matches docs/hydration.md - mounted-gate in GameLogs/RecentGameLogs/HomePage, the deliberate nested `<Suspense fallback=|| ()>` wrappers (game.rs:189, app.rs:791), and `try_get_untracked` raf guards; resources/boundaries created unconditionally in stable order; hidden-attribute (not structural) toggling for SSR-varying UI; GamePage's blocking resource + `track_game_seq` memo + Transition rationale are coherent; GameCommandInput's type-anywhere handler removes its window listener in `on_cleanup` (no leak); opponent_slot debounce-with-sequence prevents stale results; `shell()`/lib.rs ssr-gating and the Sentry before-send scrubber are sound.

## Severity tally

- critical: 2
- major: 12
- minor: 28
- nit: 18
- total: 60

Cross-unit notes: the concede/undo TOCTOU (commands.rs) share their db.rs root
cause with the web-domain unit's server-fn findings (undo flagged critical
there); the forgeable-From auth interacts with web-domain's proposal
`email_token` leak; unvalidated `bot:` slots are the same class as web-domain's
client-supplied bot-slot finding. Curation merged the RFC 8058 / unsubscribe
header defect (inbound.rs:1070 + render.rs:235) into one major.
