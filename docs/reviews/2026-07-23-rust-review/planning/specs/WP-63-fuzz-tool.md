# WP-63: tools/fuzz

**Findings:** bo F26 (major), bo F27 (minor), bo F28 = dp F20 (minor, ONE
change - do not apply twice), bo F29 (nit), bo F30 (nit), bo F31 (nit).

**Source used:** `findings/bot-operator-tools.md` (`## tools/fuzz` section) and
`findings/dependencies.md` (`### num_cpus where std suffices`). Neither unit has
a file in `findings/verification/` - both were lead-verified, so every claim
below was re-derived by reading the live source.

**Crate layout (verified live):** `rust/tools/fuzz/` = `Cargo.toml`,
`src/lib.rs`, `src/main.rs`. The lib is the whole tool; `main.rs` is 8 lines.
There is **no test module anywhere in the crate**.

**Landing order:** independent of WP-07. WP-07 edits `brdgme_rand_bot` only and
does not change `spec_to_command`'s signature; nothing here waits on it.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This tree is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

- **bo F26 (major)** - `fuzz` (`rust/tools/fuzz/src/lib.rs`) creates
  `(step_tx, step_rx)` and moves only *clones* of `step_tx` into the workers.
  If every worker dies the channel never disconnects and
  `step_rx.recv().expect(...)` blocks forever.
- **bo F27** - the same fn measures its 1-second output interval with
  `SystemTime` + `duration_since(...).expect(...)`.
- **bo F28 = dp F20** - `num_cpus::get()` in `fuzz`, sole use of the
  `num_cpus` dependency in `rust/tools/fuzz/Cargo.toml`.
- **bo F29** - `fuzz` wraps its factory in `Arc<Mutex<F>>` and calls
  `new_requester.lock().unwrap()()`.
- **bo F30** - `fuzz`'s shutdown loop does `tx.send(()).unwrap()`.
- **bo F31** - `Fuzzer::command` does `player_render.clone().command_spec.unwrap()`
  right after checking `command_spec.is_none()`.

## 2. Why it's wrong

- **bo F26 is correct as written.** Verified live: no `drop(step_tx)` exists.
  The real trigger is reachable - `main.rs` passes a closure that does
  `requester::parse_args(&args).unwrap()`, and the worker also
  `.expect(...)`s `Fuzzer::try_new`, which fails whenever the requester binary
  path is bad. Every worker panics, then the process hangs silently.
- **bo F27 is correct as written.** `SystemTime` is not monotonic; a backwards
  clock step makes `duration_since` return `Err` and panics the driver.
- **bo F28 / dp F20 are correct and are the same finding.**
  `std::thread::available_parallelism()` covers this use.
- **bo F29 is correct as written.** Verified live: the only two factories
  passed to `fuzz` are the `move ||` closure in `src/main.rs` (captures a
  `Vec<String>`) and the `requester::gamer::new::<G>` fn item via `fuzz_gamer`.
  Both are `Sync`. All ~30 game `*_fuzz` bins go through `fuzz_gamer`, so
  tightening the bound breaks no caller.
- **bo F30 is correct as written.** A worker that panicked has dropped its
  `exit_rx`; the shutdown send then panics on an already-failed run.
- **bo F31 is correct as written.** The clone copies the whole render payload
  to take one already-checked `Option` field.

## 3. Required end state

All in `rust/tools/fuzz/src/lib.rs` unless stated. `src/main.rs` is unchanged.

### 3a. `fuzz` - no hang (bo F26)

`drop(step_tx)` immediately after the spawn loop, so the receiver disconnects
once the last worker is gone. Replace `step_rx.recv().expect("failed to get
step")` with a match on the `Result`: `Err(_)` means all workers exited - break
out of the loop (print a final `tally.render()` to stderr and a one-line
"all fuzz workers exited" note to stderr) instead of panicking or blocking.
`Ok(step)` keeps the existing per-variant handling verbatim.

### 3b. `fuzz` - monotonic interval (bo F27)

`last_output_at: Instant` via `Instant::now()`, and the loop condition becomes
`last_output_at.elapsed() > output_interval`. Drop the `SystemTime` import;
keep `Duration`. No `expect` remains in this path.

### 3c. `fuzz` - drop num_cpus (bo F28 = dp F20)

Thread count becomes
`std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)`.
Remove the `num_cpus` line from `rust/tools/fuzz/Cargo.toml` `[dependencies]`
and let `Cargo.lock` update. Do **not** move it to `[dev-dependencies]`.

### 3d. `fuzz` - no Mutex round-trip (bo F29)

Bound becomes `F: Fn() -> R + Send + Sync + 'static`; wrap in `Arc::new(f)`
only, and the worker calls `new_requester()` directly. Drop the `Mutex` import
if nothing else uses it.

### 3e. `fuzz` - best-effort shutdown (bo F30)

`let _ = tx.send(());` in the shutdown loop.

### 3f. `Fuzzer::command` - no whole-render clone (bo F31)

Replace the `is_none()` check plus `.clone().command_spec.unwrap()` with a
single `let Some(spec) = player_render.command_spec.clone() else { return
Err(anyhow!("player {}'s command_spec is None", player)); };` and use `spec`.
Error message text unchanged.

## 4. Non-goals

- **Dedup against `brdgme_rand_bot::commands()`: RULED OUT - keep separate.**
  `commands()` is private, returns `Vec<BotCommand>` (a quality-scored wrapper
  the fuzzer has no use for), and would have to be made public and then
  unwrapped straight back to a `String`. The genuine shared primitive is
  `spec_to_command`, which `rand_command` already calls. After WP-07 makes
  `commands()` join with `""` the two outputs match, which removes the only
  substantive reason to merge them. Do not change `rand_command`.
- Do not touch `rust/tools/fuzz/Cargo.toml` beyond deleting `num_cpus`:
  workspace-dependency migration (WP-64), sqlx unification (WP-66), sentry
  feature trim (WP-67), `deny.toml` (WP-69) and game-bins consolidation
  (WP-73) are all blocked on decisions.
- Do not restructure worker shutdown beyond 3a/3e (the workers' own
  `send(...).expect(...)` on a dropped receiver stays as-is), do not change
  the tally/report format beyond the new exit line, and do not touch
  `rust/lib/rand_bot/`.

## 5. Regression test cases

The crate has no tests. Add `#[cfg(test)] mod tests` at the end of
`rust/tools/fuzz/src/lib.rs`. **No new dependencies** - the stub below uses
`brdgme_cmd`, already a normal dependency.

- Stub: a unit struct implementing `brdgme_cmd::requester::Requester` whose
  `request` always returns `Err(RequestError::Stdin)`. That makes
  `Fuzzer::try_new` fail, so every worker panics on its `expect`.
- **bo F26 regression (the important one):** spawn a thread running
  `fuzz(|| StubRequester)` and signal completion over a channel; assert the
  parent's `recv_timeout(Duration::from_secs(10))` succeeds, i.e. `fuzz`
  *returns* rather than hanging. Without `drop(step_tx)` this test hangs to
  timeout and fails. (Worker panic messages on stderr during the run are
  expected.)
- **bo F31:** assert the `command_spec: None` path still yields the
  `"player 0's command_spec is None"` error text - reachable by driving
  `Fuzzer` with a stub requester that answers `PlayerCounts` and `New` with an
  `Active` status and a `PlayerRender` whose `command_spec` is `None`. If
  building that fixture proves disproportionate, skip it and note why; the
  F26 test is mandatory.

## 6. Riders

| File | One-line fix | Test needed |
| --- | --- | --- |
| `src/lib.rs` `fuzz` (bo F27) | `SystemTime` -> `Instant` / `elapsed()` | n |
| `Cargo.toml` + `src/lib.rs` `fuzz` (bo F28 = dp F20) | `num_cpus::get()` -> `available_parallelism()`, delete the dep | n |
| `src/lib.rs` `fuzz` (bo F29) | add `Sync` bound, `Arc<F>` not `Arc<Mutex<F>>`, call directly | n (compile-only) |
| `src/lib.rs` `fuzz` (bo F30) | `tx.send(()).unwrap()` -> `let _ = tx.send(());` | n |
| `src/lib.rs` `Fuzzer::command` (bo F31) | `let Some(spec) = ... else { return Err(...) }` | y (see 5) |
