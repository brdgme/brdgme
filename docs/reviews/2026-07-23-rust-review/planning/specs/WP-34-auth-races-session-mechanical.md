# WP-34: auth races and session mechanical

Scope: ws F1 (major), F3, F5, F6, F10 (minor), F12, F13, F14, F15 (nit).
Files: `rust/web/src/auth/server.rs`, `rust/web/src/auth/session.rs`,
one new migration `rust/web/migrations/0NN_login_email_sends.sql` (the number
`023` is NOT guaranteed - four packages this cycle add a migration and only the
first lander gets `023`; see `landing-order.md` section 6.4, and `ls
rust/web/migrations/` immediately before writing the file).

**Read the named function before editing it. If a function does not match
what this spec describes, STOP and report rather than improvising.** No line
numbers are given deliberately; earlier specs in this review had a 33-46%
citation error rate.

**Landing order:** WP-34 lands BEFORE WP-35. Both touch
`verify_turnstile_token` and `request_confirmation_code`.

---

## 1. Problem

- **F1 (MAJOR).** `validate_confirmation_code` enforces the 10-attempt cap as
  check-then-act: `SELECT` the row, compare `attempts` and `code` in Rust,
  and only on mismatch `UPDATE ... attempts = attempts + 1`. All statements
  run on the bare pool with no lock. Concurrent confirms all pass the
  pre-check before any increment commits.
- **F3.** `confirm_login` writes the user into the pre-authentication
  tower-sessions session; the session id is not rotated on privilege change.
- **F5.** `get_current_user` does `validate_session_token(..).unwrap_or(false)`
  and the `else` branch clears the session, so a transient `sqlx::Error`
  logs the user out.
- **F6.** The global 24h cap in `request_confirmation_code` is
  `SUM(sent_count) WHERE last_sent_at > NOW() - INTERVAL '24 hours'`, but
  `sent_count` is lifetime-cumulative on the row and is never reset (the
  upsert always does `sent_count = login_confirmations.sent_count + 1`).
- **F10.** `add_email_address` calls `request_confirmation_code(...).await?`
  and discards the returned `LoginResponse`, so a `success: false` global-cap
  refusal is reported to the UI as success.
- **F12 (nit).** `format!("{:06}", rand::random::<u32>() % 1_000_000)` has
  modulo bias.
- **F13 (nit).** `confirm_login` does no shape validation, unlike `login`.
- **F14 (nit).** `verify_turnstile_token` builds `reqwest::Client::new()` per
  call; a shared client already lives in context.
- **F15 (nit).** `logout` discards the `invalidate_auth_token` error with
  `let _ =` and never flushes the session record.

## 2. Why it's wrong

- **F1 is correct as written** in mechanism; the verification's severity
  adjustment (critical -> major) is the operative one, because failed
  guesses still commit, so the race is a bounded window multiplier over the
  cap of 10, not unbounded. Do not revert either the finding or the
  downgrade.
- **F1 recommendation has an off-by-one.** With increment-before-compare, the
  comparison MUST be `>` not `>=`: after 9 failures `attempts` is 9, the
  correct 10th attempt increments to 10, and `>= 10` would reject it.
- **F3, F5, F10, F12, F13, F14, F15 are all correct as written** (verified
  against live source).
- **F6 is correct, and its option 1 is not.** Resetting `sent_count` on code
  rotation UNDER-counts: hourly rotation yields 120 real sends against a
  50/day cap. Only a windowed counter is sound. The per-email cap
  (`LOGIN_MAX_SENDS_PER_EMAIL`) is NOT affected - it is guarded by
  `code_valid &&`, so it never spans a rotation. Do not change it.

## 3. Required end state

`rust/web/src/auth/server.rs`:

- `validate_confirmation_code`: a single atomic statement
  `UPDATE login_confirmations SET attempts = attempts + 1 WHERE email = $1
  RETURNING *` on the pool replaces the `SELECT`, the Rust cap check and the
  mismatch `UPDATE`. Then, in Rust, against the returned row, in order:
  no row -> `invalid()`; `created_at` older than 1 hour -> `invalid()`;
  `attempts > CONFIRM_MAX_ATTEMPTS_PER_CODE` -> increment
  `login_confirm_attempt_cap_hit_total` and `invalid()`; `code != token` ->
  `invalid()`; otherwise `Ok(row)`. Return type stays `LoginConfirmation`.
  Both callers (`confirm_login_inner`, `confirm_email_address`) are unchanged.
- `confirm_login`: call `session.cycle_id().await`, mapped through
  `internal(...)`, before `set_user_session`.
- `confirm_login`: before touching the DB, early-return the same
  `"Invalid or expired token"` error when `token.len() != 6`, `token` is not
  all ASCII digits, or `email` is empty / has no `@`.
- `request_confirmation_code`: the global cap is counted from a new
  append-only table, not from `sent_count`. Inside the existing
  advisory-locked transaction: the GC statement also deletes
  `login_email_sends` rows older than 24 hours; the cap query becomes
  `SELECT COUNT(*) FROM login_email_sends WHERE sent_at > NOW() - INTERVAL
  '24 hours'` compared against `LOGIN_GLOBAL_MAX_SENDS_PER_DAY`; and one row
  is inserted into `login_email_sends` alongside the existing
  `login_confirmations` upsert. `sent_count` and `last_sent_at` keep their
  current upsert behaviour (the per-email cap and cooldown still use them).
- `add_email_address`: bind the `LoginResponse` from
  `request_confirmation_code` and, when `success` is false, return
  `Err(ServerFnError::new(resp.message))` instead of `Ok(())`.
- `request_confirmation_code`: generate the code with
  `rand::rng().random_range(0..1_000_000)`. Ignore the constant-time and
  plaintext-storage halves of F12 (explicitly declined - 1e6 keyspace,
  network-bound, attempt-capped).
- `verify_turnstile_token`: take the shared client as a parameter
  (`client: &reqwest::Client`) instead of constructing one.
  `expect_context::<reqwest::Client>()` must be called in `login` (server-fn
  scope) and the client passed in. WP-35 also edits this function - it must
  not revert the signature.
- `logout`: replace `let _ = invalidate_auth_token(...)` with an explicit
  match that `tracing::error!`s the failure (still returning `Ok(true)`), and
  call `session.flush().await` after `clear_user_session`.

`rust/web/migrations/0NN_login_email_sends.sql` (new; take the next free number
from a fresh `ls rust/web/migrations/` at write time - `023` is not guaranteed,
see `landing-order.md` section 6.4): create
`login_email_sends (id uuid primary key default gen_random_uuid(), email text
not null, sent_at timestamp not null default now())` plus an index on
`sent_at`. Match the style of the existing migrations in that directory.

`rust/web/src/auth/session.rs`: no behavioural change in this package.

## 4. Non-goals

- Session expiry, session GC, or any `user_auth_tokens` lifetime change
  (D-14 (iv): sessions must NOT expire). Revoke-all is WP-35's.
- Turnstile fail-open / fail-closed semantics, the encryption key, email
  squatting, blocked-domain enumeration, token expiry - all WP-35.
- Email normalisation (ws F9) - not in this package.
- The `SECURE_COOKIE` default and the session cookie flags - WP-36.
- Any change to `LOGIN_MAX_SENDS_PER_EMAIL` or the cooldown.
- Do not hash confirmation codes; do not make the code compare constant-time.

## 5. Regression test cases

All in the existing `#[cfg(test)] mod tests` at the bottom of
`rust/web/src/auth/server.rs` (`#[sqlx::test]`, `with_pool_context`,
`seed_confirmation`, `get_confirmation` helpers already there). `web` tests
need `--features ssr`. Mandatory per CODING.md: `auth/` changes ship tests.

- F1: N concurrent `confirm_login_inner` calls with a WRONG code against one
  seeded row (mirror the existing `login_concurrent_requests_do_not_overshoot_*`
  pattern) -> final `attempts` never exceeds
  `CONFIRM_MAX_ATTEMPTS_PER_CODE + 1`, and every call errored.
- F1 off-by-one: seed `attempts = 9`, submit the CORRECT code -> `Ok`. Seed
  `attempts = 10`, submit the correct code -> `Err`. This is the test that
  pins `>` vs `>=`.
- F3: after `confirm_login`, the session id differs from the pre-login id.
- F5: not DB-testable cheaply - assert instead that `get_current_user`
  returns `Err` (not `Ok(None)`) when validation fails with a DB error, via a
  closed/poisoned pool if the harness allows; otherwise assert only that a
  valid token still returns the user and that session state is untouched on
  the error path by inspection, and say so in a comment.
- F6: two `login` calls for the same email spanning a simulated rotation
  (backdate `created_at` past 1 hour between them) -> the global counter
  counts 2, not the row's cumulative `sent_count`. Also: 50 seeded
  `login_email_sends` rows inside the window -> the 51st `login` returns
  `success: false`; the same 50 rows backdated past 24h -> `login` succeeds.
- F10: seed the global cap to full, call `add_email_address` -> `Err` whose
  message is the cap message, and the caller sees no success.
- F13: `confirm_login("a@b.com", "12345")` and `("a@b.com", "abcdef")` ->
  `Err("Invalid or expired token")` with no `login_confirmations` row read
  (assert the error message only).
- F15: `logout` still returns `Ok(true)` and the session record is gone.

## 6. Riders

| finding | file | one-line fix | test needed |
|---|---|---|---|
| F3 | `auth/server.rs` `confirm_login` | `session.cycle_id().await` before `set_user_session` | Y |
| F5 | `auth/server.rs` `get_current_user` | propagate the `sqlx::Error` with `?`/`internal(...)`; clear the session only on a definitive `Ok(false)` | Y |
| F10 | `auth/server.rs` `add_email_address` | check `LoginResponse.success`, return `Err` carrying the message | Y |
| F12 | `auth/server.rs` `request_confirmation_code` | `rand::rng().random_range(0..1_000_000)`; decline parts (2) and (3) | N |
| F13 | `auth/server.rs` `confirm_login` | early-return the uniform error on bad token/email shape | Y |
| F14 | `auth/server.rs` `verify_turnstile_token` | take `&reqwest::Client`; `expect_context` in `login` | N |
| F15 | `auth/server.rs` `logout` | log the `invalidate_auth_token` error; `session.flush()` after clearing | Y |
