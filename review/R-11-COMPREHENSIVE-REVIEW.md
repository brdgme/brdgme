# R-11 Comprehensive Review - Shutdown drain: bot consumer + email sweeps

**Role:** sole end-of-package reviewer for R-11. One broad independent pass.
No code/test/tracker edits, no commits, no pushes, no banned web commands
(build/test/run/clippy/fmt). Inspection performed against the actual committed
tree and the actual diff, not the worker reports.

**Commit under review:** `13ab0ffd3896f3b0804997a36b2b24a02c2c8147`
(`fix(web): drain background tasks on shutdown (R-11, F-109)`), confirmed as
`HEAD` via `git rev-parse HEAD`.

**Diff scope (confirmed via `git show --stat`):**
- `rust/web/src/nats.rs` (+193/-...)
- `rust/web/src/email/sweep.rs` (+128/-...)
- `rust/web/src/game/mod.rs` (+252/-...)
- `rust/web/src/main.rs` (+67/-...)
- `docs/reviews/2026-07-30-review-session/97-REMEDIATION-PROGRESS.md` (1 line, the R-11 row)

`rust/web/src/websocket.rs` and `rust/web/src/events.rs` are NOT in the diff
(verified). No `TaskTracker` reintroduced; `tokio-util` `rt` feature not
re-added (only the already-present `sync` feature / `CancellationToken` is
used). No migration, no CI config, no `Cargo.toml` change.

**Inputs read in full:** `review/R-11-SURVEY.md`, `review/R-11-IMPLEMENTATION.md`,
`review/R-11-COMMIT.md`; the R-11 spec/AC (`98-REMEDIATION-PLAN.md:415-446`);
the tracker + owner rulings (`97-REMEDIATION-PROGRESS.md:36,118-130,176-235`);
F-109 canonical row (`90-findings-part2.md:34`) and detailed evidence
(`05b-web-admin-bot-db.md:345-415`); WP-84 §3g
(git `868094a6:.../planning/specs/WP-84-sse-migration.md:226-248`); the R-10
comprehensive review (`docs/reviews/r-10-comprehensive-review.md`); and current
`main.rs`, `nats.rs`, `email/sweep.rs`, `game/mod.rs`, `events.rs`,
`websocket.rs`, `tests/sse_events.rs`.

---

## 0. Verification gate (fresh evidence)

Ran the single allowed command from `rust/`:

```
SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr
```

**Exit 0.** Only diagnostic is the pre-existing, unrelated `proc-macro-error2
v2.0.1` future-incompat warning (present on the pristine baseline; nothing from
the R-11 files). Corroborates the implementation worker's GREEN evidence
(`R-11-IMPLEMENTATION.md:42-56`). No new `unused`/`unreachable`/`dead_code`
warning surfaced. Runtime tests remain deferred to CI per the standing
web-crate ban (`97-REMEDIATION-PROGRESS.md:128`).

---

## 1. Verdict

**CONDITIONAL ACCEPT.**

The committed code is correct, minimal, panic-free, and fully implements the
owner-mandated work (AC2). The cancellation/drain sequencing is sound and the
new tests call the real production shutdown paths. No Critical findings.

Two Important findings, neither a functional defect in the committed code:

- **I1 (AC1):** the tooth-4 amendment records the deletion correctly but names
  the WRONG successor test - it cites the keepalive test
  (`sse_events.rs:551-595`) instead of the actual WP-84 §3g proof test
  (`graceful_shutdown_ends_sse_stream_and_server_completes`, `sse_events.rs:601-657`).
  One-line doc correction required before AC1 is fully met.
- **I2 (AC3):** R-10's committed mechanism bounds the SSE drain for the normal
  case, but a task blocked in `client.subscribe()` at shutdown under a broken
  NATS connection is not bounded in-code (axum's graceful shutdown has no
  timeout). Narrow residual, acknowledged by the survey; owner confirmation
  recommended. Not fixable by the prescribed `TaskTracker` at its placed
  location either.

AC status: **AC1 PASS-with-defect** (deletion recorded; successor citation must
be corrected), **AC2 PASS**, **AC3 PASS** (normal case bounded by R-10, no
TaskTracker reintroduced) **with documented residual**.

A **targeted re-review is required, doc-only**: confirm the AC1 citation is
corrected from `:551-595` to `:601` (and the test name updated). No code
re-review needed.

---

## 2. AC / spec conformance

| AC | Requirement | Status | Evidence |
|----|-------------|--------|----------|
| AC1 | WP-36 row amended to record its fix + test (`websocket_hygiene.rs`) deleted by `efad81f`, with the WP-84 §3g proof test named as successor (tooth-4) | MET-with-defect | Deletion correctly recorded at `97-REMEDIATION-PROGRESS.md:36` (full SHA `efad81f92b0a...`); deletion independently confirmed (`websocket_hygiene.rs` absent; `efad81f9 --stat` deletes it, 153 lines). **Defect:** the named successor is `sse_events.rs:551-595` (`sse_stream_survives_past_request_timeout_with_keepalive`, the Group-4 keepalive test), but the §3g proof test is `graceful_shutdown_ends_sse_stream_and_server_completes` at `sse_events.rs:601-657` (Group 5). See I1. |
| AC2 | Bot consumer + email sweep get a shutdown signal, implemented with a test that calls each task's shutdown path (owner ruling 6.3b) | MET | Process token threaded into `supervise_consumer` (`nats.rs:280`), `run_bot_command_consumer`/`run_bot_command_consume_loop` (`game/mod.rs:263,311`), `run_max_deliveries_advisory_listener` (`nats.rs:214`), and all six sweeps (`sweep.rs:324,635`). Tests call the real production shutdown paths: `bot_command_consume_loop_exits_on_shutdown` (`game/mod.rs:1284`), `sweep_stops_on_shutdown` (`sweep.rs:1736`), `supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down` (`nats.rs:467`), `supervisor_backoff_sleep_is_interrupted_by_shutdown` (`nats.rs:517`). |
| AC3 | Detached SSE spawns are bounded (the concrete harm F-109 cites) | MET (normal case) + residual | R-10's per-connection token (`events.rs:42-59`) + global `shutdown.cancelled()` arms (`events.rs:156-158,243-245`) + axum `with_graceful_shutdown` bound an in-loop SSE task promptly; proven by `graceful_shutdown_ends_sse_stream_and_server_completes` (`sse_events.rs:601-657`). No `TaskTracker` reintroduced (correct per task constraint). Residual: a task blocked in `client.subscribe()` (`events.rs:86,93,206`) under broken NATS is not bounded in-code. See I2. |

Owner ruling 6.3b (`97-REMEDIATION-PROGRESS.md:125`, "Implement it (shutdown
signal for bot consumer + email sweep, with tests)") is satisfied. The
max-deliveries-advisory listener - a third task family F-109 §1c identifies but
AC2 does not name - is also wired up (bonus completeness).

---

## 3. Cancellation / drain sequencing (the core of the review)

Shutdown order in `main.rs`:

1. `shutdown_signal().await` - SIGTERM/SIGINT (`main.rs:157`).
2. `broadcaster.begin_shutdown()` - cancels `GameBroadcaster.shutdown`
   (`main.rs:158` -> `websocket.rs:78-80`); this is the token the SSE loops
   observe.
3. `shutdown.cancel()` - cancels the process-level token (`main.rs:159`);
   this is the token the consumers/sweeps observe.
4. The `with_graceful_shutdown` future completes; axum stops accepting and
   drains in-flight requests (SSE bodies end when their task drops `tx`).
5. `axum::serve(..).await` returns (`main.rs:162-163`).
6. Bounded drain: `timeout(5s, join_all(background_tasks))` (`main.rs:173-184`);
   on timeout, warn and abandon.

**Sequencing is correct.** Steps 2 and 3 fire the two tokens back-to-back, so
SSE tasks (observe `broadcaster.shutdown`) and consumers/sweeps (observe the
process token) begin winding down concurrently with axum's drain. By the time
step 6 runs, the consumers/sweeps have usually already finished; the 5s bound
(matching WP-36's original) caps the wait.

**Supervisor wind-down (`nats.rs:293-340`) is correct and is the subtle part:**

```
let mut run = tokio::spawn(make_run());
let result = select! { res = &mut run => Some(res), _ = shutdown.cancelled() => None };
match result {
    None => { log; let _ = run.await; return; }   // wait for in-flight run, do NOT abort
    ...
}
```

- On shutdown the `JoinHandle` is polled by `&mut run`, so the spawned run is
  **not dropped/aborted** - it keeps running and is then awaited (`let _ =
  run.await`, `nats.rs:306`). This is the intended "wait for the consumer body
  to finish its current message and ack" behaviour, and it is what makes the
  drain graceful rather than a mid-`execute_command` kill (the F-101 coupling,
  `05b-web-admin-bot-db.md:380-381`).
- The backoff `sleep` is likewise `select!`-interrupted (`nats.rs:332-338`), so
  a supervisor parked in backoff exits promptly.
- The shutdown path returns **before** the `nats_consumer_restarts_total`
  increment (`nats.rs:327`), so a clean shutdown is not miscounted as a restart.

**Consume-loop wind-down (`game/mod.rs:324-343`) is correct:** the
`shutdown.cancelled()` arm is polled only while parked in `messages.next()`.
If a message is in flight (`handle(message).await`, `:342`), the select is not
polled, so the message is fully parsed/applied/acked-or-termed
(`process_bot_command_message`, `:350`) before the loop iterates and exits on
shutdown. No message is left half-processed. (A pathologically long
`execute_command` can still exceed the 5s bound and be abandoned at process
exit - the documented WP-36 semantics, `R-11-IMPLEMENTATION.md:213-217`.)

**Sweeps (`sweep.rs:338-347`):** `select!` on `shutdown.cancelled()` vs
`tick.tick()`; on shutdown the task returns and its `JoinHandle` resolves.
`spawn_sweep`/`spawn_*_sweep` now return their handles and
`spawn_periodic_sweeps` returns `Vec<JoinHandle<()>>` (`sweep.rs:635`), all
retained by `main` (`main.rs:118-125`). Consistent across all six sweeps.

**Wiring ownership (`main.rs:81-117`):** the bot-command and advisory
supervisor closures capture cloned dependencies and a `shutdown` clone, and
re-clone per restart call (correct for `FnMut`). Both supervisor handles and
the six sweep handles are pushed into `background_tasks` (8 handles total) and
joined in step 6. No handle is discarded (the pre-R-11 defect).

No deadlock, lost-wakeup, or abort-on-shutdown defect found. `CancellationToken`
cancellation is sticky, so a token cancelled before a task's first `select!`
still resolves immediately (the same race-safety noted for R-10,
`r-10-comprehensive-review.md:104-107`).

---

## 4. Test coverage - claimed shutdown-path coverage verified

| Test | Calls production fn? | Shutdown path exercised | Assessment |
|------|----------------------|--------------------------|------------|
| `bot_command_consume_loop_exits_on_shutdown` (`game/mod.rs:1284`) | Yes - `run_bot_command_consume_loop` (the generic loop the real consumer uses) | Parks the loop in `stream::pending().next()`, cancels, asserts prompt exit with zero messages handled | Genuine. Proves the active consume loop (not just the supervisor gap) observes shutdown. The generic bound `S: Stream<Item=Result<jetstream::Message,E>>+Unpin+Send, E: Display+Send` is satisfied by both the test's `stream::pending::<Result<_, async_nats::Error>>()` and the production `pull::Stream` (Item `Result<_, MessagesError>`, async-nats 0.49.1 - version confirmed in `Cargo.lock`). Compiles (gate exit 0). |
| `sweep_stops_on_shutdown` (`sweep.rs:1736`) | Yes - `spawn_sweep` | Lets the sweep tick (`runs >= 2`), cancels, asserts the handle resolves | Genuine. `start_paused` makes the 10ms interval and 600s timeouts virtual. Asserts exit, not exact post-cancel run count (robust to `select!` arm bias). |
| `supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down` (`nats.rs:467`) | Yes - `supervise_consumer` | Cancels while a run is in flight (run parked in `cancelled().await`); asserts `started == 1` (no restart) AND `finished == 1` (waited for wind-down) | Genuine and precise - pins both halves of the contract (stop restarting + await the in-flight run). |
| `supervisor_backoff_sleep_is_interrupted_by_shutdown` (`nats.rs:517`) | Yes - `supervise_consumer` | Cancels during the (paused) backoff sleep; asserts `calls == 1` (no restart) | Genuine. See M2 for a theoretical timing nuance. |

Pre-existing supervisor tests (`supervisor_restarts_on_err_ok_and_panic`,
`supervisor_keeps_retrying_under_persistent_failure`) were updated only to pass
a fresh never-cancelled token to the new signature, preserving their original
restart assertions. Correct.

The tests satisfy the F-109/Unit-08 rule that a regression test must actually
CALL the function under test (`90-findings-part2.md:34`): all four call real
production functions, not test-only seams.

**Coverage gap (M1):** `run_max_deliveries_advisory_listener`'s own shutdown
arm (`nats.rs:229-235`) has no direct unit test - its first action is
`client.subscribe(..)` (`nats.rs:220-223`), which needs live NATS. It is
covered indirectly (its supervisor's shutdown tests use an in-flight run that
mirrors a shutdown-aware body) and by CI integration. This is beyond AC2's
named scope (bot consumer + email sweep) and is disclosed
(`R-11-IMPLEMENTATION.md:208-213`).

---

## 5. AC3 / SSE - independent verification (no TaskTracker recreated)

Confirmed the task constraint held: `websocket.rs` and `events.rs` are absent
from the diff; no `TaskTracker`, no `track_future`, no `drain_ws_tasks`, no
`tokio-util` `rt` feature. The `main.rs:165-172` drain comment records the
boundary explicitly so the absent SSE tracker is not mistaken for an oversight.

**What R-10's committed behaviour establishes:** an SSE task that is inside its
`select!` loop observes `shutdown.cancelled()` (the `broadcaster.shutdown`
token fired by `begin_shutdown()`), breaks, drops `tx`, closes the response
body, and lets axum finish the in-flight request - promptly and boundedly. This
is proven by a real-listener test, `graceful_shutdown_ends_sse_stream_and_server_completes`
(`sse_events.rs:601-657`): open a stream, call `begin_shutdown()` (`:619`),
assert the stream ends (`:647-650`) and the server task completes within 5s
(`:652-656`). That test is the WP-84 §3g proof (see I1) and is direct AC3
evidence for the normal case.

**The residual (I2):** the `client.subscribe(..)` calls happen BEFORE the loop
(`events.rs:86,93` auth; `:206` public). A task blocked there at the moment of
shutdown is not polling the select, so it observes neither token; it holds `tx`,
the response body never ends, and axum's `with_graceful_shutdown` (which has no
timeout) waits indefinitely - `axum::serve(..).await` hangs and the R-11 drain
at `main.rs:173` never runs. The survey acknowledges exactly this
(`R-11-SURVEY.md:73-76`: "a task blocked in `client.subscribe(...)` at shutdown
is neither cancelled nor waited for ... A TaskTracker + bounded drain closes
this"). The practical window is narrow (requires an unreachable NATS at the
exact shutdown instant; healthy-NATS subscribe is fast) and is externally
bounded by k8s SIGKILL after `terminationGracePeriodSeconds`, but it is not
bounded in code.

Note the asymmetry this creates: the consumers/sweeps ARE bounded in code by the
5s drain (a subscribe-blocked consumer makes its supervisor's `run.await` hang,
which `join_all` times out at 5s and abandons), whereas the SSE tasks are not -
axum hangs before the drain runs. SSE is the worse-bounded family in this edge.

---

## 6. Correctness / quality / security / regressions

- **Behaviour preservation:** the per-message bot body moved verbatim into
  `process_bot_command_message` (`game/mod.rs:350`); the only semantic change is
  `continue` -> `return` (now a separate function). The parse-error ack, the
  `Ok|Conflict` ack, the `UserError` ack, and the `Other` term/unacked-by-delivery
  logic are all preserved. The `messages.next()` `Err` arm (`:335-341`) is
  preserved. No behaviour drift.
- **Simplicity/maintainability:** the generic `run_bot_command_consume_loop`
  plus the split-out `process_bot_command_message` is the minimal seam that makes
  the shutdown path testable without live NATS; the double-clone handler closure
  in `run_bot_command_consumer` (`game/mod.rs:283-307`) is verbose but matches
  surrounding style. Comments are *why* comments per CODING.md. Reasonable.
- **Security:** no new attack surface; the tokens are process-internal; logs use
  static consumer names and game UUIDs only; no secrets logged. No new `unwrap`
  on a request/runtime path (subscribe failures `return`/`?`; drain timeout only
  `warn!`s). CODING.md no-panic invariant honoured.
- **Concurrency:** see §3 - no deadlock/abort/lost-wakeup defect.
- **Regressions:** the changed signatures (`supervise_consumer`,
  `run_bot_command_consumer`, `run_max_deliveries_advisory_listener`,
  `spawn_*_sweep`, `spawn_periodic_sweeps`) are all crate-internal; the compile
  gate over `--all-targets` confirms every call site (incl. tests) was updated.
  No public API/contract change, no migration, no CI/config change.

---

## 7. Findings

### Critical
None.

### Important

- **I1 - AC1 amendment names the wrong successor test (bookkeeping defect in the
  bookkeeping AC).** The R-11 row (`97-REMEDIATION-PROGRESS.md:36`) and
  `review/R-11-COMMIT.md:31-39` cite the WP-84 §3g successor proof test as
  `rust/web/tests/sse_events.rs:551-595`
  (`sse_stream_survives_past_request_timeout_with_keepalive`). That is the
  Group-4 **keepalive** test (F-163); it never triggers a graceful shutdown.
  WP-84 §3g (git `868094a6:.../WP-84-sse-migration.md:226-248`) requires a
  real-listener test that "open[s] a stream, trigger[s] graceful shutdown,
  assert[s] the server task completes and the stream ended." The test that does
  exactly that is `graceful_shutdown_ends_sse_stream_and_server_completes` at
  `sse_events.rs:601-657` (Group 5: Graceful shutdown; `begin_shutdown` at
  `:619`, "SSE stream did not end after graceful shutdown" at `:649`, "server
  task did not complete within 5s of shutdown" at `:655`). The deletion record
  itself is correct (full SHA, `websocket_hygiene.rs` confirmed deleted by
  `efad81f9`); only the successor citation is wrong. This is precisely the
  F-109/F-147 failure mode (a citation that is present/reachable but not
  correct). **Fix:** change `:551-595`/`sse_stream_survives_past_request_timeout_with_keepalive`
  to `:601-657`/`graceful_shutdown_ends_sse_stream_and_server_completes` in the
  R-11 row (and the commit record). One line. The error originated in the survey
  (`R-11-SURVEY.md:58-59,334`), which mislabeled the keepalive test as "the
  graceful-shutdown test."

- **I2 - AC3 residual: a subscribe-blocked SSE task is not bounded in code.**
  Per §5: R-10 bounds the normal case (proven by `sse_events.rs:601-657`), but a
  task blocked in `client.subscribe()` (`events.rs:86,93,206`) at shutdown under
  a broken NATS connection holds `tx`, so the response body never ends and axum's
  timeout-less graceful shutdown hangs until k8s SIGKILL. The survey acknowledges
  this residual (`R-11-SURVEY.md:73-76`). It is narrow and externally bounded,
  and the prescribed `TaskTracker`+`tracker.wait()` remedy would NOT close it at
  the after-`axum::serve` placement (axum hangs first), so this is not a case of
  a skipped obvious fix. **Recommendation:** owner explicitly confirms acceptance
  of this residual for AC3 (it is the "concrete harm F-109 cites",
  `05b-web-admin-bot-db.md:387-391`); a true in-code close would require observing
  shutdown during/around the subscribe (e.g. subscribe inside the select or under
  a timeout), which is new scope, not part of this commit.

### Minor

- **M1 - no direct unit test for the advisory listener's shutdown arm**
  (`nats.rs:229-235`); needs live NATS, covered indirectly + CI. Beyond AC2's
  named scope; disclosed (`R-11-IMPLEMENTATION.md:208-213`). See §4.

- **M2 - `supervisor_backoff_sleep_is_interrupted_by_shutdown` timing nuance.**
  The assertion `calls == 1` requires `cancel()` to land during the first backoff
  window. Under `start_paused = true` this is deterministic in practice (the test
  task stays runnable after observing `calls >= 1`, so the runtime does not
  auto-advance the 1s backoff before `cancel()` runs), but it is a theoretical
  test-reliability nuance; runtime is deferred to CI. (`nats.rs:517-547`)

- **M3 - runtime unverified (disclosed).** All four new tests are compile-verified
  only (gate exit 0, §0); the web crate's build/test/run are banned
  (`97-REMEDIATION-PROGRESS.md:128`). They are pure-tokio (no DB/NATS) and should
  pass in CI as written. Same disclosed limitation as R-08/R-09/R-10.

---

## 8. Scope / constraint compliance

- Only the four implementation files + the single tracker row changed; no
  `websocket.rs`/`events.rs`, no review artefact, no untracked R-07/R-10 file
  touched (confirmed via `git show --stat` and `git status`). The untracked
  `docs/reviews/r-10-*.md`, `R-07-HANDOVER.md`, `R-08-*`, and `review/` remain
  `??`, unmodified. Scope honoured.
- No `TaskTracker`/`rt` feature reintroduced; no migration; no push (commit only,
  per commit policy `97-REMEDIATION-PROGRESS.md:129`).
- Reviewer constraints honoured: no edits/commits/pushes, no banned web command;
  only the single allowed compile gate was run.

---

## 9. Final verdict

**CONDITIONAL ACCEPT.** R-11 correctly and completely implements the
owner-mandated AC2 (shutdown signal + drain for the bot consumer, the advisory
listener, and the six sweeps, with tests that call the real production shutdown
paths); the cancellation/drain sequencing is sound; AC3 is met for the normal
case by R-10's committed mechanism without recreating the `TaskTracker`. No
Critical findings.

Conditions / follow-ups:
1. **Required (doc-only):** correct the AC1 successor citation from
   `sse_events.rs:551-595` (keepalive) to `sse_events.rs:601-657`
   (`graceful_shutdown_ends_sse_stream_and_server_completes`) - I1.
2. **Recommended:** owner confirms acceptance of the AC3 subscribe-blocked
   residual - I2.

**Targeted re-review required: yes, doc-only** - to confirm the I1 citation
correction. No code re-review needed; the committed code is accepted as-is.
