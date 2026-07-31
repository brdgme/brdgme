# Unit 05a - Web server: authentication and cryptography

Review of the auth/crypto portion of Unit 05 (2026-07-25..2026-07-30 remediation).

## Scope

Commits reviewed (auth/crypto):

- `13a1e693` WP-36 crypto/deploy
- `ea9f7a2b` WP-34 auth races/session
- `0a0f7e6d` WP-35 fail-closed posture
- `c3b90122` WP-35 test
- `a9609e57` WP-43 web cargo deps - **no auth/crypto content**: F63 is a 100 MiB
  size check in `bin/import_game.rs`, F64-F67 are dependency declarations. No
  crypto crate was added, removed or re-versioned. Nothing further to review
  here; the `import_game` guard belongs to whoever owns export/import (Unit 07).
- `4d31f6eb` WP-82 db.rs module split - auth/token portion only. The three
  helpers WP-35 prescribed now live in `db/emails.rs:250,260`
  (`delete_login_confirmation{,_tx}`) and `db/users.rs:383`
  (`invalidate_all_auth_tokens`) with the prescribed signatures and SQL, and
  `db/mod.rs` re-exports them so `auth/server.rs` call sites are unchanged. The
  auth-facing surface survived the relocation intact. The rest of the split
  (14 files, 9,512 lines) is 05b's.

Files read in full: `rust/web/src/auth/server.rs`, `rust/web/src/auth/session.rs`,
`rust/web/src/crypto.rs`, `rust/web/src/main.rs`, `rust/bot/src/crypto.rs`, plus
targeted reads of `rust/web/src/db/emails.rs`, `rust/web/src/db/users.rs`,
`rust/bot/src/main.rs`, `k8s/base/web/deployment.yaml`,
`k8s/dev/web-patch.yaml`, `k8s/prod/app/web-patch.yaml`.

Specs recovered from `868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/`:
WP-34, WP-35, WP-82 exist and were checked criterion by criterion. **WP-36 and
WP-43 have no spec file** - see Coverage gaps.

Left to sub-unit 05b:

- `b49df619` WP-37 admin.rs
- `baa5fc64` WP-41 db.rs
- `347970a0` WP-39 bot consumer supervision
- `13a1e693` WP-36 - the non-crypto half (ws F55 WebSocket close-frame shutdown
  in `websocket.rs`, `tests/websocket_hygiene.rs`, `admin.rs`). The crypto half
  (F17/F52/F54) is reviewed here.
- `914aa0c6` WP-38 bot-turn wedge recovery
- `618156a7` WP-68 term_size->terminal_size
- `4d31f6eb` WP-82 db.rs module split (non-auth remainder)

## Findings

### F-85 (High) - `logout_everywhere` reports success without revoking anything when the session store errors

`rust/web/src/auth/server.rs:590-612`, via
`rust/web/src/auth/session.rs:68-74`.

`logout_everywhere` gates the revocation on
`if let Some(user) = get_user_from_session(&session).await`, and
`get_user_from_session` is `session.get::<SessionUser>(..).await.ok().flatten()`
- it collapses a `tower_sessions::session::Error` (store/DB failure,
deserialization failure) into `None`. On that path the `if let` body is skipped,
no `user_auth_tokens` rows are deleted, and the function still returns
`Ok(true)`. The UI reports "logged out everywhere" while every stolen token on
every device remains valid.

Why it matters: this is the only revoke-all primitive in the product (WP-35's
answer to F11, where the decision was explicitly "no token expiry, revoke-all
instead"). A silent no-op on the sole revocation path defeats the entire
mitigation, and the user has no way to tell it did not happen. `logout`
(`:566-588`) has the same shape, so a single-session logout can likewise leave
its token row live while returning `Ok(true)`.

This is the same defect class as ws F5, which WP-34 fixed one function away:
`get_current_user` (`:537-564`) now correctly propagates the
`validate_session_token` `Err` instead of `unwrap_or(false)`, but its immediate
neighbour on the identical path, `get_user_from_session`, still swallows its
error. Systemic pattern 2 (hardening one function while its neighbours stay
unguarded).

Remediation: make `get_user_from_session` return
`Result<Option<SessionUser>, tower_sessions::session::Error>` and propagate at
every call site; in `logout`/`logout_everywhere` return `Err` rather than
`Ok(true)` when the session cannot be read, and only report success once the
delete has committed. WP-35's F11 test ("two tokens -> both gone") passes on the
happy path and cannot catch this.

### F-86 (Medium) - `get_user_from_session` error-swallowing also silently de-authenticates on a transient store error

`rust/web/src/auth/session.rs:68-74`.

Same root cause as F-85 on the read path: a transient session-store error makes
`get_current_user` return `Ok(None)`, i.e. "you are logged out", which is
precisely the mass-logout-on-a-blip behaviour ws F5 was raised about. WP-34's
fix addressed only the `validate_session_token` half of that path, so the
finding is only half closed. Additionally `get_current_user:555` discards the
`clear_user_session` error with `let _ =`, so a definitively-invalid token can
be left in a session that the code believes it cleared.

Remediation: as F-85 - propagate the error; distinguish "no user in session"
from "could not read the session".

### F-87 (Medium) - the F2 fix destroys a legitimate in-progress add-address flow and forks the account

`rust/web/src/auth/server.rs:459-501` (`confirm_login_inner` pending branch),
against WP-35 section 3 first bullet.

WP-35's F2 fix assumes an unverified `user_emails` row for the presented address
can only be a squatter's. It cannot: `add_email_address` (`:853-900`) creates
exactly such a row for the *legitimate* owner and mails a code to the same
address, and both `confirm_email_address` and `confirm_login` accept that same
code for that same `login_confirmations` row (the table is keyed by email
alone - there is no flow discriminator). A user who pastes their add-address
code into the login form instead of the settings form therefore hits the
"squatted" branch: their pending row is deleted and a **second, separate user
account** is created owning the address, with no error and no way back.

Why it matters: silent account forking plus loss of the pending add. It is
reachable by ordinary user error, not just by an attacker.

Remediation: discriminate the two flows - either add a purpose/kind column to
`login_confirmations` (and require it to match), or in the pending branch only
delete rows whose owning user is *not* mid-flow, or resolve the login to the
existing owner of the unverified row when that row's user has a verified
address. The spec's own analysis missed this case; it needs an owner decision,
not a silent behaviour change.

### F-88 (Low) - `confirm_email_address` never got `confirm_login`'s shape validation

`rust/web/src/auth/server.rs:904-924` vs `:340-355`.

WP-34's F13 added a `token.len() != 6` / all-ASCII-digits / non-empty-email
early return to `confirm_login`, but `confirm_email_address` calls
`validate_confirmation_code` with completely unvalidated `email`/`token`. Both
are `#[server]` endpoints reaching the same function, so the guard covers one
caller of two. Not exploitable on its own (the DB compare still rejects), but it
means arbitrary attacker-supplied strings reach the SQL layer and every such
call burns an attempt against the victim's live code - see F-89.

Remediation: hoist the shape check into `validate_confirmation_code` so both
callers get it.

### F-89 (Medium) - any authenticated caller can burn another account's live login code

`rust/web/src/auth/server.rs:394-423` + `:904-924`.

`validate_confirmation_code` increments `attempts` for whatever `email` string
it is handed, and `confirm_email_address` passes the caller-supplied `email`
straight through with no check that the address has anything to do with the
caller (the ownership check, `mark_email_verified`, happens only *after*
validation). `confirm_login` is likewise unauthenticated by design. So 11 calls
with `email = victim@x.com` and any wrong token push `attempts` past
`CONFIRM_MAX_ATTEMPTS_PER_CODE` and kill the victim's outstanding login code;
repeated against the 60s resend cooldown this is a targeted login-denial
primitive. This is a direct consequence of WP-34's chosen fix (increment
unconditionally, before any authorization) and was not considered in the spec.

Remediation: the row is keyed by email only, so the cap cannot distinguish
attacker from owner. Cap per (email, source IP) as well as per code, and/or
require the `confirm_email_address` path to prove the address is pending on the
caller's own account *before* incrementing.

### F-90 (Medium) - ws F16/F17 were applied to `rust/web/src/crypto.rs` only; `rust/bot/src/crypto.rs` is an unhardened duplicate with the same hardcoded key

`rust/bot/src/crypto.rs:59-76` vs `rust/web/src/crypto.rs:45-75`.

ws F16 (major) was "hardcoded public fallback encryption key silently used when
`DATABASE_ENCRYPTION_KEY` unset". WP-35/WP-36 fixed the web copy: `load_key`
returns `CryptoError::MissingKey` unless `ALLOW_INSECURE_DEFAULT_KEY=true`, and
`main` eagerly `expect()`s it. The bot crate has its own, byte-identical
`default_key()` (`b"brdgme-dev-key-not-for-prod!!!"`) and its `load_key` still
does exactly what the finding described:

```rust
let hex_str = match std::env::var("DATABASE_ENCRYPTION_KEY") {
    Ok(v) => v,
    Err(_) => return Ok(LoadedKey::Default(default_key())),
};
```

There is no `MissingKey` variant, no `ALLOW_INSECURE_DEFAULT_KEY` gate, and no
`zeroize`/`Zeroizing` (the key is a plain `[u8; 32]` moved through
`LoadedKey`). Its own tests actively pin the old behaviour:
`load_key_missing_env` asserts `LoadedKey::Default(default_key())` is returned -
so the fix cannot be applied to this crate without deleting a green test, which
is presumably why nobody did.

Why it matters: both crates operate on the *same* database column (bot LLM API
credentials) with the same ciphertext format. Verified mitigating facts:
`rust/bot/` has no `crypto::encrypt` call site at all (the only use is
`rust/bot/src/config.rs:91`, decrypt), and `rust/bot/src/main.rs:800-816` does
warn on the default key, so a mis-deployed bot degrades to "decryption fails,
env-var fallback" rather than writing weakly-encrypted data. That is why this is
Medium, not High. What remains is real:

- The same public key is still compiled in and still silently selected on a
  missing env var, in a crate whose startup does *not* fail closed while its
  sibling now does. The moment any encrypt path is added to `bot` (e.g. key
  rotation from the bot side) this becomes F16 verbatim.
- ws F17 ("zeroize AES master key material") was implemented in `web` only.
  `rg zeroize rust/` shows **zero** usage in `rust/bot/`: `LoadedKey` holds a
  bare `[u8; 32]` for the process lifetime, so the master key is left in
  reclaimable memory in exactly the process the finding was about.
- Two divergent hand-maintained copies of the same AES-GCM wrapper (`web` uses
  `getrandom::fill`, `bot` uses `Aes256Gcm::generate_nonce(&mut OsRng)`; `web`
  has a `MissingKey` variant, `bot` does not) is precisely the duplication that
  lets one copy get fixed and the other not.

Fixing only one of two copies of the same primitive is the classic half-fix; the
review's location citation (`web/src/crypto.rs:42-57`) is what scoped the
remediation, and nobody swept for the duplicate.

Remediation: extract one crypto module (a shared lib crate) used by both `web`
and `bot`, carrying the web version's `MissingKey` + `ALLOW_INSECURE_DEFAULT_KEY`
gate and `Zeroizing`; update `rust/bot/src/main.rs`'s startup to fail fast the
same way `rust/web/src/main.rs:33` does; replace `load_key_missing_env` with a
test asserting the error.

### F-91 (Low) - the AAD decline is recorded only in a git commit message, and its stated rationale is the coupling of F-90

`rust/web/src/crypto.rs:20-43`, `rust/bot/src/crypto.rs:17-39`.

`encrypt`/`decrypt` take only key and plaintext/ciphertext - nothing binds a
ciphertext to the row or column it was stored against, so an actor able to write
the DB can relocate a credential blob between bots and it will decrypt and
authenticate cleanly. WP-36 has **no spec file** in the recovered corpus, and
both WP-34 and WP-35 list AAD among WP-36's responsibilities, so the only
acceptance record is `13a1e693`'s message:

> ws F17: zeroize AES master key material (AAD declined - shared format with bot
> + existing prod ciphertexts)

The decline itself is defensible. Two things are wrong with how it landed:

1. It exists nowhere a maintainer will see it - no `D-NN` decision entry, no
   comment in `crypto.rs`, no spec. `00-STATE.md`'s D-39 ruling ("unverifiable
   and that is the finding") applies to this the same way.
2. The rationale is "shared format with bot", i.e. the decline is justified by the
   very duplication F-90 identifies as unfixed. Both halves of the reason are
   also solvable rather than blocking: a version byte prefix handles "existing
   prod ciphertexts", and a shared crate handles "shared format with bot".

Remediation: record the decline as an accepted risk in `crypto.rs` and in the
decisions log with its expiry condition (revisit when the format is versioned or
the two copies are merged). If it is later taken up: `aad =
b"bot_provider:{id}:api_key"` plus a leading format-version byte.

### F-92 (Medium) - three of WP-34's mandated regression tests were never written

`rust/web/src/auth/server.rs:1001-1996` against WP-34 section 5 and the section-6
rider table ("test needed" = Y).

The rider table demands tests for F3 (session-id rotation), F10
(`add_email_address` surfaces a global-cap refusal as `Err`) and F15 (`logout`
returns `Ok(true)` and the session record is gone). The test module has none of
them - no test references `cycle_id`, `logout`, `session.flush()` or
`add_email_address`. The closest substitute,
`request_confirmation_code_returns_failure_at_global_cap` (`:1758`), exercises
the inner helper, not `add_email_address`, so the actual F10 defect (the caller
discarding `LoginResponse.success`) is still unpinned. `confirm_login`'s
`cycle_id` call and `logout`'s `flush` are entirely unexercised.

The structural cause is that `confirm_login`, `logout` and `add_email_address`
all `extract::<Session>()` / `get_current_user()`, which the `with_pool_context`
harness cannot supply - the tests all drive `confirm_login_inner` instead. That
is a harness gap, not an oversight to be argued away, and the spec's "Mandatory
per CODING.md: `auth/` changes ship tests" was not met.

Remediation: build a request-parts test harness (a `tower_sessions`
`MemoryStore` + a constructed `http::Request` through `build_router`, which
`main.rs:117-121` says already exists for SSR page tests) and add the three
tests. Until then the F3/F10/F15 fixes are unverified.

### F-93 (Medium) - `add_email_address` is an authenticated registered-address oracle, and parks rows even when no mail is sent

`rust/web/src/auth/server.rs:853-900`.

Three distinct error strings distinguish "already on your account", "owned by
someone else" (`"Address unavailable"`) and success, so any logged-in user can
test arbitrary addresses for registration. ws F4 raised exactly this leak class
for `login`'s blocked-domain branch and D-14 (ii) accepted the asymmetry *there*
on the specific grounds that uniform rejection would lock out verified users -
that rationale does not transfer to `add_email_address`, which nobody looked at.

Separately, `insert_unverified_email` commits before
`request_confirmation_code` runs, so when the global 50/day cap is hit the
function returns `Err` while the unverified `user_emails` row stays parked on the
caller's account with no confirmation email ever sent. Post-F2 this no longer
locks the real owner out of login, but it is still unbounded free claiming of
arbitrary addresses (the 24h unverified sweep is the only bound).

Remediation: collapse the two "unavailable" branches into one message; insert the
unverified row and request the code in one transaction, or roll the insert back
when the send is refused.

### F-94 (Medium) - there is no rate limiting anywhere on the auth routes, and two doc comments claim there is

`rust/web/src/auth/server.rs:31-48`, `:196-213`.

`LOGIN_GLOBAL_MAX_SENDS_PER_DAY = 50` is platform-wide, and
`LOGIN_MAX_SENDS_PER_EMAIL = 5`, so ten attacker-controlled addresses exhaust the
day's budget and every other user's login returns "Logins are temporarily
limited". The `LOGIN_CAP_LOCK_KEY` doc comment defends the design with "the
endpoint is already IP-rate-limited", and
`confirm_login_inner`'s doc comment (`:425`) claims `confirm_login` has a
"per-IP rate limit" - neither `login` nor `confirm_login` contains any rate
limiting. WP-34 made this cap correct (it previously over-counted), which is what
makes the lever reliable.

Verified: `rg -i 'governor|rate_?limit|ConcurrencyLimit' rust/web/src
rust/web/Cargo.toml` matches only three outbound HTTP header-name string
literals in `admin.rs:779-781`. `router.rs`'s layer stack
(`:163-207`) is session, cache-control, body limit, timeout, CORS and Sentry -
no rate or concurrency limit at any layer. So no such control exists anywhere in
the service.

Consequences that follow: this global-cap lever; F-89's unthrottled
attempt-burning; and unbounded hammering of `login`/`confirm_login` bounded only
by Turnstile. Raised to Medium because the two comments are load-bearing - they
are the recorded justification for choosing a global advisory lock and for not
throttling confirm, so an absent control is being credited as a present one in
design decisions.

Remediation: either add the per-IP limit the comments claim (a `tower_governor`
layer scoped to the `/api` auth server-fn routes) or delete both claims and
re-derive the cap design without them; consider reserving cap headroom for
addresses that already have a verified account.

### F-95 (Low) - the F1 concurrency test asserts the inverse of the criterion it was specified to pin (pattern 4b)

`rust/web/src/auth/server.rs:1621-1652` against WP-34 section 5, first bullet.

The spec's acceptance criterion: "N concurrent `confirm_login_inner` calls with a
WRONG code ... -> final `attempts` never exceeds `CONFIRM_MAX_ATTEMPTS_PER_CODE +
1`, and every call errored." The delivered test fires 15 concurrent calls and
asserts

```rust
assert!(row.attempts >= CONFIRM_MAX_ATTEMPTS_PER_CODE, ...);
```

i.e. a **lower** bound where the spec prescribed an **upper** bound. The
prescribed upper bound is in fact unachievable under the design the same spec
mandated - `UPDATE ... attempts = attempts + 1 ... RETURNING *` increments
unconditionally before any cap check, so 15 concurrent wrong guesses leave
`attempts = 15`. Rather than flagging the contradiction, the test was written to
agree with the code. This is the fourth confirmed instance of `00-STATE.md`
pattern 4b.

The cap itself *is* enforced (the check is post-increment, and the test's trailing
`correct.is_err()` assertion does prove that), which is why this is Low rather
than higher. But the criterion as written is unmet and the discrepancy is now
invisible.

Remediation: correct the spec's criterion to "no attempt after the cap is reached
can succeed" and keep the trailing assertion, or drop the unbounded-counter
design in favour of a `LEAST(attempts + 1, cap + 1)` update so the original
criterion holds. Either way, record the divergence rather than erase it.

### F-96 (High) - WP-35 made a missing `TURNSTILE_SECRET_KEY` a startup panic but no manifest anywhere provisions it, so the next prod web deploy crash-loops

`rust/web/src/main.rs:40-45`, against `k8s/base/web/deployment.yaml:33-39` and
`k8s/prod/app/web-patch.yaml`.

`0a0f7e6d` (WP-35) added:

```rust
let turnstile_secret = std::env::var("TURNSTILE_SECRET_KEY").unwrap_or_default();
if turnstile_secret.is_empty()
    && std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() != Ok("true")
{
    panic!("TURNSTILE_SECRET_KEY not set - refusing to start without CAPTCHA verification");
}
```

`TURNSTILE_SECRET_KEY` occurs in this repository in exactly three code locations
(`main.rs:40`, `main.rs:44`, `auth/server.rs:289`) and one unimplemented design
doc. **Zero occurrences under `k8s/`, `.github/` or `k8s/argocd/`.** The web
container's env comes from `envFrom` on three `secretRef`s -
`postgres-config`, `email-config`, `database-encryption-key` - none of which is
named for Turnstile, and `k8s/prod/app/web-patch.yaml` adds only the two Sentry
DSNs. `ALLOW_INSECURE_DEFAULT_KEY=true` exists only in `k8s/dev/web-patch.yaml`
and `scripts/rust-test.sh`, so the dev escape hatch is not available in prod
either. `0a0f7e6d` touched only `k8s/dev/web-patch.yaml` and
`rust/web/.env.template`.

Why it matters: this is a fail-closed hardening change whose deployment half was
never done. The service will `panic!` before binding its listener on the next
prod image rollout. WP-35's spec text covers the `ALLOW_INSECURE_DEFAULT_KEY`
deployment question in detail ("must be added to the dev overlay /
`.env.template` only, never to `k8s/base` or `k8s/prod`") but says nothing about
*supplying* `TURNSTILE_SECRET_KEY`, and the checklist row it satisfied was about
the startup refusal only.

Contrast `DATABASE_ENCRYPTION_KEY`, where the same commit's panic is safe: it is
supplied by the `database-encryption-key` `secretRef` already present in
`k8s/base/web/deployment.yaml`. Note that no `Secret`/`SealedSecret`/
`ExternalSecret` manifest for *any* of the three referenced secrets exists in the
repo - they are created out-of-band, matching the repo's existing documented
pattern, so that is unverifiable from here rather than wrong.

Remediation: add `TURNSTILE_SECRET_KEY` to an `envFrom` secret consumed by the
web deployment (or a fourth `secretRef`) and create the secret before the next
rollout. Confirm with `kubectl kustomize k8s/prod` output, per WP-35's own
instruction, and add the same check to whatever pre-deploy verification exists.
More generally: any WP that converts a soft default into a startup panic must
have a deployment acceptance criterion, and this programme had none.

## Verified good

Checked against the recovered acceptance criteria, end state read in full.

WP-34:

- **F1.** `validate_confirmation_code` (`auth/server.rs:394-423`) is exactly the
  prescribed single atomic `UPDATE ... attempts = attempts + 1 ... RETURNING *`,
  with the checks in the prescribed order and `> CONFIRM_MAX_ATTEMPTS_PER_CODE`
  (not `>=`) - the off-by-one the spec called out is correct, and
  `confirm_login_allows_tenth_attempt_with_correct_code` /
  `..._rejects_eleventh_attempt...` (`:1654-1697`) pin both sides of it.
- **F3.** `session.cycle_id()` before `set_user_session` (`:360-372`). Present
  and correctly ordered (untested - F-92).
- **F5.** `get_current_user` (`:537-564`) propagates the `sqlx::Error` and clears
  the session only on `Ok(false)`. The `unwrap_or(false)` is gone. (Half of the
  path - see F-86.)
- **F6.** Windowed counter implemented as specified: `login_email_sends` table
  (migration `024_login_email_sends.sql`, the correct next free number),
  `COUNT(*) ... sent_at > NOW() - INTERVAL '24 hours'`, a matching GC statement,
  and one insert per send alongside the unchanged `login_confirmations` upsert.
  `sent_count`/`last_sent_at` semantics untouched, so the per-email cap and
  cooldown are unaffected - which the spec explicitly required. Four tests cover
  it (`:1697-1774`), including the "counts windowed, not cumulative" case the
  spec named.
- **F10.** `add_email_address:895-898` binds the `LoginResponse` and returns
  `Err(resp.message)` when `!success` (untested - F-92).
- **F12.** `rand::rng().random_range(0..1_000_000)` - a CSPRNG with no modulo
  bias, exactly as prescribed, and the constant-time/hashing halves correctly
  left declined.
- **F13.** Shape validation in `confirm_login:349-355` matches the spec wording
  and returns the same uniform error (one caller only - F-88).
- **F14.** `verify_turnstile_token(client: &reqwest::Client, ...)` with
  `expect_context::<reqwest::Client>()` in `login:288`. WP-35 did not revert it.
- **F15.** `logout:574-585` logs the `invalidate_auth_token` error instead of
  `let _ =`, and calls `session.flush()` after `clear_user_session`.

WP-35:

- **F2.** `confirm_login_inner:459-472` deletes only `verified_at IS NULL` rows
  and falls through to signup; the verified branch above is untouched.
  `confirm_login_steals_unverified_claim` (`:1564-1619`) asserts all four of the
  spec's post-conditions and
  `confirm_login_resolves_non_primary_verified_address` (`:1514`) is still there
  as the negative. (Correct against the spec; the spec's own case analysis was
  incomplete - F-87.)
- **F4.** Differential response kept, with the D-14 (ii) comment at `:326-327`
  in the exact terms the spec required, and `login_blocked_domain_asymmetry`
  (`:1819`) pins the asymmetry so it cannot be "fixed" later. This is the right
  handling of a rejected recommendation.
- **F7.** `send_login_email` returns `Result<(), ()>` and
  `request_confirmation_code:246-251` maps a failure to
  `success: false` with the prescribed message.
- **F8.** `Err` arm returns `false`, warn-logs and increments
  `turnstile_verify_error_total` (`:272-276`); the empty-secret dev carve-out is
  intact and now gated by the `main.rs:40-45` startup refusal. Both prescribed
  tests exist (`:1857`, `:1864`).
- **F11.** `db::invalidate_all_auth_tokens` (`db/users.rs:383-389`),
  `logout_everywhere` server fn, and the "Log out everywhere" button wired
  through a `ServerAction` in `settings.rs:334/399/543`. No expiry or GC was
  added anywhere, per D-14 (iv). `c3b90122` adds the two-token regression test.
- **(b)** Both raw `DELETE FROM login_confirmations` statements are gone,
  replaced by `db::delete_login_confirmation{,_tx}` (`db/emails.rs:250`, `:260`)
  at `auth/server.rs:522` and `:920`.
- **(c)** `make_email_address_active:954-964` fetches `SWITCH_DIGEST_CAP + 1`,
  logs and increments `switch_digest_capped_total` when over, then caps for
  sending. Signature unchanged.
- **(a)** `make_active_rejects_unverified_address` (`:1775`) exists - the
  email-change re-verification invariant is pinned, which was the whole of what
  the spec asked for after its honest re-derivation.

WP-36 (no spec; judged against `13a1e693`'s own claims):

- **F52.** `secure_cookie(env_value)` is a pure, tested helper defaulting to
  Secure with an exact-literal `"false"` opt-out (`auth/session.rs:33-35`,
  three tests at `:110-126`), and the dev opt-out is confined to
  `k8s/dev/web-patch.yaml`. Prod sets nothing and therefore gets Secure. The
  helper being pure and unit-tested is the right shape for an env-driven
  security default.
- **F54.** `rustls::crypto::aws_lc_rs::default_provider().install_default()` at
  `main.rs:18-20` with a comment explaining the dual-backend graph.
- **F17.** `Zeroizing<[u8; 32]>` return types, and the decoded hex `Vec` is
  `zeroize()`d on both the error and success paths of `load_key` (`crypto.rs:66-74`)
  - the easy-to-miss intermediate is handled.

Other:

- Nonce generation is `getrandom::fill` (`crypto.rs:77-81`) - a CSPRNG, 96-bit
  random nonce, correct for AES-GCM. No `rand::random` anywhere in the key or
  nonce paths.
- `decrypt` length-checks before `split_at(12)` (`crypto.rs:34-36`), so no panic
  on attacker-supplied short ciphertext, and there is a test for it.
- Every authenticated `#[server]` fn in `auth/server.rs` derives the user id from
  the session via `get_current_user()` and passes it to a `db` helper that scopes
  by `user_id` (`db/emails.rs:109-199`: `mark_email_verified`,
  `set_primary_email`, `remove_user_email` all carry `WHERE user_id = $1`). No
  request-supplied user or email id is trusted for authorization - no
  confused-deputy issues found on this surface.
- `ALLOW_INSECURE_DEFAULT_KEY` appears only in `k8s/dev/web-patch.yaml` and
  `scripts/rust-test.sh`; nothing under `k8s/base` or `k8s/prod` sets it, which
  is what WP-35 required.
- No `unwrap()`/`expect()`/`todo!()`/`#[allow(...)]` in any non-test fallible
  path in `auth/server.rs`, `auth/session.rs` or `crypto.rs`. The only `expect`s
  are `main.rs` startup panics, which project rules permit and WP-35 prescribed.
- No secrets in logs: `send_login_email` prints the code to stdout only on the
  `resend == None` dev path (`:86`), and no error message anywhere interpolates
  a token, code or key.

## Coverage gaps

- **No request-parts test harness**, so `confirm_login`, `logout`,
  `logout_everywhere`, `add_email_address`, `confirm_email_address` and every
  other `#[server]` fn in this file are untested end to end; tests reach only
  `confirm_login_inner` and the `db` helpers. This is the direct cause of F-92
  and it also means F-85 could not have been caught by any existing test.
- **`crypto.rs` has no test for `load_key`.** The web crate's three tests cover
  encrypt/decrypt only; the `MissingKey` vs `ALLOW_INSECURE_DEFAULT_KEY` vs
  malformed-hex branches - the entire F16 fix - are unexercised. Ironically
  `rust/bot/src/crypto.rs` *does* test all of them, for the unfixed behaviour
  (F-90).
- **WP-36 and WP-43 have no spec file** in the recovered corpus
  (`git ls-tree 868094a6 .../planning/specs/` confirms). WP-36 is the crypto
  package, so the highest-stakes item in this unit has no written acceptance
  criteria at all - only its own commit message. WP-43 has no reference anywhere
  in the corpus. Reviewed against commit-message claims plus first-principles
  security review; treat the WP-36 verdict as weaker evidence than the WP-34/35
  ones.
- **`rust/web/src/auth/email_addr.rs` (`canonicalize_email`) was not reviewed** -
  it is WP-50/Unit 07's. Every auth entry point now routes user input through it
  before any DB lookup or uniqueness check, so a canonicalization bug is an
  account-takeover bug. Unit 07 must treat it as security-critical, not as an
  email-formatting concern.
- **`rust/web/src/email/inbound.rs:520 find_user_by_settings_token`** is a second
  authentication mechanism (email-borne settings token) outside this unit's
  scope. Unit 07 (WP-44) owns it; nothing in 05a verified it.
- `admin.rs` is the largest consumer of `crypto::encrypt`/`decrypt`
  (18 call sites) and is 05b's - the key-handling behaviour there was not
  reviewed here.
