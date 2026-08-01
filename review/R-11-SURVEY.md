# R-11 Survey - Shutdown drain: bookkeeping + ws F55 second half

**Purpose:** factual handover for the R-11 implementer. No code changes made.
**HEAD at investigation:** `4ca8aa9f6391b23c62ce80b4f7d821cefb523361`
(confirmed via `git rev-parse HEAD`).

---

## 0. Tracker path and status

| Artefact | Location |
|----------|----------|
| R-11 progress row (pending) | `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md:36` |
| Owner ruling 6.3b | `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md:125` |
| R-11 spec + acceptance criteria | `docs/reviews/2026-07-30-review-session/98-REMEDIATION-PLAN.md:415-446` |
| F-109 canonical row | `docs/reviews/2026-07-30-review-session/90-findings-part2.md:34` |
| F-109 detailed evidence | `docs/reviews/2026-07-30-review-session/05b-web-admin-bot-db.md:345-415` |
| WP-84 spec §3g (successor proof) | git history `868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/WP-84-sse-migration.md:226-248` |
| WP-36 checklist reference | git history `868094a6:docs/reviews/2026-07-23-rust-review/planning/checklists/T3-B6-outbound-email-websocket.md:98-101` |
| R-10 survey (R-11 interaction) | `docs/reviews/r-10-survey.md:83-90,382-388,479` |
| R-10 comprehensive review (no TaskTracker, deliberate) | `docs/reviews/r-10-comprehensive-review.md:102-103` |
| R-10 evidence (tracker row) | `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md:176-235` |

**Owner ruling 6.3b (2026-07-31):** "Implement it (shutdown signal for bot
consumer + email sweep, with tests)." This converts the "unsized second half"
into scoped work. The bookkeeping half (AC1) remains mandatory.

**R-11 tracker row wording (current):**
`| R-11 | pending | | implement ws F55 second half (owner ruling) |`

---

## 1. Root cause - F-109 (three problems)

### 1a. Bookkeeping: WP-36's fix and test silently deleted

Commit `13a1e693` (WP-36) added to `rust/web/src/websocket.rs`:
`ws_tasks: TaskTracker`, `track_future`, `drain_ws_tasks`, `ws_tasks.close()`
in `begin_shutdown`, and a 5s bounded drain in `main.rs`. It also added
`rust/web/tests/websocket_hygiene.rs` with a real-client close-frame test.

Commit `efad81f9` (WP-84, SSE migration) deleted all of it:
- `TaskTracker`, `drain_ws_tasks`, `ws_handler`, `handle_socket`, `Message::Close` - gone from `websocket.rs`
- `rust/web/tests/websocket_hygiene.rs` - deleted (153 lines)
- `main.rs` 5s `timeout(.., drain_ws_tasks())` - deleted
- `tokio-util` `features=["rt"]` (added for TaskTracker) - removed from `web/Cargo.toml`

WP-36's checklist row (T3-B6, "Not in this checklist" section, line 98-101)
still reads: "already shipped by specs/WP-36-crypto-deploy-hardening.md; the
GameBroadcaster::begin_shutdown / drain_ws_tasks fns and handle_socket's
shutdown select arm exist in live source. Do not disturb them." The referenced
spec file does not exist in the recovered corpus. No WP-36 checklist file with
tracked rows exists (confirmed: `git ls-tree 868094a6 .../planning/checklists/`
has no WP-36 entry; `git ls-tree 868094a6 .../planning/specs/` has no WP-36 spec).

WP-84 spec §3g **anticipated** this deletion and required a proof test before
deleting the TaskTracker. The proof test exists:
`rust/web/tests/sse_events.rs:503-560` (the graceful-shutdown test, now at
`:551-595` post-R-10 with `#[serial]`). So the deletion was spec-sanctioned.
The bookkeeping fix is: amend the WP-36 reference to record the deletion and
name the WP-84 §3g proof test as successor.

### 1b. SSE spawns are detached (bounded by R-10, not by a tracker)

Post-R-10 (`a9ea19d`), both SSE handlers in `events.rs` have:
- Per-connection `CancellationToken` fired by `SseStream::Drop` (`:42-59`)
- `task_disconnected.cancelled()` select arm (`:153-155` auth, `:240-242` public)
- Global `shutdown.cancelled()` select arm (`:156-158` auth, `:243-245` public)

R-10 deliberately did NOT add a `TaskTracker` - the spawns are unchanged in
shape so R-11 can still register them (`r-10-comprehensive-review.md:102-103`).
The concrete harm F-109 cites (detached SSE spawns with nothing bounding the
drain) is partially mitigated: the shutdown token ends each stream, and axum's
`with_graceful_shutdown` waits for in-flight requests. But a task blocked in
`client.subscribe(...)` at shutdown is neither cancelled nor waited for, and
nothing bounds the total drain time.

### 1c. Bot consumer and email sweep get no shutdown signal (the open half)

`rust/web/src/main.rs:72-103` spawns three task families with no shutdown path:

1. **bot-command supervisor** (`:72-90`): `tokio::spawn` wrapping
   `supervise_consumer("bot-command", ...)`. `supervise_consumer`
   (`nats.rs:258-297`) is an infinite restart loop with no cancellation check.
   `run_bot_command_consumer` (`game/mod.rs:262-360`) is a
   `while let Some(message) = messages.next().await` loop with no shutdown arm.
   SIGTERM kills it mid-`execute_command` with no drain - the mechanism behind
   F-101's five-minute post-deploy bot stall.

2. **max-deliveries-advisory supervisor** (`:91-96`): same shape,
   `supervise_consumer("max-deliveries-advisory", ...)`.

3. **Email sweeps** (`:97-103`): `spawn_periodic_sweeps` calls six
   `spawn_sweep` invocations (`sweep.rs:605-618`). `spawn_sweep`
   (`sweep.rs:322-336`) is a bare `tokio::spawn` with an infinite
   `loop { tick.tick().await; run().await; }` and no shutdown signal.

The only `CancellationToken` in the crate is `GameBroadcaster.shutdown`
(`websocket.rs:25`), consumed solely by `events.rs`. None of these tasks
observe it.

---

## 2. Current architecture (post-R-10)

### Startup (`main.rs:72-103`)

```
tokio::spawn(supervise_consumer("bot-command", run_bot_command_consumer))  // :72-90
tokio::spawn(supervise_consumer("max-deliveries-advisory", ...))           // :91-96
spawn_periodic_sweeps(pool, resend, http_client, broadcaster, jetstream)   // :97-103
  -> 6x spawn_sweep(name, interval, closure)                               // sweep.rs:322-336
```

All JoinHandles discarded. No CancellationToken threaded.

### Shutdown (`main.rs:127-139`)

```
axum::serve(listener, app)
  .with_graceful_shutdown(async {
      shutdown_signal().await;       // SIGTERM/SIGINT
      broadcaster.begin_shutdown();  // cancels GameBroadcaster.shutdown token
  })
  .await
```

`begin_shutdown` (`websocket.rs:78-80`) is only `self.shutdown.cancel()`.
No drain, no tracker, no bounded wait. After `with_graceful_shutdown`
completes (all SSE streams ended), `main` returns and the process exits,
killing all remaining spawned tasks.

### SSE task lifecycle (`events.rs`)

Both handlers: `tokio::spawn` with select loop over NATS subs, revalidation
(auth only), `task_disconnected.cancelled()`, `shutdown.cancelled()`.
`SseStream` wraps the response; its `Drop` fires the per-connection token.
`SseConnectionGuard` decrements the gauge on task exit.

---

## 3. Acceptance criteria (from 98-REMEDIATION-PLAN.md:435-445)

1. **AC1 (bookkeeping):** WP-36's checklist reference amended to record that
   its fix and test were deleted by `efad81f`, with the WP-84 §3g proof test
   named as successor. Tooth-4 amendment.

2. **AC2 (second half):** Bot consumer and email sweep tasks get a shutdown
   signal, implemented with a test that **calls each task's shutdown path**.
   Owner ruling 6.3b mandates implementation (not just recording a gap).

3. **AC3 (SSE bound):** Detached SSE spawns are bounded (the concrete harm
   F-109 cites). R-10's per-connection token + shutdown arm already handle
   the normal case; the residual is a task blocked in `client.subscribe()`
   at shutdown. A `TaskTracker` + bounded drain closes this.

---

## 4. Test seams and minimal test-first approach

### Existing test infrastructure

- `rust/web/tests/sse_events.rs`: `#[sqlx::test]` integration tests with real
  Axum router, NATS, and Postgres. Helpers: `make_state` (`:21-50`),
  `login_cookie` (`:52-84`), `sse_request` (`:172-196`).
- Graceful-shutdown proof test (WP-84 §3g successor):
  `sse_events.rs:551-595` (`sse_stream_survives_past_request_timeout_with_keepalive`,
  `#[serial]`). This is the test that proves SSE streams end on shutdown.
- `nats.rs:299-413`: unit tests for `supervise_consumer` (restart behaviour,
  backoff). Uses `tokio::spawn` + `AtomicUsize` counters.
- `sweep.rs:620-674`: unit tests for `parse_duration`, thresholds. No
  shutdown-path tests.

### Test-first approach for AC2

The shutdown signal must be testable without a full process. The natural seam:

- **`supervise_consumer`:** add a `CancellationToken` parameter (or a
  `shutdown: CancellationToken` field). Test: spawn with a token, cancel it,
  assert the supervisor exits (does not restart). The existing test pattern
  (`nats.rs:372-413`) already spawns `supervise_consumer` in a task and asserts
  restart counts - extend with a cancellation arm.

- **`spawn_sweep`:** same shape - add a `CancellationToken`, select on it
  alongside `tick.tick()`. Test: spawn with a short interval, cancel, assert
  the task exits within a bound.

- **`run_bot_command_consumer`:** the `while let Some(message)` loop needs a
  `select!` on a shutdown token alongside `messages.next()`. Test seam:
  `handle_bot_command_event` is already split out (`game/mod.rs:370`) for
  direct testing; the consumer loop itself needs a NATS integration test or
  a mock stream.

- **`main.rs` wiring:** thread `broadcaster.shutdown.clone()` (or a new
  process-level token) into all three spawn sites. After
  `begin_shutdown()`, await a bounded drain (5s, matching WP-36's original
  bound).

### Test-first approach for AC3

Register the `events.rs` spawns with a `TaskTracker` on `GameBroadcaster`.
After `begin_shutdown()`, `main.rs` awaits
`timeout(5s, tracker.wait())`. The existing shutdown proof test
(`sse_events.rs:551-595`) already exercises the graceful-shutdown path;
extend or add a test that asserts the drain completes within the bound.

### Constraints

- **Web crate: `cargo check`/`clippy -p web` ALLOWED; build/test/run BANNED**
  (owner ruling, `97-REMEDIATION-PROGRESS.md:128`). Tests are compile-verified
  only; runtime deferred to CI.
- `tokio-util` `features=["rt"]` was removed by `efad81f` (it was added for
  `TaskTracker`). Re-adding it to `web/Cargo.toml` is required if AC3 uses
  `TaskTracker`. Check `Cargo.lock` for the current `tokio-util` version.
- No panics in runtime paths (CODING.md). Shutdown paths must not `unwrap`.
- `db.rs`, `game/mod.rs`, `auth/` require tests (CODING.md). R-11 touches
  `game/mod.rs` (bot consumer) - tests mandatory.

---

## 5. R-10 SSE evidence (committed)

Commit `a9ea19d5e9f4640b8d6cafe64068fbcbbbe6cf3c`:
- `rust/web/src/events.rs`: 110+/21- (SseStream, per-connection token,
  session re-validation, per-id public subscribe)
- `rust/web/tests/sse_events.rs`: 278+ (revocation test, idle-disconnect
  gauge test, per-id subscribe test, F-163 un-ignore)

R-10 comprehensive review (`docs/reviews/r-10-comprehensive-review.md`):
ACCEPT, no Critical/Important. Explicitly notes: "No TaskTracker added; the
spawns are unchanged in shape so R-11 can still register them (does not
preclude R-11). Correct." (`:102-103`)

R-10 survey D-3 ruling (`docs/reviews/r-10-survey.md:382-388`): "implement
the per-connection token in R-10 and leave the TaskTracker drain to R-11...
the implementer should at minimum not preclude R-11's TaskTracker
registration of the events.rs spawns."

---

## 6. Relevant existing worktree changes

Untracked files (not staged, not committed):
- `docs/reviews/2026-07-30-review-session/R-07-HANDOVER.md`
- `docs/reviews/2026-07-30-review-session/R-08-CONTEXT-HANDOVER.md`
- `docs/reviews/2026-07-30-review-session/R-08-REVIEW.md`
- `docs/reviews/r-10-comprehensive-review.md`
- `docs/reviews/r-10-implementation.md`
- `docs/reviews/r-10-survey.md`
- `docs/reviews/r-10-test-first.md`

These are prior R-package artefacts. Per owner ruling
(`97-REMEDIATION-PROGRESS.md:130`): "agents must never delete/move files or
changes outside their own work scope - leave unrelated working-tree changes
alone." R-11 must not touch these.

---

## 7. Proposed serial task breakdown

### T1: Bookkeeping (AC1) - S

Amend the WP-36 reference in the recovered checklist
(`T3-B6-outbound-email-websocket.md:98-101`, accessible only in git history
at `868094a6`). Since the checklist lives in git history and not the current
tree, the amendment goes in the R-11 tracker evidence section of
`97-REMEDIATION-PROGRESS.md`: record that WP-36's ws F55 fix and
`websocket_hygiene.rs` were deleted by `efad81f9`, that WP-84 §3g anticipated
this and required a proof test, and that the successor proof is
`sse_events.rs:551-595` (the graceful-shutdown/keepalive test). This is the
tooth-4 amendment the plan requires.

### T2: Shutdown token plumbing (AC2) - M

1. Add a process-level `CancellationToken` (or reuse
   `GameBroadcaster.shutdown`) and thread it into:
   - `supervise_consumer` (`nats.rs:258`): add a `shutdown` parameter, select
     on it in the restart loop, break instead of restarting.
   - `spawn_sweep` (`sweep.rs:322`): add a `shutdown` parameter, select on it
     alongside `tick.tick()`.
   - `run_bot_command_consumer` (`game/mod.rs:262`): add a `shutdown`
     parameter, select on it alongside `messages.next()`.
2. Update `main.rs:72-103` to pass the token.
3. Update `spawn_periodic_sweeps` signature to accept the token.

### T3: Bounded drain in main.rs (AC2 + AC3) - S

After `broadcaster.begin_shutdown()` in the `with_graceful_shutdown` future
(`main.rs:131-137`), await a bounded drain of the background tasks. Options:
- `TaskTracker` (re-add `tokio-util` `features=["rt"]` to `web/Cargo.toml`),
  register all spawns, `timeout(5s, tracker.wait())`.
- Or: hold `JoinHandle`s and `timeout(5s, join_all(handles))`.

The `TaskTracker` approach matches WP-36's original design and the 05b
remediation prescription (`05b-web-admin-bot-db.md:409-415`).

### T4: Tests (AC2 + AC3) - M

- `supervise_consumer` shutdown test: cancel token, assert exit (no restart).
- `spawn_sweep` shutdown test: cancel token, assert task exits within bound.
- `run_bot_command_consumer` shutdown test: cancel token with a mock/empty
  NATS stream, assert the loop exits.
- SSE drain bound test: register events.rs spawns with tracker, trigger
  shutdown, assert drain completes within 5s.
- All tests compile-verified only (web test/run banned); runtime deferred to CI.

### T5: Tracker update + commit - S

Update `97-REMEDIATION-PROGRESS.md` R-11 row to done with commit SHA and
evidence. Commit per owner ruling (commit after each item, never push).

---

## 8. File:line reference index

| Item | Location |
|------|----------|
| main.rs bot-command spawn | `rust/web/src/main.rs:72-90` |
| main.rs advisory spawn | `rust/web/src/main.rs:91-96` |
| main.rs sweep spawn | `rust/web/src/main.rs:97-103` |
| main.rs graceful shutdown | `rust/web/src/main.rs:127-139` |
| `shutdown_signal` | `rust/web/src/main.rs:227-247` |
| `GameBroadcaster` + shutdown token | `rust/web/src/websocket.rs:22-81` |
| `begin_shutdown` | `rust/web/src/websocket.rs:78-80` |
| `supervise_consumer` | `rust/web/src/nats.rs:258-297` |
| `run_bot_command_consumer` | `rust/web/src/game/mod.rs:262-360` |
| `handle_bot_command_event` (test seam) | `rust/web/src/game/mod.rs:370-384` |
| `spawn_periodic_sweeps` | `rust/web/src/email/sweep.rs:605-618` |
| `spawn_sweep` | `rust/web/src/email/sweep.rs:322-336` |
| SSE auth handler | `rust/web/src/events.rs:61-168` |
| SSE public handler | `rust/web/src/events.rs:170-255` |
| `SseStream` (per-connection token) | `rust/web/src/events.rs:42-59` |
| `SseConnectionGuard` | `rust/web/src/events.rs:23-36` |
| SSE shutdown proof test (WP-84 §3g) | `rust/web/tests/sse_events.rs:551-595` |
| SSE test helpers | `rust/web/tests/sse_events.rs:21-84,172-196` |
| `supervise_consumer` tests | `rust/web/src/nats.rs:299-413` |
| sweep unit tests | `rust/web/src/email/sweep.rs:620-674` |
| F-109 evidence | `docs/reviews/2026-07-30-review-session/05b-web-admin-bot-db.md:345-415` |
| WP-84 §3g | git `868094a6:.../planning/specs/WP-84-sse-migration.md:226-248` |
| WP-36 checklist ref | git `868094a6:.../planning/checklists/T3-B6-outbound-email-websocket.md:98-101` |
| efad81f9 (SSE migration) | `efad81f92b0a1f585410e6f30fdd8de8a3dac518` |
| a9ea19d (R-10) | `a9ea19d5e9f4640b8d6cafe64068fbcbbbe6cf3c` |
| 13a1e693 (WP-36, ws F55) | referenced in `05b-web-admin-bot-db.md:347` |
