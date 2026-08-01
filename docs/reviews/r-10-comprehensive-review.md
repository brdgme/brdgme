# R-10 Comprehensive Review - SSE authorization lifetime and task hygiene

**Role:** sole end-of-package reviewer. One broad pass. No code/test/tracker
edits, no commits, no writes other than this file. Inspection was performed
against the actual working tree, not the worker reports.
**HEAD:** `89b0b39c7c82a00832169ee55a7ec37b4d179e24` (matches survey/test-first/
implementation reports; confirmed via `git rev-parse HEAD`).
**Working-tree scope (confirmed via `git status`/`git diff`):** modified
`rust/web/src/events.rs` (production) and `rust/web/tests/sse_events.rs`
(tests) only. Untracked `docs/reviews/r-10-*.md` are the worker reports; the
untracked `R-07-HANDOVER.md` / `R-08-*` files are pre-existing and untouched by
R-10. No other production file, no CI config, no migration, no tracker file,
and no `R-07-HANDOVER.md` was changed. Scope honoured.
**Inputs read in full:** `r-10-survey.md`, `r-10-test-first.md`,
`r-10-implementation.md`, current `rust/web/src/events.rs`, current full
`rust/web/tests/sse_events.rs`, the R-10 plan
(`docs/reviews/2026-07-30-review-session/98-REMEDIATION-PLAN.md:376-411`), and
the referenced APIs: `auth/session.rs`, `visibility_cache.rs`, `websocket.rs`,
`db/visibility.rs`, `db/proposals.rs` (signature), `nats.rs`, `router.rs`.

---

## 0. Verification gate (evidence as supplied; runtime deferred to CI)

Per the standing web-crate ban and the reviewer "no write commands" constraint,
the gate was **not re-run here**; the workers' supplied evidence is recorded.
Both workers report the only permitted gate, exit 0:

```
SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr
```

- Test-first worker (`r-10-test-first.md:14-30`): EXIT=0; lone warning is the
  pre-existing `proc-macro-error2` future-incompat note, nothing from
  `sse_events.rs`.
- Implementation worker (`r-10-implementation.md:18-35`): EXIT=0; same lone
  pre-existing warning, nothing from `events.rs`.

**Runtime tests are deferred to CI**, exactly as R-08/R-09 landed. The new
tests additionally require Postgres and NATS (with `-m 8222` monitoring) which
are unavailable in a plain local run (AGENTS.md; BACKLOG #40). This review is
therefore a static/conformance review; the AC tests are compile-verified only.

---

## 1. Verdict

**ACCEPT.**

All four acceptance criteria are met at the root-cause level; the production
change is correct, minimal, fail-closed, and panic-free on request-reachable
paths; the tests are handler-driven and have plausible pre-fix red behaviour.
No Critical or Important production defects. Four Minor observations and two
test-reliability residual risks are recorded below; all are either intended
behaviour or already documented by the workers and deferred to CI. None block.

---

## 2. AC / spec conformance

| AC | Requirement | Status | Evidence |
|----|-------------|--------|----------|
| AC1 | Test calls the SSE handler, revokes mid-stream, asserts termination; re-validation periodic or event-driven, not connect-only | MET | Handler test `auth_stream_terminates_after_token_revocation` (`sse_events.rs:668-739`) drives a real listener + reqwest; production adds a periodic 30s `interval` re-validation arm (`events.rs:102-103,145-152`) |
| AC2 | Test asserts an idle connection's task + subscription drop within bounded time | MET | `idle_anonymous_connection_releases_task_on_disconnect` (`sse_events.rs:750-792`) asserts the `sse_connections` gauge falls within 10s; production adds a per-connection `CancellationToken` fired from `SseStream::Drop` and selected in both loops (`events.rs:42-59,80-81,153-155,196-197,240-242`) |
| AC3 | Public handler uses `VisibilityCache` and does not subscribe `game.>` unfiltered; reviewer records remaining auth-vs-public differences with justifications | MET | Public handler now builds a per-task `VisibilityCache` and routes the check through it (`events.rs:216,234`), subscribes per-id via `select_all` and never `game.>` (`events.rs:204-214`); test `public_handler_subscribes_per_game_not_firehose` (`sse_events.rs:803-834`); difference table in §4 below |
| AC4 | F-163 `#[ignore]` removed, test runs in CI | MET | `#[ignore]` gone (grep for `ignore` in `sse_events.rs` returns nothing); `sse_stream_survives_past_request_timeout_with_keepalive` now `#[sqlx::test] #[serial]` with unchanged 32s body (`sse_events.rs:551-595`) |

F-131 (concretised as F-158) and F-163 are closed by AC1/AC4. F-123 remains
refuted and was not reopened (each `VisibilityCache` is a per-task local:
`events.rs:101` auth, `events.rs:216` public - one instance = one connection =
one viewer). F-94 rate limiting is correctly out of scope (R-37/edge) and was
not added.

---

## 3. Root-cause correctness (per finding)

### F-158 - session validated once, never re-validated - FIXED AT ROOT
- Connect-time resolution now yields `(viewer, auth_token_id)`, both `Some`
  only when `validate_session_token` returns `Ok(true)` (`events.rs:66-75`).
- The spawn owns `auth_token_id`; a guarded `interval(SESSION_REVALIDATE_PERIOD)`
  arm re-runs `validate_session_token` and `break`s unless `Ok(true)`
  (`events.rs:145-152`). Period 30s (`events.rs:21`) matches the
  `VisibilityCache` TTL (`visibility_cache.rs:8`) and is below the 45s AC1
  bound. `MissedTickBehavior::Delay` avoids catch-up bursts (`events.rs:103`).
- Anonymous preserved: the arm guard `if auth_token_id.is_some()` disables it
  for anonymous connections (never polled, no DB query) (`events.rs:145`).
- This is the survey's prescribed fix (§5 item 2). Correct.

### F-159 - tasks/subscriptions leak past disconnect - FIXED AT ROOT
- New `SseStream<S>` wraps the response stream; its `Drop` fires a
  per-connection `CancellationToken` (`events.rs:42-59`). Axum drops the SSE
  body on client disconnect, firing the token.
- Both loops add `_ = task_disconnected.cancelled() => break`
  (`events.rs:153-155` auth, `240-242` public). Because it is a `select!` arm,
  it wakes an idle/no-event stream with no NATS message required - exactly the
  permanent-leak case (idle anonymous, `tx.send` never reached).
- `SseConnectionGuard` is unchanged and remains a task local; now that the task
  exits on disconnect, the gauge decrements with task exit and becomes truthful,
  and the NATS subscription(s) drop at the same moment (`events.rs:23-36,84,200`).
- Global shutdown arm retained in both loops (`events.rs:156-158,243-245`).
- No `TaskTracker` added; the spawns are unchanged in shape so R-11 can still
  register them (does not preclude R-11). Correct.
- Race check: the token clone is created before spawn and the original is moved
  into the returned stream; cancellation is sticky, so an ultra-fast disconnect
  before the task's first `select!` still resolves `cancelled()` immediately and
  the task breaks. No lost-wakeup race.

### F-160 - public firehose + uncached query - FIXED AT ROOT (in-scope halves)
- Per-id subscribe: one `client.subscribe(format!("game.{id}"))` per requested
  id, merged with `select_all`; never `game.>` (`events.rs:204-214`).
- Cached predicate: per-task `VisibilityCache::default()`; check routed through
  `cache.check_game(game_id, || is_game_publicly_visible(&pool, game_id))`
  (`events.rs:216,234`), mirroring the auth handler and its `pool.clone()`
  closure ownership.
- `select_all` cannot be reached empty: `requested_ids` is guaranteed non-empty
  by the 400 check (`events.rs:189-191`), and a failed subscribe `return`s
  before the merge (`events.rs:208-211`). `select_all` panics only on an empty
  iterator, which is unreachable here. No request-path panic.
- Topic parsing, the 16-cap, and the 400 responses are unchanged
  (`events.rs:176-191`). The `requested_ids.contains` guard is retained as
  defence-in-depth (`events.rs:231`).
- Rate limiting deliberately not added (out of scope; survey §7 D-1). Correct.

### F-163 - `#[ignore]`d regression test - FIXED
- Attribute removed, `#[serial]` added, body and intrinsic 32s wall-clock
  unchanged (`sse_events.rs:551-595`). Reachable in CI (tooth 2 restored).

---

## 4. Authenticated vs public handler - post-change difference table (AC3)

`events_handler` (auth, `events.rs:61-168`) vs `events_public_handler` (public,
`events.rs:170-255`). Every remaining material difference is recorded with a
justification.

| Aspect | Auth (`events_handler`) | Public (`events_public_handler`) | Justified? |
|--------|-------------------------|----------------------------------|------------|
| Session extraction | `tower_sessions::Session` (`:62`) | none | Yes - public is unauthenticated by design (SSE_TOPOLOGY D4) |
| Viewer/token resolution | `get_user_from_session` + `validate_session_token` -> `(viewer, auth_token_id)` (`:66-75`) | none | Yes - public has no auth to resolve |
| Game subscription | `game.>` (`:86`) | per-id `game.{id}` via `select_all` (`:204-214`) | Yes - auth is viewer-scoped and must watch every game the viewer may see (no client topic list for `/events`); public has explicit client-supplied topics (D6/D7). This is the F-160 fix |
| Proposal subscription | `proposal.>` (`:93`) | none | Yes - proposals are never public |
| Visibility cache | `VisibilityCache::default()` (`:101`) | `VisibilityCache::default()` (`:216`) | Converged - both cached now (F-160 fixed) |
| Game visibility predicate | `is_game_visible_to_viewer` (dispatcher: `None`->public, `Some`->user) (`:119`) | `is_game_publicly_visible` direct (`:234`) | Yes - auth serves both anonymous and authenticated viewers so uses the dispatcher; public is always public-only. Both fail closed via the cache. For anonymous `/events` the dispatcher reduces to `is_game_publicly_visible`, so the two public paths now converge on the same predicate AND caching (survey §3 predicate note) |
| Topic filtering | none (viewer-scoped) | `requested_ids.contains(&game_id)` retained (`:231`) | Yes - explicit-topic design; guard is defence-in-depth (by construction every delivered subject is already a requested id) |
| Topic cap | n/a | 16 (`:189-191`) | Yes (D7) |
| Session re-validation | 30s `interval` arm, guarded by `auth_token_id.is_some()` (`:145-152`) | n/a | Yes - F-158 fix; public has no session to re-validate |
| Disconnect detection | per-connection token via `SseStream::Drop` + `task_disconnected.cancelled()` arm (`:80-81,153-155,163-166`) | same (`:196-197,240-242,250-253`) | Converged - both now release on disconnect (F-159 fixed) |
| Shutdown observation | `shutdown.cancelled()` (`:156-158`) | `shutdown.cancelled()` (`:243-245`) | Same |
| Gauge guard | `SseConnectionGuard` (`:84`) | `SseConnectionGuard` (`:200`) | Same |
| Return type | `Sse<...>` infallible (`:65`) | `Result<Sse<...>, StatusCode>` 400 on bad topics (`:174`) | Yes - public validates input |
| KeepAlive | `KeepAlive::default()` (`:167`) | `KeepAlive::default()` (`:254`) | Same |
| Rate limiting | none in-app | none in-app | Yes (out of R-10 scope; R-37/edge; F-94 deferred; F-123 refuted) |

The previously unjustified rows from the survey table (public `game.>` firehose,
public missing cache) are now resolved. All surviving differences are by-design
(auth vs public semantics) or correct predicate/return-type distinctions.

---

## 5. Safety / security / session / visibility / NATS / errors

- **Security posture improved, no new attack surface.** Revoked tokens drop
  within one 30s TTL (fail-closed); idle/disconnected tasks no longer leak;
  anonymous connections no longer decode the whole firehose nor run an uncached
  query per message. Per-id subscribe + cap of 16 bounds anonymous NATS interest
  to known, unguessable game UUIDs the client supplies.
- **Fail-closed on re-validation error.** `_ => break` (`events.rs:149`) ends
  the stream on both `Ok(false)` and `Err`. This is the survey's prescribed
  behaviour (§1: "breaking when it returns anything but `Ok(true)`"); a
  transient DB blip disconnects authenticated streams, but clients auto-reconnect
  and re-auth. See Minor M3.
- **No panics on request-reachable paths.** Re-validation uses `match`
  (`events.rs:147-150`); the token id is handled with `if let Some(..)`
  (`:146`); `select_all` is provably non-empty at call site (§3/F-160); subscribe
  failures `return` from the task (`:88-91,95-98,208-211`). No new `unwrap` in
  production. CODING.md no-panic invariant honoured.
- **Visibility semantics unchanged and converged.** Auth uses the viewer-scoped
  dispatcher; public uses the public predicate; both via `VisibilityCache`
  (30s TTL, 256 cap, fails closed - `visibility_cache.rs:7-8,61-64`). F-123
  stays refuted (per-task cache ownership preserved).
- **NATS semantics sound.** Auth keeps `game.>`/`proposal.>` (viewer-scoped);
  public subscribes exact `game.{id}` subjects. Publish/flush behaviour in the
  broadcaster is untouched (`websocket.rs:36-76`). `ensure_stream_and_consumers`
  touches only `bot.>` jetstream subjects (`nats.rs:121-179`), so it creates no
  `game.>` core subscription that could perturb the AC3 assertion.
- **Cancellation/shutdown.** Per-connection token and the global shutdown token
  coexist as independent `select!` arms in both loops; either terminates the
  task. Drop-ordering race is safe (sticky cancellation, §3/F-159).

---

## 6. Test integrity

All three behavioural AC tests are handler-driven through a real `TcpListener`
+ `axum::serve` + `reqwest` (the `serve_router` helper, `sse_events.rs:204-216`),
not DB-seam-only assertions, and each has a plausible pre-fix red mechanism:

- **AC1** (`:668-739`): seeds a visible game, asserts a frame arrives (proves
  live AND authenticated), revokes via the real seam
  `web::auth::session::invalidate_auth_token` (`auth/session.rs:98-104`), then
  asserts the stream terminates within 45s. Pre-fix: `viewer` captured once,
  never re-validated, `tx.send` keeps succeeding, loop never breaks -> deadline
  exhausted -> fail. Post-fix: 30s arm breaks the loop. The per-iteration
  broadcasts make it robust to either a time-based or event-driven re-check.
- **AC2** (`:750-792`): anonymous `/events`, no games seeded (the permanent-leak
  case), drops the client, asserts the `sse_connections` gauge falls within 10s.
  Pre-fix: anonymous task lives until process shutdown, gauge stays -> fail.
  Post-fix: token fires on stream drop, task exits, guard decrements. Observable
  is the exact metric the finding names as hiding the leak; read through the
  production Prometheus recorder via a `OnceLock`-guarded install
  (`:222-245`) - no test-only production API added.
- **AC3** (`:803-834`): opens `/events/public?topic=game:{A}`, reads NATS
  `subsz?subs=1`, asserts `game.{A}` present and `game.>` absent. Pre-fix:
  handler subscribes `game.>` -> wildcard present -> fail. Post-fix: per-id
  subscribe. Frame-level filtering (`public_events_receives_matching_game_only`,
  `:462-508`) and cache mechanics (`visibility_cache.rs:69-191`) remain valid
  regression guards; behaviour at the frame level is unchanged.
- **AC4** (`:551-595`): scheduling fix only; body unchanged.

**Suite adjustment - `#[serial]` on the whole file** (`:7` import; every test
annotated): necessary, not cosmetic - the AC2 gauge and AC3 `subsz` observables
are process/server-global, and `#[sqlx::test]` otherwise runs tests in parallel
threads within the one binary. `serial_test` is an existing dev-dependency and
`#[sqlx::test] + #[serial]` is an established in-repo pattern. Cost (~80s serial
section, dominated by the intrinsic 32s keepalive and up-to-45s AC1) is
acceptable for a remediation.

**Regression check on existing tests (static):** Group 1 (400s) unchanged;
Group 2 (anonymous 200) unchanged; Group 3 frame delivery preserved (anonymous
`/events` public game via cached public predicate; private-game filtering via
viewer-scoped predicate; proposals gated on `viewer.is_some()`; public per-id
delivery and multi-topic delivery); Group 4 keepalive unchanged (now runs by
default); Group 5 graceful shutdown unchanged (shutdown arm retained). No
existing assertion is invalidated by the change.

---

## 7. Findings

### Critical
None.

### Important
None (production).

### Minor

- **M1 - AC3 test-reliability residual risk (test-only).** The `subsz?subs=1`
  JSON shape (`subscriptions[].subject`) and the client-port+4000 monitoring
  convention could not be verified at runtime locally (web runtime banned).
  Verified statically: NATS monitoring `/subsz?subs=1` does return a
  `subscriptions` array of objects with a `subject` field, and the port
  convention holds (CI `-m 8222` with client 4222; local script 14222->18222).
  The helper panics with a diagnosable message if the endpoint is unreachable or
  unparseable and prints observed subjects on assertion failure
  (`sse_events.rs:260-281,826-833`), so a CI environment failure is
  distinguishable from a real pre-fix failure. There is also a small theoretical
  race: `#[serial]` releases when the prior test's function returns, but a prior
  auth test's detached SSE task tears down its `game.>` subscription
  asynchronously; AC3's substantial setup (DB seeds + NATS connect + 300ms
  sleep) before the `subsz` read makes the window negligible in practice.
  Cross-binary pollution is not a concern: the only `game.>` wildcard subscriber
  in the codebase is the auth handler (`events.rs:86`), reached only via
  `/events`, exercised only by this (serialized) binary; other binaries'
  `game.{id}` subscriptions (`websocket.rs:106`, `game/mod.rs:1205`) are
  specific subjects that do not match AC3's `== "game.>"` / `== "game.{A}"`
  assertions. Documented by the test-first worker (§4). Deferred to CI.
  (`sse_events.rs:249-281,803-834`)

- **M2 - redundant validation at connect for authenticated connections.**
  `tokio::time::interval` fires its first tick immediately, so
  `validate_session_token` runs once at task start in addition to the
  connect-time validation in the handler. One extra primary-key lookup per
  authenticated connection; negligible. Not worth complicating the code to
  avoid. (`events.rs:102,145-152`)

- **M3 - transient DB errors disconnect authenticated streams (intended).**
  `_ => break` ends the stream on a re-validation `Err`. This is the prescribed
  fail-closed behaviour (survey §1) and is secure; recorded only so the
  operational consequence (a DB blip drops live SSE streams, clients
  auto-reconnect) is visible. Not a defect. (`events.rs:147-150`)

- **M4 - unused `Interval` constructed for anonymous connections.** The
  re-validation `Interval` is created unconditionally but never polled for
  anonymous connections (guarded arm). Negligible memory; no timer wakeup since
  it is never awaited. (`events.rs:102-103,145`)

### None
Root-cause correctness, safety/security, session semantics, public
authorization/visibility semantics, task/subscription cancellation, races/drop
timing, NATS semantics, errors/panics, maintainability/readability/simplicity,
and regressions: no defects found. The change is minimal (one module), follows
in-repo patterns (`VisibilityCache`, `CancellationToken`, RAII guard), adds only
*why* comments per CODING.md, and introduces no new dependency.

---

## 8. Scope / constraint compliance

- Only `events.rs` (production) and `sse_events.rs` (tests) modified; no
  tracker, CI config, migration, or `R-07-HANDOVER.md` touched (confirmed via
  `git status`/`git diff`).
- No migration needed and none added (session revocation already deletes
  `user_auth_tokens`; visibility predicates pre-exist).
- F-123 not reopened; F-132 out of scope; rate limiting (F-94) correctly
  deferred to R-37/edge; no `TaskTracker` (R-11) and the spawns remain
  registerable for it.
- Gate evidence as supplied (§0); runtime tests deferred to CI.

---

## 9. Final verdict

**ACCEPT.** R-10 closes F-158, F-159, F-160 (in-scope halves), F-131, and F-163
at the root. The production change is correct, minimal, fail-closed, and
panic-free; the tests are handler-driven with plausible pre-fix red behaviour;
all remaining auth-vs-public differences are justified. The four Minor items and
the AC3 test-reliability residual risk do not block. Runtime confirmation is
deferred to CI per the standing web-crate ban.
