# R-11 Implementation - Shutdown drain: bot consumer + email sweeps

**Scope executed:** AC2 (owner ruling 6.3b - "Implement it (shutdown signal for
bot consumer + email sweep, with tests)") plus the propagation/wiring work the
ruling implies. AC1 (bookkeeping) and AC3 (SSE bound) are deliberately out of
scope for this worker - see "No-SSE rationale" and "Out of scope" below.

**Gate used (only allowed command):**
`SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`
run from `rust/`. Web build/test/run/clippy/fmt were NOT run (banned by owner
ruling, `97-REMEDIATION-PROGRESS.md:128`). Tests are compile-verified only;
runtime is deferred to CI.

**HEAD at implementation:** unchanged from survey (`4ca8aa9f...`); no commits
made (per task instruction "Do not commit").

---

## 1. TDD evidence - exact red then green

### RED (tests added first, referencing not-yet-existing API)

Command: `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`
Exit status: **101**

```
    Checking web v0.1.0 (/home/beefsack/Development/brdgme/rust/web)
error[E0277]: expected a `FnMut()` closure, found `CancellationToken`
   --> web/src/nats.rs:424:13
error[E0061]: this function takes 2 arguments but 3 arguments were supplied   (nats.rs - supervise_consumer)
error[E0277]: expected a `FnMut()` closure, found `CancellationToken`         (x6, nats.rs)
error[E0061]: this function takes 3 arguments but 4 arguments were supplied   (sweep.rs - spawn_sweep)
error[E0277]: `()` is not a future                                            (x3, sweep.rs - spawn_sweep returned ())
error[E0425]: cannot find function `run_bot_command_consume_loop` in this scope (game/mod.rs)
error: could not compile `web` (lib test) due to 14 previous errors
```

14 errors total: the new tests call `supervise_consumer(name, shutdown, ..)`,
`spawn_sweep(name, interval, shutdown, ..) -> JoinHandle`, and
`run_bot_command_consume_loop(..)`, none of which existed yet.

### GREEN (after minimal implementation)

Command: `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`
Exit status: **0**

```
    Checking web v0.1.0 (/home/beefsack/Development/brdgme/rust/web)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.16s
warning: the following packages contain code that will be rejected by a future version of Rust: proc-macro-error2 v2.0.1
note: to see what the problems were, use the option `--future-incompat-report`, or run `cargo report future-incompatibilities --id 1`
```

No errors. The only warning is the pre-existing, unrelated `proc-macro-error2`
future-incompat notice (present on the pristine baseline too). No new
`unused`/`unreachable`/`dead_code` warnings were introduced.

Baseline (pristine tree, before any edit) was also green (exit 0), confirming
the red above was caused solely by the new tests, not a pre-existing breakage.

---

## 2. Files changed

`git diff --stat` (tracked changes only):

```
 rust/web/src/email/sweep.rs | 128 ++++++++++++++++------
 rust/web/src/game/mod.rs    | 252 +++++++++++++++++++++++++++++++-------------
 rust/web/src/main.rs        |  67 ++++++++++--
 rust/web/src/nats.rs        | 193 ++++++++++++++++++++++++++++-----
 4 files changed, 496 insertions(+), 144 deletions(-)
```

NOT touched (verified absent from the diff): `rust/web/src/websocket.rs`,
`rust/web/src/events.rs`, any tracker/history doc, any review file, and the
untracked R-07/R-10 handover artefacts (still `??` in `git status`, unmodified).
Nothing deleted/moved/reverted/stashed. No commit made.

---

## 3. Shutdown lifecycle (what was implemented)

A single process-level `tokio_util::sync::CancellationToken` is created in
`main` and threaded into every background task family. `tokio-util`'s
`CancellationToken` was already a dependency (used by `websocket.rs`), so no
`Cargo.toml` change and no `TaskTracker`/`rt` feature were needed.

### Startup (`main.rs`)

- `let shutdown = CancellationToken::new();`
- `let mut background_tasks: Vec<JoinHandle<()>> = Vec::new();`
- bot-command supervisor: `supervise_consumer("bot-command", shutdown.clone(), move || run_bot_command_consumer(.., shutdown.clone()))` - handle pushed.
- max-deliveries-advisory supervisor: `supervise_consumer("max-deliveries-advisory", shutdown.clone(), move || run_max_deliveries_advisory_listener(advisory_client.clone(), shutdown.clone()))` - handle pushed.
- sweeps: `background_tasks.extend(spawn_periodic_sweeps(.., shutdown.clone()))` - returns 6 `JoinHandle`s.

All handles are now retained (previously discarded).

### Shutdown signal

In the `with_graceful_shutdown` future, after `shutdown_signal().await` and
`broadcaster.begin_shutdown()`, the new line `shutdown.cancel();` fires the
process token. axum then finishes in-flight SSE requests as before.

### Bounded drain (`main.rs`, after `axum::serve(...).await`)

```
let drain_bound = Duration::from_secs(5);
if tokio::time::timeout(drain_bound, futures_util::future::join_all(background_tasks)).await.is_err() {
    tracing::warn!("background tasks did not drain within {drain_bound:?}; abandoning them");
}
```

5s matches WP-36's original bound. `join_all` awaits all supervisor + sweep
handles; the timeout caps the wait before the process exits.

### Per-task shutdown behaviour

- `supervise_consumer` (`nats.rs`): new `shutdown` param. The restart loop now
  `select!`s the running `JoinHandle` against `shutdown.cancelled()`. On
  shutdown it stops restarting, **awaits the in-flight run** (so a shutdown-aware
  consumer body finishes its current message and acks before the supervisor
  returns), then returns. The backoff `sleep` is likewise `select!`-interrupted
  by shutdown so a supervisor parked in backoff exits promptly.
- `run_bot_command_consumer` (`game/mod.rs`): new `shutdown` param. The active
  `while let Some(message) = messages.next().await` loop was restructured into a
  generic `run_bot_command_consume_loop` that `select!`s `messages.next()`
  against `shutdown.cancelled()` - so cancellation is honoured **while parked
  waiting for the next message**, not only in the supervisor restart gap. The
  per-message parse/`handle_bot_command_event`/ack/term body moved verbatim into
  a new `process_bot_command_message` helper (behaviour unchanged; `continue`
  became `return`).
- `run_max_deliveries_advisory_listener` (`nats.rs`): new `shutdown` param; the
  `while let Some(msg) = sub.next().await` loop became a `loop` + `select!` with
  a shutdown arm, so its supervisor's `run.await` returns promptly on shutdown
  (propagation to the second currently-started consumer supervisor).
- `spawn_sweep` / `spawn_*_sweep` / `spawn_periodic_sweeps` (`sweep.rs`): each
  takes a `shutdown` param; the tick loop `select!`s `tick.tick()` against
  `shutdown.cancelled()`. `spawn_sweep` and each `spawn_*_sweep` now return their
  `JoinHandle<()>`; `spawn_periodic_sweeps` returns `Vec<JoinHandle<()>>`.

---

## 4. Tests (all compile-verified; runtime deferred to CI)

New tests, each calling a task's shutdown path:

| Test | File | Shutdown path exercised |
|------|------|--------------------------|
| `supervisor_stops_on_shutdown_and_waits_for_run_to_wind_down` | `rust/web/src/nats.rs` | Cancels the token while a run is in flight; asserts the supervisor exits, does NOT restart (`started == 1`), and waits for the run to wind down (`finished == 1`). |
| `supervisor_backoff_sleep_is_interrupted_by_shutdown` | `rust/web/src/nats.rs` | Cancels while the supervisor is in its (paused) backoff sleep; asserts prompt exit with no further restart. |
| `sweep_stops_on_shutdown` | `rust/web/src/email/sweep.rs` | Calls the (now `JoinHandle`-returning) `spawn_sweep` with a short interval, lets it tick, cancels, and asserts the task exits within bound. |
| `bot_command_consume_loop_exits_on_shutdown` | `rust/web/src/game/mod.rs` | Drives `run_bot_command_consume_loop` with a `stream::pending()` message stream (so the loop is parked in `messages.next()`), cancels, and asserts the loop returns promptly with zero messages handled - proving the active consume loop, not just the supervisor gap, observes shutdown. Needs no NATS/DB. |

Pre-existing tests updated only to pass a fresh (never-cancelled) token to the
new `supervise_consumer` signature, preserving their original restart-behaviour
assertions:

- `supervisor_restarts_on_err_ok_and_panic` (`nats.rs`)
- `supervisor_keeps_retrying_under_persistent_failure` (`nats.rs`)

Test seam note: `run_bot_command_consume_loop` is generic over
`S: Stream<Item = Result<jetstream::Message, E>> + Unpin + Send` with
`E: Display + Send`, so the real `pull::Stream` (Item =
`Result<jetstream::Message, MessagesError>`, confirmed in async-nats 0.49.1
`pull.rs:1096`) satisfies the bound at the production call site, while the test
substitutes `stream::pending::<Result<jetstream::Message, async_nats::Error>>()`.

---

## 5. No-SSE rationale / evidence (AC3 deliberately untouched)

Per task instruction: "Do NOT recreate the deleted WebSocket/SSE TaskTracker
architecture or modify SSE mechanisms: R-10's committed behavior satisfies
AC3." Accordingly:

- `rust/web/src/websocket.rs` and `rust/web/src/events.rs` are NOT in the diff
  (verified via `git diff --stat`).
- No `TaskTracker` was reintroduced and `tokio-util`'s `rt` feature was NOT
  re-added; only the already-present `sync` feature (`CancellationToken`) is
  used.
- SSE spawns remain bounded by R-10's committed mechanism: axum
  `with_graceful_shutdown` waits for in-flight requests, and each SSE handler
  has a per-connection `CancellationToken` (fired on `SseStream::Drop`) plus a
  global `shutdown.cancelled()` select arm (survey §1b, §5; R-10 comprehensive
  review `:102-103` ACCEPT, no TaskTracker by design). The bounded drain added
  here covers only the bot/advisory supervisors and the six sweeps - the task
  families F-109 §1c identifies as having NO shutdown path.
- The `main.rs` drain comment records this boundary explicitly so a future
  reader does not mistake the absent SSE tracker for an oversight.

---

## 6. Out of scope / not done (per task constraints)

- **AC1 bookkeeping** (amending the WP-36 checklist reference in
  `97-REMEDIATION-PROGRESS.md`): not done - this worker is forbidden from
  modifying tracker/history docs and review files. The survey (§7 T1) records
  the exact amendment text for whoever owns the doc pass.
- **AC3 SSE TaskTracker drain**: not done by design (see §5).

## 7. Limitations

- All tests are **compile-verified only** (`cargo check`); the web crate's
  build/test/run/clippy/fmt are banned for this worker, so runtime behaviour is
  deferred to CI. The four new tests are pure-tokio unit tests (no DB/NATS) and
  should pass in CI as written.
- `run_max_deliveries_advisory_listener`'s shutdown arm is exercised indirectly
  (via the supervisor shutdown tests, whose in-flight run mirrors a
  shutdown-aware consumer body). A direct unit test of that listener's arm would
  require a live NATS connection (its first action is `client.subscribe(..)`),
  so it is covered by the same pattern + CI integration rather than a standalone
  unit test here.
- The 5s drain bound is a wall-clock cap: if a sweep's in-flight `run()` or a
  bot `execute_command` somehow exceeded it, the task is abandoned with a warn
  log (same semantics as WP-36's original bound). The shutdown arms make this a
  rare edge rather than the norm.
- `cargo fmt` could not be run (banned); edits were hand-formatted to match
  surrounding style. CI's fmt gate should be watched on push.
