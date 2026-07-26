# Fuzz throughput evaluation (input to D-43)

Scope: does the fuzzer keep per-game `_fuzz` bins (in-process), move to the
generic out-of-process `rust/tools/fuzz` driver, or do something faster than
either? Selection criterion set by Michael: **throughput**. Simplicity and the
non-Rust portability argument are subordinate here (they still hold for `_repl`,
where D-41 stands).

Everything below is from reading the LIVE code. **No measurements were taken -
cargo cannot be run in this session.** Anything not establishable by reading is
in the UNKNOWN section with the command to settle it. No estimate below is a
measurement.

Files read in full: `rust/tools/fuzz/src/{main,lib}.rs`,
`rust/lib/cmd/src/requester/{mod,local,gamer}.rs`, `rust/lib/cmd/src/{api,cli}.rs`,
`rust/game/tic-tac-toe-2/src/bin/tic_tac_toe_2_fuzz.rs`, the `Gamer` trait in
`rust/lib/game/src/game.rs`, `brdgme_rand_bot::spec_to_command`,
`brdgme_markup::to_string`.

## 1. Headline finding: the in-process path is NOT free of serialisation

This is the load-bearing result and it inverts the intuition behind D-43.

`requester::gamer::GameRequester` is **not** a shortcut around the API. It
implements the exact same `api::Request` -> `api::Response` contract as the
out-of-process path; it only removes the transport. `api::Request::Play` carries
the game as `game: String` (JSON), and `GameRequester::request` does
`serde_json::from_str(game)` on it. `api::GameResponse::from_gamer` does
`serde_json::to_string(gamer)` to build the reply.

Per move, **in-process today**:

| Work | Count per move | Used by the fuzzer? |
| --- | --- | --- |
| `serde_json::from_str::<G>` of full game state | 1 | required |
| `G::command(...)` - the actual thing under test | 1 | yes |
| `serde_json::to_string(&game)` full state (`from_gamer`) | 1 | only as opaque input to the next move |
| `serde_json::to_string(&pub_state)` | 1 | no - discarded |
| `serde_json::to_string(&player_state)` | player_count | no - discarded |
| `brdgme_markup::to_string(render())` | player_count + 1 | no - discarded |
| `game.command_spec(p)` | player_count | only `[acting player]` used |
| `String` clone of the state (`state.to_string()` in `Fuzzer::command`) | 1 | avoidable |
| `PlayerRender` clone (`player_render.clone().command_spec.unwrap()`) | 1 | avoidable; clones two large JSON strings and a render string to take one field |
| mpsc `send` of a `FuzzStep` to the tally thread | 1 | yes (cheap) |

`renders()` in `requester/gamer.rs` computes the pub render and **every**
player's render and state JSON on every single move. `brdgme_markup::to_string`
is a recursive `format!`-per-node walk - allocation-heavy by construction.

The fuzz loop consumes exactly two things out of all that: the acting player's
`command_spec`, and `game.state` (which it hands straight back as the next
request's input). Everything else is built and dropped.

So the status quo already pays a full JSON encode + decode of game state per
move, plus N+1 renders and N+1 state encodes it throws away.

## 2. Out-of-process cost via `LocalRequester`

Request count is 1:1 with fuzz steps. Per `Fuzzer` construction: 1
`Request::PlayerCounts`. Per game: 1 `Request::New`. **Per move: exactly 1
`Request::Play`.** There is no batching and no persistent child.

`LocalRequester::request` does, per request: `Command::spawn` (fork/exec of the
`_cli` binary, dynamic linking, runtime init), write the serialised `Request` to
the child's stdin pipe, `wait_with_output()`, then `serde_json::from_slice` the
`Response`.

On top of the section 1 work, out-of-process adds per move:

- one process spawn + teardown,
- serialisation of the outer `Request` - the state JSON string is **escaped
  again** inside it (double encoding),
- serialisation of the outer `Response` - which contains the state string, the
  pub_state string, N player_state strings and N+1 render strings, all escaped,
- parse of that outer `Response` in the parent,
- three pipes' worth of I/O and two syscall-heavy copies of the payload.

`fuzz()` runs `num_cpus::get()` worker threads, so this is `num_cpus`
concurrent spawns continuously, not one.

Directionally this is strictly worse than in-process by a process spawn plus a
second full JSON layer over a payload that is already the largest thing in the
loop. Magnitude is UNKNOWN (section 5).

## 3. The loop is already parallel

`fuzz()` spawns one thread per logical CPU; each thread builds its own
`Requester` and its own `Fuzzer` with its own `ThreadRng`, and pushes a
`FuzzStep` down a shared mpsc to the main thread, which only tallies. There is
**no shared mutable state in the hot loop** - the `Arc<Mutex<F>>` is touched once
per thread at startup only.

`Gamer` has no `Send` bound, but nothing in the loop needs one: the game value
never crosses a thread boundary (in-process it is created, mutated and dropped
inside `GameRequester::request`). A grep for `Rc<`, `RefCell` and `Cell<` across
`rust/game/*/src/` returned **no hits**, so the concrete game types look like
plain data and would be `Send` anyway.

Conclusion: "add parallelism" is not available as a win - it is already there.
The remaining headroom is entirely per-move work.

## 4. Options, cheapest first

### (a) Status quo - per-game `_fuzz` bin, in-process

Ceiling is set by the section 1 table, not by `G::command`. On a game with a
large state or a busy render (Acquire, Seven Wonders, Starship Catan), the
useful work is plausibly a minority of the per-move cost. 27 near-identical
3-line bins remain.

### (b) Generic `fuzz_main::<G>()` in `brdgme_game_bin` (original WP-73 design)

Speed-neutral versus (a), and the code supports saying so: `brdgme_fuzz::fuzz_gamer::<G>`
is already a generic function monomorphised per game, and the per-game bin is
literally `fn main() { brdgme_fuzz::fuzz_gamer::<Game>(); }`. Moving that call
behind `brdgme_game_bin::fuzz_main::<G>()` changes which crate the `main` lives
in, not what is called - the same monomorphisation, same static dispatch, same
`GameRequester<G>`. It removes the per-game boilerplate without touching the hot
loop.

`brdgme_game_bin` does not exist yet (`rust/lib/` has no `game_bin`), so this is
still a design choice, not a revert.

### (c) Out-of-process via `rust/tools/fuzz local <path-to-_cli>` (what D-41 proposed)

Strictly slower than (a)/(b) per section 2. Reject on the stated criterion.

### (d) Faster than status quo: strip the API layer out of the fuzz hot loop

Keep the game value **live in memory** across moves and drive `Gamer` directly:

```
loop:
  spec = game.command_spec(player)          // no render, no serde
  cmd  = rand_bot::spec_to_command(spec, ...)
  game.command(player, &cmd, &names)        // the thing under test
  status = game.status()
```

This deletes, per move: 1 state decode, 1 state encode, 1 pub_state encode, N
player_state encodes, N+1 markup renders, the state `String` clone and the
`PlayerRender` clone. It keeps 100% of the game-logic coverage the fuzzer has
today.

Cost: it changes what is fuzzed. Render and state-serialisation panics are real
bugs that the current loop does catch as a side effect. Two ways to keep that:

- sample it - run the full `renders()` + `from_gamer()` every Nth move, or once
  at game end plus on a configurable probability; or
- flag it - `--check-renders` for a slow exhaustive mode, fast path by default.

**This is a decision for Michael, not for the spec author**: coverage of the
render/serialise path is being traded for throughput.

Reproduction is unaffected: `Fuzzer` already reports `seed` plus the full
`command_log`, and replay reconstructs state from those. The only loss is that
the error report can no longer print the pre-command state JSON verbatim - it
can serialise the live game at failure time instead, which is equivalent.

### (e) One binary fuzzing many games

A single bin linking all 27 crates, choosing a game per worker thread (or
round-robin per game instance). Deletes 27 bins and lets one process saturate
all cores across the whole catalogue. **No per-move speed change** - it composes
with (b) or (d) rather than competing with them. Costs: one crate that depends
on every game (compile time, and a rebuild of that bin whenever any game
changes). Worth it only if Michael wants "fuzz everything" as one command.

### (f) Persistent child process (rejected)

If out-of-process were kept, a long-lived child speaking newline-delimited JSON
on stdin/stdout would remove the spawn. It still loses to in-process on the JSON
layer, and `cli.rs` is a one-shot (`serde_json::from_reader` on the whole stream,
write one line, return). Not worth building given (d) exists.

## 5. RECOMMENDATION

1. **Keep fuzzing in-process. Do not adopt `LocalRequester` for fuzz.** Reverse
   the fuzz half of D-41 permanently - D-43's instinct is correct, though for a
   weaker reason than assumed (the gap is a process spawn plus a second JSON
   layer, not "serialised vs not serialised" - both paths serialise).
2. **Adopt (b)** as the WP-73 shape: `brdgme_game_bin::fuzz_main::<G>()`, one
   3-line bin per game. Speed-neutral, removes the boilerplate, no risk.
3. **Pursue (d) as separate work**, and treat it as the actual throughput
   project. The status quo's per-move cost is dominated by work the fuzzer
   discards. This is where the multiple-x is, if it is anywhere.
4. **Ask Michael to decide the render-coverage tradeoff in (d)** before anyone
   specs it. Default suggestion: fast path by default, `--check-renders` for the
   thorough mode, plus always doing one full `renders()` at game end.
5. **(e) is optional convenience**, not a throughput item. Defer.
6. Regardless of the above, two free wins in `Fuzzer::command` (in
   `rust/tools/fuzz/src/lib.rs` - read the function, do not trust a line number):
   the `player_render.clone().command_spec.unwrap()` clones an entire
   `PlayerRender` (two large JSON strings plus a render string) to take one
   field, and `state.to_string()` clones the whole state JSON every move. Both
   are removable without changing behaviour.

`_repl` is untouched by all of this: D-41 stands, out-of-process is correct for
an interactive tool.

## 6. UNKNOWN - requires measurement, not reasoning

None of the following was measured. Do not let any of it be quoted as a number.

| Unknown | Why it matters | Command to settle it |
| --- | --- | --- |
| Baseline in-process throughput (moves/sec) per game | Every ratio below needs a denominator | `cargo run --release --bin tic_tac_toe_2_fuzz` and read the once-per-second tally line; repeat for a heavy game once WP-73 lands a bin for it |
| Fraction of per-move time in `G::command` vs serde vs markup | Decides whether (d) is a 1.3x or a 10x | `cargo build --release --bin <game>_fuzz` then `perf record -g ./target/release/<game>_fuzz` + `perf report`, or run it under `samply record` |
| Actual `LocalRequester` spawn cost on this machine | Sizes the (c) penalty | `cargo build --release --bin tic_tac_toe_2_cli` then `hyperfine --shell=none 'echo "\"PlayerCounts\"" \| ./target/release/tic_tac_toe_2_cli'` |
| End-to-end in-process vs out-of-process fuzz rate | The direct comparison | run `cargo run --release --bin tic_tac_toe_2_fuzz` for 60s, then `cargo run --release -p brdgme_fuzz -- local ./target/release/tic_tac_toe_2_cli` for 60s, and compare the tally lines |
| Whether the mpsc tally send is material at high rates | Only matters if (d) lands and the loop gets ~10x faster | `perf stat` before/after replacing the per-step `send` with per-thread atomics |
| Scaling across `num_cpus` threads | Whether any hidden contention exists | run the fuzzer with the thread count forced to 1, 2, 4, N and compare tallies (requires a temporary edit to `fuzz()`) |
| Whether any game's `command_spec` is itself expensive | (d) still calls it every move | included in the `perf report` above |

## 7. Instruction for whoever specs the follow-up

Read the named functions - `fuzz()`, `Fuzzer::command`, `Fuzzer::next`,
`GameRequester::request`, `renders()`, `LocalRequester::request` - before
editing. This code is under concurrent edit and no line numbers are cited on
purpose. If a function does not match what this document describes, **STOP and
report** rather than improvising.
