# WP-57: inbound webhook delivery semantics

**Findings:** wfe F2 (major), wfe F10 (minor), wfe F16 (nit).
**Decision:** D-2 answered **option A - at-least-once**: dedupe marker only
*after* successful processing, 5xx so svix retries, idempotency rests on the
marker. **No enqueue/worker split (option C).**

**Landing order:** **WP-59 must land first.** It rewrites `select_route`, adds
`extract_addr_spec` plus a `from` local inside `resend_webhook`, and collapses
the three inline body-fetch blocks into `fetch_inbound_text`. Build on those
shapes. All code below is in `rust/web/src/email/inbound.rs`.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This file is under
> concurrent edit; line numbers are omitted on purpose (the one marked
> "approximate" is a hint - verify it).

## 1. Problem

- **wfe F2** - `resend_webhook` calls `mark_event_processed` *before* any
  processing and returns `StatusCode::OK` regardless of outcome. Any downstream
  failure (JSON parse, Resend raw-email fetch, invite transaction DB errors,
  reply send) leaves the event marked, svix never retries, and the handler just
  logs. A player's move or invite response vanishes silently.
- **wfe F10** - all work (Resend fetch, per-command game-service round trips,
  MJML render, outbound send) runs inline before the 200; svix times out ~15s.
- **wfe F16** - `verify_webhook` does `HeaderValue::from_str(...).unwrap()` on
  its three caller-supplied string arguments.

## 2. Why it's wrong

- **wfe F2 is correct as written.** Verified live: `mark_event_processed`
  (`INSERT ... ON CONFLICT DO NOTHING`) runs right after signature verification,
  before the payload is even deserialized; `handle_game_reply`,
  `handle_invite_reply` and `handle_settings_reply_route` all return `()`, and
  the function tail is an unconditional `StatusCode::OK`.
- **wfe F10 is correct as written**, but D-2 declined option C. Rider only.
- **wfe F16 is correct as written.** Three `.unwrap()`s, `verify_webhook` is
  `pub`.

**Constraint the fix must respect (not in the finding):** a retry re-executes
what the first attempt did, and game commands and invite responses are **not**
idempotent. A 5xx is therefore permitted only for failures occurring **before**
any state mutation.

## 3. Required end state

### 3a. Outcome type

A private `enum RouteOutcome { Done, Retry }` (doc-comment it: `Done` =
finished or failed unrecoverably, mark and 200; `Retry` = transient failure
before any mutation, do not mark, 5xx). `handle_game_reply`,
`handle_invite_reply` and `handle_settings_reply_route` return it instead of
`()`.

### 3b. Which failures are `Retry`

Only these, and only before any mutation:

- any `sqlx` `Err(_)` on a lookup - `find_game_player_by_email_token`,
  `from_matches_verified_email`, `resolve_user_by_verified_from`, the invite
  transaction's begin / `lock_proposal_for_update` / roster load;
- the body fetch - `RESEND_API_KEY` missing, or `fetch_raw_email` failing.
  Post-WP-59 this is `fetch_inbound_text`; widen its `Option<String>` return to
  a `Result`/three-state so the caller can tell "fetch failed" from "no body".
  Change its return shape only, not its body.

Everything else is `Done`, behaviour unchanged: unknown/absent token, From not
matching a verified address, empty command list, command dispatch outcomes
(applied *or* failed), and every failure inside the five `send_*` reply helpers.
**Do not** change those helpers' signatures or their swallow-and-log behaviour.

### 3c. `resend_webhook`

- Replace the pre-processing marker with a read-only duplicate check: new
  `async fn event_already_processed(pool, event_id) -> sqlx::Result<bool>`
  (`SELECT EXISTS`). `Ok(true)` -> `OK`, do nothing; `Err(_)` -> log +
  `INTERNAL_SERVER_ERROR`, as today.
- Payload deserialize failure, non-`email.received` event type, and WP-59's
  unextractable-From path: mark the event, return `OK` (permanent).
- After the dispatch `match`: `Done` -> `mark_event_processed` (ignore the bool,
  log an `Err`) then `OK`; `Retry` -> do **not** mark, log at `error`, return
  `INTERNAL_SERVER_ERROR`.
- Keep `mark_event_processed` itself as-is; its `ON CONFLICT DO NOTHING` makes
  the post-success write safe. Note in its doc comment that the check-then-mark
  window lets two simultaneous deliveries of one `svix-id` both process - the
  accepted at-least-once cost under D-2.

### 3d. `verify_webhook` (wfe F16)

Add `#[error("invalid header value")] InvalidHeaderValue` to `VerifyError`;
replace the three `HeaderValue::from_str(...).unwrap()` with
`.map_err(|_| VerifyError::InvalidHeaderValue)?`. No caller change -
`resend_webhook` already maps any `Err` to `UNAUTHORIZED`.

## 4. Non-goals

- **WP-59** (wfe F4, F6-F9, F12-F15, F21, F23, F24, F26-F29): addr-spec
  extraction, `select_route`, quote stripping, invite-intent ordering, the
  invite `tx.rollback()`s, the missing-roster log, subject degradation,
  reply-address helpers, dead-code deletion.
- **WP-56** (wfe F1, F5, F17): From authentication, `s-` token, SPF/DKIM, the
  `None`-route fallthrough, `emails *` verb removal. Do not touch
  `from_matches_verified_email` or `resolve_user_by_verified_from`.
- **WP-58** (wfe F3, F25, blocked on D-10): unsubscribe / RFC 8058.
- **WP-46** (wfe F11, F30-F32): `processed_webhook_events` pruning and the
  sweep's mark-before-do semantics. Do not edit `sweep.rs`.
- No `tokio::spawn`, job table or worker (D-2 rejected C). No migration, no
  change to reply-email content.

## 5. Regression test cases

**Fixture gap - confirmed against live code.** `inbound.rs`'s `mod tests` is
pure unit tests; `outbound.rs` holds the only other `#[cfg(test)]` in `email/`.
No `AppState` or webhook fixture exists for the email handlers. Build one: add
`rust/web/tests/inbound_webhook.rs` modelled on `rust/web/tests/ssr_pages.rs` -
copy its `make_state(pool)` (`AppState` with a real `PgPool`, NATS
`GameBroadcaster`, `resend: None`) and its `#[sqlx::test]` + `build_router` +
`tower::ServiceExt::oneshot` pattern, driving the real
`POST /api/webhooks/resend`. Sign bodies with
`svix::webhooks::Webhook::new(secret)` then `.sign(...)`, and set
`RESEND_WEBHOOK_SECRET` to that secret in one place (edition 2024:
`std::env::set_var` is `unsafe`). Do not build a Resend HTTP double - leaving
`RESEND_API_KEY` unset *is* the transient path this WP is about.

- Transient -> 5xx and **no** `processed_webhook_events` row: signed
  `email.received` for a valid `g-` token with the body fetch failing.
- Retry succeeds: re-POSTing that `svix-id` once the condition clears is
  processed, not short-circuited as a duplicate.
- Success -> 200 and exactly one marker row; re-POSTing it -> 200, no work.
- Permanent -> 200 and marked: malformed JSON body; non-`email.received` type.
- Unit test in `inbound.rs mod tests`: `verify_webhook` with a `\n` in `msg_id`
  returns `Err(VerifyError::InvalidHeaderValue)` instead of panicking.

## 6. Riders

| File | One-line fix | Test? |
|---|---|---|
| `inbound.rs` `resend_webhook` (wfe F10) | Enqueue **rejected by D-2**; instead time the dispatch `match` and `tracing::warn!` past 10s, so option C can be revisited on evidence. | n |
| `inbound.rs` `verify_webhook` (wfe F16) | The three `.unwrap()`s -> `.map_err(...)?` plus the new `VerifyError` variant (section 3d; approximate location inbound.rs:144-146, verify). | y |
