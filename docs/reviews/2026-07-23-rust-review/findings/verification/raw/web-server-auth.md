# Verification: web-server auth findings F1-F15

Verifier: independent Worker, read-only pass against snapshot
`/home/beefsack/Development/brdgme-review-snapshot/rust` (f8763a5).
Files read in full: `web/src/auth/server.rs`, `web/src/auth/session.rs`;
plus `web/src/db.rs:5120-5165`, `web/migrations/005_login_confirmations.sql`,
`web/migrations/001_initial_schema.sql` (user_emails), `web/src/router.rs`
comments, `web/src/db.rs:2784-2815`, `web/Cargo.toml`, vendored
tower-sessions-core 0.14.0.

## F1 (critical) Concurrent confirm requests bypass the per-code attempt cap

Verdict: CONFIRMED (mechanism); severity ADJUSTED critical -> major.

Evidence: `validate_confirmation_code` (server.rs:354-390) runs entirely on
the bare pool with no transaction and no lock:

- :361-369 `SELECT * FROM login_confirmations WHERE email = $1` via
  `fetch_optional(pool)`
- :374 `if confirmation.attempts >= CONFIRM_MAX_ATTEMPTS_PER_CODE` checked in
  Rust on the fetched snapshot
- :378-386 only on mismatch: `UPDATE login_confirmations SET attempts =
  attempts + 1 WHERE email = $1` via `execute(pool)`

The advisory lock (`pg_advisory_xact_lock`, server.rs:139-142, key at :48)
covers only `request_confirmation_code` (the send path). Nothing in
`confirm_login_inner` (:398-500) locks before `validate_confirmation_code`
is called at :405 - the tx there begins after validation. The review's
locking claim is exactly right.

Precision correction on impact: the bypass is NOT "effectively unbounded".
Every failed attempt does commit `attempts + 1`, so once 10 increments have
committed, all subsequent SELECTs observe `attempts >= 10` and are refused.
The excess guesses are bounded by the number of requests whose SELECT lands
before the 10th increment commits - a race window of milliseconds, further
bounded by the sqlx pool's connection limit and any edge throttling. A
flooding attacker gets a cap of ~10 + (in-flight during the window), i.e.
maybe an order of magnitude or two over 10 per code, against a 1e6 keyspace
with a 1-hour window and re-send caps limiting code rotation. Real atomicity
bug, real cap erosion, but per-code success probability stays small.

Severity: major, not critical. The cap self-heals as increments commit; the
attack multiplies guesses by a bounded constant rather than removing the cap.
(The original text itself allowed a downgrade if edge throttling holds; the
downgrade is warranted even without relying on Cloudflare, on the increment-
accumulation argument above. Per-IP-limit-at-edge-only claim confirmed by
router.rs:163-171 comments.)

Recommendation validity: sound approach, one trap. The proposed
`UPDATE ... SET attempts = attempts + 1 ... RETURNING code, attempts`
increments BEFORE the compare, so the returned `attempts` for the Nth try is
N, not N-1. A naive port of the existing `attempts >= 10` check against the
returned value rejects the 10th attempt even when the code is correct -
current semantics allow 9 failures and a correct 10th. Must be
`attempts > CONFIRM_MAX_ATTEMPTS_PER_CODE` (or compare pre-image). Also note
it now burns an attempt on successful confirms - harmless today because the
row is deleted on success (server.rs:486-489), but `confirm_email_address`
(:842-853) deletes it too, so no live path is affected. Flag the off-by-one
in any fix instructions.

## F2 (major) Email-squatting DoS via add_email_address

Verdict: CONFIRMED. Severity major appropriate (correctness/DoS on signup).

Evidence: `add_email_address` (server.rs:789-831) inserts an unverified row
for any address after only ownership-by-someone-else checks (:809-818);
`insert_unverified_email` (db.rs:2797-2815) is a plain INSERT with no cap,
no expiry, no proof of mailbox control. In `confirm_login_inner`, the
pending branch (server.rs:428-436):

    let pending: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM user_emails WHERE email = $1)")
        ...
    if pending {
        return Err(invalid());
    }

So a valid, mailbox-proving code is rejected with "Invalid or expired token"
whenever any unverified row exists. No cap on rows per account confirmed
(db.rs insert has none; no expiry job exists - migrations show none, and the
opportunistic GC only touches `login_confirmations`). The test
`confirm_login_rejects_pending_unverified_address` (server.rs:1499-1529)
pins this behavior deliberately, but the squatting consequence stands.

Recommendation validity: workable with a caveat. "Delete (or reassign) the
unverified row and proceed with signup" is safe in the squatter case. Edge
case worth noting: a legitimate user mid add-address flow who lands on the
login-confirm path instead of the add-confirm path would have their pending
row deleted and a brand-new second account created for that address -
surprising but not data-losing, and arguably correct (the code proves the
mailbox). Per-account pending cap + expiry suggestions are sound. Not buggy,
but implementers should decide the reassign-vs-delete question consciously.

## F3 (minor) No session ID rotation on login

Verdict: CONFIRMED.

Evidence: `confirm_login` (server.rs:314-339) extracts the session (:318)
and goes straight to `set_user_session` (:325-332); no `cycle_id` call
anywhere in the crate. `cycle_id` exists in the vendored
tower-sessions-core-0.14.0/src/session.rs:843 (`pub async fn cycle_id(&self)
-> Result<()>`). tower-sessions generating IDs server-side (limiting classic
fixation) is external-basis but consistent with the vendored source.

Severity: minor is right (hygiene, needs a cookie-planting primitive).

Recommendation validity: valid; `session.cycle_id().await` before
`set_user_session`, mapped via `internal`, matches the crate API.

## F4 (minor) Blocked-domain check leaks account existence

Verdict: CONFIRMED.

Evidence: server.rs:293-308. For a blocked domain, the verified-exists check
(:295-301) gates a distinctive refusal:

    if !is_existing_verified {
        return Ok(LoginResponse { success: false,
            message: "This email domain is not supported" ... });
    }

Verified addresses on blocked domains fall through to
`request_confirmation_code` and get the uniform "Login email sent". An
unauthenticated caller distinguishes registered vs unregistered addresses on
blocked domains by the response. This is the one carve-out from the
otherwise uniform responses (cooldown/cap both return generic_success,
:170-182).

Severity: minor is right (enumeration limited to blocked/disposable domains).

Recommendation validity: first option (generic success for blocked domains,
suppress the send when unverified) is behavior-preserving and correct. The
alternative "reject uniformly regardless of registration state" would lock
out existing users whose verified address is on a since-blocked domain -
that is a user-visible regression, not just a hardening; flag it as the
inferior option.

## F5 (minor) Transient DB error in get_current_user clears the session

Verdict: CONFIRMED.

Evidence: server.rs:512-524:

    if validate_session_token(&pool, user.auth_token_id)
        .await
        .unwrap_or(false)
    { ... } else {
        // Token invalid, clear session
        let _ = clear_user_session(&session).await;
    }

`validate_session_token` (session.rs:73-85) returns `Result<bool,
sqlx::Error>`; `unwrap_or(false)` maps pool exhaustion / network errors to
"invalid", and the else branch removes the session user. A transient DB
error logs the user out. Severity minor/quality is fair (recoverable by
re-login; not a security hole).

Recommendation validity: valid - propagate the error (500, session intact),
clear only on definitive `Ok(false)`.

## F6 (minor) Global 24h send cap over-counts historical sends

Verdict: CONFIRMED.

Evidence: the cap sums `sent_count` over rows with recent `last_sent_at`
(server.rs:185-192), but the upsert (:205-217) does
`sent_count = login_confirmations.sent_count + 1` unconditionally - the
CASE-guarded rotation resets `code`/`created_at`/`attempts` on expiry but
never `sent_count`. Schema (005_login_confirmations.sql) has no other reset;
GC (:144-149) only deletes rows idle > 24h, so any address re-sent at least
daily carries its whole history into the SUM. The crate's own test pins it:
server.rs:1106 `assert_eq!(row.sent_count, 4, "sent_count keeps
accumulating")`. Severity minor is right (over-counting is fail-closed -
refuses logins, never exceeds the Resend quota).

Recommendation validity: PARTLY INVALID - the first option is buggy.
"Reset `sent_count` with `created_at` on code rotation" makes the global
24h cap under-count: sends made under previous (expired) codes inside the
same 24h window vanish from the SUM. An attacker rotating hourly gets
5 sends/hour x 24h = 120 real Resend sends/day against a 50/day cap -
exactly the quota-burn the cap exists to prevent. Per-email cap semantics
survive (that cap only applies while the code is valid), but the global cap
breaks. The second option (separate windowed counter / append-only sends
log) is the correct fix. Flag option 1 as a defective recommendation.

## F7 (minor) Resend failure silent, consumes quota

Verdict: CONFIRMED.

Evidence: the tx (row upsert + sent_count bump) commits at server.rs:225-227
before `send_login_email(resend, email, &code).await` at :229;
`send_login_email` returns `()` and on error only
`tracing::error!("Failed to send login email...")` (:101-103). `login()`
then returns `generic_success` (:231). During a Resend outage the per-email
cap (5, :177) is consumed by failed sends, after which :177-182 silently
suppresses until the code expires. Severity minor/quality fair.

Recommendation validity: directionally valid. "Don't count provider-failed
sends against the cap" needs care in implementation (either send inside the
tx - which would hold the global advisory lock across network I/O, bad - or
compensating decrement after failure); "surface a retryable failure" is the
simpler safe form. Not buggy as written since it prescribes goals, not code.

## F8 (minor) Turnstile fails open; unset secret silent

Verdict: CONFIRMED with one wording adjustment.

Evidence: server.rs:235-256. Unset/empty secret: `if secret.is_empty() ||
token.is_empty() { return secret.is_empty(); }` (:236-238) - empty secret
returns true, verification disabled; no startup warning exists (checked
main.rs has none for this var; login() reads it per-call at :265 with
`unwrap_or_default()`). Verifier network error: returns `true` (:251-254).

Adjustment: the fail-open on verifier error is NOT fully silent - :252 logs
`tracing::warn!("Turnstile verification failed (fail-open): {e}")`, and the
"(fail-open)" text shows it is a deliberate choice. The genuinely silent
case is the unset secret. Finding substance stands; severity minor stands.

Recommendation validity: valid (metric via axum_prometheus already in use
at :88/:197, startup warn, explicit fail-open/closed decision).

## F9 (minor) Emails not normalized (case/whitespace)

Verdict: CONFIRMED.

Evidence: `login` (server.rs:264-311), `confirm_login` (:315),
`add_email_address` (:789-831) all use the raw client string; only the
domain is lowercased, and only for the blocked check (:293, :800). Schema:
`login_confirmations.email TEXT PRIMARY KEY`
(005_login_confirmations.sql:11) and `user_emails_email_key UNIQUE (email)`
on plain `text` (001_initial_schema.sql:274-275) - both case-sensitive
Postgres text, so `Alice@x.com` and `alice@x.com` are distinct rows and can
become distinct accounts. The uncertainty note about client-side
normalization is honest (out of unit scope; not checked here either).

Recommendation validity: valid (trim + lowercase at entry points; citext
longer-term - citext migration would need to handle any existing
case-duplicate rows, worth a note but not wrong).

## F10 (minor) add_email_address discards the send-cap refusal

Verdict: CONFIRMED.

Evidence: server.rs:828-830:

    let resend = expect_context::<Option<resend_rs::Resend>>();
    request_confirmation_code(&pool, resend.as_ref(), &email).await?;
    Ok(())

`request_confirmation_code` returns `Ok(LoginResponse { success: false,
message: "Logins are temporarily limited..." })` on the global cap
(:193-202); `?` only propagates `Err`, so the response is dropped and the
fn returns `Ok(())` after the unverified row was already inserted (:820).
UI reports success; no email was sent. Severity minor is right.

Recommendation validity: valid. Note the unverified row remains either way;
combined with the cooldown there is no add-flow re-send path, so the user
must wait for the cap window - propagating the message at least tells them.

## F11 (minor) Auth tokens never expire DB-side

Verdict: CONFIRMED.

Evidence: `validate_session_token` (session.rs:73-85) is a pure existence
check (`SELECT id FROM user_auth_tokens WHERE id = $1`); deletion only in
`invalidate_auth_token` (session.rs:88-94) called from logout
(server.rs:539). The db.rs test (in the `session_token_validation` test,
around db.rs:5140) explicitly sets `created_at = NOW() - INTERVAL '40
days'` and asserts validation still passes, with a comment stating expiry
is cookie-side only. No revoke-all path exists in the crate. Severity minor
with "defense-in-depth gap, not active exposure" framing is accurate - the
test documents it as accepted design.

Recommendation validity: valid and correctly hedged as optional.

## F12 (nit) Modulo bias / non-constant-time compare / plaintext codes

Verdict: CONFIRMED.

Evidence: server.rs:204 `format!("{:06}", rand::random::<u32>() %
1_000_000)` - 2^32 mod 1e6 = 967296, so 967296 residues occur 4295 times vs
4294 for the rest: relative bias ~= 1/4294 ~= 0.023%, matching the claimed
~0.02%. ThreadRng-is-CSPRNG is external-basis (rand crate docs) and
standard. server.rs:378 `if confirmation.code != token` - ordinary string
compare, not constant-time; the attempt cap and network jitter make this
academic, as the finding says. Codes stored plaintext: :205-217 inserts
`code` directly; schema `code CHAR(6)`. Nit severity correct on all three.

Recommendation validity: valid for this codebase - web/Cargo.toml pins
`rand = "0.10.2"`, which carries the 0.9+ API (`rand::rng()`,
`random_range`), so `rand::rng().random_range(0..1_000_000)` compiles and
is exact.

## F13 (nit) confirm_login has no shape validation

Verdict: CONFIRMED.

Evidence: `confirm_login` (server.rs:314-339) goes session-extract ->
`confirm_login_inner` with no checks on `email`/`token`; contrast `login`
(:273-289) which checks empty/@/plus-addressing. Harmless (lookup miss
returns the uniform invalid error, :359/:369). Nit correct.

Recommendation validity: valid; the digit/length guard matches the CHAR(6)
code format and returns the same uniform message, so no oracle is added.

## F14 (nit) New reqwest::Client per Turnstile call

Verdict: CONFIRMED.

Evidence: server.rs:239 `let client = reqwest::Client::new();` inside
`verify_turnstile_token`, called per `login()` (:266). A shared client is
provided via context - used at :883
(`expect_context::<reqwest::Client>()` in `make_email_address_active`).
Nit correct.

Recommendation validity: valid with a placement note -
`verify_turnstile_token` is a plain async fn, so the `expect_context` call
belongs in `login()` (server-fn scope) with the client passed in as a
parameter, or relies on the owner scope propagating into the awaited call.
Passing it in is the robust form.

## F15 (nit) Logout swallows invalidation error; no session flush

Verdict: CONFIRMED.

Evidence: server.rs:539 `let _ = invalidate_auth_token(&pool,
user.auth_token_id).await;` - error dropped, fn proceeds to
`clear_user_session` and returns `Ok(true)`. `clear_user_session`
(session.rs:67-70) only does `session.remove::<SessionUser>(...)`; the
tower-sessions record itself lives until the 30-day inactivity expiry
(session.rs:38). Impact correctly bounded (each confirm mints a fresh token,
server.rs:476-484). Nit correct.

Recommendation validity: valid; `session.flush()` exists in tower-sessions
0.14 and both suggestions are optional-grade, matching the severity.

## Summary of deviations from the prior review

- F1: severity critical -> major (increments accumulate; bypass is a bounded
  multiplier, not unbounded). Recommendation has an off-by-one trap
  (increment-before-compare changes the cap comparison to `>`).
- F6: recommendation option 1 (reset sent_count on rotation) is itself buggy
  - it breaks the global 24h cap (up to 120 sends/day vs 50). Option 2 is
  the correct fix.
- F4: recommendation's second alternative (uniform rejection) would lock out
  existing verified users on blocked domains - inferior option, flag it.
- F8: minor wording fix - verifier-error fail-open is warn-logged, not
  silent; only the unset secret is silent.
- F14: recommendation should pass the shared client into
  `verify_turnstile_token` rather than expect_context inside a plain fn.
- All 15 findings verified as substantively real; none rejected.
