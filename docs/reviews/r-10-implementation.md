# R-10 Implementation Worker Report - SSE authorization lifetime and task hygiene

**Role:** R-10 implementation worker. Production fix only.
**HEAD:** `89b0b39c7c82a00832169ee55a7ec37b4d179e24` (matches survey/test-first).
**Scope honoured:** edited `rust/web/src/events.rs` only. No test rewrite (the
test-first worker's `rust/web/tests/sse_events.rs` is untouched and its intent
preserved). No tracker files, CI config, migrations, or `R-07-HANDOVER.md`
touched. No commit, no push. F-123 not re-opened.
**Inputs read in full:** `docs/reviews/r-10-survey.md`,
`docs/reviews/r-10-test-first.md`, then the current `events.rs`,
`visibility_cache.rs`, `auth/session.rs`, `websocket.rs`, `db/visibility.rs`,
`router.rs` (SSE routes), `Cargo.toml`, and the full `tests/sse_events.rs`.

---

## 0. Verification gate (the only command permitted)

```
SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr
```

**Result: EXIT=0.** Output:

```
Checking web v0.1.0 (/home/beefsack/Development/brdgme/rust/web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.72s
warning: the following packages contain code that will be rejected by a future version of Rust: proc-macro-error2 v2.0.1
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
```

The lone warning is the pre-existing `proc-macro-error2` future-incompat note,
unrelated to this change (present in the test-first gate too). No warnings or
errors originate from `events.rs` or `sse_events.rs`. Per the web-crate ban, no
`cargo build/test/run/clippy/fmt`, no workspace cargo, no `scripts/rust-test.sh`;
runtime behaviour is deferred to CI exactly as R-08/R-09 landed.

---

## 1. Root-cause-to-change mapping

All changes are in `rust/web/src/events.rs`.

### F-158 / AC1 - session validated once, never re-validated

- **Root cause:** `events_handler` resolved `viewer: Option<Uuid>` once before
  the spawn and never re-ran `validate_session_token`; a revoked token kept
  delivering events on the open stream indefinitely.
- **Change:**
  - Capture the auth-token id alongside the viewer: the connect-time resolution
    now yields `(viewer, auth_token_id): (Option<Uuid>, Option<Uuid>)`, both
    `Some` only when `validate_session_token` returns `Ok(true)`, both `None`
    otherwise (anonymous or invalid). The spawn now owns `auth_token_id`, not
    merely the resolved viewer state.
  - Added a `tokio::time::interval(SESSION_REVALIDATE_PERIOD)` arm to the auth
    loop's `select!`, guarded by `if auth_token_id.is_some()`. On each tick it
    re-runs `validate_session_token(&pool, token_id)` and `break`s unless the
    result is `Ok(true)` (errors and `Ok(false)` both end the stream).
  - `SESSION_REVALIDATE_PERIOD = 30s`, matching the `VisibilityCache` TTL (the
    survey's natural period) and below the 45s AC1 regression bound.
  - `MissedTickBehavior::Delay` so a slow re-validation query shifts the
    schedule rather than bursting catch-up ticks.
- **Anonymous preserved:** for an anonymous `/events` connection
  `auth_token_id` is `None`, so the guarded re-validation arm is disabled
  (never polled, no wasted wakeups, no DB query); only a real existing token is
  re-validated. `viewer: None` still flows through the existing public
  visibility path unchanged.

### F-159 / AC2 - tasks/subscriptions leak past client disconnect

- **Root cause:** both spawned tasks exited only on NATS-sub end, global
  shutdown, or a *visible* `tx.send(..).is_err()`; a disconnected client was not
  noticed until the next event the viewer was allowed to see arrived. An idle
  anonymous task (no visible games) never reached `tx.send` and leaked until
  process shutdown, holding its NATS subscription(s) and reporting a false
  `sse_connections` gauge.
- **Change:**
  - New per-connection `CancellationToken` (`disconnected`), cloned into the
    task as `task_disconnected`, created in BOTH handlers.
  - New `SseStream<S>` wrapper around the returned `UnboundedReceiverStream`.
    It implements `Stream` by delegating `poll_next` to the inner stream, and
    its `Drop` fires `disconnected.cancel()`. Axum drops the SSE body (and thus
    this wrapper) when the client disconnects, so the token fires on disconnect.
  - Both loops add a `select!` arm `_ = task_disconnected.cancelled() => break`.
    Because it is a `select!` arm, it wakes the loop even on an idle/no-event
    stream (no NATS message required), so idle anonymous connections release
    promptly.
  - The existing `SseConnectionGuard` is unchanged and remains a task local: it
    decrements the `sse_connections` gauge on task `Drop`. Now that the task
    exits on disconnect (via the token), the gauge decrements with task exit and
    becomes truthful. The NATS subscription(s), owned by the task, drop at the
    same moment.
  - Global shutdown semantics preserved: the `shutdown.cancelled()` arm is
    retained in both loops alongside the new disconnect arm.
  - **No `TaskTracker` added** - R-11 owns the shutdown drain. The per-connection
    token does not preclude R-11 registering these spawns with a `TaskTracker`
    later (the spawns are unchanged in shape; only their exit conditions grew).

### F-160 / AC3 - public handler: uncached query + `game.>` firehose

- **Root cause:** `events_public_handler` subscribed to `game.>` and filtered
  in-process, and called `is_game_publicly_visible` directly (uncached) - the
  sibling of the hardened auth handler left raw.
- **Change (public handler only):**
  - Instantiate a per-task `VisibilityCache::default()` and route the public
    visibility check through `cache.check_game(game_id, ||
    crate::db::is_game_publicly_visible(&pool, game_id))`, mirroring the auth
    handler's cached pattern (and its `let pool = pool.clone();` closure
    ownership). Per-request cache ownership is preserved (one cache = one
    connection = one task local; F-123 stays refuted).
  - Subscribe to exactly the requested `game.{id}` subjects (one
    `client.subscribe(format!("game.{id}"))` per id in `requested_ids`), merged
    with `futures_util::stream::select_all`. Never subscribes to `game.>`.
  - Topic parsing, the 16-topic cap, and the 400 responses are unchanged. The
    `requested_ids.contains(&game_id)` guard is retained (defence-in-depth; by
    construction every delivered subject is already a requested id). Public
    filtering behaviour is otherwise identical.
  - `select_all` cannot be reached empty: `requested_ids` is guaranteed
    non-empty by the earlier 400 check, and a failed subscribe `return`s before
    the merge - so no request-path panic.
  - **Rate limiting deliberately NOT added** (out of scope for R-10; survey §7
    D-1, deferred to R-37/the edge).

### F-163 / AC4 - regression test `#[ignore]`

- Test-only; already completed by the test-first worker (`#[ignore]` removed,
  `#[serial]` added, body unchanged). I did not alter it and preserved its
  semantic intent.

---

## 2. Code paths (post-change)

```
events_handler (auth):
  connect: (viewer, auth_token_id) = resolve+validate once   # both Some only if valid
  disconnected = CancellationToken::new(); task gets a clone
  spawn:
    _guard = SseConnectionGuard (gauge +1)
    game_sub = subscribe("game.>")          # auth still watches all games (viewer-scoped)
    proposal_sub = subscribe("proposal.>")
    cache = VisibilityCache::default()
    revalidate = interval(30s, Delay)
    loop select! {
      game_sub.next()      -> cache.check_game(is_game_visible_to_viewer) -> tx.send
      proposal_sub.next()  -> cache.check_proposal(is_proposal_visible_to_user) -> tx.send
      revalidate.tick(), if auth_token_id.is_some()
                           -> validate_session_token; break unless Ok(true)   # F-158
      task_disconnected.cancelled() -> break                                  # F-159
      shutdown.cancelled()          -> break                                  # preserved
    }
  return Sse::new(SseStream { inner: rx, disconnected }).keep_alive(..)
    # SseStream::Drop fires disconnected.cancel() on client disconnect

events_public_handler (public):
  parse requested_ids (cap 16, 400 on bad/empty/over-cap)     # unchanged
  disconnected = CancellationToken::new(); task gets a clone
  spawn:
    _guard = SseConnectionGuard (gauge +1)
    subs = [subscribe("game.{id}") for id in requested_ids]   # F-160: per-id, never game.>
    game_sub = select_all(subs)
    cache = VisibilityCache::default()                         # F-160: cached predicate
    loop select! {
      game_sub.next() -> contains guard -> cache.check_game(is_game_publicly_visible) -> tx.send
      task_disconnected.cancelled() -> break                   # F-159
      shutdown.cancelled()          -> break                   # preserved
    }
  return Ok(Sse::new(SseStream { inner: rx, disconnected }).keep_alive(..))
```

---

## 3. Test compatibility (no test edits; intent preserved)

- **AC1 `auth_stream_terminates_after_token_revocation`:** the 30s re-validation
  arm breaks the loop after revocation, ending the stream. Worst case ~30s after
  revocation, inside the test's 45s deadline. The test also broadcasts visible
  events each iteration, so the loop is exercised; termination is driven by the
  time-based arm regardless. Handler-driven (real listener + reqwest), as
  required.
- **AC2 `idle_anonymous_connection_releases_task_on_disconnect`:** dropping the
  client drops the `SseStream`, firing the token; the idle anonymous loop wakes
  on `task_disconnected.cancelled()` and breaks; the task exits and the
  `SseConnectionGuard` decrements `sse_connections`. Prompt (well inside the 10s
  deadline), not dependent on the 15s keepalive write.
- **AC3 `public_handler_subscribes_per_game_not_firehose`:** the public handler
  now subscribes only to `game.{A}` (via `select_all` over per-id subs) and never
  to `game.>`, so the NATS `subsz` assertion (specific subject present, wildcard
  absent) holds. Frame-level filtering (`public_events_receives_matching_game_only`)
  and cache mechanics (`VisibilityCache` unit tests) remain valid - behaviour at
  the frame level is unchanged.
- **AC4:** untouched.
- Existing Group 1-5 tests: the auth handler still subscribes to `game.>` /
  `proposal.>` and uses the same cached viewer-scoped predicates, so frame
  delivery, anonymous-200, keepalive, and graceful-shutdown behaviour are
  unchanged. The only auth-loop additions are the guarded re-validation arm
  (inactive for anonymous) and the disconnect arm (only fires on drop/shutdown).

---

## 4. Constraints satisfied / decisions

- No request-path `unwrap`/panic: re-validation uses `match` (`Ok(true)` vs
  `_ => break`); the token id is handled with `if let Some(..)`; `select_all`
  is provably non-empty at call site.
- Per-request cache ownership preserved (per-task local in each handler).
- No migration, no new dependency (`tokio-util`/`CancellationToken`,
  `futures_util::stream::select_all`, and `VisibilityCache` are all already in
  use). No `TaskTracker` (R-11). Rate limiting out of scope (R-37).
- Comment discipline: only *why* comments added (re-validation period choice,
  the `SseStream` Drop mechanism, the per-id subscribe rationale).
- F-123 not re-opened; F-132 out of scope.

## 5. Constraints / blockers

None. The fix is compile-verified by the allowed gate; runtime validation is
deferred to CI per the standing web-crate ban. The AC3 residual risk flagged by
the test-first worker (dependence on the NATS `subsz` monitoring endpoint and
the client-port+4000 convention) is unchanged by this implementation - the
production fix supplies exactly the per-id subscription that test asserts; no
alternative observability hook was added (per scope).
