# R-10 Survey - SSE authorization lifetime and task hygiene

**Purpose:** systematic-debugging Phase 1/2 handover for the R-10 implementer.
Evidence-backed root causes, full lifetime trace, authenticated-vs-public
handler diff, comparable working patterns, a minimal hypothesis, and a
test-first execution sequence. **No code changes were made.**

**Status:** pending (`97-REMEDIATION-PROGRESS.md:35`)
**Closes:** F-158 (High), F-159 (Medium), F-160 (Medium), F-131 (Low/Medium,
concretised as F-158), F-163 (Low)
**Size:** M (one module, three defects, plus an `#[ignore]` to remove)
**Depends on:** nothing. **Interacts with:** R-11 (F-109 shutdown drain) and
R-37 (rate limiting / F-94) - see §7.
**HEAD at investigation:** `89b0b39c7c82a00832169ee55a7ec37b4d179e24`
(matches the expected HEAD; no discrepancy).

---

## 0. Tracker path and status

| Artefact | Location |
|----------|----------|
| R-10 spec + acceptance criteria | `docs/reviews/2026-07-30-review-session/98-REMEDIATION-PLAN.md:376-411` |
| R-10 progress row (pending) | `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md:35` |
| Canonical findings (F-158..F-163) | `docs/reviews/2026-07-30-review-session/90-findings-part3.md:9-18` |
| Canonical findings table (section 11) | `docs/reviews/2026-07-30-review-session/99-UNIFIED-REPORT.md:1884-1893` |
| Detailed unit-report evidence | `docs/reviews/2026-07-30-review-session/09-web-frontend-email-sse.md:68-365` |
| F-131 (origin, routed to Unit 09) | `docs/reviews/2026-07-30-review-session/90-findings-part2.md:56` |
| F-94 (no rate limiting anywhere) | `docs/reviews/2026-07-30-review-session/90-findings-part2.md:17` |
| F-109 interaction (shutdown drain) | `docs/reviews/2026-07-30-review-session/90-findings-part2.md:34` |
| Design decisions constraining F-160 | `docs/decisions/SSE_TOPOLOGY.md` (Decisions 3, 4, 7) |

---

## 1. Phase 1 - Root cause investigation (evidence-backed)

All four defects live in `rust/web/src/events.rs` (183 lines, two handlers) plus
one test attribute in `rust/web/tests/sse_events.rs`.

### F-158 (High) - session validated once at connect, never re-validated

- `events_handler` resolves `viewer: Option<Uuid>` **once**, before spawning
  (`events.rs:33-41`). `validate_session_token` (`auth/session.rs:83-95`) is a
  pure existence check: `SELECT id FROM user_auth_tokens WHERE id = $1`.
- `viewer` is moved into the `tokio::spawn` at `:47` and consulted on every
  message (`:81` game, `:96`/`:100` proposal) for the **entire life of the
  task**. Nothing re-runs `validate_session_token`.
- **Revocation mechanism (the thing that must take effect):**
  `invalidate_auth_token` (`auth/session.rs:98-104`) `DELETE`s the
  `user_auth_tokens` row. `logout` (`auth/server.rs:572-594`) and
  `logout_everywhere` (`:596-618`, via `db::invalidate_all_auth_tokens`,
  `db/users.rs:384`) call it. After revocation `validate_session_token` returns
  `Ok(false)` - but the open SSE stream never re-checks, so it keeps delivering
  every event the user could see at connect time.
- **Two staleness windows, only one bounded:** visibility changes are bounded by
  the `VisibilityCache` TTL (`visibility_cache.rs:8`, `TTL = 30s`) - recorded as
  **acceptable**. Session revocation is **unbounded**: `KeepAlive::default()`
  (`events.rs:114`) holds the connection open indefinitely and no maximum
  connection lifetime is imposed. A stolen-then-revoked token is revoked for
  HTTP requests only, not for the live event feed.
- **Suggested fix (unit report `09...:108-112`):** a `tokio::time::interval`
  arm in the existing `select!` re-running `validate_session_token`, breaking
  when it returns anything but `Ok(true)`. The 30s `VisibilityCache` TTL is the
  natural period. Alternative: a maximum connection lifetime forcing reconnect
  (and re-auth).

### F-159 (Medium) - tasks/subscriptions leak past client disconnect

- Both spawned tasks (`events.rs:47-112` auth, `:144-180` public) exit only on:
  NATS subscription ending (`:72`, `:90`, `:160`), global shutdown
  (`:107`, `:175`), or `tx.send(..).is_err()` (`:82`, `:101`, `:170`).
- `tx.send` is reached **only after** a message passes the visibility gate. So a
  disconnected client is not noticed until the next event **that viewer was
  allowed to see** arrives. Until then the task stays alive holding its NATS
  subscription(s), decoding every payload (`:74`, `:92`, `:162`) and running
  `cache.check_game` for every game event in the system (`:81`).
- **Permanent-leak case:** a viewer with no visible games (idle account, or the
  anonymous `viewer: None` case) never reaches `tx.send`, so the task **never**
  terminates - a leak until process shutdown.
- **Metric hides the leak:** `SseConnectionGuard` (`events.rs:13-26`) decrements
  the `sse_connections` gauge only on `Drop` of the spawned task's local, so
  leaked tasks are reported as live connections.
- **Interaction with F-109/R-11:** `efad81f` deleted WP-36's ws F55 shutdown
  drain (`TaskTracker`/`drain_ws_tasks`/bounded wait) and its regression test;
  the SSE replacement reintroduces the same lifecycle family. Unit 05b
  (`05b-web-admin-bot-db.md:409-414`) prescribes: put a `TaskTracker` back on
  `GameBroadcaster`, register both `events.rs` spawns with it, and bound the
  drain after `begin_shutdown()`. **The F-159 per-connection cancellation fix
  and the R-11 `TaskTracker` drain are the same lifecycle problem and should be
  designed together** (R-11 is a separate package; coordinate, do not fold in).
- **Suggested fix (unit report `09...:141-145`):** pass the connection's
  cancellation signal into the task. Axum does not hand the handler a disconnect
  future directly; wrap the returned stream in a guard whose `Drop` fires a
  `CancellationToken`, and `select!` on that token in the loop. This also fixes
  the gauge (the guard drops when the client disconnects, not when a visible
  event happens to arrive).

### F-160 (Medium) - unauthenticated public handler: uncached query + firehose + no rate limit

`events_public_handler` (`events.rs:117-183`), specifically `:147` and `:169`.
Three independent problems, all in the unauthenticated handler:

1. **No `VisibilityCache`.** `events_handler` creates a cache at `:65` and routes
   every check through it; `events_public_handler` calls
   `crate::db::is_game_publicly_visible(&pool, game_id).await.unwrap_or(false)`
   directly at `:169`. Byte-for-byte the same concern, one site hardened and its
   sibling ten lines up left raw - **confirmed pattern 2**. One DB round trip per
   matching message per connection, no dedup across connections.
2. **`game.>` firehose instead of the known subjects.** The handler parses the
   exact `requested_ids` set at `:123-135` *before* subscribing, yet subscribes
   to `game.>` (`:147`) and filters in-process at `:168`. Every anonymous
   connection receives, decodes (`:162`) and discards every game event in the
   system.
3. **No authentication and no rate limiting (confirms F-94).** The router
   middleware stack (`router.rs:163-207`) is: `session_layer`,
   `set_cache_control`, `RequestBodyLimitLayer`, `TimeoutLayer`, `TraceLayer`,
   and the Sentry layers. **There is no rate-limiting middleware anywhere in
   `rust/web`.** Nothing bounds concurrent anonymous SSE connections, so problems
   1 and 2 multiply by attacker-chosen N. Combined with F-159, the connections
   also do not reliably go away.

**Mitigating facts worth recording (do not over-state the amplification):**
- The `requested_ids.contains(&game_id)` guard at `:168` precedes the DB call in
  the `&&` chain, so the query fires only for requested games. DB amplification
  factor is `(16 x event rate x connections)`, **not** `(all games x connections)`.
  Problem 2 still applies to the **decode** path regardless.
- `unwrap_or(false)` at `:169` fails closed, matching `VisibilityCache`'s own
  error policy (`visibility_cache.rs:61-64`) - that part is correct.
- The topic cap of 16 (`events.rs:136-138`) is enforced and tested
  (`sse_events.rs:237-246`).

**Suggested fix (unit report `09...:185-188`):** give the public handler a
`VisibilityCache` (already a per-task local; two-line change), subscribe to the
specific `game.{id}` subjects it parsed, and put a connection/rate limit in front
of the route. **But see §7 - the rate-limit half conflicts with documented design
decisions and needs an owner ruling.**

### F-163 (Low) - regression test `#[ignore]`d, property no longer checked

- `sse_events.rs:456-457`: `#[ignore = "takes 32+ seconds"]` on
  `sse_stream_survives_past_request_timeout_with_keepalive` (`:457-499`).
- It replaces `live_websocket_survives_idle_past_request_timeout`
  (`websocket_hygiene.rs:67-68`), deleted by `efad81f9`; the original was a plain
  `#[sqlx::test]` that ran by default (introduced `0093291`, 2026-07-10, guarding
  `TimeoutLayer` against bounding long-lived connections).
- The test asserts the stream survives past the 30s `REQUEST_TIMEOUT`
  (`router.rs:185-188` `TimeoutLayer`) by receiving SSE keepalive comments. The
  32s wall-clock is intrinsic to asserting "survives past 30s".
- **Explicitly NOT pattern 4e** - the original test predates the programme, so no
  checklist row is falsified. It is the obligation-1 near-miss.
- **Suggested fix (unit report `09...:364-365`):** run it in a nightly/slow CI
  job rather than `#[ignore]`, **or** make the timeout under test configurable so
  the test can run fast by default. AC4 requires the `#[ignore]` removed and the
  test running in CI; if flaky, fix the flake (an `#[ignore]`d regression test is
  a citation present but **not reachable**, tooth 2).

---

## 2. Task / session / subscription lifetime data flow

```
HTTP GET /events (or /events/public)
  -> axum extracts Session + State(PgPool, GameBroadcaster)
  -> events_handler:
       viewer = get_user_from_session(session)          # events.rs:33
                -> validate_session_token(pool, token)  # :35  (ONCE, never repeated)  <- F-158
       (tx, rx) = mpsc::unbounded_channel               # :43
       shutdown = broadcaster.shutdown.clone()          # :44  (global CancellationToken)
       client   = broadcaster.client.clone()            # :45  (async_nats::Client)
       tokio::spawn:                                    # :47  (DETACHED - no JoinHandle kept)
         _guard = SseConnectionGuard (gauge +1)         # :48  (gauge -1 only on task Drop) <- F-159
         game_sub     = client.subscribe("game.>")      # :50  (auth) / :147 (public) <- F-160 firehose
         proposal_sub = client.subscribe("proposal.>")  # :57  (auth only)
         cache = VisibilityCache::default()             # :65  (auth only; per-task local) <- F-160 missing
         loop { select! {
           game_sub.next()      -> visibility gate -> tx.send  # :69-86   (send only if visible)
           proposal_sub.next()  -> visibility gate -> tx.send  # :87-106  (auth only; viewer.is_some)
           shutdown.cancelled() -> break                       # :107     (global shutdown only)
         }}
       return Sse::new(UnboundedReceiverStream::new(rx)).keep_alive(...)  # :114
```

**Lifetime facts:**
- The spawned task is **detached** - no `JoinHandle` is held; nothing bounds it
  except the three `select!` exit conditions. There is **no per-connection
  cancellation token** tied to client disconnect. (Contrast: the WS design WP-36
  had a `TaskTracker` drain, deleted by `efad81f` - F-109.)
- `tx` (the mpsc sender) is moved into the task; `rx` is wrapped into the SSE
  stream returned to axum. When the client disconnects, axum drops the response
  body, which drops `rx` - but the task only observes this **lazily**, as
  `tx.send(..).is_err()`, and only when it next tries to send a *visible* event.
  Dropping `rx` does not wake the `select!`.
- `game_sub`/`proposal_sub` are owned by the task and dropped only when the task
  exits - so a leaked task holds its NATS subscription(s) for the process
  lifetime.
- Global shutdown: `main.rs:131-137` wires `with_graceful_shutdown` to
  `broadcaster.begin_shutdown()` (`websocket.rs:78-80`, cancels the shared
  token). Both handlers observe it (`events.rs:107`, `:175`) - so SIGTERM ends
  the streams and lets axum complete. This is the **only** existing disconnect
  path besides a visible-event send failure.
- The bot-command consumer (`main.rs:72-90`) and email sweeps (`main.rs:97-103`)
  get **no** shutdown signal - that is the R-11/F-109 second half, separate from
  R-10 but sharing the lifecycle theme.

**Session revocation flow (for the F-158 test):**
`login_cookie` (`sse_events.rs:57-84`) inserts a `user_auth_tokens` row and a
session, returning the `id={session_id}` cookie. Revocation = `DELETE FROM
user_auth_tokens WHERE id = $1` (via `invalidate_auth_token`,
`auth/session.rs:98-104`). The reference test `session_token_validation`
(`db/mod.rs:119-159`) already proves `validate_session_token` flips `true ->
false` across an `invalidate_auth_token` call - the F-158 test reuses exactly
this seam, mid-stream.

---

## 3. Authenticated vs public handler - full difference table

`events_handler` (auth, `:28-115`) vs `events_public_handler` (public,
`:117-183`). AC3 requires the implementer to record each remaining difference
with a justification.

| Aspect | `events_handler` (auth) | `events_public_handler` (public) | Justified? |
|--------|-------------------------|----------------------------------|------------|
| Session extraction | `tower_sessions::Session` (`:29`) | none | Yes - public is unauthenticated by design (SSE_TOPOLOGY D4) |
| Viewer resolution | `validate_session_token` once (`:33-41`) | none (`viewer` absent) | Partially - F-158 is that the *auth* handler never re-validates; public has no auth to re-validate |
| Game subscription | `game.>` (`:50`) | `game.>` (`:147`) | **No** - public has the exact `requested_ids` and should subscribe per-id (F-160 problem 2) |
| Proposal subscription | `proposal.>` (`:57`) | none | Yes - proposals are never public |
| Visibility cache | `VisibilityCache::default()` (`:65`) | **none** - direct DB call (`:169`) | **No** - pattern 2 (F-160 problem 1) |
| Game visibility check | `cache.check_game(.., is_game_visible_to_viewer)` (`:81`) | `is_game_publicly_visible(..).unwrap_or(false)` uncached (`:169`) | The *predicate* differs correctly (viewer-scoped vs public-only); the *caching* differs unjustifiably |
| Topic filtering | none (viewer-scoped, all visible games) | `requested_ids.contains(&game_id)` (`:168`) | Yes - public client supplies explicit topics (SSE_TOPOLOGY D6/D7) |
| Topic cap | n/a | 16 (`:136-138`) | Yes (SSE_TOPOLOGY D7) |
| Disconnect detection | none (F-159) | none (F-159) | **No** - both leak |
| Session re-validation | none (F-158) | n/a | **No** for auth |
| Rate limiting | none (F-94) | none (F-94/F-160) | **No** - but see §7 design tension |
| Return type | `Sse<...>` (infallible) | `Result<Sse<...>, StatusCode>` (400 on bad topics) | Yes - public validates input |
| KeepAlive | `KeepAlive::default()` (`:114`) | `KeepAlive::default()` (`:182`) | Same |
| Shutdown observation | `shutdown.cancelled()` (`:107`) | `shutdown.cancelled()` (`:175`) | Same |

**Predicate note:** `is_game_visible_to_viewer` (`db/visibility.rs:122-131`)
dispatches `None -> is_game_publicly_visible`, `Some(v) -> is_game_visible_to_user`.
So the anonymous `/events` path already uses the *public* predicate via the cache;
the dedicated `/events/public` handler calls the same `is_game_publicly_visible`
but **without** the cache. The two public-visibility paths should converge on the
cached predicate.

---

## 4. Phase 2 - Comparable working patterns

- **`VisibilityCache` is the in-repo hardening pattern** (`visibility_cache.rs`).
  Per-task local, 30s TTL, 256-entry LRU cap, fails closed on lookup error
  (`:61-64`). Fully unit-tested (`:69-191`), including a `start_paused = true`
  TTL test (`:123-140`). The F-160 fix is to reuse this exact type in the public
  handler - it is already constructed as a per-task local in the sibling.
- **`CancellationToken` is the in-repo shutdown pattern.** `GameBroadcaster`
  holds one (`websocket.rs:25`), `begin_shutdown` cancels it (`:78-80`), and both
  SSE loops already `select!` on `shutdown.cancelled()` (`events.rs:107`, `:175`).
  The F-159 fix is a *second*, per-connection token of the same kind, fired from
  a stream guard's `Drop`.
- **`SseConnectionGuard` (`events.rs:13-26`)** is the RAII-gauge pattern - a guard
  whose `Drop` does the cleanup. The F-159 disconnect guard is the same shape:
  extend or pair it so `Drop` also fires the per-connection token.
- **`graceful_shutdown_ends_sse_stream_and_server_completes`
  (`sse_events.rs:503-560`)** is the working test pattern for "stream terminates
  on a signal": bind a real `TcpListener`, `axum::serve` with
  `with_graceful_shutdown`, drive a `reqwest` client, poll `stream.next()` against
  a deadline, assert the stream ends. The F-158 and F-159 tests reuse this shape
  (revoke the token / drop the client instead of calling `begin_shutdown`).
- **`session_token_validation` (`db/mod.rs:119-159`)** is the working pattern for
  "token validity flips across invalidation" - the F-158 assertion at the DB seam.
- **`TaskTracker` drain (deleted, F-109/R-11):** the historical WS pattern
  (`05b-web-admin-bot-db.md:409-414`) registered spawns and bounded the drain
  after `begin_shutdown()`. R-11 re-expresses it; the F-159 per-connection token
  is the complementary half (detect disconnect, not just shutdown).

---

## 5. Minimal hypothesis

A single per-connection `CancellationToken`, fired from the SSE stream guard's
`Drop` and `select!`ed in both loops, plus a periodic `validate_session_token`
re-check arm in the auth loop, plus converging the public handler on the cached
predicate and per-id subscriptions, closes F-158/F-159/F-160 at the root (not the
symptom): the task's lifetime becomes bound to the connection's lifetime, the
auth stream's lifetime becomes bound to the session's validity, and the public
stream stops amplifying. F-163 is independent (test scheduling).

**Concretely, the minimal change set:**
1. **F-159 (both handlers):** a guard wrapping the returned `Sse` stream whose
   `Drop` fires a per-connection `CancellationToken`; add a `select!` arm on
   `token.cancelled()` that `break`s the loop. This makes idle/anonymous/disconnected
   viewers drop their task + NATS sub promptly, and makes the `sse_connections`
   gauge truthful. (Coordinate the guard with the existing `SseConnectionGuard`.)
2. **F-158 (auth handler):** a `tokio::time::interval` arm (period ~30s, matching
   the cache TTL) that re-runs `validate_session_token` for the auth token and
   `break`s unless it returns `Ok(true)`. Requires capturing `auth_token_id` (not
   just `viewer: Option<Uuid>`) into the spawn.
3. **F-160 (public handler):** construct a `VisibilityCache` local and route the
   `is_game_publicly_visible` check through it; subscribe to the specific
   `game.{id}` subjects in `requested_ids` instead of `game.>`. Rate limiting is
   deferred to the owner ruling in §7.
4. **F-163:** remove the `#[ignore]`; either move the test to a slow/nightly CI
   job or make `REQUEST_TIMEOUT` configurable so the test runs fast by default.

---

## 6. Test-first execution sequence

Per the plan's ACs (`98-REMEDIATION-PLAN.md:399-411`) and the session's
test-integrity rules (a test must **call the code under test** and **fail against
pre-fix code**). Order is write-test-first per defect.

1. **F-158 test** (AC1) - `sse_events.rs`, new `#[sqlx::test]`:
   - `login_cookie` to get an authenticated cookie; open `/events` via a real
     `TcpListener` + `axum::serve` (reuse the `graceful_shutdown...` test shape,
     `sse_events.rs:503-560`).
   - Seed a game visible to the user; broadcast; assert a frame arrives (proves
     the stream is live and authenticated).
   - Call `invalidate_auth_token(&pool, auth_token_id)` (the revocation seam,
     `auth/session.rs:98-104`) mid-stream.
   - Keep broadcasting visible events; assert the stream **terminates** within a
     bounded time (slightly over the re-validation period). **Must fail pre-fix**
     (today the stream never terminates because `viewer` is captured once).
   - The test must drive the **handler**, not assert on `validate_session_token`
     alone (that DB seam is already covered by `db/mod.rs:119-159`).
2. **F-159 test** (AC2) - `sse_events.rs`, new `#[sqlx::test]`:
   - Open `/events` (anonymous, `viewer: None`, so `tx.send` is never reached -
     the permanent-leak case). Drop the client/body. Assert the task and its NATS
     subscription are dropped within a bounded time.
   - Observation options (implementer picks): the `sse_connections` gauge
     (decremented on guard `Drop`) returns to 0; or a second NATS client asserts
     the subscription count on `game.>` drops; or a `TaskTracker`/`Weak`-based
     handle. The gauge is the metric the finding names as hiding the leak, so
     asserting on it is the most direct.
   - **Must fail pre-fix** (today the anonymous task lives until process shutdown).
3. **F-160 test** (AC3) - `sse_events.rs`, new `#[sqlx::test]`:
   - Assert the public handler no longer subscribes to `game.>` unfiltered: e.g.
     a second NATS client confirms subscriptions exist only for the requested
     `game.{id}` subjects (or that broadcasting an *unrequested* game produces no
     decode/DB activity on the public connection). The existing
     `public_events_receives_matching_game_only` (`:366-413`) already asserts the
     filtering behaviour at the frame level; the new test asserts the
     *subscription* is per-id and the visibility check is cached.
   - Cache reuse: assert repeated broadcasts of the same public game do not issue
     a DB query per message (e.g. via a counting seam, or by asserting the cached
     path - mirror the `VisibilityCache` counting tests at `visibility_cache.rs:87-121`).
   - Reviewer deliverable: the §3 difference table, with a justification per row.
4. **F-163** (AC4) - remove `#[ignore]` at `sse_events.rs:456`; wire the test into
   a CI path that actually runs it (slow/nightly job) or make `REQUEST_TIMEOUT`
   configurable and shorten it under test. Confirm it is no longer skipped.
5. **Gate (allowed):** `SQLX_OFFLINE=true cargo check -p web --all-targets
   --features ssr` must exit 0. Then `cargo clippy -p web --all-targets --features
   ssr -- -D warnings`. **Do not build/test/run web locally** (§8).

---

## 7. Owner decisions needed (ambiguities surfaced, not resolved)

These are genuine conflicts between the findings and documented design decisions;
the implementer must get a ruling rather than guess.

- **D-1 (F-160 rate limiting vs SSE_TOPOLOGY D3/D7).** `SSE_TOPOLOGY.md` Decision 3
  says rate-limit connection **establishment** for `/events` but the **public
  stream is deliberately left UNMATCHED by any edge rate rule** ("navigation
  reopens it constantly"). Decision 7 says the topic cap of 16 "is the only bound
  on what one connection can ask the server to watch." F-160/F-94 want a rate
  limit in front of the public route. **Conflict:** an in-app establishment limit
  on `/events/public` contradicts the documented "deliberately unmatched" design.
  **Question for owner:** does R-10 add in-app establishment limiting to
  `/events/public` (overriding D3), or do the per-id subscription + cached
  predicate + existing cap suffice to bound the load, leaving rate limiting to
  R-37/the edge? The unambiguous halves of F-160 (cache + per-id subscribe) can
  proceed regardless.
- **D-2 (F-160 visibility predicate vs SSE_TOPOLOGY D4).** Decision 4 says the
  public stream "needs no auth and **no visibility predicate**, because public
  game ids are already public." The implementation added `is_game_publicly_visible`
  anyway (uncached). AC3 says use `VisibilityCache`. **Question:** keep the
  predicate but cache it (AC3 literal reading), or drop the predicate per D4
  (design reading)? Keeping-and-caching is the conservative choice and matches the
  authenticated handler; dropping it is what the design document argues for. Record
  the ruling.
- **D-3 (F-159 vs R-11 coordination).** F-159's per-connection cancellation and
  R-11's `TaskTracker` shutdown drain are the same lifecycle family (both descend
  from `efad81f` deleting WP-36's drain). **Question:** implement the
  per-connection token in R-10 and leave the `TaskTracker` drain to R-11, or design
  them together? The plan keeps them as separate packages; the implementer should
  at minimum not preclude R-11's `TaskTracker` registration of the `events.rs`
  spawns.

---

## 8. Repository constraints (from docs/AGENTS.md)

- **Web crate: `cargo check`/`clippy -p web` ALLOWED; build/test/run against web
  BANNED** (owner ruling, `97-REMEDIATION-PROGRESS.md:128`). The new tests are
  **compile-verified only**; runtime behaviour is deferred to CI. This matches how
  R-08 and R-09 landed (tests compile-verified, runtime deferred).
- **Never run `scripts/rust-test.sh` on the developer's laptop** (web-ssr OOMs);
  no workspace-wide cargo. Target `-p web` only.
- **DB tests fail locally without Postgres** (AGENTS.md; BACKLOG #40) and these
  tests also need **NATS** at `NATS_URL` (default `nats://localhost:4222`,
  `sse_events.rs:22-23`). Pre-existing condition; do not chase it.
- **`db.rs`, `game/mod.rs`, `auth/` require tests** (CODING.md). R-10 touches auth
  (session re-validation) - tests are mandatory, and the test-integrity rules
  apply: each test must call the code under test and fail against pre-fix code
  (`98-REMEDIATION-PLAN.md:28-32`).
- **Migrations are immutable** once applied. R-10 needs **no** schema change
  (session revocation already deletes `user_auth_tokens`; visibility predicates
  exist). If a fix tempts a migration, stop and re-scope.
- **No rate-limiting middleware exists in `rust/web`** (F-94, CODING.md request
  invariants). Adding one is a larger decision (see §7 D-1), not a R-10 given.
- **`async-nats` buffers publishes** - call `.flush()` after `.publish()` when
  timing matters (CODING.md). The broadcaster already does (`websocket.rs:52-54`,
  `:73-75`); tests relying on prompt delivery depend on this.
- **No panics in runtime paths; propagate with `?`** (CODING.md). The handlers are
  request-reachable - the re-validation and disconnect paths must not `unwrap`.
- **Comment discipline** (CODING.md): default to no comments; only the *why* for
  non-obvious constraints (e.g. the per-viewer cache-ownership invariant, the
  re-validation period choice).
- **Commit after each completed item, never push** (owner ruling,
  `97-REMEDIATION-PROGRESS.md:16`). Review-dir edits are allowed but never
  delete/move files outside own scope (`:130`).

---

## 9. Settled / refuted - do not re-derive

- **F-123 REFUTED** (the `VisibilityCache` cross-user leak): each `VisibilityCache`
  instance is a plain local inside the per-request spawn at `events.rs:65`; one
  instance = one connection = one viewer. (`98-REMEDIATION-PLAN.md:387-388`,
  `90-findings-part2.md:48`.) Do not re-open.
- **F-132** (Low, **NOT in R-10 scope**): the downgraded remnant of F-123 -
  `VisibilityCache` keys on id alone, correct only because of per-task ownership;
  fix is to doc the invariant or key on `(id, Option<Uuid>)`. Separate item.
- **WP-42 was NOT reverted by the SSE migration** (`efad81f`). Useful negative
  against pattern 4e; do not re-derive.
- **F-163 is NOT pattern 4e** - the original test predates the programme.
- **Visibility-staleness half of F-158 is bounded (~30s TTL) and acceptable**;
  only **session revocation** is unbounded. Do not "fix" the visibility TTL.
- **`efad81f` contains exactly one pattern-4e instance (F-109)**; F-163 is the
  near-miss. Settled by obligation 1.
- The `RouteOutcome` sweep (F-162/F-169) is settled and **out of R-10 scope**
  (R-08/R-09, already done).

---

## 10. File:line reference index

| Item | Location |
|------|----------|
| Auth handler `events_handler` | `rust/web/src/events.rs:28-115` |
| Public handler `events_public_handler` | `rust/web/src/events.rs:117-183` |
| F-158 viewer capture (once) | `rust/web/src/events.rs:33-41` |
| F-158 auth spawn + loop | `rust/web/src/events.rs:47-112` |
| F-159 auth task exits | `rust/web/src/events.rs:72,82,90,101,107` |
| F-159 public task exits | `rust/web/src/events.rs:160,170,175` |
| F-159 gauge guard | `rust/web/src/events.rs:13-26` |
| F-160 firehose subscribe | `rust/web/src/events.rs:147` |
| F-160 uncached visibility query | `rust/web/src/events.rs:169` |
| F-160 topic parse + cap | `rust/web/src/events.rs:123-138` |
| Auth cache construction | `rust/web/src/events.rs:65` |
| KeepAlive (both) | `rust/web/src/events.rs:114,182` |
| `VisibilityCache` (TTL 30s, cap 256) | `rust/web/src/visibility_cache.rs:7-8,33-66` |
| `VisibilityCache` tests (counting seam) | `rust/web/src/visibility_cache.rs:69-191` |
| `GameBroadcaster` + shutdown token | `rust/web/src/websocket.rs:22-81` |
| `begin_shutdown` | `rust/web/src/websocket.rs:78-80` |
| `validate_session_token` | `rust/web/src/auth/session.rs:83-95` |
| `invalidate_auth_token` (revocation seam) | `rust/web/src/auth/session.rs:98-104` |
| `get_user_from_session` / `SessionUser` | `rust/web/src/auth/session.rs:14-20,68-74` |
| `logout` / `logout_everywhere` | `rust/web/src/auth/server.rs:572-618` |
| `invalidate_all_auth_tokens` | `rust/web/src/db/users.rs:384` |
| `is_game_publicly_visible` | `rust/web/src/db/visibility.rs:71-81` |
| `is_game_visible_to_user` | `rust/web/src/db/visibility.rs:93-117` |
| `is_game_visible_to_viewer` (dispatcher) | `rust/web/src/db/visibility.rs:122-131` |
| Router SSE routes | `rust/web/src/router.rs:141-145` |
| Router middleware stack (no rate limit) | `rust/web/src/router.rs:163-208` |
| `TimeoutLayer` (REQUEST_TIMEOUT) | `rust/web/src/router.rs:185-188` |
| main.rs graceful shutdown wiring | `rust/web/src/main.rs:131-137` |
| main.rs bot/sweep tasks (no shutdown - R-11) | `rust/web/src/main.rs:72-103` |
| Test helpers `make_state`/`login_cookie` | `rust/web/tests/sse_events.rs:21-84` |
| Test helpers `read_sse_text`/`sse_request` | `rust/web/tests/sse_events.rs:172-196` |
| F-163 `#[ignore]`d test | `rust/web/tests/sse_events.rs:455-499` |
| Working shutdown-test pattern | `rust/web/tests/sse_events.rs:503-560` |
| Public filtering test (frame-level) | `rust/web/tests/sse_events.rs:366-413` |
| Token-validation DB test (F-158 seam) | `rust/web/src/db/mod.rs:119-159` |
| Design decisions (F-160 constraints) | `docs/decisions/SSE_TOPOLOGY.md` (D3, D4, D7) |
