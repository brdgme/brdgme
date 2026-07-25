# Raw findings: web auth + crypto review

Reviewer: Worker (auth/crypto). Snapshot: `/home/beefsack/Development/brdgme-review-snapshot`.
Read-only review; no code was modified, no builds/tests run.

Files in scope (all read IN FULL, every line):

- `rust/web/src/auth/server.rs` (1,530 lines, incl. test module)
- `rust/web/src/auth/session.rs` (94 lines)
- `rust/web/src/auth/mod.rs` (6 lines)
- `rust/web/src/crypto.rs` (69 lines)

Cross-checked read-only for verification (not in scope, not reviewed for
findings of their own): `migrations/005_login_confirmations.sql`,
`migrations/001_initial_schema.sql` (user_emails/user_auth_tokens DDL),
`src/main.rs:1-60` (default-key warning), `src/router.rs:160-184`
(rate-limit posture), `src/admin.rs` (crypto callers, grep only).

---

## Pass 1: `src/auth/server.rs`

### Concurrent confirm requests bypass the per-code attempt cap (brute-force amplification)
- severity: critical
- category: correctness
- location: web/src/auth/server.rs:354-390 (`validate_confirmation_code`), constants at :38
- finding: The attempt-cap enforcement is check-then-act across separate,
  non-locked statements: (1) `SELECT` the row, (2) in-Rust check
  `attempts >= CONFIRM_MAX_ATTEMPTS_PER_CODE` (10), (3) compare code,
  (4) only on mismatch, `UPDATE ... SET attempts = attempts + 1`. Any number
  of concurrent requests can all perform step 1-3 before any step-4 UPDATE
  commits, so each gets an independent guess against the 6-digit code
  (1e6 keyspace, 1-hour validity window). Firing e.g. 100k parallel
  confirm requests against one email yields ~10% success; the "10 attempts
  per code" cap is effectively unbounded under concurrency. The only in-app
  mitigations are the (concurrency-bypassable) cap itself and per-IP rate
  limiting, which per `router.rs:163-171` exists only at the Cloudflare
  edge - direct-to-LB traffic bypasses it. Successful guess = full account
  login (or hijack of a pending address confirm).
- recommendation: Make validation atomic at the row level. E.g. one
  statement: `UPDATE login_confirmations SET attempts = attempts + 1
  WHERE email = $1 AND created_at > NOW() - INTERVAL '1 hour'
  RETURNING code, attempts`, then compare the returned `code` to `token`
  and check the returned `attempts` against the cap. The row lock taken by
  the UPDATE serializes concurrent attempts so the cap actually holds;
  successful confirms can then delete the row as today (optionally only
  bump attempts on mismatch via a second short statement inside a
  transaction with `SELECT ... FOR UPDATE`).

### Email-squatting DoS: unverified `add_email_address` permanently blocks the real owner's signup
- severity: major
- category: correctness
- location: web/src/auth/server.rs:789-831 (`add_email_address`), :428-436 (pending branch in `confirm_login_inner`)
- finding: Any logged-in user can attach ANY address to their account as an
  unverified `user_emails` row (`insert_unverified_email`, :820) - no proof
  of control needed to create the row. Later, when the real owner of that
  mailbox tries to sign up, `login()` happily sends them a valid code, but
  `confirm_login_inner` finds the pending unverified row and returns
  `Err("Invalid or expired token")` (:434-436) even though the code is
  correct. The confirmation code is delivered to the address itself, so
  presenting it *proves mailbox control* - yet the flow treats the
  squatter's unverified row as authoritative. Result: victim is locked out
  of registering with their own address indefinitely (until the squatter
  removes it), with a misleading error message. Also note there is no cap
  on how many addresses one account can squat (each triggers an outbound
  email, bounded only by the 50/day global cap).
- recommendation: Honor the code as proof of ownership: in the pending
  branch of `confirm_login_inner`, when the code is valid for that email,
  delete (or reassign) the unverified `user_emails` row and proceed with
  signup. Also consider a per-account cap on pending addresses and/or
  expiring unverified rows.

### `add_email_address` discards the send-cap refusal and reports success
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:828-830
- finding: `request_confirmation_code` returns `Ok(LoginResponse { success:
  false, message: "Logins are temporarily limited..." })` when the global
  50/day cap is hit (:193-202) - an `Ok` value, not an `Err`.
  `add_email_address` calls it with `?` and discards the `LoginResponse`,
  so a global-cap refusal still yields `Ok(())` to the client. The
  unverified address row was already inserted (:820), the UI reports
  success, but no confirmation email was (or will be) sent - the user is
  stuck with a pending address they cannot confirm until they somehow
  trigger a resend.
- recommendation: Check the returned `LoginResponse.success` in
  `add_email_address` and propagate the refusal as a `ServerFnError`
  carrying the message (or change `request_confirmation_code` to return a
  richer enum).

### Blocked-domain check leaks account existence (email enumeration)
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:293-308
- finding: `login()` returns a distinctive error ("This email domain is not
  supported") for blocked domains ONLY when no verified `user_emails` row
  exists for the address; for a registered (verified) address on a blocked
  domain it proceeds and sends the code. The differential response lets an
  unauthenticated caller enumerate which addresses on blocked domains are
  registered - a carve-out in the otherwise uniform "Login email sent"
  response that the rest of the flow (and the tests) works hard to keep
  indistinguishable. Narrow surface (only blocked/disposable domains),
  hence minor rather than major.
- recommendation: Return the generic success for blocked domains too and
  simply suppress the send for unverified ones (mirroring the
  cooldown/cap suppression pattern), or reject uniformly regardless of
  registration state.

### No session ID rotation on login (session fixation hygiene)
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:315-332 (`confirm_login`)
- finding: After validating the code, the user is written into the EXISTING
  tower-sessions session (`set_user_session`); the session ID that existed
  pre-authentication is kept post-authentication. tower-sessions generates
  the ID itself, so classic fixation needs a cookie-planting primitive
  (e.g. cookie tossing from a sibling subdomain), but rotating the session
  ID on privilege change is standard practice and tower-sessions provides
  `Session::cycle_id()` for exactly this.
- recommendation: Call `session.cycle_id().await` (and map the error via
  `internal`) before `set_user_session` in `confirm_login`.

### Transient DB error in `get_current_user` clears the session (mass-logout on a blip)
- severity: minor
- category: quality
- location: web/src/auth/server.rs:512-524
- finding: `validate_session_token(...).await.unwrap_or(false)` collapses a
  real `sqlx::Error` (pool exhaustion, network blip, restart failover) into
  "token invalid", and the `else` branch then CLEARS the user's session.
  A transient DB error during a traffic burst would log out every active
  user whose request hits it (their `user_auth_tokens` rows survive, but
  they must re-login). Fail-closed on validation is right; clearing local
  session state on an *error* (as opposed to a definitive "row not found")
  is not.
- recommendation: Distinguish the two outcomes: propagate DB errors with
  `?`/`internal(...)` (a 500 to the client, session untouched), and only
  clear the session when the token is definitively absent.

### Global 24h send cap over-counts historical sends
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:185-192 (SUM query), :204-223 (upsert never resets `sent_count`)
- finding: The global cap computes `SUM(sent_count) WHERE last_sent_at >
  NOW() - INTERVAL '24 hours'`. But `sent_count` is cumulative for the
  lifetime of the row and is never reset - the upsert increments it even
  when the code rotates after expiry (test
  `login_expired_row_gets_fresh_code_and_attempts_reset` asserts 3 -> 4).
  So a row whose LATEST send is inside the window contributes ALL its
  historical sends (possibly days old) to today's quota. A handful of
  frequently-retried addresses could accumulate large counts and push the
  platform over the 50/day cap, refusing legitimate logins with
  "temporarily limited".
- recommendation: Count only sends within the window: e.g. reset
  `sent_count` together with `created_at` on code rotation and keep the
  existing GC, or keep a separate windowed counter/append-only sends log.
  (As a bonus this would let `sent_count` semantics match its name.)

### Resend API failure is silent, consumes send quota, and can lock a user out for the window
- severity: minor
- category: quality
- location: web/src/auth/server.rs:79-104 (`send_login_email`), :225-231 (call site)
- finding: The code row + `sent_count` are committed BEFORE the email is
  sent, and a Resend failure is only logged (`tracing::error!`), with
  `login()` still returning "Login email sent". During a Resend outage a
  user can burn all 5 per-email sends on silent failures within the hour;
  the per-email cap then suppresses further attempts while STILL reporting
  success - the user has no path to a working code until the code expires.
- recommendation: Surface a retryable failure to the client when the Resend
  call errors (the code is already committed; a retry within cooldown could
  resend the same code without counting against the cap), or decrement /
  don't count sends that failed at the provider.

### Turnstile verification fails open on verifier/network errors (and silently disables when unset)
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:235-256, call site :265-271
- finding: If the POST to challenges.cloudflare.com errors (DNS, TLS,
  Cloudflare outage), `verify_turnstile_token` returns `true`
  ("fail-open", warn-logged) - CAPTCHA protection silently disappears
  exactly when an attacker could plausibly be hammering the endpoint.
  Additionally, an unset/empty `TURNSTILE_SECRET_KEY` disables verification
  entirely with no startup warning (intended for dev, but a prod
  misconfiguration would be silent). Fail-open is arguably a deliberate
  availability choice; flagging for an explicit decision + observability.
- recommendation: Emit a metric on verifier failure (there is
  `axum_prometheus` in use already) and consider fail-closed for the
  `login` endpoint, or at least a startup warn when the secret is unset
  outside dev.

### 6-digit code: minor modulo bias; comparison not timing-safe; codes stored plaintext
- severity: nit
- category: quality
- location: web/src/auth/server.rs:204 (generation), :378 (compare), migrations/005_login_confirmations.sql:12 (plaintext `code CHAR(6)`)
- finding: Three small notes bundled: (1) `rand::random::<u32>() %
  1_000_000` has a tiny modulo bias (~0.02%; ThreadRng is a CSPRNG, so
  entropy is otherwise fine) - `gen_range(0..1_000_000)` is the idiomatic
  form; (2) `confirmation.code != token` is not constant-time - irrelevant
  across a network with an attempt cap, noted only for completeness;
  (3) codes are stored plaintext in the DB - with a 1e6 keyspace hashing
  adds little (a DB reader can brute-force the hash instantly), but rows
  can live longer than the 1-hour validity (GC is opportunistic, on
  login), so plaintext codes for still-valid rows sit in dumps/backups.
- recommendation: Use `rand::rng().random_range(0..1_000_000)`; ignore (2);
  optionally hash codes if backup exposure is a concern (low value).

### Email addresses are not normalized (case / whitespace) anywhere in the auth flow
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:264-311 (`login`), :789-831 (`add_email_address`), :315+ (`confirm_login`)
- finding: The raw client-supplied string is used as the
  `login_confirmations` PK and the `user_emails.email` value throughout -
  no trim, no lowercase. The DB UNIQUE constraint on `user_emails.email`
  is case-sensitive Postgres `text` (migrations/001:96,275), so
  `Alice@x.com` vs `alice@x.com` (or trailing whitespace) create distinct
  confirmation rows and DISTINCT ACCOUNTS for the same mailbox; the
  blocked-domain check lowercases only the domain. Uncertain: the Leptos
  client may normalize before calling - not verifiable within these files.
- recommendation: `trim()` + lowercase (or a documented normalization
  policy) server-side at the entry points; longer-term consider `citext`.

### `confirm_login` does not validate email/token shape
- severity: nit
- category: quality
- location: web/src/auth/server.rs:314-339
- finding: Unlike `login()`, `confirm_login` applies no format checks
  (empty email, oversized strings, non-digit token) before hitting the DB.
  Harmless - a lookup miss returns the uniform invalid error - but a cheap
  guard would match the endpoint's sibling and keep junk out of queries.
- recommendation: Early-return the same "Invalid or expired token" error
  when `token.len() != 6 || !token.bytes().all(|b| b.is_ascii_digit())` or
  the email is obviously malformed.

### New `reqwest::Client` built per Turnstile verification
- severity: nit
- category: quality
- location: web/src/auth/server.rs:239
- finding: `reqwest::Client::new()` per call discards connection pooling
  and TLS session reuse; the app already provides a shared
  `reqwest::Client` via context (`expect_context::<reqwest::Client>()`,
  used at :883).
- recommendation: Reuse the context-provided client.

---

## Pass 2: `src/auth/session.rs`

### `Secure` cookie attribute defaults to off unless `SECURE_COOKIE=true`
- severity: minor
- category: correctness
- location: web/src/auth/session.rs:32-38
- finding: `with_secure(secure)` is driven by an opt-IN env var defaulting
  to `false`. A production deploy that forgets `SECURE_COOKIE=true` serves
  session cookies without the `Secure` attribute - the browser will then
  send them over any plaintext HTTP connection to the host. Safe default
  should be secure-on with an explicit opt-out for local dev. Uncertain:
  the k8s/infra manifests may set it (not checked; outside scope) - but
  the code default is the wrong way around regardless.
- recommendation: Default to `true`, e.g. `SECURE_COOKIE != "false"`, or
  gate the insecure mode on `debug_assertions`.

### Session record is not destroyed server-side on logout; token-invalidation error is swallowed
- severity: nit
- category: quality
- location: web/src/auth/session.rs:67-70 (`clear_user_session`), web/src/auth/server.rs:531-547 (`logout`)
- finding: `logout` removes the `user` key from the session and deletes the
  `user_auth_tokens` row - which is sufficient to actually invalidate
  access, since `get_current_user` re-validates the token against the DB
  on every call. Two residuals: (1) the `invalidate_auth_token` error is
  discarded (`let _ =`, server.rs:539) - if the DELETE fails, logout
  reports `Ok(true)` and clears the local session, but the token row
  survives; impact is limited because each confirm mints a fresh token per
  session, but the failure is invisible; (2) the tower-sessions record
  itself (and its cookie) persists until the 30-day inactivity expiry -
  `session.flush()`/`delete` would be tidier.
- recommendation: Log (or propagate) the invalidation failure; optionally
  `session.flush().await` after clearing.

### `user_auth_tokens` never expire and are never garbage-collected
- severity: nit
- category: quality
- location: web/src/auth/session.rs:73-94, migrations/001_initial_schema.sql:100-105
- finding: Auth tokens are created per login and deleted only on explicit
  logout. Abandoned sessions leave valid token rows indefinitely; in
  practice access is gated by the tower-sessions row expiring after 30
  days of inactivity (the cookie + session record, not the token, is what
  ages out), so this is table hygiene rather than an exposure - but a
  stolen long-lived session cookie paired with its token stays valid for
  the full inactivity window regardless of password-equivalent events
  (there is no "revoke all sessions" path for a user).
- recommendation: Optional: add `expires_at` to tokens (new migration) or
  a periodic GC; consider a "log out everywhere" = delete all user tokens.

### Clean parts of session.rs
- `SessionUser` shape, `set_user_session`/`get_user_from_session`/
  `clear_user_session` are thin, correct wrappers; session fixation /
  staleness concerns are flagged against their call sites in server.rs
  instead. `SameSite::Lax` and 30-day inactivity expiry are reasonable.
  The `store.migrate().await.expect(...)` at :28-31 is startup code
  (called from main), so the panic is acceptable per project rules.

---

## Pass 3: `src/auth/mod.rs`

Clean. Two `pub mod` + two glob re-exports (6 lines). `blocked_domains` is
out of scope. No findings.

---

## Pass 4: `src/crypto.rs`

### Hardcoded public fallback key silently used when `DATABASE_ENCRYPTION_KEY` is unset
- severity: major
- category: correctness
- location: web/src/crypto.rs:42-57 (`default_key`, `load_key`), web/src/main.rs:25-29 (warning only)
- finding: When the env var is missing, `load_key()` returns
  `default_key()` - a hardcoded, in-repo, publicly known key
  ("brdgme-dev-key-not-for-prod!!!"). Startup only logs a `tracing::warn!`
  (main.rs:25-29) and continues serving. If production (or any real
  environment) ever runs with the variable unset/misnamed, ALL stored LLM
  provider API keys are encrypted under a key anyone can read from the
  repo, with no runtime failure to signal it - exactly the failure mode
  encryption-at-rest is meant to prevent, and worse than plaintext because
  it looks protected. Warnings in logs are routinely missed; the crypto
  callers in `admin.rs` have no way to tell which key was used. Note
  `using_default_key()` only checks var PRESENCE - a set-but-invalid value
  errors at first use instead (inconsistent posture).
- recommendation: Fail closed: refuse to start when the key is unset
  outside an explicit dev mode (a startup panic in main is acceptable per
  project rules), e.g. require `ALLOW_INSECURE_DEFAULT_KEY=true` for the
  fallback, or compile `default_key()` only under `debug_assertions`. Also
  validate the key at startup (call `load_key()` once in main and store
  it) so a malformed value fails fast instead of on the first admin
  request.

### No key/ciphertext context binding (AAD) and no key zeroization
- severity: nit
- category: quality
- location: web/src/crypto.rs:17-40 (encrypt/decrypt signatures)
- finding: (1) No AAD is used, so ciphertexts are not bound to their row -
  an attacker with DB write access could swap encrypted API keys between
  provider rows and the app would decrypt them happily (marginal threat
  model, standard hardening). (2) Key material and decrypted API keys
  linger in memory (`[u8; 32]`, `Vec<u8>`) without `zeroize`. Both are
  defense-in-depth polish for this threat model.
- recommendation: Optional: pass the provider row id as AAD; add `zeroize`
  for the key and decrypted buffers if the project wants the hygiene.

### Clean parts of crypto.rs
- Nonce generation is correct: fresh random 96-bit nonce per message via
  `getrandom` (crypto.rs:65-68), prepended to the output - safe for any
  realistic message count (random-nonce collision bound ~2^32 messages
  under one key); no counter/reuse hazards.
- `decrypt` length-checks before `split_at` (:31-33), and `Nonce::from_slice`
  is always given exactly 12 bytes on both paths, so it cannot panic.
- AEAD errors are mapped to opaque variants; no internals leak.
- `load_key` hex/length validation is correct; no unwrap/panic in any
  function (request-path safe).

---

## Coverage statement

Every line of all four in-scope files was read in full (server.rs 1-1530
including the test module, session.rs 1-94, mod.rs 1-6, crypto.rs 1-69).
Verification-only reads outside scope: migrations 001/005 (schema facts),
main.rs:1-60 (default-key handling), router.rs:160-184 (rate-limit
posture), admin.rs (crypto call sites, grep-level only).

### Explicitly checked and found CLEAN / non-issues
- No `.unwrap()`/`.expect()`/`panic!` in any request path in scope (test
  module unwraps at server.rs:953+ are tests; the session-store migrate
  `expect` is startup).
- All SQL is parameterized (`query!`/`query_as!`/bound `query`) - no
  injection surface. All queries use the bound-parameter idiom
  consistently.
- Error handling follows the project convention: opaque `internal(...)`
  to clients, `Ok(Some(msg))`/data for expected rejections; confirm-flow
  errors are uniform ("Invalid or expired token") across
  unknown-email/expired/capped/wrong-code - good anti-enumeration there
  (the blocked-domain carve-out is the exception, flagged above).
- Login code is sent only to the address itself; the email body embeds
  only the (numeric) token - no HTML injection via user input.
- The advisory-lock serialization of the send-cap check-and-bump is sound
  and well-tested (two concurrency tests); the lock key is an arbitrary
  unique constant as documented.
- Logout invalidation design (DB-backed token check on every
  `get_current_user`) makes session invalidation immediate and
  replica-safe.
- Test coverage of the login/confirm state machine is genuinely strong
  (cooldown, caps, expiry rotation, concurrency, wrong-email scoping,
  pending-address rejection, repeat-confirm consumption).

### Uncertainties flagged for the Lead
- Attempt-cap concurrency bypass (critical finding): severity assumes
  confirm traffic can reach the app without effective edge throttling
  (direct-to-LB bypass is acknowledged in router.rs comments). If
  Cloudflare rules are verified strict, this could be downgraded to major.
- Email normalization finding: client-side normalization, if any, lives in
  the Leptos components (out of scope) - not verified.
- `SECURE_COOKIE` finding: infra manifests (out of scope) may set it in
  prod; the code-level default is still backwards.
- `add_email_address` has no visible resend path for pending addresses
  within these files; if one exists elsewhere, the
  discarded-`LoginResponse` finding's impact shrinks (the bug - reporting
  success on refusal - stands regardless).
