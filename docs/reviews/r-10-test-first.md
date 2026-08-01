# R-10 Test-First Worker Report - SSE authorization lifetime and task hygiene

**Role:** R-10 test-first worker. Tests written BEFORE any production change.
**Scope honoured:** tests only. No production files, tracker files, CI config, or
`R-07-HANDOVER.md` touched. No commit, no push.
**Source survey:** `docs/reviews/r-10-survey.md` (read in full; HEAD matches
`89b0b39c7c82a00832169ee55a7ec37b4d179e24`).
**Single edited file:** `rust/web/tests/sse_events.rs` (+276 / -2).

---

## 0. Verification gate (the only command permitted)

```
SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr
```

**Result: EXIT=0.** Output:

```
Checking web v0.1.0 (/home/beefsack/Development/brdgme/rust/web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.89s
warning: the following packages contain code that will be rejected by a future version of Rust: proc-macro-error2 v2.0.1
```

The lone warning is the pre-existing `proc-macro-error2` future-incompat note,
unrelated to this change. No warnings or errors originate from `sse_events.rs`.
Per the web-crate ban, no `cargo test/build/run/clippy/fmt` and no
`scripts/rust-test.sh` were run; runtime behaviour is deferred to CI exactly as
R-08/R-09 landed.

---

## 1. Acceptance-criteria tests added

All three are real integration tests that drive the actual handlers through a
real `TcpListener` + `axum::serve` + `reqwest` client (the working pattern from
`graceful_shutdown_ends_sse_stream_and_server_completes`, now factored into the
`serve_router` helper). None asserts on a DB seam alone; each calls the code
under test.

| AC | Test | Finding | Observable |
|----|------|---------|------------|
| AC1 | `auth_stream_terminates_after_token_revocation` | F-158 | response stream ends (output-side) |
| AC2 | `idle_anonymous_connection_releases_task_on_disconnect` | F-159 | `sse_connections` gauge falls back (existing metric) |
| AC3 | `public_handler_subscribes_per_game_not_firehose` | F-160 | NATS `subsz` subscription subjects |
| AC4 | `sse_stream_survives_past_request_timeout_with_keepalive` | F-163 | `#[ignore]` removed; body unchanged |

### AC1 - `auth_stream_terminates_after_token_revocation` (F-158)

- `login_cookie_with_token` (new helper; `login_cookie` now delegates to it)
  returns the `auth_token_id` so the test can revoke mid-stream via the real
  seam `web::auth::session::invalidate_auth_token` (`auth/session.rs:98-104`).
- Seeds a game the user can see, opens authenticated `/events`, broadcasts, and
  asserts a visible game frame arrives (proves the stream is live AND
  authenticated, not merely 200).
- Revokes the token, then keeps broadcasting visible events and asserts the
  stream **terminates**.

**Pre-fix failure mechanism (precise):** `events_handler` resolves
`viewer: Option<Uuid>` once before the spawn (`events.rs:33-41`) and never
re-runs `validate_session_token`. After revocation the captured `viewer` is
still `Some(..)`, every visible broadcast still passes the gate and
`tx.send(..)` still succeeds (the client is still reading), so the loop never
breaks and the stream never ends. The 45s bounded deadline is exhausted with
`stream_ended == false` -> assertion fails. (The DB seam itself is already
covered by `db/mod.rs:119-159`; this test deliberately exercises the handler.)

**Condition-based wait:** poll `stream.next()` under a 500ms per-iteration
timeout until `Ok(None)`/`Ok(Some(Err(_)))` (stream ended) or a 45s deadline.
Broadcasting visible events each iteration makes the test robust to either
approved fix shape - a time-based re-validation `select!` arm OR a per-visible-
event re-check - both terminate the stream. **Constraint for the implementer:**
the re-validation period must be < 45s. The survey's natural period is the 30s
`VisibilityCache` TTL (`visibility_cache.rs:8`), which fits; this test then runs
~30-35s. If a shorter period is chosen the test runs faster - its semantic
intent (stream ends after revocation) is unchanged.

### AC2 - `idle_anonymous_connection_releases_task_on_disconnect` (F-159)

- Opens an **anonymous** `/events` connection with **no games seeded**, so the
  viewer is `None` and `tx.send` is never reached - the exact permanent-leak
  case from the survey (`09-web-frontend-email-sse.md`, F-159).
- Drops the client/body, then asserts the `sse_connections` gauge falls back by
  one within a bounded deadline.

**Observable chosen and why:** the `sse_connections` gauge is the metric the
finding explicitly names as hiding the leak, incremented in
`SseConnectionGuard::new()` and decremented on the guard's `Drop`
(`events.rs:13-26`). The guard is a local in the spawned task, so the gauge
decrements exactly when the task exits - and the task owns the NATS
subscription(s), which drop at the same moment. Observing the gauge therefore
observes "the task and its subscription have gone away." It is read in-process
through the production Prometheus recorder (see §3); **no test-only production
API was added.**

**Pre-fix failure mechanism (precise):** the anonymous task exits only on NATS
sub end, global shutdown, or `tx.send(..).is_err()` (`events.rs:72,82,90,101,
107`). With no visible games there is no send; with no disconnect signal there
is no other exit. The task lives until process shutdown, the guard never drops,
the gauge stays at `peak`, and the 10s deadline is exhausted with
`released == false` -> assertion fails.

**Condition-based wait:** snapshot `before`; open; assert `peak >= before + 1`
(connection counted); drop client; poll every 100ms until
`gauge <= peak - 1.0` or a 10s deadline. The peak-relative delta isolates this
connection from any constant offset (e.g. tasks leaked by earlier tests pre-fix).
**Constraint for the implementer:** disconnect must be detected promptly (the
guard-`Drop`/`CancellationToken` shape from survey §5), not solely on the 15s
`KeepAlive` write - the deadline is 10s.

### AC3 - `public_handler_subscribes_per_game_not_firehose` (F-160)

- Opens a real `/events/public?topic=game:{A}` connection, then queries the NATS
  monitoring `subsz?subs=1` endpoint and asserts the server holds a subscription
  on the specific `game.{A}` subject and **none** on the `game.>` wildcard.

**Pre-fix failure mechanism (precise):** `events_public_handler` subscribes to
`game.>` (`events.rs:147`) and filters in-process at `:168`. The `subsz` output
therefore contains a `game.>` subscription, so the "no `game.>`" assertion fails.
Post-fix (per-id subscribe) the wildcard is absent and `game.{A}` is present.

**Condition-based wait:** a fixed 300ms settle after the 200 (subscription is
created synchronously in the spawn before the loop), then a single `subsz`
read. No polling loop is needed because the subscription subject is stable.

**What the existing suite already covers (kept, not duplicated):**
- Frame-level filtering: `public_events_receives_matching_game_only`
  (`sse_events.rs`) already asserts an unrequested game and a private game
  produce no frame. This is a suitable regression guard for the approved
  implementation's observable filtering behaviour and is preserved.
- Cache mechanics: the `VisibilityCache` unit tests
  (`visibility_cache.rs:69-191`, including the counting-seam TTL tests) fully
  cover the cache the public handler must adopt.

**Why there is no frame-level failing test for AC3:** per-id subscription vs
firehose+filter, and cached vs uncached visibility, are behaviourally identical
at the frame level (same frames delivered either way). They are internal
properties; the only way to encode "must not subscribe to `game.>`" as a
pre-fix-failing test is to observe the subscription, which is what the `subsz`
assertion does. See §4 for the residual risk and the alternative.

### AC4 - `#[ignore]` removed from the keepalive test (F-163)

- Removed `#[ignore = "takes 32+ seconds"]`; added `#[serial]` (see §2). The
  test body and its 32s wall-clock are **unchanged** - the 32s is intrinsic to
  asserting "survives past the 30s `REQUEST_TIMEOUT`" (`router.rs:31,185-188`).
  It now runs by default in CI (reachable, tooth 2 restored).

---

## 2. Test-suite adjustment: `#[serial]` on the whole file

Added `#[serial]` (`serial_test`, already a dev-dependency; pattern matches
`nats_bot_eventing.rs` / `inbound_webhook.rs`) to every test in the file.

**Why it is necessary, not cosmetic:** the AC2 gauge and AC3 `subsz` observables
are process/server-global. `#[sqlx::test]` runs tests in parallel threads within
the one `sse_events` test binary, and every SSE-opening test spawns a task that
increments the same `sse_connections` gauge (and, pre-fix, leaks a `game.>`
subscription). Concurrent connections would perturb both observables and make
the assertions racy. `#[serial]` (a process-global lock) guarantees no other
test in the binary runs during an observability measurement. The five Group-1
400-tests do not spawn tasks but are serialized too for uniformity; they are
sub-millisecond so the cost is nil.

**Cost:** the file now runs sequentially. Wall time is dominated by the two
intrinsically long tests - the 32s keepalive (AC4) and the up-to-45s F-158 test
(AC1) - so the serial section is ~80s rather than ~45s hidden behind the longest
test. Acceptable for a remediation; the owner may later move the long tests to a
slow/nightly job (survey §6.4) independently of this change.

---

## 3. How the gauge is read in-test (no production change)

`events.rs` records `sse_connections` through the global `metrics` facade, which
is a no-op until a recorder is installed. `build_router` deliberately does NOT
install one (main.rs:117-121 wraps the layer on outside, "so
`metrics::set_global_recorder` is only ever called once per process"). The test
installs the same production recorder via `axum_prometheus::PrometheusMetricLayer
::pair()`, which calls `metrics::set_global_recorder` (verified in
`axum-prometheus-0.10.0/src/lib.rs:892`) and returns a `PrometheusHandle` whose
`.render()` yields the current gauge value. Because `set_global_recorder` panics
on a second call, the install is guarded by a process-wide `std::sync::OnceLock`
(`metrics_handle()`); `gauge_value()` parses the `sse_connections <n>` line from
the Prometheus text render. This reuses an existing production mechanism, not a
test-only API.

---

## 4. Implementation constraints, discoveries, and residual risk

**Constraints the production fix must satisfy (from the approved design + survey):**
- Public handler MUST use `VisibilityCache` (route the `is_game_publicly_visible`
  check at `events.rs:169` through a per-task `VisibilityCache::default()`,
  mirroring `events.rs:65`).
- Public handler MUST NOT subscribe to `game.>`; subscribe to the specific
  `game.{id}` subjects in `requested_ids` (parsed at `events.rs:123-135`).
- Rate limiting is OUT OF SCOPE for R-10 (survey §7 D-1; deferred to R-37/edge).
- F-158 fix: capture `auth_token_id` (not just `viewer`) into the spawn and add a
  re-validation arm that breaks unless `validate_session_token` returns
  `Ok(true)`. Period < 45s (AC1 deadline); ~30s TTL is the survey's suggestion.
- F-159 fix: a per-connection `CancellationToken` fired from the SSE stream
  guard's `Drop`, `select!`ed in BOTH loops; coordinate with the existing
  `SseConnectionGuard` so the gauge becomes truthful. Disconnect detection must
  be prompt (< 10s, AC2 deadline), i.e. not solely on the 15s keepalive write.
- Do not preclude R-11's `TaskTracker` registration of the `events.rs` spawns
  (survey §7 D-3); F-159 per-connection cancellation and R-11 drain are the same
  lifecycle family but separate packages.
- No migration needed; no panics in the request-reachable re-validation /
  disconnect paths (propagate with `?`); comment discipline per CODING.md.

**Discoveries:**
- `serial_test` is already a dev-dependency and `#[sqlx::test]` + `#[serial]` is
  an established in-repo pattern - no new dependency required.
- `PrometheusMetricLayer::pair()` installs the global recorder as a side effect
  (confirmed from crate source), making the gauge readable in-test without any
  production hook.
- NATS monitoring (`-m 8222`) is enabled in BOTH CI (`.github/workflows/ci.yml`,
  ports `4222:4222` and `8222:8222`) and the local script
  (`scripts/rust-test.sh`, `14222:4222` and `18222:8222`). The monitoring host
  port is consistently the client port + 4000; `nats_monitor_base()` derives it
  from `NATS_URL` on that convention.

**Residual risk on AC3 (flagged, not hidden):** the `subsz?subs=1` JSON shape
(`subscriptions[].subject`) and the port+4000 convention could not be verified
at runtime here (web runtime tests are banned locally). The helper panics with a
diagnosable message if the endpoint is unreachable or the JSON is unparseable,
and the assertion prints the observed `game.*` subjects on failure, so a CI
failure is distinguishable from a real pre-fix failure. **If the owner prefers
not to depend on the NATS monitoring endpoint**, the clean alternative is a tiny
production observability exposure - e.g. exposing the active SSE subscription
subjects (or an `AtomicUsize` connection count) on `GameBroadcaster` - which the
implementation worker can add and this test can switch to. I did NOT add any such
hook, per the task constraint.

**AC2 gauge robustness note:** the gauge is process-global; correctness here
relies on the §2 serialization. A per-connection observable (the same optional
`GameBroadcaster` exposure as above) would make AC2 bulletproof without
serializing the file and is the recommended follow-up if the serial CI cost is
deemed too high.

---

## 5. TDD ordering and why each test fails pre-fix (summary)

Written before any production change; each fails against current production for
the reason stated, and passes once the corresponding approved fix lands:

- **AC1** fails because `viewer` is captured once and never re-validated
  (`events.rs:33-41`) -> stream never terminates after revocation.
- **AC2** fails because the idle anonymous task has no disconnect exit
  (`events.rs:67-111`) -> task/gauge/subscription never release.
- **AC3** fails because the public handler subscribes to `game.>`
  (`events.rs:147`) -> wildcard subscription present.
- **AC4** is a scheduling fix (attribute removal), not a behaviour change.

---

## 6. Blockers

None. All four ACs are delivered as compiling tests; the only judgement calls
(AC3 monitoring dependence, AC2 global-gauge serialization) are documented above
with concrete alternatives for the implementation worker / owner. Runtime
validation is deferred to CI per the standing web-crate ban.
