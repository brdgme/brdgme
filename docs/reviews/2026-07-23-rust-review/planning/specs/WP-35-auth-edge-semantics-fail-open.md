# WP-35: auth edge semantics and fail-open

Scope: ws F2 (major), F16 (major), F4, F7, F8, F11 (minor), plus three
routed-in items (a) email-change re-verification, (b) duplicated
`DELETE FROM login_confirmations`, (c) `cap_digest` off-by-one.
Files: `rust/web/src/auth/server.rs`, `rust/web/src/crypto.rs`,
`rust/web/src/main.rs`, `rust/web/src/db.rs`, `rust/web/src/settings.rs`.

**Read the named function before editing it. If a function does not match
what this spec describes, STOP and report rather than improvising.** Line
numbers are deliberately omitted (earlier specs in this review had a 33-46%
citation error rate).

**Landing order (hard):** WP-41 (db.rs quality pass) -> WP-36 (crypto/deploy
hardening, which changes `load_key`'s return type to
`Zeroizing<[u8; 32]>`) -> WP-34 -> **WP-35**. Do not attempt the `db.rs`
module split (ws F42). If WP-41 or WP-36 has not landed, stop and say so.

Decisions in force: **D-12 = option A** (fail closed in prod) and
**D-14 = option A MODIFIED** - expiring unverified claims, windowed cap
(WP-34), revoke-all-sessions, but **NO session expiry**; changing an account
email requires re-verification.

---

## 1. Problem

- **F2 (MAJOR).** Any logged-in user can insert an unverified `user_emails`
  row for ANY address (`add_email_address` -> `db::insert_unverified_email`,
  no proof of control). When the real owner later signs up,
  `confirm_login_inner` finds the pending row and returns
  `"Invalid or expired token"` even though the presented code proves mailbox
  control. Permanent lockout with a misleading error.
- **F16 (MAJOR).** `crypto::load_key` silently returns `default_key()` (a
  public in-repo key) when `DATABASE_ENCRYPTION_KEY` is unset; `main` only
  `tracing::warn!`s via `using_default_key()` and serves. `using_default_key`
  checks presence only, so a set-but-malformed value fails later, at first
  admin request.
- **F4.** `login` returns `"This email domain is not supported"` for a
  blocked domain only when no verified `user_emails` row exists - a
  registration oracle.
- **F7.** `request_confirmation_code` commits the code row and send
  accounting before calling `send_login_email`, which only
  `tracing::error!`s a Resend failure; `login` still reports success.
- **F8.** `verify_turnstile_token` returns `true` on a transport error
  (warn-logged), and an unset `TURNSTILE_SECRET_KEY` disables verification
  with no startup signal.
- **F11.** `user_auth_tokens` rows never expire and there is no revoke-all.
- **(b)** `DELETE FROM login_confirmations WHERE email = $1` is written raw
  in two places in `auth/server.rs` (`confirm_login_inner`, inside its
  transaction, and `confirm_email_address`, on the pool).
- **(c)** `make_email_address_active` calls
  `db::find_active_turn_games(&pool, user.id, SWITCH_DIGEST_CAP)` (which
  applies `LIMIT $2`) and then `db::cap_digest(games, SWITCH_DIGEST_CAP)`.
  The truncation can never fire, so "more were waiting" is undetectable.

## 2. Why it's wrong

- **F2, F7, F8, F11, F16 are all correct as written** (verified against live
  source). F8 needs one wording correction the verification already made:
  the verifier-error fail-open IS warn-logged; only the unset-secret path is
  truly silent.
- **F4 is correct as a leak, but its recommended uniform rejection is the
  inferior option and is REJECTED**: it would lock out existing verified
  users whose address is on a blocked domain. D-14 (ii) answers **A** -
  accept the differential response with an explanatory comment.
- **F11's expiry/GC half is REJECTED by D-14 (iv)**: sessions must not
  expire. Only revoke-all is in scope.
- **Re-derivation of routed-in item (a), reported honestly:** the web
  email-change chain **already exists and already enforces
  re-verification**. `add_email_address` inserts unverified and mails a
  code; `confirm_email_address` verifies it; `make_email_address_active`
  refuses with `SetPrimaryOutcome::Unverified` unless the address is
  verified; `email/sweep.rs::spawn_unverified_email_sweep` expires unverified
  rows after `UNVERIFIED_EMAIL_EXPIRY` (24h); `settings.rs::EmailSection`
  wires all four server fns into the UI. WP-56 removed only the *email-side*
  verbs. **So (a) requires no new UI or new flow** - it requires the
  invariant to be pinned by a regression test, and it requires F2 so a
  squatted address cannot block the real owner. A confirmation *link*
  instead of a 6-digit code is a cosmetic variation on an already-compliant
  flow and is a non-goal here; flag it to the Lead if the user wants it.

## 3. Required end state

`rust/web/src/auth/server.rs`:

- `confirm_login_inner`, pending branch: the presented code has already been
  proved valid by `validate_confirmation_code`, so instead of returning
  `invalid()`, DELETE the unverified `user_emails` row for that address
  inside the existing transaction and fall through to the normal new-user
  signup path. Only rows with `verified_at IS NULL` may be deleted; a
  verified row is already handled by the branch above.
- `login`, blocked-domain branch: unchanged behaviour, plus a comment
  recording that the differential response is a deliberate D-14 (ii)
  decision and that uniform rejection would lock out existing verified users
  on blocked domains.
- `request_confirmation_code` / `send_login_email`: `send_login_email`
  returns `Result<(), ()>` (or a bool); on a Resend error
  `request_confirmation_code` returns
  `LoginResponse { success: false, message: "Could not send the login email,
  please try again." }` instead of `generic_success`. Rolling back the send
  accounting is a non-goal.
- `verify_turnstile_token`: the `Err(e)` arm returns `false`, warn-logs, and
  increments a `turnstile_verify_error_total` counter
  (`axum_prometheus::metrics::counter!`, as used elsewhere in the file). The
  empty-secret dev carve-out stays as-is; it is gated by the startup check
  below. Keep WP-34's `&reqwest::Client` parameter.
- New server fn `logout_everywhere` (`#[server(LogoutEverywhere, "/api")]`):
  extract the session, resolve the user via `get_user_from_session`, call the
  new `db` helper to delete ALL `user_auth_tokens` for that user, then
  `clear_user_session` + `session.flush()`. Returns `Result<bool,
  ServerFnError>`.
- Both raw `DELETE FROM login_confirmations` statements are replaced by the
  new `db` helpers below.
- `make_email_address_active`: fetch `SWITCH_DIGEST_CAP + 1` games from
  `find_active_turn_games`, then `cap_digest(games, SWITCH_DIGEST_CAP)` for
  sending; when the fetch returned more than the cap, `tracing::info!` and
  increment a `switch_digest_capped_total` counter. Do not change the server
  fn's signature.

`rust/web/src/db.rs` (WP-41 must have landed; preserve each item's existing
`#[cfg(feature = "ssr")]` attribute exactly):

- Add `delete_login_confirmation(pool: &PgPool, email: &str) -> Result<()>`
  and `delete_login_confirmation_tx(tx, email) -> Result<()>` (the `_tx`
  suffix matches `insert_game_logs_tx` / `find_open_restart_proposal_tx`).
- Add `invalidate_all_auth_tokens(pool: &PgPool, user_id: Uuid) ->
  Result<u64>` = `DELETE FROM user_auth_tokens WHERE user_id = $1`.
- The existing db.rs test asserting a 40-day-old token still validates MUST
  remain passing and unmodified - no token expiry.

`rust/web/src/crypto.rs` + `rust/web/src/main.rs`:

- `load_key` no longer falls back silently. It returns the default key only
  when `ALLOW_INSECURE_DEFAULT_KEY=true`; otherwise a missing
  `DATABASE_ENCRYPTION_KEY` is an error variant (add one, e.g.
  `CryptoError::MissingKey`). Keep WP-36's `Zeroizing` return type.
- `main`: replace the `using_default_key()` warn block with an eager
  `load_key()` call that `.expect(...)`s (startup panic is acceptable per
  project rules), so both a missing key and a malformed key fail fast. Retain
  a warn when the insecure default IS explicitly allowed.
- `main`: also refuse to start when `TURNSTILE_SECRET_KEY` is unset/empty
  unless `ALLOW_INSECURE_DEFAULT_KEY=true` (reuse the one dev-mode flag
  rather than adding a second).
- Deployment: `ALLOW_INSECURE_DEFAULT_KEY=true` must be added to the dev
  overlay / `.env.template` only, never to `k8s/base` or `k8s/prod` (base is
  consumed by the dev overlay - see WP-36's analysis). Never apply anything
  to prod; verify with `kubectl kustomize <overlay>` output only.

`rust/web/src/settings.rs`: add a "Log out everywhere" button in
`EmailSection`'s vicinity (or the account section) wired to
`logout_everywhere` via an `Action`, following the existing `Action` +
`Effect` + success/error signal pattern in that file.

## 4. Non-goals

- Any session or auth-token expiry, or a token GC sweep (D-14 (iv)).
- Uniform blocked-domain rejection (rejected above).
- Email normalisation (ws F9), the confirm-attempt race (F1) and the windowed
  send cap (F6) - WP-34.
- `zeroize`, AAD, the `Secure` cookie flag, rustls provider, WS shutdown -
  WP-36.
- The db.rs module split (ws F42) and any db.rs refactor beyond the three new
  helpers.
- The `bump` email reply's cap disclosure (wfe finding) - WP-59.
- Replacing the 6-digit confirmation code with a confirmation link.
- A per-account cap on pending addresses (expiry already covers the DoS).

## 5. Regression test cases

`#[cfg(test)] mod tests` at the bottom of `rust/web/src/auth/server.rs`
(`#[sqlx::test]`, existing `with_pool_context` / `seed_confirmation`
helpers); db.rs helpers get tests in db.rs's own test module; run with
`--features ssr`. CODING.md makes tests mandatory for `auth/` and `db.rs`.

- **F2 (the key one).** Extend or replace
  `confirm_login_rejects_pending_unverified_address`: user A adds
  `victim@x.com` unverified; the real owner then confirms a valid code for
  `victim@x.com` -> `Ok`, a NEW user exists owning a verified `victim@x.com`
  row, and A no longer has that address. Add the negative: the same flow when
  the row is **verified** on A's account must still resolve to A, not create
  a user (the existing
  `confirm_login_resolves_non_primary_verified_address` covers this - keep it
  green).
- **(a) invariant.** `make_email_address_active` on an unverified address ->
  `Err` naming verification, and the primary is unchanged. This is the
  email-change re-verification regression test.
- **F4.** Blocked domain + no verified row -> `success: false` with the
  domain message; blocked domain + existing verified row -> `success: true`.
  Pins the deliberate asymmetry so nobody "fixes" it later.
- **F7.** Not unit-testable without a Resend double: assert instead that
  `send_login_email`'s signature is fallible and that
  `request_confirmation_code` maps a failure to `success: false` (drive it
  through a stub/`Option<Resend>` variant if the harness permits; otherwise
  state the gap in a comment).
- **F8.** `verify_turnstile_token` with a non-empty secret and an
  unreachable endpoint -> `false`. Empty secret -> `true` (dev carve-out).
- **F11.** Two logins for the same user (two tokens) -> `logout_everywhere`
  deletes both; `validate_session_token` on each returns `false`. Separately
  assert that a 40-day-old token STILL validates (no expiry).
- **(b).** Both confirm paths still delete the `login_confirmations` row
  (existing tests should already cover this - confirm they stay green).
- **(c).** `make_email_address_active` with `SWITCH_DIGEST_CAP + 3`
  outstanding turns -> exactly `SWITCH_DIGEST_CAP` digests attempted.

## 6. Riders

| finding | file | one-line fix | test needed |
|---|---|---|---|
| F4 | `auth/server.rs` `login` | keep the differential response, add a D-14 (ii) comment explaining why uniform rejection is worse | Y |
| F7 | `auth/server.rs` `send_login_email` / `request_confirmation_code` | make the send fallible and surface `success: false` on Resend error | Y |
| F8 | `auth/server.rs` `verify_turnstile_token` | `Err` arm returns `false` + metric; startup check for an unset secret | Y |
| F11 | `auth/server.rs`, `db.rs`, `settings.rs` | `invalidate_all_auth_tokens` + `logout_everywhere` server fn + UI button; NO expiry | Y |
| (b) | `auth/server.rs`, `db.rs` | replace both raw DELETEs with `delete_login_confirmation{,_tx}` | N |
| (c) | `auth/server.rs` `make_email_address_active` | fetch `SWITCH_DIGEST_CAP + 1`, cap to `SWITCH_DIGEST_CAP`, log/metric when capped | Y |
