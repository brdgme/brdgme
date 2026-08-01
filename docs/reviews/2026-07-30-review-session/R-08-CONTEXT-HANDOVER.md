# R-08 Context Handover - Transient errors must not be classified permanent

**Status:** pending (97-REMEDIATION-PROGRESS.md:33)
**Closes:** F-136 (High), F-145 (Medium)
**Size:** S (two error-classification sites, one change)
**Depends on:** nothing
**HEAD at investigation:** `1159ebe19b5a0b1c097084de1239bacaf44c8a5e`

---

## 1. Defect Summary

Both sites are instances of systemic pattern 5 ("the `_ => <default>`
substitution"): a catch-all arm collapses a transient `Err(_)` into the same
outcome as a legitimate permanent condition, causing mark-without-send.

### F-136 - sweep.rs reminder classifier

`rust/web/src/email/sweep.rs:134-138`:

```rust
let recipient = match crate::email::outbound::fetch_email_recipient(pool, game_player_id).await
{
    Ok(Some(r)) => r,
    _ => return ReminderOutcome::PermanentSkip,
};
```

`fetch_email_recipient` (`rust/web/src/email/outbound.rs:188-209`) returns
`anyhow::Result<Option<EmailRecipient>>`. The `_` arm swallows both:
- `Ok(None)` - slot does not exist (legitimate PermanentSkip)
- `Err(_)` - transient DB error (pool timeout, connection reset) - must be Retry

`sweep_once` (`sweep.rs:288-305`) treats `PermanentSkip` identically to `Sent`:
it calls `mark_reminder_sent_tx` and commits. One transient DB error permanently
sets `turn_reminder_sent_at`; no reminder is ever sent.

### F-145 - proposals.rs nudge classifier

`rust/web/src/proposals.rs:257-296`, three sites in `send_invite_core`:

1. `:268` - `let Some(recip) = mailer_recipient(...) else { return true; }`
   `mailer_recipient` (`:196-208`) returns `Option<InviteRecipient>`, collapsing
   `Err(_)` (logged at `:203-205`) and `Ok(None)` both to `None`.

2. `:282` - `let Some(proposal) = mailer_proposal(...) else { return true; }`
   `mailer_proposal` (`:180-192`) same shape: `Err(_)` logged at `:187-189`,
   returned as `None`.

3. `:288` - `let Ok(Some(pp)) = find_proposal_player_by_email_token(...) else { return true; };`
   `find_proposal_player_by_email_token` (`:952-962`) returns
   `sqlx::Result<Option<ProposalPlayer>>`. The `let-else` on `Ok(Some(_))`
   treats `Err(_)` identically to "token not found".

`send_invite_core` contract (doc comment `:250-256`): `true` = sent or
permanently unsendable (do not retry); `false` = transient only.

Nudge path (`sweep.rs:498-520`, `sweep_invite_nudge_once`): calls
`mailer.send_invite_now(...)` per candidate; if all return `true` for a
proposal, calls `mark_proposal_nudged` (`proposals.rs:995-1004`) which commits
`nudged_at = NOW()`. A transient error returning `true` marks the proposal
nudged with nothing sent.

---

## 2. Named Transient Error Variants

There is no custom error enum at either site. The transient errors are:

| Site | Return type | Transient variant |
|------|-------------|-------------------|
| `fetch_email_recipient` (outbound.rs:188) | `anyhow::Result<Option<EmailRecipient>>` | `Err(anyhow::Error)` wrapping `sqlx::Error` (pool timeout, connection reset, IO) |
| `fetch_invite_recipient` (proposals.rs:145) | `sqlx::Result<Option<InviteRecipient>>` | `Err(sqlx::Error)` - same transients |
| `find_proposal` (called by `mailer_proposal`) | `sqlx::Result<Option<Proposal>>` | `Err(sqlx::Error)` |
| `find_proposal_player_by_email_token` (proposals.rs:952) | `sqlx::Result<Option<ProposalPlayer>>` | `Err(sqlx::Error)` |

All are plain `sqlx::Error` (or `anyhow::Error` wrapping one). No domain-specific
error enum exists for these lookups. The transient/permanent distinction is
entirely by `Result` arm: `Err(_)` = transient, `Ok(None)` = permanent.

---

## 3. Persistence Behavior

### Reminder sweep (F-136)

- `sweep_once` (`sweep.rs:234-309`) iterates candidates.
- Per candidate: `pool.begin()` -> `FOR UPDATE SKIP LOCKED` claim -> `send_reminder` -> outcome.
- `Sent | PermanentSkip` -> `mark_reminder_sent_tx` (sets `turn_reminder_sent_at = NOW()`) -> `tx.commit()`.
- `Retry` -> no mark, tx dropped (rollback).
- The mark is inside the transaction; a `Retry` outcome leaves the row unmarked.

### Invite nudge sweep (F-145)

- `sweep_invite_nudge_once` (`sweep.rs:498-520`) fetches candidates, calls
  `send_invite_now` per invitee (NOT in a transaction).
- Accumulates `all_sent: HashMap<Uuid, bool>` per proposal (`&= ok`).
- After all invitees: if `*sent == true`, calls `mark_proposal_nudged` (a
  standalone UPDATE, `proposals.rs:995-1004`).
- No transaction wraps the send+mark; the mark is a separate statement.

---

## 4. Existing Testing Seams and Patterns

### sweep.rs tests (`sweep.rs:613-1522`, `#[cfg(all(test, feature = "ssr"))]`)

- `send_reminder` is called directly (private fn, same module).
- `sweep_once` is called directly.
- `seed_reminder_game` helper (`:1074-1131`) seeds a full game + user + email + game_player.
- Tests use `resend = None` (dev log mode; `try_send_rendered_email` returns `true` immediately).
- Assertions on DB state (`turn_reminder_sent_at` NULL vs set).
- Existing tests: presence suppression, concurrent marks, turn_emails_disabled, limit cap.
- No existing test injects a DB error into `fetch_email_recipient`.

### proposals.rs tests (`proposals.rs:2515+`, `#[cfg(all(test, feature = "ssr"))]`)

- `mailer_from(pool.clone(), None)` constructs a `RealInviteMailer` with no Resend.
- `send_invite_now` called directly (public trait method).
- `seed_invite_user`, `seed_game_version`, `seed_proposal`, `insert_proposal_player` helpers.
- Existing test `send_invite_core_permanent_skip_cancelled_and_stale_token` (`:2797-2847`)
  asserts `true` for cancelled proposal and stale token.
- No existing test injects a transient DB error.

### Error injection options

There is no mock/fake pool infrastructure. `#[sqlx::test]` provides a real
migrated database per test. To simulate a transient error:

1. **Drop the pool connection / use a closed pool** - construct a `PgPool` then
   close it; subsequent queries return `Err(sqlx::Error::PoolClosed)`.
2. **Query a non-existent table after migration** - not viable (migrations run).
3. **Set `statement_timeout` to 0 on the test pool** - causes immediate timeout
   errors on the next query.
4. **Extract the classifier into a pure function** that takes a
   `Result<Option<T>, E>` and returns the outcome, then unit-test the mapping
   directly without a DB. This is the minimal approach the acceptance criteria
   suggest ("calls the sweep's classifier with a transient DB error").

---

## 5. Candidate Minimal Test Approach

### Acceptance criteria (from 98-REMEDIATION-PLAN.md:335-343)

1. Neither site has a `_ =>` arm. The `match` is exhaustive over named variants.
2. A test calls the sweep's classifier with a transient DB error and asserts the
   reminder is NOT marked sent.
3. A test calls the proposals nudge path with a transient send error and asserts
   the nudge is NOT marked delivered.

### Approach A - extract pure classifier (preferred, minimal)

For F-136: extract the `fetch_email_recipient` result-mapping into a small
`fn classify_recipient_lookup(Result<Option<EmailRecipient>, E>) -> ReminderOutcome`
(or inline the three-arm match). Test the mapping directly:
- `Ok(Some(_))` -> proceed (not PermanentSkip)
- `Ok(None)` -> PermanentSkip
- `Err(_)` -> Retry

For F-145: change `mailer_recipient`/`mailer_proposal` to return
`Result<Option<T>, ()>` (or a small `Lookup` enum), and have `send_invite_core`
return `false` on the error arm. Test via `send_invite_now` with a closed pool
or by extracting the classification logic.

### Approach B - closed-pool integration test

Construct a `PgPool`, close it (`pool.close().await`), then call `send_reminder`
/ `send_invite_now` with the dead pool. Assert:
- `send_reminder` returns `Retry` (not `PermanentSkip`).
- `send_invite_now` returns `false` (not `true`).

This exercises the real code path end-to-end but requires the function signatures
to accept the pool in a way that allows substitution (they already do - `&PgPool`).

### Approach C - statement_timeout

Set `SET statement_timeout = '1ms'` on the test pool, then run a query that
takes longer. Fragile; not recommended.

**Recommendation:** Approach B (closed pool) for the integration assertion,
combined with the three-arm match fix (no `_` arm) satisfying AC1 by
construction. The closed-pool test is deterministic, needs no new
infrastructure, and directly proves the transient-error path.

---

## 6. Fix Shape (for the implementing agent)

### F-136 fix (sweep.rs:134-138)

Replace:
```rust
_ => return ReminderOutcome::PermanentSkip,
```
With:
```rust
Ok(None) => return ReminderOutcome::PermanentSkip,
Err(e) => {
    tracing::error!("turn_reminder: recipient lookup failed for {}: {}", game_player_id, e);
    return ReminderOutcome::Retry;
}
```

### F-145 fix (proposals.rs)

Change `mailer_recipient` and `mailer_proposal` return types from `Option<T>` to
`Result<Option<T>, ()>` (or a named enum). At each call site in `send_invite_core`:
- `Err(())` -> `return false` (transient, retry)
- `Ok(None)` -> `return true` (permanent skip)
- `Ok(Some(x))` -> proceed

For `:288` (`find_proposal_player_by_email_token`):
```rust
let pp = match find_proposal_player_by_email_token(pool, &token).await {
    Ok(Some(pp)) => pp,
    Ok(None) => return true,
    Err(_) => return false,
};
```

---

## 7. Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| D-1 | F-136 and F-145 are ONE change (per spec) | The surviving duplicate of the abandoned wfe F36 dedup is where F-136 lives; same defect class in two halves of one sweep module. |
| D-2 | No new error enum needed | The transient/permanent distinction is fully captured by `Err(_)` vs `Ok(None)` on the existing `Result`/`Option` returns. A three-arm match is sufficient. |
| D-3 | `mailer_recipient`/`mailer_proposal` gain a three-way return | `Option<T>` cannot carry the error/transient distinction. `Result<Option<T>, ()>` is the minimal change; the `()` error carries no data because the error is already logged inside the helper. |
| D-4 | Closed-pool test for transient-error assertion | Deterministic, no new infrastructure, exercises the real code path. `PgPool::close().await` makes all subsequent queries return `Err(PoolClosed)`. |
| D-5 | `ReminderOutcome` stays private | Tests are in-module (`mod tests`); no visibility change needed. |
| D-6 | The `notify_changed_reinvite` / `notify_owner_decline` / other spawned-task mailers are NOT in scope | They use `mailer_recipient`/`mailer_proposal` but their failure mode is "email not sent" (fire-and-forget spawn, no mark). Only `send_invite_core` feeds a mark (`mark_proposal_nudged`). F-145 names only `send_invite_core`. |
| D-7 | F-144 (per-proposal dedup key) and F-146 (threading) are NOT in R-08 scope | F-144 is R-19; F-146 is R-20. R-08 closes only F-136 + F-145. |

---

## 8. File:Line Reference Index

| Item | Location |
|------|----------|
| `ReminderOutcome` enum | `rust/web/src/email/sweep.rs:99-103` |
| F-136 defect site | `rust/web/src/email/sweep.rs:134-138` |
| `sweep_once` mark+commit | `rust/web/src/email/sweep.rs:288-305` |
| `mark_reminder_sent_tx` | `rust/web/src/email/sweep.rs:86-97` |
| `fetch_email_recipient` | `rust/web/src/email/outbound.rs:188-209` |
| `send_invite_core` | `rust/web/src/proposals.rs:257-345` |
| F-145 site 1 (`mailer_recipient`) | `rust/web/src/proposals.rs:268` |
| F-145 site 2 (`mailer_proposal`) | `rust/web/src/proposals.rs:282` |
| F-145 site 3 (`find_proposal_player_by_email_token`) | `rust/web/src/proposals.rs:288` |
| `mailer_recipient` helper | `rust/web/src/proposals.rs:196-208` |
| `mailer_proposal` helper | `rust/web/src/proposals.rs:180-192` |
| `fetch_invite_recipient` | `rust/web/src/proposals.rs:145-158` |
| `find_proposal_player_by_email_token` | `rust/web/src/proposals.rs:952-962` |
| `sweep_invite_nudge_once` (nudge mark path) | `rust/web/src/email/sweep.rs:498-520` |
| `mark_proposal_nudged` | `rust/web/src/proposals.rs:995-1004` |
| `InviteMailer` trait | `rust/web/src/proposals.rs:103-121` |
| `RealInviteMailer::send_invite_now` | `rust/web/src/proposals.rs:362-370` |
| `mailer_from` | `rust/web/src/proposals.rs:749` |
| `try_send_rendered_email` | `rust/web/src/email/outbound.rs:24-59` |
| Existing sweep tests | `rust/web/src/email/sweep.rs:613-1522` |
| Existing proposals tests | `rust/web/src/proposals.rs:2515+` |
| Existing `send_invite_core` test | `rust/web/src/proposals.rs:2797-2847` |
| `seed_reminder_game` helper | `rust/web/src/email/sweep.rs:1074-1131` |
| `seed_invite_user` helper | `rust/web/src/proposals.rs:2518-2539` |
| R-08 spec | `docs/reviews/2026-07-30-review-session/98-REMEDIATION-PLAN.md:318-344` |
| F-136 finding | `docs/reviews/2026-07-30-review-session/07-web-domain-remainder.md:342-364` |
| F-145 finding | `docs/reviews/2026-07-30-review-session/07b-wp51-wp53-tail.md:113-166` |
| Pattern 5 context | `docs/reviews/2026-07-30-review-session/00-STATE.md:513-515` |
