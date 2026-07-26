# WP-46: sweep delivery semantics

**Findings:** wfe F30, wfe F31, wd F28 (major); wfe F11, wfe F32, wfe F33,
wfe F34, wfe F35, wfe F37, wd F38, wd F39 (minor); wfe F40 (nit).
**Decision:** D-2 answered option A - at-least-once, claim-then-send, **mark
only after success; never mark on a retryable skip**. D-11 answered option A -
`reminder_emails_enabled` **alone** governs reminder emails.

**Landing order:** **WP-51 should land first.** It rewrites `send_reminder`'s
body (`NotifyKind::Reminder`), the six `RealInviteMailer` methods, and collapses
the five `spawn_*` loops into one helper - all code this WP restructures. Either
order works; whichever lands **second must rebase on, not fork,** the other's
shape. Same rule for **WP-38**, which adds one sweep and one
`spawn_periodic_sweeps` parameter (`planning/landing-order.md` 6.2). **WP-57**
owns the inbound webhook side of D-2 and must not touch `sweep.rs`; this WP owns
the outbound sweeps and touches `inbound.rs` **not at all**. **WP-76**
(email-originated moves never call `notify_game_emails`) lives in
`email/commands.rs` and does not collide.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

All in `rust/web/src/email/sweep.rs` unless stated.

- **wfe F30** - `send_reminder` returns `true` on the two suppression paths
  (`should_email_recipient` false, `suppress_for_web_presence` true) and
  `sweep_once` treats `true` as sent, calling `mark_reminder_sent`. The player
  is permanently marked reminded with no email.
- **wfe F31** - `fetch_candidates` appends `FOR UPDATE SKIP LOCKED` but runs
  `fetch_all(pool)` in autocommit; the locks die with the SELECT. Zero
  protection against two replicas double-sending.
- **wfe F32** - candidates are selected on `u.reminder_emails_enabled`, but
  `send_reminder` gates on `outbound::should_email_recipient`, which requires
  `turn_emails_enabled`. The two prefs are conflated.
- **wfe F33** - `sweep_invite_nudge_once` fire-and-forgets
  `InviteMailer::send_invite` then unconditionally calls `mark_proposal_nudged`.
- **wfe F34** - `sweep_invite_auto_decline_once` flips rows and broadcasts, but
  never mails the owner, unlike `respond_proposal`'s manual decline.
- **wfe F35 + wd F39 (same bug)** - `proposals.rs::cancel_proposal_for_expiry`
  commits `status='cancelled'`, *then* reads owner (`.ok().flatten()` + `owner?`)
  and accepted ids (`.unwrap_or_default()`). A transient error there silently
  loses every cancellation email forever - the row is no longer `'open'`.
- **wfe F37** - `is_reminder_candidate` / `should_reset_reminder` are referenced
  only by this file's tests; prod filtering is the SQL. The Rust copy already
  lacks `game_bot_id IS NULL` and `reminder_emails_enabled`.
- **wfe F40** - no sweep candidate query has a `LIMIT`.
- **wfe F11** - nothing deletes from `processed_webhook_events`; migration 014
  ships `idx_processed_webhook_events_processed_at` for a prune that was never
  written.
- **wd F28** - `proposals.rs::fetch_auto_decline_candidates` keys on
  `gp.created_at` (proposal age), not the player row's age.
- **wd F38** - `RealInviteMailer::send_invite` re-fetches the proposal but never
  re-checks `status='open'` or that the token still matches a pending row.

## 2. Why it's wrong

**Every finding above is correct as written, verified against live code.** Do
not revert any of them.

- F30/F31/F40/F37 verified in `fetch_candidates`, `send_reminder`, `sweep_once`
  and `is_reminder_candidate` exactly as described.
- F32 verified: `outbound::should_email_recipient` is
  `email.is_some() && !is_bot && turn_emails_enabled`. **D-11 rationale to
  preserve:** some users play mainly by web and do not want turn emails, but a
  reminder is exactly what they need when they have missed or forgotten a game.
  Requiring both flags makes `reminder_emails_enabled` dead for those users.
- F34: `auto_decline_proposal_player` sets `response='declined'` and nothing
  else; `respond_proposal` calls `mailer().notify_owner_decline`.
- F35/wd F39 verified in `cancel_proposal_for_expiry`; it is reached only from
  `sweep_invite_expiry_once`.
- wd F28: `reset_accepted_humans_for_roster_change` bumps `pp.updated_at` and
  resets `response='pending'`, so those players are auto-declined on the next
  tick of an old proposal, and `declined` is terminal.
- wd F38 verified in `send_invite`: the only guards are token present, recipient
  gates, presence, and `find_proposal` returning `Some`.

## 3. Required end state

### 3a. `sweep.rs` - reminder claim-then-send

- `fetch_candidates`: **delete** the `FOR UPDATE SKIP LOCKED` clause (it lies),
  **add `LIMIT`** (a module const, 200). Keep the predicate otherwise identical -
  it already filters on `reminder_emails_enabled`, which D-11 confirms correct.
- New private `enum ReminderOutcome { Sent, PermanentSkip, Retry }`.
  `send_reminder` returns it instead of `bool`:
  - `Sent` - `try_send_rendered_email` returned true.
  - `PermanentSkip` - game/player/recipient row missing, no email address, bot
    recipient, or `reminder_emails_enabled` false. Marking these is correct:
    retrying changes nothing.
  - `Retry` - `suppress_for_web_presence` true, `ensure_email_token` errored,
    game load errored, or `try_send_rendered_email` returned false.
- **F32 fix, in `send_reminder` only:** replace the
  `outbound::should_email_recipient(&recipient)` call with a reminder-specific
  gate. **Do not change `should_email_recipient`** - `notify.rs` turn mails
  depend on it and WP-60 owns `outbound.rs`. Add
  `reminder_emails_enabled: bool` to `outbound::EmailRecipient` and its SELECT
  in `fetch_email_recipient` (`COALESCE(u.reminder_emails_enabled, false)`),
  then gate on `email.is_some() && !is_bot && reminder_emails_enabled`. This is
  the one `outbound.rs` edit this WP makes; note it in the commit message.
- `sweep_once`: per candidate open a transaction, re-`SELECT ... FOR UPDATE
  SKIP LOCKED` that single `game_players` row re-checking
  `turn_reminder_sent_at IS NULL AND is_turn = true`; if it does not come back,
  skip (another replica has it). Then `send_reminder`; on `Sent` or
  `PermanentSkip` mark and **commit**; on `Retry` **roll back** and leave the
  row for the next tick. `mark_reminder_sent` gains a transaction-taking form
  (`&mut PgConnection`) so the mark is inside the claim.
- **F37:** delete `is_reminder_candidate`, `should_reset_reminder` and the five
  unit tests that exercise them. The SQL is covered by the existing
  `fetch_candidates_*` `#[sqlx::test]`s.

### 3b. `sweep.rs` + `proposals.rs` - invite nudge (F33, wd F38)

Split `RealInviteMailer::send_invite` into an `async fn` core returning an
outcome and a thin `tokio::spawn` wrapper that keeps the existing
`InviteMailer::send_invite` signature (its other callers stay fire-and-forget).
Expose the core on the trait as an awaited method (e.g.
`send_invite_now(&self, ..) -> bool`, true = sent or permanently unsendable).

Inside the core, **before rendering** (wd F38): re-read the proposal and require
`status == 'open'`, and require the supplied token still matches a `pending`
row for that user on that proposal (`find_proposal_player_by_email_token`
already exists). A mismatch is a **permanent** skip - the token rotated, the
state moved on - so it does not block marking.

`sweep_invite_nudge_once` awaits `send_invite_now` per candidate and marks a
proposal nudged only when **every** candidate of that proposal returned true.
A transient failure leaves `nudged_at IS NULL` for the next tick.

### 3c. `sweep.rs` - auto-decline notifies the owner (F34)

`fetch_auto_decline_candidates` also returns `pp.user_id`.
`auto_decline_proposal_player` returns `bool` (`rows_affected() == 1`) so only a
real `pending -> declined` transition notifies. `sweep_invite_auto_decline_once`
takes `resend: Option<&resend_rs::Resend>`, builds `proposals::mailer_from` like
the other two invite sweeps, and calls `notify_owner_decline(proposal_id,
declined_user_id)` per actual transition. Thread `resend` through
`spawn_invite_auto_decline_sweep` and its `spawn_periodic_sweeps` call site
(`main.rs` passes `resend` already; do not change `main.rs`'s argument list
beyond what the new parameter requires).

### 3d. `proposals.rs::fetch_auto_decline_candidates` (wd F28)

Key the window on **`pp.updated_at`**, not `gp.created_at`. `updated_at` is
bumped by both the insert and `reset_accepted_humans_for_roster_change`, so both
failure modes in the finding are covered.

### 3e. `proposals.rs::cancel_proposal_for_expiry` (wfe F35 / wd F39)

Read first, mutate second:

1. `SELECT owner_user_id` and the accepted non-owner ids while the proposal is
   still `'open'`. On **any** error, `tracing::error!` and return `None` - do
   not cancel. The next tick retries (at-least-once).
2. Then `UPDATE ... SET status='cancelled' WHERE id=$1 AND status='open'`;
   `rows_affected() == 0` -> `None`.
3. Return the ids read in step 1.

### 3f. `sweep.rs` + `db.rs` - webhook-event pruning (wfe F11)

Add `db::delete_old_processed_webhook_events(pool, threshold) -> Result<u64>`
directly modelled on the existing `db::delete_expired_unverified_emails`
(same `make_interval(secs => $1::double precision)` idiom, same `Ok(rows_affected)`).
Add `PROCESSED_WEBHOOK_EVENT_RETENTION` = 7 days (comfortably past the svix
retry window) and `sweep_processed_webhook_events_once(pool)` logging like
`sweep_unverified_emails_once`, called from the same tick as
`sweep_unverified_emails_once` inside `spawn_unverified_email_sweep`. **Do not
edit `inbound.rs`.**

## 4. Non-goals

- `send_reminder`'s duplication of `notify::send_one`, the reminder's thread id
  and subject, the notify gating bypass, `before=None`, N+1 sends, the five
  copy-pasted spawn loops, and `notify_owner_decline`'s missing invite gates -
  all **WP-51**. This WP changes `send_reminder`'s **return type and recipient
  gate** only.
- The inbound webhook's marker ordering and 5xx behaviour - **WP-57**.
- `outbound.rs` beyond the one `EmailRecipient` field - **WP-60**.
- The nudge query's `gp.created_at` keying: cosmetic per wd F28's own body
  (`nudged_at IS NULL` already makes it once-only). Leave it.
- `fetch_candidates`'s `($1 || ' seconds')::interval` string-concat idiom (wd
  F40, not in this package).
- No new migration, no job table, no worker split (D-2 rejected option C).

## 5. Regression test cases

`sweep.rs` `#[cfg(all(test, feature = "ssr"))] mod tests` - reuse its
`seed_reminder_game` helper and the `turn_reminder_suppressed_by_recipient_presence`
pattern:

- **F30 (the headline):** drive `sweep_once`, not `send_reminder`. With the
  recipient web-present, `turn_reminder_sent_at` stays `NULL`; once presence
  lapses, a second `sweep_once` sends and marks. This is the test the finding
  says does not exist.
- **F32:** a user with `reminder_emails_enabled = true` and
  `turn_emails_enabled = false` is selected **and** sent (not skipped, not
  marked-without-send). Inverse: `reminder_emails_enabled = false` is not even a
  candidate (the existing `fetch_candidates_excludes_reminder_disabled` covers
  the SQL half).
- **F31:** two concurrent `sweep_once` calls over one due candidate mark it once
  (`turn_reminder_sent_at` set, exactly one send attempt).
- **F40:** more than `LIMIT` due candidates yields exactly `LIMIT` rows.
- **F11:** `delete_old_processed_webhook_events` deletes rows past retention and
  keeps fresh ones - mirror `sweep_unverified_emails_deletes_expired_only`, and
  put the db-level test beside `delete_expired_unverified_emails`' test in
  `db.rs mod tests`.

`proposals.rs mod tests` - beside `sweep_candidate_queries_match_backdated_proposals`:

- **wd F28:** a proposal backdated past the auto-decline threshold with a
  freshly added (or freshly reset) pending player - that player is **not** a
  candidate; backdate `pp.updated_at` and it becomes one.
- **wfe F35:** `cancel_proposal_for_expiry` on a proposal with an accepted
  non-owner returns owner + that id, and the proposal is `'cancelled'`; on an
  already-cancelled proposal it returns `None` and changes nothing.
- **wfe F34:** `auto_decline_proposal_player` returns `true` on a pending row
  and `false` on a second call.
- **wd F38:** the invite-send core returns "permanently unsendable" for a
  cancelled proposal and for a stale token, and does not send.

## 6. Riders

| File | One-line fix | Test? |
|---|---|---|
| `email/sweep.rs` `is_reminder_candidate`, `should_reset_reminder` (wfe F37) | Delete both fns and their unit tests; the dead `unwrap_or(Duration::hours(24))` goes with them. | n (deletes tests) |
| `email/sweep.rs` all four candidate queries (wfe F40) | Add `LIMIT` (one shared const) to `fetch_candidates`, `fetch_nudge_candidates`, `fetch_expiry_candidates`, `fetch_auto_decline_candidates`. | y (reminder one) |
| `email/sweep.rs` + `db.rs` (wfe F11) | `delete_old_processed_webhook_events` + a 7-day prune on the existing unverified-email tick. | y |
