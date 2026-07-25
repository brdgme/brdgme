# Raw findings: web-frontend-email W3 (email/sweep.rs, email/notify.rs)

Scope: full read of rust/web/src/email/sweep.rs (1046 LOC) and rust/web/src/email/notify.rs (679 LOC), with supporting reads of email/outbound.rs, proposals.rs, db.rs, main.rs, game/mod.rs, email/commands.rs call sites. Snapshot line numbers.

### Suppressed turn reminder is permanently marked as sent
- severity: major
- category: correctness
- location: web/src/email/sweep.rs:132
- finding: `send_reminder` returns `true` (lines 132-137) when `should_email_recipient` is false or `suppress_for_web_presence` is true, and `sweep_once` (lines 207-209) treats `true` as "sent" and calls `mark_reminder_sent`, setting `turn_reminder_sent_at`. Result: a player who happens to have a page open (10-minute presence window) at the moment the 15-minute sweep fires gets marked reminded WITHOUT any email, and will never receive a reminder for this turn no matter how long they subsequently idle - the candidate query excludes rows with `turn_reminder_sent_at IS NOT NULL`. The test at lines 1002-1044 documents the intended behavior as "skipped while the recipient is active on the web ... and sent once they are no longer active", but it only exercises `send_reminder` directly; going through `sweep_once` the second send can never happen. The intended retry-later semantics are unreachable in production.
- recommendation: Distinguish "skipped, retry later" from "handled" in `send_reminder`'s return (e.g. an enum Sent/Skip/Retry). Only `mark_reminder_sent` on Sent (and arguably on permanent skips like no email address); on presence suppression return Retry and leave `turn_reminder_sent_at` NULL. Add a `sweep_once`-level test covering suppress-then-idle.

### FOR UPDATE SKIP LOCKED in candidate query is a no-op (no transaction)
- severity: major
- category: correctness
- location: web/src/email/sweep.rs:68
- finding: `fetch_candidates` appends `FOR UPDATE SKIP LOCKED` but runs via `fetch_all(pool)` in autocommit mode - the row locks are released the instant the SELECT statement completes, long before `send_reminder`/`mark_reminder_sent` run. The clause therefore provides zero protection: concurrent sweeps (e.g. two web replicas, which each `spawn_periodic_sweeps` unconditionally in main.rs:75) can both fetch the same candidates and double-send reminder emails, which is exactly what the clause appears intended to prevent. UNCERTAIN whether prod runs >1 web replica, but the code reads as if it is protected when it is not, and the other sweeps (nudge, expiry, auto-decline) have no locking or claim mechanism at all - `fetch_nudge_candidates` etc. in proposals.rs are plain SELECTs, so multi-replica duplicates apply to every sweep.
- recommendation: Either claim atomically (single `UPDATE ... SET turn_reminder_sent_at = NOW() WHERE ... RETURNING id, game_id` and send after claiming, accepting mark-before-send), or hold a transaction across fetch+send+mark, or serialize sweeps across replicas with `pg_try_advisory_lock`. At minimum delete the misleading `FOR UPDATE SKIP LOCKED`.

### Reminder send gate uses turn_emails_enabled, not reminder_emails_enabled
- severity: minor
- category: correctness
- location: web/src/email/sweep.rs:132
- finding: Candidates are selected on `u.reminder_emails_enabled = true` (line 67), but the send path then gates on `should_email_recipient`, which checks `turn_emails_enabled` (outbound.rs:177-179). A user with reminders enabled but turn emails disabled is selected as a candidate, silently skipped, and (per the marking bug above) permanently marked reminded. The two preferences are conflated; the reminder feature's own toggle is not the one that decides the send.
- recommendation: Decide which preference governs reminders and apply it consistently in both the SQL filter and the send-time check (likely: keep `reminder_emails_enabled` and check only bot/verified-address at send time).

### Invite nudge marked regardless of fire-and-forget send outcome
- severity: minor
- category: correctness
- location: web/src/email/sweep.rs:298
- finding: `sweep_invite_nudge_once` calls `mailer.send_invite(...)` - which `tokio::spawn`s the actual work (proposals.rs:182-186) and bails silently on missing token, missing recipient, presence suppression, or Resend failure - then unconditionally calls `mark_proposal_nudged` for every touched proposal. `nudged_at` is set the moment the sweep runs even if zero nudge emails actually went out, and there is no retry (candidate filter is `nudged_at IS NULL`). A transient Resend outage during the sweep loses the nudge for those proposals forever.
- recommendation: Make the nudge send synchronous in the sweep (or have `send_invite` report an outcome) and only mark proposals for which at least one send succeeded or was permanently unsendable.

### Auto-decline does not notify the proposal owner, unlike manual decline
- severity: minor
- category: consistency
- location: web/src/email/sweep.rs:353
- finding: The user-driven decline path calls `mailer().notify_owner_decline(...)` (proposals.rs:1274) so the owner learns their invite was declined. `sweep_invite_auto_decline_once` only flips the row and does a websocket `broadcast_proposal_update` - no email. Since a declined player blocks `start` (proposals.rs:1331-1334), an owner who is not currently connected has a proposal silently become unstartable with no notification until the 14-day expiry sweep cancels it. (The keying of auto-decline on proposal `created_at` is already flagged by a prior unit; this is the adjacent notification gap.)
- recommendation: Fire `notify_owner_decline` (or a batched "N invites auto-declined" mail) from the auto-decline sweep.

### Expiry cancellation loses notifications if owner lookup fails after status update
- severity: minor
- category: correctness
- location: web/src/email/sweep.rs:331
- finding: `cancel_proposal_for_expiry` (proposals.rs:770-806, reached only from this sweep) first sets `status = 'cancelled'`, then fetches the owner with errors folded to `None` via `.ok().flatten()` and `owner?`. If that second query fails, the function returns `None`, so the sweep's `notify_cancelled` never fires - and it never will, because the proposal is no longer `'open'` and drops out of future candidate sets. The cancellation is applied but accepted players are silently never told.
- recommendation: Fetch owner/accepted ids before (or atomically with, via `RETURNING owner_user_id`) the status update, or log-and-still-notify with whatever ids are recoverable.

### send_reminder duplicates ~90 lines of notify::send_one
- severity: minor
- category: simplicity
- location: web/src/email/sweep.rs:98
- finding: `send_reminder` re-implements the load-game / find-player / fetch-recipient / suppression / token / palette / players / render / send pipeline of `notify::send_one` (notify.rs:235-336) nearly line-for-line, differing only in header text, footer-thread parameters, and returning a bool. Two copies of the recipient-gating logic have already drifted (see the turn_emails_enabled finding above) and will continue to.
- recommendation: Add a `NotifyKind::Reminder` (and a result-returning variant or callback) to `send_one` and delete `send_reminder`'s duplicated body.

### Five copy-pasted spawn/interval loops
- severity: nit
- category: simplicity
- location: web/src/email/sweep.rs:214
- finding: `spawn_turn_reminder_sweep`, `spawn_unverified_email_sweep`, `spawn_invite_nudge_sweep`, `spawn_invite_expiry_sweep`, and `spawn_invite_auto_decline_sweep` each repeat the identical tokio interval/MissedTickBehavior::Skip/loop boilerplate (lines 214-229, 245-256, 308-319, 340-351, 374-388).
- recommendation: One `spawn_sweep(name, interval, closure)` helper.

### Rust candidate predicate is prod-dead and drifting from the SQL
- severity: minor
- category: quality
- location: web/src/email/sweep.rs:31
- finding: `is_reminder_candidate` and `should_reset_reminder` are public but referenced only by unit tests in this file (verified via grep across web/src); the production candidate logic lives in the SQL of `fetch_candidates`, and the reset logic in db.rs (lines 1314/1441/1759 set `turn_reminder_sent_at = NULL`). The Rust predicate already lacks the SQL's `game_bot_id IS NULL` and `reminder_emails_enabled` conditions, so the tests exercise a spec that differs from what runs. It also contains a dead fallback (`unwrap_or(time::Duration::hours(24))` at line 42) for a conversion that cannot fail for any parseable env value.
- recommendation: Either drive `fetch_candidates` filtering through the predicate (fetch wide, filter in Rust) or delete the predicate and test the SQL via the existing `#[sqlx::test]`s, which already cover the same cases.

### game_log_count error collapses to 0, corrupting threading and first-message flag
- severity: minor
- category: correctness
- location: web/src/email/notify.rs:227
- finding: `game_log_count` returns `unwrap_or(0)` on any DB error. In `send_one` (lines 307-311) that makes `is_first_message = true` (wrong greeting/threading behavior in `render_game_email`) and gives every affected turn email the identical subject `"{type} {game_id}-0"` - the unique-subject-per-turn de-threading lever (documented at lines 68-73) collapses, so mail clients thread unrelated turns together. Same count feeds `failure_report_content` (line 211).
- recommendation: Propagate the error and skip/degrade explicitly (e.g. fall back to a timestamp suffix), or at least `unwrap_or`-log with a sentinel that keeps subjects unique.

### notify_game_emails with before=None re-notifies every player already on turn
- severity: minor
- category: correctness
- location: web/src/email/notify.rs:480
- finding: With `before = None`, `was_turn` defaults to false for every player, so every player currently on turn is treated as newly-on-turn and emailed. `email/commands.rs:426` calls `notify_game_emails(..., None)` after inbound email command handling; in simultaneous-turn games, players who were already on turn before the command (and had already been notified) receive a duplicate turn email on every such call. Same defaulting applies to `was_elim`. (The `was_finished`-with-`before=None` variant of this was flagged as a nit by web-domain; this is the turn/elimination side, which has a concrete duplicate-send path via commands.rs.)
- recommendation: Make `before` non-optional at call sites that can capture it, or treat `before = None` as "unknown" and skip transition-based sends rather than defaulting to false.

### Per-recipient game reload and serial sends in notify_game_emails
- severity: minor
- category: quality
- location: web/src/email/notify.rs:445
- finding: `notify_game_emails` loads `after` via `find_game_extended`, then each `send_one` reloads the same `GameExtended` from the DB (line 244) plus a separate `game_log_count` query per recipient - N+1 loads of an already-held snapshot - and the sends are awaited serially, including the Resend HTTP round-trip, inside request paths (game/server_fns.rs:509/796/848/1108). A 4-human game does 4 game loads, 4 log counts, 4 render-service calls, and 4 sequential mail API calls before the server fn returns.
- recommendation: Pass the loaded `GameExtended` (and log count) into `send_one`, and consider spawning the send loop (the module contract is already best-effort/log-only).

### Reminder email threads into the game thread its turn email opted out of
- severity: nit
- category: consistency
- location: web/src/email/sweep.rs:183
- finding: Turn notifications deliberately de-thread via unique per-turn subjects with `thread_id = None` (notify.rs:309-313), but the reminder for that same turn uses `game_subject` plus `thread_id = Some("game-{id}")` - the thread otherwise reserved for eliminated/finished mails. The reminder will not thread with the turn email it is nudging about, and will thread with game-over mails instead.
- recommendation: Use the same per-turn subject scheme (`turn_subject` with the current log count) for reminders, or document why the game thread is preferred.

### Sweep candidate queries are unbounded
- severity: nit
- category: quality
- location: web/src/email/sweep.rs:58
- finding: None of the sweep candidate queries (`fetch_candidates`, `fetch_nudge_candidates`, `fetch_expiry_candidates`, `fetch_auto_decline_candidates`) have a LIMIT. After downtime longer than the thresholds, a single sweep iteration will fetch and serially email every overdue row in one loop with no batching or pacing. Low risk at current scale.
- recommendation: Add a per-sweep LIMIT (remaining rows are picked up next tick anyway, 15 minutes later).

## Areas reviewed and found clean

- Sweep scheduling: `interval` with `MissedTickBehavior::Skip` and an awaited `sweep_once` means no self-overlap within a replica; drift is bounded and acceptable for this workload. First tick fires immediately at boot, which is harmless.
- `parse_duration`-backed env thresholds and their defaults/tests (24h reminder, 15m interval, 24h nudge, 14d expiry, 48h auto-decline) are consistent between constants, parsing, and tests.
- Interval SQL construction (`($1 || ' seconds')::interval` with a bound stringified i64) is bound-parameter safe; no injection.
- `send_reminder` transient-failure paths (game load error, token error, Resend failure) correctly return false and leave `turn_reminder_sent_at` NULL for retry next sweep - the marking bug is confined to the "skip" paths.
- `sweep_unverified_emails_once` is a simple idempotent delete with correct error logging; verified against the sqlx test.
- notify.rs pure helpers (`reply_address`, `turn_header_text`, `format_player_result`, `finished_header_text`, `game_subject`, `turn_subject`, `browser_url`, `rules_url`) and their tests.
- `digest_since_last_turn` filter semantics (`logged_at > last_turn_at`, best-effort None on error) and `render_board_and_you_can` degradation on render failure.
- SendMode ladder (Normal/BypassSuppression/Forced) matches its documented semantics; Forced still excludes bots and requires an address.
- Cross-reference confirmed: sweep.rs contains NO bot-turn reconciliation - `fetch_candidates` explicitly filters `game_bot_id IS NULL` and no other sweep touches bot turns, so the prior units' finding stands that a wedged bot turn (bot.turn never re-published) is invisible to every periodic job here; the recommended reconciliation sweep does not exist.
- Not re-flagged per instructions: proposal auto-decline/nudge/expiry keying on proposal `created_at`; `was_finished` with `before=None` (finished-mail aspect); custom parser combinator usage in `render_board_and_you_can`.

Severity tally: critical 0, major 2, minor 8, nit 3.
