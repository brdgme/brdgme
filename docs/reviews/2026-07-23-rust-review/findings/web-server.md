# Findings: web-server (web crate server infrastructure)

Scope: `main.rs`, `router.rs`, `state.rs`, `config.rs`, `db.rs`, `nats.rs`,
`error.rs`, `crypto.rs`, `admin.rs`, `auth/`, `websocket.rs`,
`websocket_client.rs`, `bin/import_game.rs`, plus manifest-level `Cargo.toml`
ssr deps. Snapshot `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`; ~11.3k
effective reviewable LOC (19.5k total minus the 8.2k-line vendored
`blocked_domains.rs` data table). Raw worker dumps and the review log are in
`findings/raw/web-server-*.md`; every finding below was verified by the Lead
against the snapshot.

## auth

### Concurrent confirm requests bypass the per-code attempt cap
- severity: critical
- category: correctness
- location: web/src/auth/server.rs:354-390
- finding: `validate_confirmation_code` enforces the 10-attempt cap on the
  6-digit login code as check-then-act across separate, unlocked statements:
  `SELECT` the row, compare `attempts`/code in Rust, and only on mismatch
  `UPDATE ... SET attempts = attempts + 1`. Any number of concurrent requests
  can all pass the pre-checks before any increment commits, so the cap is
  effectively unbounded under parallelism against a 1e6 keyspace with a 1-hour
  validity window. Per-IP rate limiting exists only at the Cloudflare edge
  (router.rs comments acknowledge direct-to-LB bypass), and a successful guess
  is a full account login. Verified against the snapshot.
- recommendation: Make validation atomic at the row level, e.g.
  `UPDATE login_confirmations SET attempts = attempts + 1 WHERE email = $1
  AND created_at > NOW() - INTERVAL '1 hour' RETURNING code, attempts`, then
  compare the returned code and cap in Rust — the row lock serializes
  concurrent attempts so the cap actually holds. (Severity assumes confirm
  traffic can reach the app without effective edge throttling; if Cloudflare
  rules are verified strict this could be downgraded to major.)

### Email-squatting DoS: unverified `add_email_address` blocks the real owner's signup
- severity: major
- category: correctness
- location: web/src/auth/server.rs:789-831, 428-436
- finding: Any logged-in user can attach ANY address to their account as an
  unverified `user_emails` row — no proof of control needed to create the
  row. When the real mailbox owner later signs up, `login()` sends them a
  valid code, but `confirm_login_inner` finds the squatter's pending row and
  returns "Invalid or expired token" (:434-436) even though the presented
  code proves mailbox control. The victim is locked out of registering with
  their own address indefinitely, with a misleading error. There is also no
  cap on how many addresses one account can squat.
- recommendation: Honor the code as proof of ownership: in the pending branch
  of `confirm_login_inner`, when the code is valid for that email, delete (or
  reassign) the unverified row and proceed with signup. Consider a
  per-account cap on pending addresses and expiry of unverified rows.

### No session ID rotation on login (session fixation hygiene)
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:315-332
- finding: After code validation the user is written into the EXISTING
  tower-sessions session; the pre-authentication session ID is kept
  post-authentication. tower-sessions generates the ID itself so classic
  fixation needs a cookie-planting primitive, but rotating on privilege
  change is standard practice and `Session::cycle_id()` exists for it.
- recommendation: Call `session.cycle_id().await` (map the error via
  `internal`) before `set_user_session` in `confirm_login`.

### Blocked-domain check leaks account existence
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:293-308
- finding: `login()` returns a distinctive "This email domain is not
  supported" error for blocked domains ONLY when no verified `user_emails`
  row exists; a registered address on a blocked domain proceeds and gets a
  code. The differential response lets an unauthenticated caller enumerate
  registered addresses on blocked/disposable domains — a carve-out in the
  otherwise deliberately uniform "Login email sent" response.
- recommendation: Return the generic success for blocked domains too and
  suppress the send for unverified ones (mirroring the cooldown/cap
  suppression pattern), or reject uniformly regardless of registration state.

### Transient DB error in `get_current_user` clears the session
- severity: minor
- category: quality
- location: web/src/auth/server.rs:512-524
- finding: `validate_session_token(...).await.unwrap_or(false)` collapses a
  real `sqlx::Error` (pool exhaustion, network blip) into "token invalid",
  and the `else` branch then CLEARS the user's session. A transient DB error
  during a traffic burst mass-logs-out every active user whose request hits
  it. Failing closed on validation is right; clearing local session state on
  an *error* (vs a definitive row-not-found) is not.
- recommendation: Propagate DB errors with `?`/`internal(...)` (500, session
  untouched); only clear the session when the token is definitively absent.

### Global 24h send cap over-counts historical sends
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:185-223
- finding: The global cap computes `SUM(sent_count) WHERE last_sent_at >
  NOW() - INTERVAL '24 hours'`, but `sent_count` is cumulative for the row's
  lifetime and never reset (code rotation after expiry increments it). A row
  whose latest send is inside the window contributes its entire history to
  today's quota, so a few frequently-retried addresses can push the platform
  over the 50/day cap and refuse legitimate logins.
- recommendation: Count only sends within the window: reset `sent_count` with
  `created_at` on code rotation (matching its name), or keep a separate
  windowed counter/append-only sends log.

### Resend API failure is silent, consumes quota, and can lock a user out for the window
- severity: minor
- category: quality
- location: web/src/auth/server.rs:79-104, 225-231
- finding: The code row and `sent_count` are committed BEFORE the email is
  sent; a Resend failure is only `tracing::error!`-logged and `login()` still
  returns "Login email sent". During a Resend outage a user can burn all 5
  per-email sends on silent failures, after which the per-email cap
  suppresses further attempts while still reporting success — no path to a
  working code until expiry.
- recommendation: Surface a retryable failure to the client when the Resend
  call errors, or don't count provider-failed sends against the cap.

### Turnstile verification fails open on verifier errors; unset secret silently disables it
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:235-256, 265-271
- finding: If the POST to challenges.cloudflare.com errors (DNS, TLS,
  Cloudflare outage), `verify_turnstile_token` returns `true` — CAPTCHA
  protection silently disappears exactly under attack conditions. An
  unset/empty `TURNSTILE_SECRET_KEY` also disables verification with no
  startup warning, so a prod misconfiguration would be silent. Fail-open may
  be a deliberate availability choice; it needs to be an explicit, observable
  one.
- recommendation: Emit a metric on verifier failure (axum_prometheus is
  already in use), warn at startup when the secret is unset outside dev, and
  decide explicitly whether `login` should fail closed.

### Email addresses are not normalized (case/whitespace) in the auth flow
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:264-311, 789-831
- finding: The raw client-supplied string is used as the
  `login_confirmations` PK and `user_emails.email` value throughout — no
  trim, no lowercase. The UNIQUE constraint on `user_emails.email` is
  case-sensitive Postgres text, so `Alice@x.com` vs `alice@x.com` (or
  trailing whitespace) create distinct confirmation rows and potentially
  distinct accounts for the same mailbox; the blocked-domain check lowercases
  only the domain. (Uncertain: the Leptos client may normalize before
  calling — not verifiable within this unit's scope.)
- recommendation: `trim()` + lowercase server-side at the entry points (or a
  documented normalization policy); longer-term consider `citext`.

### `add_email_address` discards the send-cap refusal and reports success
- severity: minor
- category: correctness
- location: web/src/auth/server.rs:828-830
- finding: `request_confirmation_code` returns
  `Ok(LoginResponse { success: false, ... })` (not `Err`) when the global
  50/day cap is hit; `add_email_address` calls it with `?` and discards the
  response, returning `Ok(())`. The unverified row was already inserted and
  the UI reports success, but no confirmation email was sent.
- recommendation: Check `LoginResponse.success` in `add_email_address` and
  propagate the refusal as a `ServerFnError` carrying the message.

### Auth tokens never expire DB-side and are never garbage-collected
- severity: minor
- category: correctness
- location: web/src/auth/session.rs:73-94, web/src/db.rs:5140
- finding: `user_auth_tokens` rows are created per login and deleted only on
  explicit logout; the db.rs test at :5140 deliberately asserts a 40-day-old
  token still validates. In practice access ages out via the tower-sessions
  cookie (30-day inactivity expiry), and the test documents this as accepted
  design — but there is no "revoke all sessions" path, and any caller
  treating `validate_session_token` as proof of a fresh session accepts
  stale tokens. Defense-in-depth gap, not an active exposure.
- recommendation: Optional: add a DB-side `created_at` window or periodic GC,
  and a "log out everywhere" = delete-all-user-tokens path. No change
  strictly required if the design is confirmed intentional.

### 6-digit code: modulo bias, non-constant-time compare, plaintext storage
- severity: nit
- category: quality
- location: web/src/auth/server.rs:204, 378
- finding: Three bundled notes: (1) `rand::random::<u32>() % 1_000_000` has a
  tiny modulo bias (~0.02%; ThreadRng is a CSPRNG so entropy is otherwise
  fine) — `random_range` is the idiomatic form; (2) the code comparison is
  not constant-time — irrelevant across a network with an attempt cap, noted
  for completeness; (3) codes are stored plaintext — with a 1e6 keyspace
  hashing adds little, but valid rows sit in dumps/backups.
- recommendation: Use `rand::rng().random_range(0..1_000_000)`; ignore (2);
  optionally hash codes if backup exposure is a concern (low value).

### `confirm_login` does not validate email/token shape
- severity: nit
- category: quality
- location: web/src/auth/server.rs:314-339
- finding: Unlike `login()`, `confirm_login` applies no format checks (empty
  email, oversized strings, non-digit token) before hitting the DB. Harmless
  — a lookup miss returns the uniform error — but a cheap guard matches its
  sibling endpoint and keeps junk out of queries.
- recommendation: Early-return "Invalid or expired token" when
  `token.len() != 6 || !token.bytes().all(|b| b.is_ascii_digit())` or the
  email is obviously malformed.

### New `reqwest::Client` built per Turnstile verification
- severity: nit
- category: quality
- location: web/src/auth/server.rs:239
- finding: `reqwest::Client::new()` per call discards connection pooling and
  TLS session reuse; the app already provides a shared client via context
  (used at :883).
- recommendation: Reuse `expect_context::<reqwest::Client>()`.

### Logout swallows token-invalidation error; session record not flushed
- severity: nit
- category: quality
- location: web/src/auth/server.rs:531-547, web/src/auth/session.rs:67-70
- finding: `logout` discards the `invalidate_auth_token` error (`let _ =`) —
  if the DELETE fails, logout reports `Ok(true)` but the token row survives
  invisibly (impact limited: each confirm mints a fresh token). The
  tower-sessions record also persists until the 30-day expiry;
  `session.flush()` would be tidier.
- recommendation: Log (or propagate) the invalidation failure; optionally
  `session.flush().await` after clearing.

Clean: `auth/mod.rs`; nonce generation and length-safe decrypt in crypto call
paths; no unwrap/panic in any request path; all SQL parameterized; uniform
confirm-flow errors (good anti-enumeration apart from the blocked-domain
carve-out); sound advisory-lock serialization of the send cap; DB-backed
logout invalidation; genuinely strong login/confirm state-machine tests.

## crypto

### Hardcoded public fallback key silently used when `DATABASE_ENCRYPTION_KEY` is unset
- severity: major
- category: correctness
- location: web/src/crypto.rs:42-57, web/src/main.rs:25-29
- finding: When the env var is missing, `load_key()` returns `default_key()`
  — a hardcoded, in-repo, publicly known key ("brdgme-dev-key-not-for-prod!!!
  "). Startup only logs a `tracing::warn!` and continues serving. If any real
  environment runs with the variable unset/misnamed, ALL stored LLM provider
  API keys are encrypted under a key anyone can read from the repo, with no
  runtime failure — worse than plaintext because it looks protected.
  `using_default_key()` also only checks var PRESENCE: a set-but-invalid
  value errors later at first use (inconsistent posture).
- recommendation: Fail closed: refuse to start when the key is unset outside
  an explicit dev mode (startup panic in main is acceptable per project
  rules), e.g. require `ALLOW_INSECURE_DEFAULT_KEY=true` for the fallback.
  Also validate the key at startup (call `load_key()` once in main) so a
  malformed value fails fast instead of on the first admin request.

### No AAD context binding and no key zeroization
- severity: nit
- category: quality
- location: web/src/crypto.rs:17-40
- finding: (1) No AAD, so ciphertexts aren't bound to their row — an attacker
  with DB write access could swap encrypted keys between provider rows
  (marginal threat model, standard hardening). (2) Key material and decrypted
  keys linger in memory without `zeroize`. Both are defense-in-depth polish.
- recommendation: Optional: pass the provider row id as AAD; add `zeroize`
  for key and plaintext buffers.

Clean: fresh random 96-bit nonce per message (safe for any realistic volume),
decrypt length-checks before `split_at`, no panic paths, opaque error
variants.

## admin

### `reorder_bots` is not atomic
- severity: major
- category: correctness
- location: web/src/admin.rs:184-197
- finding: N separate `UPDATE bots SET display_order ...` statements on the
  pool with no transaction. A failure mid-loop leaves display_order
  half-migrated (old and new orderings mixed) which the admin UI then
  re-fetches and renders. Interleaving with a concurrent `create_bot` (which
  computes `MAX(display_order)+1`) can also produce duplicate display_order
  values — there is no unique constraint in migration 013. Verified against
  the snapshot.
- recommendation: Wrap the loop in one transaction, or better, a single
  `UPDATE ... FROM (SELECT * FROM unnest($1::uuid[]) WITH ORDINALITY ...)`
  statement. A uniqueness constraint, if wanted, must be a NEW migration.

### `create_bot` display_order race (MAX+1 without lock)
- severity: minor
- category: correctness
- location: web/src/admin.rs:134-145
- finding: `COALESCE((SELECT MAX(display_order) + 1 FROM bots), 0)` is a
  read-modify-write; two concurrent creates can compute the same value.
  Low likelihood (admin-only) but real.
- recommendation: Serialize via the same transaction fix as reorder, or
  accept and document.

### API-key mask exposes the whole key for keys ≤ 4 chars; fabricated `sk-` prefix
- severity: minor
- category: quality
- location: web/src/admin.rs:228-236, 279-289
- finding: The mask is `format!("sk-...{last4}")`. For a key of ≤ 4 chars the
  "last 4" is the entire key, so `api_key_masked` round-trips the full
  plaintext secret to the client; for non-`sk-` keys the displayed prefix is
  misleading. Defeats the masking guarantee the tests assert for
  normal-length keys.
- recommendation: For short keys return a fixed mask like "(set)"; drop the
  hardcoded `sk-` prefix and show only `...{last4}`.

### No way to clear a provider API key once set
- severity: minor
- category: correctness
- location: web/src/admin.rs:301-341
- finding: `api_key: Option<String>` conflates "keep existing" (None) with
  removal — `None` skips the column, `Some("")` would store an empty key.
  No mass-assignment hole (columns enumerated), but "unset key" is
  unrepresentable on a public API surface.
- recommendation: Tri-state (enum or separate `clear_api_key: bool`) if
  clearing is wanted; otherwise document that keys can only be replaced.

### `test_provider` hardcodes model "gpt-4o-mini"
- severity: minor
- category: correctness
- location: web/src/admin.rs:496-501
- finding: The provider-level health check always sends
  `"model": "gpt-4o-mini"`; providers not serving that exact model id (most
  non-OpenAI endpoints) fail the test even when healthy — a false negative
  built into the admin's primary health check. The per-link test
  (`test_bot_provider`) does use the configured model.
- recommendation: Take the model as a parameter, or reuse one of the
  provider's configured `bot_providers.model` values with a fallback.

### Upstream test response body/headers returned to client uncapped
- severity: minor
- category: quality
- location: web/src/admin.rs:511-528, 599-615
- finding: `resp.text().await` reads the upstream body with no size limit and
  returns it (plus all headers) verbatim in the server fn response. A
  misbehaving admin-configured endpoint can return an arbitrarily large body
  within the 10s timeout → memory spike and huge serialized response; error
  bodies may embed sensitive detail.
- recommendation: Cap the read (first N KB) and trim the header list to a
  safe subset.

### Test result can be attributed to the wrong row (Action input/value race)
- severity: minor
- category: correctness
- location: web/src/admin.rs:1424-1435, 1760-1771
- finding: The completion Effect reads `test_action.input().get()` (latest
  *dispatched* input) together with `test_action.value().get()` (latest
  *completed* value). The UI only blocks re-dispatch for the same id, so
  tests for two different providers can overlap; if A resolves after B was
  dispatched, A's result is stored under B's id. (Uncertain: exact Leptos 0.8
  `Action::input()` semantics — tracks latest dispatch, not the dispatch that
  produced `value()`.)
- recommendation: Have the async block return `(Uuid, Result<…>)` and key
  results off `value()` only.

### No server-side validation of bot/provider inputs
- severity: minor
- category: quality
- location: web/src/admin.rs:637-666, 668-701, 760-781, 846-879
- finding: All constraints live in the HTML form (`min/max` on temperature,
  `required` on name/url/model). The server fns accept crafted values: empty
  names, out-of-range/non-finite temperature (Postgres REAL accepts 'NaN';
  whether the server-fn codec rejects non-finite floats at the boundary is
  unverified), unbounded `extra_body` JSONB and `prompt` length. Admin gating
  caps the blast radius, but there is no defense-in-depth.
- recommendation: Cheap server-side checks: non-empty trimmed name/url/model,
  `temperature.is_finite() && (0.0..=2.0).contains(&temperature)`, a length
  cap on `prompt`. A DB CHECK constraint would require a NEW migration.

### Admin page hard-fails entirely if any single provider key is undecryptable
- severity: minor
- category: quality
- location: web/src/admin.rs:218-247, 1019-1037
- finding: In `list_providers` one corrupt/undecryptable `api_key_encrypted`
  row (e.g. after a key rotation that missed a row) makes the whole listing
  `Err`, and `AdminPage` renders any resource error as a full-page failure —
  one bad row takes down the entire admin UI until fixed via direct DB
  access.
- recommendation: Degrade per-row: on decrypt failure render that provider
  with `api_key_masked: "(undecryptable)"` and log, instead of failing the
  list.

### Renaming/deleting a bot can strand in-flight games
- severity: minor
- category: correctness
- location: web/src/admin.rs:159-181, 200-207
- finding: Games reference bots by NAME (`game_bots.bot_name`), and the bot
  worker resolves config at turn time by `WHERE name = $1 AND enabled` —
  admin rename or delete of a bot changes/breaks resolution for existing
  games using it. (Uncertain: the worker's fallback path on `None` was not
  traced; impact depends on it.)
- recommendation: At minimum warn in the delete/rename flows; structurally,
  referencing bots by id from `game_bots` removes the coupling (schema change
  → NEW migration).

### Admin-gate boilerplate duplicated 15 times
- severity: minor
- category: simplicity
- location: web/src/admin.rs:618-976
- finding: Every server fn repeats the same 6-line authenticate +
  `is_user_admin` + reject block verbatim. Beyond noise, the risk is a future
  admin fn forgetting the check. Precedent exists: `friends::require_user()`.
- recommendation: Extract `async fn require_admin(pool: &PgPool) ->
  Result<User, ServerFnError>` and call it in one line per server fn.

### Mutations don't verify rows affected
- severity: minor
- category: quality
- location: web/src/admin.rs:159-207, 301-351, 437-469
- finding: All UPDATE/DELETE helpers `.execute()` and discard
  `rows_affected()`; updating or deleting a non-existent id returns `Ok(())`
  so the UI reports success for a no-op (e.g. stale row after another admin
  deleted it).
- recommendation: Check `.rows_affected() == 0` and return a "not found"
  error where the UX matters (updates at least); deletes can stay idempotent.

### `update_bot_provider` omits `updated_at = now()`
- severity: nit
- category: consistency
- location: web/src/admin.rs:446-458
- finding: Every other UPDATE in the file sets `updated_at = now()`;
  `update_bot_provider` does not. Note the `bots`/`llm_providers`/
  `bot_providers` tables (migration 013) have NO `update_updated_at` trigger
  (those exist only on migration-001 tables), so the manual set is required
  here and the column silently goes stale without it.
- recommendation: Add `updated_at = now()` to the UPDATE.

### AdminPage non-admin redirect matches on error-message text
- severity: nit
- category: quality
- location: web/src/admin.rs:1005-1012
- finding: `msg.contains("Admin access required")` decides the redirect; if
  the server wording changes, non-admins silently stop being redirected.
- recommendation: Centralize the message in a shared constant, or use a
  structured error variant if ServerFnError ever carries one.

### Action-value `.unwrap()`s in completion Effects
- severity: nit
- category: quality
- location: web/src/admin.rs:1089-1137, 1386-1435, 1722-1771
- finding: `if action.value().get().is_some() && !action.pending().get() {
  match action.value().get().unwrap() {...} }` appears 10 times. Safe in
  practice (single-threaded reactive graph, guarded), but copy-prone.
- recommendation: `if let Some(res) = action.value().get() { ... }`.

### Local `type BotProviderRow` shadows the public struct
- severity: nit
- category: quality
- location: web/src/admin.rs:538-544
- finding: A tuple type alias named `BotProviderRow` inside
  `test_bot_provider` shadows the file's public `BotProviderRow` struct
  (:53-65). Confusing when reading/refactoring.
- recommendation: Rename the local alias (e.g. `BotProviderTestRow`).

Clean: ALL 15 admin server fns are gated server-side with session-derived
user id and fail-closed `is_user_admin` — no privilege-escalation path; no
plaintext key exposure in any DTO/log/error path; SQL fully parameterized; no
panics/unwraps in server request paths; no mass-assignment; FK cascades make
deletes referentially safe; good sqlx::test coverage of the crypto-critical
paths.

## db

### `undo_game` does not clear rating state — a re-finished game keeps the voided result's ratings
- severity: major
- category: correctness
- location: web/src/db.rs:1407-1463, 1536-1680
- finding: `apply_rating_changes` runs when a game finishes with placings,
  mutates `game_type_users.rating`/`peak_rating`, and stamps
  `game_players.rating_change` as its idempotency token (guard at :1554).
  `undo_game` resets `is_finished`/`finished_at`/placings but does NOT clear
  `rating_change`/`rating_before` or rewind `game_type_users` (verified
  against the snapshot). If the same game later finishes again, the guard
  sees the old `rating_change` and skips re-rating entirely: players keep ELO
  from the voided result and the new outcome is never rated. `delete_game`
  documents "ratings are deliberately NOT rewound" for admin deletes, so a
  no-rewind policy exists — but silently NOT rating the *new* outcome looks
  unintended. UNCERTAIN: reachability depends on whether game-ending commands
  carry `can_undo = true` (game-engine dependent; undo is reachable via
  `game/server_fns.rs:784` and `email/commands.rs:966`). Needs a
  product/engine ruling.
- recommendation: If finished games can be undone: clear
  `rating_change`/`rating_before` in `undo_game` and either rewind
  `game_type_users` by the stored deltas or recompute on next finish. If they
  cannot, add a comment in `undo_game` stating why rating fields are left
  alone.

### Broad test-coverage gap: ~20 public DB functions have no test
- severity: major
- category: quality
- location: web/src/db.rs:2961 (whole test module, 2961-6380)
- finding: Cross-referencing every public function (lines 15-2959) against
  all test bodies, ~20 are never exercised, including `find_active_turn_games`
  (:2923, ORDER BY is_turn_at NULLS LAST + cap, feeds the 22-day digest),
  `generate_unique_username` (:856, retry-on-conflict loop), `is_user_admin`
  (:560), `get_user`/`get_user_by_email`, `set_user_name`,
  `get/set_user_pref_colors`, `mark_game_read`,
  `find_open_restart_proposal`, `find_enabled_bots`, `should_hide_add_friend`
  (spot-verified by grep). docs/CODING.md requires tests for db.rs changes.
  Secondary unasserted behaviors: `recent_games_for_index` never tests
  exclusion of games the user isn't in; `find_active_game_summaries` ordering
  unasserted. Separately (worker A range): `choose_colors` (:940-1004, the
  most intricate pure logic in the file), the ELO helpers, and `concede_game`'s
  2-player-only constraint (only a `debug_assert!` at :1308 — release builds
  would silently mis-place 3+ player games) lack coverage.
- recommendation: Add `#[sqlx::test]` coverage at minimum for
  `find_active_turn_games` (ordering, NULLS LAST, cap),
  `generate_unique_username` (conflict retry), `choose_colors` (table-driven
  `#[test]`), and a hard check or error in `concede_game` for 3+ player
  games; one cheap round-trip test each for the simple getters/setters.

### Pervasive redundant `updated_at = NOW()` in UPDATEs on trigger-maintained tables
- severity: minor
- category: consistency
- location: web/src/db.rs:1293 (also 1314, 1349, 1357, 1363, 1396, 1417, 1441, 1654, 1716, 1760, 1799, 1915, 1937, 2621, 2649, 2674, 2688)
- finding: Migration 001 installs `update_updated_at` BEFORE UPDATE triggers
  on all migration-001 tables (users, user_emails, friends, chats, games,
  game_players, game_logs, etc.) that unconditionally overwrite
  `NEW.updated_at`. Every manual `updated_at = NOW()` on those tables is dead
  SQL — misleading to readers and applied inconsistently (some UPDATEs omit
  it). Mixed idioms too: `NOW()` vs `timezone('utc', now())`. (Conversely,
  the migration-013 bot tables have NO trigger — the manual sets in admin.rs
  are required there; see the admin `update_bot_provider` nit.)
- recommendation: Sweep db.rs and drop manual updated_at assignments on
  trigger-maintained tables; add a doc comment at the top of db.rs stating
  the trigger convention.

### `update_game_command_success` can leave `is_finished = false` with `finished_at` set
- severity: minor
- category: correctness
- location: web/src/db.rs:1716, 4685
- finding: The UPDATE writes `is_finished = $2` unconditionally while
  `finished_at` is `COALESCE($3, finished_at)` (verified). The test at :4685
  deliberately drives a second command with `is_finished: false` on a
  finished game and asserts `finished_at` is preserved — but never asserts
  `is_finished`, enshrining a possible `is_finished = false AND finished_at
  IS NOT NULL` state. The inline comment admits the behavior "differs from
  the plan's phrasing, see report" — a dangling reference. Reachability
  depends on the game service accepting commands on finished games.
- recommendation: Guard the path (ignore non-finish updates on finished
  games, or clear finished_at), assert `is_finished` in the test, and resolve
  the dangling "see report" comment.

### `apply_rating_changes`: zero-change results leave rating_change NULL, defeating the idempotency guard
- severity: minor
- category: correctness
- location: web/src/db.rs:1646-1677
- finding: The guard trips on "any player already has a rating_change", but
  the write loop skips players whose computed change is 0
  (`if change == 0 { continue; }`). In an exact-tie game between equally
  rated players every change is 0, no rows are written, and a duplicate
  invocation would silently re-run — the invariant "finished-and-rated ⇒
  rating_change set" is violated. Latent unless a caller can invoke twice.
- recommendation: Write `rating_change = 0` (and rating_before) even when the
  change is 0, so the guard is reliable.

### `send_friend_request`: concurrent opposite-direction requests race to a raw 23505 error
- severity: minor
- category: correctness
- location: web/src/db.rs:1877-1925
- finding: Read-then-insert at READ COMMITTED: if A→B and B→A requests arrive
  concurrently and both read "no existing row", the loser hits the
  `friends_pair_key` unique index and gets a raw DB error instead of the
  function's own mutual-intent auto-accept contract.
- recommendation: Map 23505 on the INSERT to a re-read + auto-accept of the
  reverse row, or use `INSERT ... ON CONFLICT` / an advisory lock on the
  ordered pair.

### `friend_recent_visible_game` is N+1 by construction
- severity: minor
- category: quality
- location: web/src/db.rs:2316-2342
- finding: Fetches up to `scan_limit` candidate games, then issues one
  `is_game_visible_to_user` query per candidate. The visibility predicate is
  pure SQL and could be inlined into the candidate query, avoiding per-row
  round trips and future predicate drift.
- recommendation: Inline the NOT EXISTS visibility predicate into the
  candidate SELECT.

### `insert_game_logs_tx` is row-at-a-time
- severity: minor
- category: quality
- location: web/src/db.rs:1238-1265
- finding: One INSERT per log plus one per log target, sequentially awaited
  inside the transaction; a command producing many logs multiplies round
  trips while holding the tx open. Volume is probably low.
- recommendation: If profiling shows it matters, batch with `QueryBuilder` or
  UNNEST arrays; otherwise leave.

### db.rs is a 6.4k-line grab-bag (well-sectioned)
- severity: minor
- category: simplicity
- location: web/src/db.rs:1
- finding: The file mixes row builders, game CRUD, lifecycle writes, ELO,
  presence, friends, blocks, user settings, multi-email management,
  visibility predicates, color assignment, plus ~3.4k lines of tests. Every
  function is individually small, documented, and consistently styled, and
  section comments (`--- #30 friends ---` etc.) already mark natural module
  boundaries — so cohesion is poor but navigability is fine.
- recommendation: Split along the existing section comments into
  `db/games.rs`, `db/friends.rs`, `db/users.rs`, `db/emails.rs` etc. when the
  file next needs major surgery; not urgent on its own.

### `build_game_type_user` silently fabricates a default rating row
- severity: minor
- category: quality
- location: web/src/db.rs:59-110
- finding: On any NULL component of the LEFT JOINed `game_type_users` row the
  function returns a synthetic `GameTypeUser` with `id: Uuid::nil()`,
  rating 1200 — callers cannot distinguish "no rating row yet" from "real row
  at 1200" except by the undocumented nil-id sentinel. Presumably deliberate
  (new players start at 1200) but undocumented at the struct level.
- recommendation: Document the nil-id sentinel; consider
  `Option<GameTypeUser>` if any caller needs the distinction.

### `update_game_command_success` resets `is_turn_at` for continuing-turn players
- severity: nit
- category: correctness
- location: web/src/db.rs:1746
- finding: `let is_turn_at = if is_turn { now } else { p_is_turn_at };` resets
  the turn-start timestamp on every command for players who remain on turn
  (multi-action turns), while the `update_is_turn_at` trigger only stamps
  false→true transitions — the two mechanisms fight. If is_turn_at drives
  "how long has it been their turn" UI/reminders, continuing-turn players
  look like their turn just started. May be intended ("last activity").
- recommendation: Confirm intended semantics; if "turn started", only set
  `now` on transition and let the trigger cover it; if "last activity",
  rename/document.

### `is_user_admin` returns `sqlx::Result` while neighbors use `anyhow::Result`
- severity: nit
- category: consistency
- location: web/src/db.rs:560
- finding: The only public DB fn in the file returning `sqlx::Result`
  instead of `anyhow::Result`; callers juggle two error types.
- recommendation: Unify on `Result<bool>` (anyhow) or document why.

### `generate_unique_username` check-then-act race (mitigated)
- severity: nit
- category: correctness
- location: web/src/db.rs:864-871
- finding: The availability SELECT and the caller's INSERT are separate
  statements; a concurrent transaction could claim the same generated name
  between them. Mitigated by the `users_name_lower_key` unique index — the
  loser gets 23505, surfacing as a game-creation error rather than a retry.
- recommendation: Acceptable as-is; optionally retry on 23505 or document the
  reliance on the unique index.

### `delete_expired_unverified_emails` builds an interval via string interpolation of a bound parameter
- severity: nit
- category: quality
- location: web/src/db.rs:2950-2957
- finding: `created_at < NOW() - ($1 || ' seconds')::interval` with
  `.bind(secs.to_string())` — not injectable (bound parameter, Rust-formatted
  i64), but round-trips an integer through text.
- recommendation: Use `make_interval(secs => $1::bigint)`.

### `send_friend_request` has no application-level self-request guard
- severity: nit
- category: quality
- location: web/src/db.rs:1877
- finding: Self-friending relies on the DB CHECK constraint, surfacing as a
  generic DB error rather than a domain outcome. The DB does enforce it.
- recommendation: Early-return `Ok(())` (silent no-op, matching the
  function's other silent paths) when `source == target`, keeping the DB
  check as backstop.

### `choose_colors` clones the whole prefs vec each outer-loop pass
- severity: nit
- category: quality
- location: web/src/db.rs:970
- finding: `for (pos, pref) in rem_prefs.clone()` clones the full vec every
  iteration; the loop body doesn't mutate `rem_prefs`, so the clone is
  unnecessary. Player counts are small — impact nil.
- recommendation: Iterate over `&rem_prefs` or by index; drop the `.clone()`.

### `apply_rating_changes` convoluted all-pairs loop idiom
- severity: nit
- category: simplicity
- location: web/src/db.rs:1627-1632
- finding: `.iter().take(len.saturating_sub(1)).enumerate()` + `.skip(a_index
  + 1)` computes each unordered pair once — correct but obscure.
- recommendation: `for (i, a) in rated_players.iter().enumerate() { for b in
  &rated_players[i+1..] }`.

### Test-quality nits
- severity: nit
- category: quality
- location: web/src/db.rs:4014, 3584, 6032
- finding: (1) `suggestions_exclude_blocked_and_self` (:4014) never tests
  self-exclusion (a user can't be their own co-player via the fixture) — the
  name over-promises. (2) `is_game_visible_to_user` tests (:3584) lack the
  two-'friends'-player case (viewer friends with only one — must NOT see).
  (3) `count_rows` helper (:6032) uses `format!`-built SQL — test-only and
  literal-only, safe, but a copy-paste hazard.
- recommendation: Rename (1) or add a direct case; add the 3-player
  visibility test; comment (3) against reuse outside tests.

Clean: no string-built SQL with user input anywhere; no panics/unwraps in any
non-test DB function; all multi-statement writes transactional;
`update_game_command_success`'s optimistic-concurrency guard (WHERE
updated_at = $5 + rows_affected) is correct; LIKE escaping in `search_users`
correct; `remove_user_email`'s re-check-in-DELETE is race-safe; ELO math and
pairwise multi-player application correct with bots excluded; the
friends/blocks silent-shield semantics consistently implemented; test module
assertions are specific and meaningful throughout (undo-stash state machine,
ELO invariants, delete cascades, email invariants).

## main / router / state / config / error

### Session cookies lack the Secure flag in production (`SECURE_COOKIE` never set)
- severity: major
- category: correctness
- location: web/src/auth/session.rs:32-36, k8s/base/web/deployment.yaml:40-44
- finding: `create_session_layer` reads `SECURE_COOKIE`, defaulting to
  `with_secure(false)`. A snapshot-wide grep finds `SECURE_COOKIE` in exactly
  two places: this read and a completed plan doc. It is NOT set in any k8s
  manifest (base/dev/prod) and not documented in `.env.template` — so
  production pods run with `with_secure(false)`: the session cookie is set
  over HTTPS but browsers will also transmit it over any plaintext HTTP
  request to the domain (ssl-strip / any non-redirected HTTP endpoint),
  exposing the session token. Verified by the Lead. (SameSite=Lax, HttpOnly,
  and the 30-day OnInactivity expiry are fine.)
- recommendation: Set `SECURE_COOKIE=true` in `k8s/base/web/deployment.yaml`
  and document it in `web/.env.template`; better, default to secure with an
  explicit `SECURE_COOKIE=false` opt-out for local dev.

### Bot command consumer task is unsupervised — silent permanent bot outage on exit/panic
- severity: major
- category: correctness
- location: web/src/main.rs:55-74
- finding: `run_bot_command_consumer` is spawned once; if it returns `Err`
  one log line is emitted and the task exits forever — bots stop moving in
  every game until the pod restarts. If it panics, the JoinHandle is dropped
  and nothing restarts it either. Meanwhile `/healthz` stays green (it's
  deliberately DB-independent and doesn't cover the consumer), so k8s sees a
  healthy pod. In-flight messages are safe (un-acked → redelivered after
  `ack_wait`), but no new processing ever resumes. Verified against the
  snapshot.
- recommendation: Supervise: wrap the consumer in a restart loop with
  backoff, or `tokio::select!` the consumer future against `axum::serve` so
  consumer death aborts the process and lets k8s restart it, or add consumer
  liveness to a deeper health check. At minimum emit a metric/Sentry event on
  exit so the outage is alertable.

### No rustls CryptoProvider installed in web's main
- severity: minor
- category: correctness
- location: web/src/main.rs:5
- finding: Project rule: binaries using crates that read the process-default
  rustls CryptoProvider must install one in main, because the workspace
  enables both `aws-lc-rs` and `ring` (both confirmed in the workspace
  Cargo.lock). `web`'s main installs no provider. Today this apparently
  doesn't fire (prod resend email works, so presumably exactly one provider
  is enabled in web's graph), but nothing guards against a dependency bump
  flipping it into the dual-provider panic. UNCERTAIN: resolving definitively
  needs dependency-graph analysis (`cargo tree`), outside this unit's
  no-build mandate.
- recommendation: Defensively install the provider at the top of main
  (`rustls::crypto::aws_lc_rs::default_provider().install_default().ok()` —
  always safe per the project rule), or have the dependencies unit run
  `cargo tree -e features -i rustls` for the web binary and record the answer
  in CODING.md.

### Graceful shutdown does not cover WS connections or background tasks
- severity: minor
- category: quality
- location: web/src/main.rs:104-110, web/src/websocket.rs:108, web/src/main.rs:55-80
- finding: `with_graceful_shutdown` drains in-flight HTTP requests, but
  upgraded WebSocket connections live in detached `tokio::spawn`ed tasks and
  are dropped when the runtime shuts down; the bot consumer and email sweep
  tasks get no shutdown signal either. Impact is low (WS clients reconnect;
  aborted bot.commands redeliver), but every deploy hard-drops all connected
  clients.
- recommendation: Acceptable as-is for a beta; if desired, track WS tasks
  with a `CancellationToken`/TaskTracker and close sockets with a proper
  close frame on shutdown.

Clean: `router.rs` middleware ordering is correct (route-level `Router::layer`
makes `MatchedPath` available — no tracing-cardinality bug; `/healthz`
correctly escapes the session layer; sentry layer order correct; 256 KiB body
limit and 30s timeout sane); `main.rs` panics are startup-only (allowed);
tracing→sentry init order deliberate and correct; `send_default_pii: false`;
metrics port correctly not exposed in k8s; `state.rs`, `config.rs`,
`error.rs` clean (`error::internal` logs server-side, returns opaque client
message — intentional pattern).

## nats

### Messages that exhaust `max_deliver=3` strand silently (no DLQ, no advisory handling)
- severity: minor
- category: correctness
- location: web/src/nats.rs:63-94
- finding: Both durable pull consumers use `AckPolicy::Explicit`,
  `ack_wait = 5 min`, `max_deliver = 3` on a WorkQueue-retention stream. A
  message that fails delivery 3 times is never redelivered and (WorkQueue)
  only deleted on ack — it sits in the stream indefinitely and the bot never
  moves in that game again, with no signal anywhere (NATS emits a
  MAX_DELIVERIES advisory; nothing subscribes). UNCERTAIN: the consumer
  (`web::game::run_bot_command_consumer`, out of scope) may `term()` poison
  messages — cross-check for the web-domain unit; downgrade to nit if it
  does, upgrade to major if it doesn't.
- recommendation: `term()` + compensate (re-publish with backoff, surface
  "bot stuck" in the UI) on permanent failure, or a DLQ subject / advisory
  listener with alerting; consider a stream `max_age`.

### `get_or_create_stream/consumer` never reconcile config drift
- severity: minor
- category: correctness
- location: web/src/nats.rs:52-94
- finding: Both return the existing object untouched when it exists —
  changing `ack_wait`/`max_deliver`/`retention`/subjects in code is silently
  a no-op against an existing NATS deployment, so the values in code can
  diverge from what the server enforces with no warning at startup.
- recommendation: After get-or-create, compare the returned config against
  the desired values and warn (or fail startup) on mismatch; document that
  consumer config changes require manual consumer deletion/recreation.

### `ack_wait = 5 min` may be shorter than consumer processing → duplicate delivery
- severity: minor
- category: correctness
- location: web/src/nats.rs:63
- finding: If `bot.command` processing (bot HTTP call + DB writes + retries)
  ever exceeds 5 minutes without an ack or in-progress extension, JetStream
  redelivers to another replica and the command runs twice. Likely
  bounded (stale-state conflict handling exists per `BotCommandEvent::attempt`
  docs). UNCERTAIN: consumer ack cadence is out of scope (web/src/game/).
- recommendation: Cross-check with the web-domain unit: confirm the consumer
  acks promptly or sends `in_progress()` pings; otherwise raise `ack_wait`.

Clean: WorkQueue retention with two disjoint filter-subject consumers is
valid; explicit ack policy is the right choice; event schemas are minimal and
versionable.

## websocket

### `/ws` has no authentication; every connection gets the site-wide firehose
- severity: minor
- category: correctness
- location: web/src/websocket.rs:82-87, 112-125, web/src/router.rs:142
- finding: `ws_handler` takes no `Session` and does no auth check. Every
  connection — including anonymous — creates two core-NATS subscriptions on
  `game.>` and `proposal.>` and receives EVERY game/proposal update signal
  site-wide. Impact: (a) information disclosure — an unauthenticated client
  gets a live firehose of all game/proposal UUIDs and activity timing
  (payloads are skinny UUIDs and game pages presumably enforce authorization
  on fetch, so this is metadata leak, not data leak); (b) scalability —
  outbound WS traffic is O(connections × total site update rate) and every
  client reactively processes every signal. Per-game subjects already exist
  on the publish side, so per-connection filtering is feasible.
- recommendation: If the firehose is an accepted design (plausible at current
  scale), document the decision near `ws_handler`. Otherwise accept an
  initial subscribe message listing game/proposal IDs and subscribe
  per-connection; require a valid session if anonymous activity metadata
  matters. Revisit the O(N×M) fan-out before user counts grow.

### Client calls `open()` on every visibilitychange/online — tears down healthy sockets
- severity: minor
- category: correctness
- location: web/src/websocket_client.rs:70-85
- finding: The `visibilitychange` (→ visible) and `online` listeners call
  leptos-use's `open()` unconditionally. Verified against leptos-use v0.19.0
  source: `open()` closes any existing socket and creates a new one with no
  ready-state guard; the old socket's `onclose` likely schedules a second
  gratuitous reconnect ~3s later (source-verified, not runtime-observed). Net
  effect: every tab refocus causes ~1–2 gratuitous WS reconnects plus
  server-side churn (two new NATS subscriptions per reconnect). The local
  `bump_game_update` on action success already covers the "WS was down"
  refetch case these handlers protect.
- recommendation: Gate the calls on `ready_state` (already returned by
  `use_websocket_with_options`, currently destructured as `ready_state: _`) —
  only call `open()` when state is `Closed`.

### WS inbound message/frame limits left at tungstenite defaults
- severity: nit
- category: quality
- location: web/src/websocket.rs:82-87
- finding: `ws.on_upgrade` without `.max_message_size()`/`.max_frame_size()`
  leaves ~64 MiB/16 MiB defaults. Inbound messages are drained and ignored —
  the server never needs more than close/pong — but each anonymous connection
  can buffer up to the max.
- recommendation: Set small explicit limits, e.g.
  `ws.max_message_size(4 * 1024).max_frame_size(4 * 1024)`.

### No dead-connection detection beyond send failure
- severity: nit
- category: quality
- location: web/src/websocket.rs:127-164
- finding: The server sends a Ping every 30s (good) but never verifies Pongs
  and has no read timeout; a silently half-open client keeps its task, gauge
  count, and two NATS subscriptions alive until TCP keepalive/kernel timeout
  (typically hours). Bounded and self-healing; common practice.
- recommendation: Optional: track last-pong timestamp and close connections
  idle for >2–3 ping intervals.

Clean: publish side follows the project rule exactly (publish + `.flush()`,
flush errors logged not propagated); `WsConnectionGuard` covers all gauge
exit paths; non-UTF8 NATS payloads skipped not fatal; client parse path and
relative-URL normalization correct (verified against leptos-use source);
`ReconnectLimit::Infinite` appropriate.

## import_game

### Unbounded file read, no input size guard
- severity: nit
- category: quality
- location: web/src/bin/import_game.rs:20
- finding: `std::fs::read_to_string` loads the whole bundle into memory with
  no size cap. Dev-only tool run by hand against trusted local files —
  defensive polish only.
- recommendation: None required; an optional size sanity check with a clear
  error.

Otherwise clean: proper usage exit code, anyhow error chains with path
context, prints resulting URL.

## Cargo.toml (ssr deps, manifest level)

### `gloo-net` dependency is unused
- severity: minor
- category: dependencies
- location: web/Cargo.toml:75
- finding: `gloo-net = { version = "0.7", features = ["websocket"] }` — a
  repo-wide grep finds zero `gloo_net` references in `src/` or `tests/`. The
  websocket feature is dead weight since the client WS moved to leptos-use,
  and the dep is non-optional so it lands in the WASM hydrate bundle.
  (`gloo-timers` on line 76 IS used — keep it.)
- recommendation: Delete the `gloo-net` line. (Optional cleanup: the gloo
  family is effectively in maintenance mode; `gloo-timers` could move to
  leptos-use's `use_interval_fn`, already a dependency.)

### tokio `net` and `time` features used but not declared
- severity: minor
- category: consistency
- location: web/Cargo.toml:24
- finding: The `tokio` dep declares only `["rt-multi-thread", "macros",
  "signal"]`, but the crate directly uses `tokio::net::TcpListener`
  (main.rs:103,184) and `tokio::time::interval`/`timeout` (websocket.rs).
  It compiles only because transitive deps enable those features via feature
  unification; a dependency upgrade could break the build with confusing
  errors.
- recommendation: Add `"net"` and `"time"` to the tokio feature list.

### `futures-util` non-optional but only used in ssr/test code
- severity: nit
- category: dependencies
- location: web/Cargo.toml:74
- finding: All `futures_util` uses are ssr-side or tests, yet the dependency
  is unconditional, so it joins the hydrate bundle's dependency graph.
- recommendation: Make it `optional = true` and add `dep:futures-util` to the
  `ssr` feature.

### Dependency currency spot-check (crates.io, 2026-07-24)
- severity: nit
- category: dependencies
- location: web/Cargo.toml
- finding: The ssr-relevant set is current (axum 0.8.9, tower-http 0.7,
  sentry 0.48.5, resend-rs 0.28, mrml 6.0.1, reqwest 0.13.4, leptos-use
  0.19.0, etc.) except: `async-nats` 0.49.1 → 0.50.0 available (:79) and
  `svix` 1.98 → 1.99.1 available (:50). sqlx 0.8 / tower-sessions 0.14
  holdbacks and the wasm-bindgen =0.2.121 pin are documented intentional —
  not flagged.
- recommendation: Bump `async-nats` to 0.50 (check the JetStream changelog
  first) and `svix` to 1.99.1 at the next dependency pass.

Clean: ssr/hydrate feature split is disciplined; `hash-files = true` matches
the cache-control strategy.

## blocked_domains

Clean (skim per plan — 8,152-line vendored data table): loaded as a lazily
initialized `HashSet<&'static str>`; `is_blocked` lowercases before lookup;
both call sites go through `is_blocked`; O(1) lookup. No issues.
