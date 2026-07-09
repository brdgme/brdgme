# zombie-dice-2 test determinism - Implementation Plan

## Goal

Make the `zombie-dice-2` test suite fully deterministic (no CI flakes from real
dice RNG) without losing test coverage, AND establish a reusable,
best-practice pattern for deterministic randomness testing that other game
crates in this workspace (`farkle-2`, `liars-dice-2`, etc.) can adopt later.

## Root cause

`Game::roll()` (`rust/game/zombie-dice-2/src/lib.rs:287`) can, on a 3-shotgun
bust, call `self.next_player()` internally, which in turn calls
`self.start_turn()` for the next player, which calls `self.roll()` again. This
is a real, correct game rule (a bust ends your turn immediately), but it means
**any roll can recursively cascade through an arbitrary number of players'
turns** depending on real dice outcomes (`rand::rng()` in `Dice::roll`,
`rust/game/zombie-dice-2/src/lib.rs:77-81`, and `Game::shake_cup`,
line 216).

Several existing tests call `keep()`/`next_player()` (which trigger this
auto-roll cascade) and then assert an exact resulting value of `current_turn`,
`finished`, or similar, implicitly assuming the cascade always stops at the
"obvious" next player. It doesn't always - a bust chain can skip further than
expected. This is not unique to the test that most recently failed in CI; it
is a systemic hazard for every test that triggers an auto-roll and then reads
exact post-roll state.

Two prior commits (`1b10f4e`, `a4bed83`) hit this exact bug in two different
tests and patched each one reactively by asserting on rendered log content
instead of exact state - a valid but narrow fix applied test-by-test as CI
happened to flake, not a systemic fix. A full audit (below) found the bug is
live in at least one more test (`test_rolloff_skips_non_rolloff_players`,
currently failing in CI) and latent (not yet observed, but reproducible) in
one more (`test_rolloff_resolves_when_unique_leader`) plus a much
lower-probability case in `test_start_initial_state`.

Empirical confirmation (this investigation, `cargo test -p zombie-dice-2
--lib`, run from `rust/`): `test_rolloff_skips_non_rolloff_players` failed
3/40 runs (~7.5%), with **two distinct panic sites** depending on how far the
bust cascade travels:
- `lib.rs:741` - `g.keep(0).unwrap()` panics because a bust during the
  rolloff-start auto-roll for player 0 (triggered by the preceding
  `next_player()`) bounces `current_turn` away from 0 before `keep(0)` runs.
- `lib.rs:744` - `assert_eq!(2, g.current_turn)` fails (the originally
  reported CI failure, `left: 2, right: 0`) because a bust during player 2's
  auto-roll (triggered by `keep(0)`) cascades the turn around to player 0.

## Chosen approach

Per explicit direction from the human reviewing this plan:

1. **Primary mechanism - scripted/injectable randomizer (test-double pattern).**
   Introduce a small `Randomizer<T>` trait plus a real `RngRandomizer` (production,
   draws a uniform random outcome - byte-for-byte the same entropy source
   `Dice::roll` uses today) and a `ScriptedRandomizer<T>` test double (returns a
   pre-queued exact sequence of outcomes, ignoring the real face list). Tests
   that need dice outcomes to matter script the outcomes they need (e.g.
   "three brains, no shotguns") and get back **exact, strong assertions** on
   game state - stronger than both plain invariant checks and the log-content
   band-aid, because the test states its intent directly ("this roll must not
   bust") instead of hoping an invariant happens to tolerate every outcome.
2. **Shared home for the abstraction.** `brdgme_game` (`rust/lib/game`) is
   the shared crate every game crate already depends on, and it already
   depends on `rand = "0.10.2"` (same version zombie-dice-2 uses). The
   *production* half (`Randomizer` trait + `RngRandomizer`) goes in a new
   always-compiled `brdgme_game::randomizer` module so any game crate can adopt
   it. The *test-only* half (`ScriptedRandomizer`) goes in a new
   `brdgme_game::test_support` module gated behind a `test-support` Cargo
   feature - this exactly mirrors the existing, proven idiom in this
   workspace: `brdgme_cmd` already ships `test_support::assert_gamer_contract`
   behind its own `test-support` feature
   (`rust/lib/cmd/src/lib.rs:13-14`, `Cargo.toml` `[features] test-support =
   []`), consumed by zombie-dice-2 today as `brdgme_cmd = { path =
   "../../lib/cmd", features = ["test-support"] }` in `[dev-dependencies]`.
   We add the identical pattern to `brdgme_game`.
3. **Seeded real RNG only for smoke/integration tests, and only where
   architecturally forced.** `Game::start()` and `Game::command()` are the
   fixed `Gamer` trait entry points (`rust/lib/game/src/game.rs`) used
   externally by `brdgme_cmd`, the bot, the web server, etc. Their signatures
   cannot take an injected randomizer without changing that trait across every
   game crate in the workspace - clearly out of scope for this fix. Both
   methods keep using the real `RngRandomizer` internally, unconditionally.
   Consequently, two tests that exercise these entry points directly
   (`test_command_roll_and_keep`, `test_start_initial_state`) **cannot** use
   `ScriptedRandomizer` no matter how the abstraction is designed. These are kept
   on deterministic-invariant / log-content assertions (the existing
   `a4bed83` pattern), with an explanatory comment added so a future reader
   doesn't mistake this for an oversight. This is the one place the plan
   deliberately does not force the preferred pattern, because the
   architecture (fixed trait boundary) prevents it, not because it's
   discretionary.
4. **Direct state construction stays where rolls are incidental.** Several
   tests already construct game state directly (setting `scores`,
   `current_turn`, `round_brains`, etc.) and only care about a mid-game
   situation; where the actual dice-roll outcome triggered along the way is
   never asserted on and is fully overwritten by the test afterward (e.g. the
   very first roll from `Game::start()` before a test overwrites
   `round_brains`), no scripting is added - it would be test-double
   boilerplate protecting nothing.
5. **Pattern documentation.** A module-level doc comment on
   `brdgme_game::randomizer` (and a doc comment on `brdgme_game::test_support`
   mirroring the existing `lib/cmd` one) records: why the abstraction exists,
   how production vs. test code differ, and when to prefer `ScriptedRandomizer`
   vs. direct state construction vs. (rare) real-RNG smoke tests. It also
   records the design boundary set by the human (see the module doc in
   Task 1): `Randomizer` abstracts game-meaningful outcomes (dice rolls,
   shuffled deck order, tile draws), not `rand`'s primitive API - scripting
   primitives like `gen_range` would couple test doubles to algorithm
   internals and is explicitly rejected. This is the "future game crates
   follow it" reference point requested; no new standalone doc file is
   created.

### Alternatives rejected

- **Seeded `StdRng` injected everywhere (`seed_from_u64`) as the primary
  mechanism.** Rejected per explicit direction: seed-dependent assertions are
  brittle (a rand-crate version bump or algorithm change silently changes
  outcomes) and encode no readable intent ("why does seed 42 produce a bust
  on turn 3?"). The scripted-randomizer pattern expresses intent directly and is
  robust to `rand` internals changing.
- **Storing an RNG/seed inside `Game` itself (serialized state).** `Game` is
  `Serialize`/`Deserialize` and round-trips through JSON between calls (used
  by `brdgme_cmd`'s requester/HTTP/bot layers). Persisting RNG state would
  require the RNG type to serialize, add a migration-shaped concern to a
  networked game format, and still couldn't be seeded from outside `start()`
  without changing the `Gamer` trait. Rejected as far more invasive than the
  problem warrants.
- **Changing the `Gamer` trait (`start`/`command`) to accept an injectable
  randomizer.** Would ripple through every game crate, `brdgme_cmd`, the bot, and
  the web server. Rejected as out of scope; noted as the reason two specific
  tests keep the log-assertion pattern instead of being forced onto
  `ScriptedRandomizer`.
- **Continuing the log-content band-aid pattern for every newly-found flaky
  test.** This is what commits `1b10f4e` and `a4bed83` already did reactively.
  It works but is strictly weaker than scripted-randomizer assertions (can't
  check exact scores/turn index, only substrings of rendered log text) and
  doesn't stop the next occurrence from being discovered via a red CI run.
  Kept only for the two tests where the architecture leaves no alternative
  (see point 3 above).

## Full test audit (24 tests in `rust/game/zombie-dice-2/src/lib.rs`)

| Test | RNG-touched? | Status | Action |
|---|---|---|---|
| `test_player_counts` | no | safe | none |
| `test_start_initial_state` | yes (`Game::start`) | **latent flaky** (asserts `current_turn == 0`; a bust on the very first roll, low but nonzero probability, breaks it) | Task 7: fix via log invariant |
| `test_dice_face_counts` | no | safe | none |
| `test_all_dice_counts` | no | safe | none |
| `test_take_dice_basic` | no (cup large enough, no shuffle) | safe | none |
| `test_take_dice_refills_from_kept_when_cup_low` | yes (shuffle only) | safe (count-based assertions, order-independent) | none |
| `test_take_dice_zero_returns_empty` | no (early return) | safe | none |
| `test_roll_distributes_faces` | yes | safe (count invariant holds under any cascade) | none (mechanical signature update only, Task 3) |
| `test_keep_banks_brains_and_advances` | yes | already band-aided (`1b10f4e`, log assertions) | Task 4: upgrade to `ScriptedRandomizer` + strong assertions |
| `test_keep_wrong_player_errors` | no (errors before rolling) | safe | none (mechanical signature update only, Task 3) |
| `test_can_roll_and_can_keep` | no | safe | none |
| `test_leaders` | no | safe | none |
| `test_finished_unique_leader_at_threshold` | no (returns before any roll) | safe | none (mechanical signature update only, Task 3) |
| `test_finished_not_triggered_below_threshold` | yes | safe (`finished` can never flip true here regardless of cascade depth, since scores are untouched by rolling) | none (mechanical signature update only, Task 3) |
| `test_rolloff_starts_on_tie_at_threshold` | yes | safe (assertions are idempotent/order-independent under cascade) | none (mechanical signature update only, Task 3) |
| `test_rolloff_skips_non_rolloff_players` | yes | **CONFIRMED FLAKY** (the reported CI failure; empirically reproduced 3/40 local runs, two distinct panic sites) | Task 5: convert to `ScriptedRandomizer` |
| `test_rolloff_resolves_when_unique_leader` | yes | **latent flaky** (a bust on player 1's post-skip auto-roll can move `current_turn` away from 1 and can even prematurely set `finished`, breaking the assertion and possibly panicking the next `.unwrap()`) | Task 6: convert to `ScriptedRandomizer` |
| `test_placings_standard_competition_ties` | no | safe | none |
| `test_command_roll_and_keep` | yes (via `command()`, fixed trait entry point) | already band-aided (`a4bed83`, log assertions) - **cannot** be converted (architecture forces `RngRandomizer`) | Task 7: add explanatory comment only, no logic change |
| `test_command_wrong_player_errors` | no | safe | none |
| `test_command_unknown_input_errors` | no | safe | none |
| `test_command_after_finished_errors` | no | safe | none |
| `test_cup_refill_returns_kept_to_cup` | yes | safe (count invariant holds under any cascade) | none (mechanical signature update only, Task 3) |
| `test_pub_state_captures_rendered_fields` | yes | safe (compares `Game`'s own fields to its own snapshot; RNG-outcome-invariant by construction) | none |

## Coverage trade-offs

None taken deliberately for the tests being fixed - the scripted-randomizer
conversions in Tasks 4-6 **restore exact state assertions** (stronger than
the log-content band-aid they replace/would have needed). The two tests kept
on log assertions (`test_command_roll_and_keep`, and the invariant fix in
`test_start_initial_state`) are strictly no weaker than they are today; they
were never candidates for stronger assertions because the fixed `Gamer` trait
boundary makes the actual dice outcome opaque to the test either way.

## Global Constraints

These apply to every task below:

- **No changes to game rules or runtime behavior.** `RngRandomizer` must draw
  from the same entropy source (`rand::rng()` via `Rng::random_range` over
  the die's real face list) that `Dice::roll` uses today - production players
  see identical randomness characteristics.
- **All existing test intents preserved.** Every test's original comment/
  intent (what game-logic property it verifies) must still hold after its
  conversion; only the assertion mechanism (and, where specified, the
  RNG source) changes.
- **Do not modify the `Gamer` trait** (`rust/lib/game/src/game.rs`) or the
  external signatures of `Game::start` / `Game::command`
  (`rust/game/zombie-dice-2/src/lib.rs`). Both must keep using `RngRandomizer`
  internally, unconditionally.
- **Do not touch other game crates** (`farkle-2`, `liars-dice-2`, etc.) -
  only `brdgme_game` (shared infra) and `zombie-dice-2` are in scope. The
  plan documents the pattern so other crates *can* adopt it later, but does
  not migrate them now.
- **Exact names to use:**
  - Crate `brdgme_game`, path `rust/lib/game`.
  - Crate `zombie-dice-2`, path `rust/game/zombie-dice-2`.
  - New module `brdgme_game::randomizer` (file `rust/lib/game/src/randomizer.rs`):
    `pub trait Randomizer<T> { fn next(&mut self, faces: &[T]) -> T; }` and
    `pub struct RngRandomizer;` implementing it generically for `T: Copy`.
  - New module `brdgme_game::test_support` (file
    `rust/lib/game/src/test_support.rs`, gated on Cargo feature
    `test-support`): `pub struct ScriptedRandomizer<T> { .. }` with
    `ScriptedRandomizer::new(outcomes: impl IntoIterator<Item = T>) -> Self` and
    a `Randomizer<T>` impl that pops the queue (panics with a clear message if
    exhausted).
- **Cargo workspace root for all `cargo` commands is `rust/`** (verify with
  `test -f rust/Cargo.toml`). All `cargo` commands in this plan must be run
  with cwd `rust/` (or with equivalent `--manifest-path rust/Cargo.toml`).
- **Test filter convention:** the test module in `zombie-dice-2/src/lib.rs`
  is `mod test` (singular), so filters must use the full path
  `test::<test_name>`, e.g. `cargo test -p zombie-dice-2 --lib
  test::test_rolloff_skips_non_rolloff_players -- --exact`. Note `cargo
  test`'s `--exact` flag must be passed after `--`, not as a top-level cargo
  argument.
- **Final verification (after Task 7):** `cd rust && for i in $(seq 1 10);
  do cargo test -p zombie-dice-2 --lib || { echo "FAILED on run $i"; break;
  }; done` must complete all 10 runs green. Also confirm
  `cargo build --workspace` (or at minimum `cargo check -p brdgme_game
  --features test-support -p zombie-dice-2`) still succeeds, since
  `brdgme_game`'s Cargo.toml and public module surface are changing.
- **Match existing code style:** no project-level `rustfmt.toml` was found,
  so rely on `cargo fmt` defaults; run `cargo fmt -p brdgme_game -p
  zombie-dice-2` (or `cargo fmt --check` first to see the diff) before
  considering a task done. Match the doc-comment style already used in
  `rust/lib/cmd/src/test_support.rs` (a `//!` module doc explaining the
  feature gate and usage) for the new `brdgme_game::test_support` module.
- **Do not create any new files beyond the ones explicitly named above** (no
  extra abstractions, no speculative generalization beyond what's specified).

## Tasks

### Task 1: Add the `Randomizer` trait and `RngRandomizer` to `brdgme_game`

**Files:** `rust/lib/game/src/randomizer.rs` (new), `rust/lib/game/src/lib.rs`
(edit).

**Spec:**

Create `rust/lib/game/src/randomizer.rs`:

```rust
//! Injectable randomness for game mechanics that roll dice (or draw from any
//! other fixed set of discrete outcomes).
//!
//! Production code always uses [`RngRandomizer`], which draws a real uniformly
//! random outcome - identical behavior to calling `rand::rng()` directly.
//! Tests that need dice outcomes to matter to an assertion should use
//! `brdgme_game::test_support::ScriptedRandomizer` (behind the `test-support`
//! feature) to script an exact sequence of outcomes instead of leaving the
//! result to chance. Tests where the outcome is incidental (fully overwritten
//! by direct state construction afterward, or exercising a fixed `Gamer`
//! trait entry point like `start()`/`command()` that cannot take an
//! injected randomizer) should keep using `RngRandomizer` and assert only on
//! RNG-outcome-invariant properties (counts, deterministic log content,
//! structural equality) - see `zombie-dice-2` for a worked example of all
//! three cases.
//!
//! # Design boundary
//!
//! `Randomizer` abstracts *game-meaningful outcomes* - dice rolls, shuffled
//! deck order, tile draws - NOT `rand`'s primitive API. Production
//! implementations use `rand` internally; tests script the outcomes
//! directly (e.g. the exact post-shuffle order of a deck). The trait
//! intentionally does not mirror `rand`'s trait surface: scripting
//! primitives like `gen_range` makes test doubles depend on algorithm
//! internals (e.g. how many draws a Fisher-Yates shuffle consumes) and is
//! brittle. Input-taking operations beyond what `zombie-dice-2` needs
//! ("shuffle this deck", "roll N dice") are deliberately out of scope for
//! now - extend the pattern, at the outcome level, when a real consumer
//! (e.g. a card game) adopts it. For genuinely distribution-heavy needs,
//! the escape hatch is a test-local `Randomizer` impl over a seeded
//! `RngCore`, accepting that test's local seed-brittleness.
use rand::prelude::*;

/// A source of "which outcome did this roll show" decisions, given the list
/// of possible outcomes for a single roll.
pub trait Randomizer<T> {
    /// Return one of `faces` for a single roll. Implementations may ignore
    /// `faces` entirely (e.g. a scripted test double that always returns a
    /// pre-chosen value regardless of what was actually rolled).
    fn next(&mut self, faces: &[T]) -> T;
}

/// Production randomizer: draws a real uniformly random outcome from `faces`.
#[derive(Default)]
pub struct RngRandomizer;

impl<T: Copy> Randomizer<T> for RngRandomizer {
    fn next(&mut self, faces: &[T]) -> T {
        faces[rand::rng().random_range(0..faces.len())]
    }
}
```

In `rust/lib/game/src/lib.rs`, add `pub mod randomizer;` alongside the existing
`pub mod bot;` / `pub mod command;` / etc. lines (keep alphabetical order
consistent with the existing list: `bot`, `command`, `errors`, `game`,
`game_log`, `randomizer`).

**Verify:**
- `cd rust && cargo check -p brdgme_game` succeeds.
- `cd rust && cargo test -p brdgme_game` succeeds (no behavior change, just
  confirms nothing else broke).

---

### Task 2: Add the `test-support`-gated `ScriptedRandomizer` to `brdgme_game`

**Files:** `rust/lib/game/Cargo.toml` (edit), `rust/lib/game/src/lib.rs`
(edit), `rust/lib/game/src/test_support.rs` (new).

**Spec:**

In `rust/lib/game/Cargo.toml`, add a `[features]` section (there isn't one
currently):

```toml
[features]
test-support = []
```

In `rust/lib/game/src/lib.rs`, add (after the `randomizer` module line added
in Task 1):

```rust
#[cfg(feature = "test-support")]
pub mod test_support;
```

Create `rust/lib/game/src/test_support.rs`, mirroring the doc-comment style
of `rust/lib/cmd/src/test_support.rs`:

```rust
//! Deterministic-randomness test doubles for `Gamer` implementations.
//!
//! Enabled via the `test-support` feature so it is never compiled into
//! release builds. Game crates depend on it as a `dev-dependency` (with
//! `features = ["test-support"]`) and use `ScriptedRandomizer` in place of
//! `crate::randomizer::RngRandomizer` wherever a test needs specific dice/roll
//! outcomes to make an assertion meaningful, instead of asserting on
//! whatever real RNG happened to produce.

use std::collections::VecDeque;

use crate::randomizer::Randomizer;

/// A [`Randomizer`] that returns a pre-scripted sequence of outcomes instead of
/// drawing randomly, so a test can express an exact scenario (e.g. "roll two
/// brains then three shotguns") deterministically. Ignores the `faces`
/// argument entirely - the scripted outcome is returned regardless of what
/// the real roll's possible outcomes would have been.
pub struct ScriptedRandomizer<T> {
    queue: VecDeque<T>,
}

impl<T> ScriptedRandomizer<T> {
    /// Build a randomizer that yields `outcomes` in order, one per `next` call.
    pub fn new(outcomes: impl IntoIterator<Item = T>) -> Self {
        Self {
            queue: outcomes.into_iter().collect(),
        }
    }
}

impl<T> Randomizer<T> for ScriptedRandomizer<T> {
    fn next(&mut self, _faces: &[T]) -> T {
        self.queue
            .pop_front()
            .expect("ScriptedRandomizer: no more scripted outcomes queued")
    }
}
```

**Verify:**
- `cd rust && cargo check -p brdgme_game --features test-support` succeeds.
- `cd rust && cargo check -p brdgme_game` (without the feature) still
  succeeds and does NOT expose `brdgme_game::test_support` (confirms the
  `#[cfg(feature = ...)]` gate works) - e.g. run `cargo doc -p brdgme_game
  --no-deps 2>&1 | grep -i test_support` and confirm no output.

---

### Task 3: Wire `zombie-dice-2` onto `Randomizer`, mechanically thread it through `Game`

**Files:** `rust/game/zombie-dice-2/Cargo.toml` (edit),
`rust/game/zombie-dice-2/src/lib.rs` (edit).

**Spec:** This task is purely mechanical - it must not change any test's
assertions or behavior, only make the crate compile against the new
`Randomizer`-based API. (The suite may still show the same flakiness as today
after this task; Tasks 4-7 fix that.)

In `rust/game/zombie-dice-2/Cargo.toml`, add to `[dev-dependencies]`
(alongside the existing `brdgme_cmd` dev-dependency):

```toml
brdgme_game = { path = "../../lib/game", features = ["test-support"] }
```

In `rust/game/zombie-dice-2/src/lib.rs`:

1. Add import near the top (with the other `use brdgme_game::...` lines):
   `use brdgme_game::randomizer::{Randomizer, RngRandomizer};`

2. Delete the `Dice::roll` inherent method (currently lines 77-81):
   ```rust
   pub fn roll(self) -> Face {
       let faces = self.faces();
       let i = rand::rng().random_range(0..faces.len());
       faces[i]
   }
   ```
   (Keep `Dice::faces`. `use rand::prelude::*;` at the top of the file stays
   - `Game::shake_cup`'s `.shuffle()` call still needs it.)

3. Change `roll_dice` (currently lines 153-160) to take a randomizer:
   ```rust
   pub fn roll_dice(dice: &[Dice], roller: &mut impl Randomizer<Face>) -> DiceResultList {
       dice.iter()
           .map(|d| DiceResult {
               dice: *d,
               face: roller.next(d.faces()),
           })
           .collect()
   }
   ```

4. Add a `roller: &mut impl Randomizer<Face>` parameter to these `Game` methods,
   threading it through every internal call that currently calls one of
   these methods on `self` (do not add a parameter to `shake_cup` or
   `take_dice` - they stay as-is, real RNG, unchanged):
   - `start_turn(&mut self)` -> `start_turn(&mut self, roller: &mut impl Randomizer<Face>)`;
     its body's `self.roll()` call becomes `self.roll(roller)`.
   - `next_player(&mut self)` -> `next_player(&mut self, roller: &mut impl Randomizer<Face>)`;
     its two recursive calls `self.next_player()` / `self.start_turn()`
     become `self.next_player(roller)` / `self.start_turn(roller)`.
   - `player_roll(&mut self, player: usize)` -> add `roller: &mut impl
     Randomizer<Face>`; its `self.roll()` call becomes `self.roll(roller)`.
   - `roll(&mut self)` -> add `roller: &mut impl Randomizer<Face>`; its
     `roll_dice(&dice)` call becomes `roll_dice(&dice, roller)`, and its
     `self.next_player()` call (in the bust branch) becomes
     `self.next_player(roller)`.
   - `keep(&mut self, player: usize)` -> add `roller: &mut impl
     Randomizer<Face>`; its `self.next_player()` call becomes
     `self.next_player(roller)`.

5. In `impl Gamer for Game`, update the two call sites that construct the
   top-level randomizer:
   - `fn start`: `let logs = g.start_turn();` -> `let logs =
     g.start_turn(&mut RngRandomizer);`
   - `fn command`: `let logs = self.player_roll(player)?;` -> `let logs =
     self.player_roll(player, &mut RngRandomizer)?;`, and `let logs =
     self.keep(player)?;` -> `let logs = self.keep(player, &mut
     RngRandomizer)?;`

6. Update every existing test call site so the crate compiles, passing
   `&mut RngRandomizer` (do not use `ScriptedRandomizer` yet - that's Tasks 4-6).
   Exact call sites in `mod test`:
   - `test_roll_distributes_faces`: `g.roll();` -> `g.roll(&mut RngRandomizer);`
   - `test_keep_banks_brains_and_advances`: `g.keep(0).unwrap();` -> `g.keep(0, &mut RngRandomizer).unwrap();`
   - `test_keep_wrong_player_errors`: `g.keep(1).is_err()` -> `g.keep(1, &mut RngRandomizer).is_err()`
   - `test_finished_unique_leader_at_threshold`: `g.next_player();` -> `g.next_player(&mut RngRandomizer);`
   - `test_finished_not_triggered_below_threshold`: `g.next_player();` -> `g.next_player(&mut RngRandomizer);`
   - `test_rolloff_starts_on_tie_at_threshold`: `g.next_player();` -> `g.next_player(&mut RngRandomizer);`
   - `test_rolloff_skips_non_rolloff_players`: `g.next_player();` -> `g.next_player(&mut RngRandomizer);`, and `g.keep(0).unwrap();` -> `g.keep(0, &mut RngRandomizer).unwrap();`
   - `test_rolloff_resolves_when_unique_leader`: both `g.keep(0).unwrap();` and `g.keep(1).unwrap();` -> pass `&mut RngRandomizer`
   - `test_cup_refill_returns_kept_to_cup`: `g.roll();` -> `g.roll(&mut RngRandomizer);`
   - `test_command_roll_and_keep`, `test_command_wrong_player_errors`,
     `test_command_unknown_input_errors`, `test_command_after_finished_errors`:
     **no change** - these call `g.command(...)`, whose signature is
     unchanged (it builds `RngRandomizer` internally per step 5).

**Verify:**
- `cd rust && cargo build -p zombie-dice-2` succeeds.
- `cd rust && cargo test -p zombie-dice-2 --lib` runs (it is fine/expected if
  `test::test_rolloff_skips_non_rolloff_players` still occasionally fails at
  this point - not yet fixed).
- `cd rust && cargo test --workspace --lib` still builds and runs across the
  whole workspace (confirms no other crate was affected).

---

### Task 4: Convert `test_keep_banks_brains_and_advances` to `ScriptedRandomizer`

**Files:** `rust/game/zombie-dice-2/src/lib.rs` (single test function + one
import line).

**Spec:** This test currently exists as the `1b10f4e` log-assertion
band-aid:

```rust
    #[test]
    fn test_keep_banks_brains_and_advances() {
        let mut g = Game::start(2).unwrap().0;
        g.current_turn = 0;
        g.round_brains = 4;
        g.scores = vec![0, 0];
        let logs = g.keep(0).unwrap();
        // The turn passes to player 1, whose turn opens with an automatic
        // roll. That roll can bust (three shotguns) and legitimately pass
        // the turn straight back, so current_turn is not deterministic;
        // assert the advance via the logs instead.
        let rendered: String = logs
            .iter()
            .map(|l| brdgme_markup::to_string(&l.content))
            .collect::<Vec<String>>()
            .join("");
        assert!(rendered.contains("{{player 0}} kept {{b}}4{{/b}} brains"));
        assert!(rendered.contains("{{player 1}} rolled"));
    }
```

Replace it with (add `use brdgme_game::test_support::ScriptedRandomizer;` once,
near the top of `mod test`, right after `use super::*;`):

```rust
    #[test]
    fn test_keep_banks_brains_and_advances() {
        let mut g = Game::start(2).unwrap().0;
        g.current_turn = 0;
        g.round_brains = 4;
        g.scores = vec![0, 0];
        // Player 1's auto-roll is scripted to three brains so it cannot bust
        // and bounce the turn back - the turn advance is now a hard
        // assertion instead of a log-content check.
        let mut roller = ScriptedRandomizer::new(vec![Face::Brain, Face::Brain, Face::Brain]);
        let logs = g.keep(0, &mut roller).unwrap();
        assert_eq!(4, g.scores[0]);
        assert_eq!(1, g.current_turn);
        assert!(!logs.is_empty());
    }
```

**Verify:**
- `cd rust && cargo test -p zombie-dice-2 --lib
  test::test_keep_banks_brains_and_advances -- --exact` passes.
- Run it 10x in a loop to confirm determinism: `cd rust && for i in $(seq 1
  10); do cargo test -p zombie-dice-2 --lib
  test::test_keep_banks_brains_and_advances -- --exact || break; done`.

---

### Task 5: Convert `test_rolloff_skips_non_rolloff_players` to `ScriptedRandomizer` (fixes the CI failure)

**Files:** `rust/game/zombie-dice-2/src/lib.rs` (single test function).

**Spec:** Current (flaky) test:

```rust
    #[test]
    fn test_rolloff_skips_non_rolloff_players() {
        let mut g = Game::start(4).unwrap().0;
        g.scores = vec![WIN_SCORE, 5, WIN_SCORE, 5];
        g.current_turn = 3;
        // After next_player: wraps to 0, sees tie, starts rolloff with [0, 2].
        let _ = g.next_player(&mut RngRandomizer);
        assert_eq!(vec![0, 2], g.roll_off_players);
        // Player 0's turn now (rolloff participant). Keep some brains.
        g.round_brains = 1;
        let _ = g.keep(0, &mut RngRandomizer).unwrap();
        // After player 0 keeps, next_player skips 1 (not in rolloff) and starts
        // player 2's turn.
        assert_eq!(2, g.current_turn);
        assert!(!g.finished);
    }
```

(the `&mut RngRandomizer` calls above are what Task 3 leaves it at). Two rolls
happen in this test and both can bust with real RNG: player 0's
rolloff-start auto-roll (inside the `next_player()` call) and player 2's
auto-roll (inside the `keep(0)` cascade) - each draws exactly 3 dice. Replace
the whole function body with a single `ScriptedRandomizer` supplying 6
non-shotgun outcomes (3 for each roll), so neither can ever bust:

```rust
    #[test]
    fn test_rolloff_skips_non_rolloff_players() {
        let mut g = Game::start(4).unwrap().0;
        g.scores = vec![WIN_SCORE, 5, WIN_SCORE, 5];
        g.current_turn = 3;
        // Six scripted brains: 3 for player 0's rolloff-start auto-roll
        // (inside next_player, below), 3 for player 2's auto-roll (inside
        // keep(0), below). Neither can bust, so the turn sequence is
        // deterministic and the exact-state assertions below always hold.
        let mut roller = ScriptedRandomizer::new(vec![Face::Brain; 6]);
        // After next_player: wraps to 0, sees tie, starts rolloff with [0, 2].
        let _ = g.next_player(&mut roller);
        assert_eq!(vec![0, 2], g.roll_off_players);
        // Player 0's turn now (rolloff participant). Keep some brains.
        g.round_brains = 1;
        let _ = g.keep(0, &mut roller).unwrap();
        // After player 0 keeps, next_player skips 1 (not in rolloff) and starts
        // player 2's turn.
        assert_eq!(2, g.current_turn);
        assert!(!g.finished);
    }
```

**Verify:**
- `cd rust && cargo test -p zombie-dice-2 --lib
  test::test_rolloff_skips_non_rolloff_players -- --exact` passes.
- Confirm the fix under the load that reproduced the bug empirically during
  this investigation: `cd rust && for i in $(seq 1 100); do cargo test -p
  zombie-dice-2 --lib test::test_rolloff_skips_non_rolloff_players --
  --exact || { echo "FAILED on run $i"; break; }; done` - all 100 must pass
  (the original bug reproduced at ~7.5% failure rate over 40 runs, so 100
  green runs is strong evidence of a real fix, not luck).

---

### Task 6: Convert `test_rolloff_resolves_when_unique_leader` to `ScriptedRandomizer`

**Files:** `rust/game/zombie-dice-2/src/lib.rs` (single test function).

**Spec:** Current test (same latent bug class as Task 5: a bust on player
1's post-skip auto-roll, triggered inside the first `keep(0)` call, can move
`current_turn` away from 1 and can even cascade all the way to prematurely
setting `g.finished = true` - which would then make the second `g.keep(1,
&mut RngRandomizer).unwrap()` panic with a "can't keep, game finished" error
instead of the expected outcome):

```rust
    #[test]
    fn test_rolloff_resolves_when_unique_leader() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![WIN_SCORE, WIN_SCORE, 5];
        g.roll_off_players = vec![0, 1];
        // Player 0 keeps 1 brain, player 1 keeps 0 brains, then wrap to 0:
        // 0 has WIN_SCORE+1, 1 has WIN_SCORE -> unique leader, finished.
        g.current_turn = 0;
        g.round_brains = 1;
        let _ = g.keep(0, &mut RngRandomizer).unwrap();
        // Now player 1's turn.
        assert_eq!(1, g.current_turn);
        g.round_brains = 0;
        let _ = g.keep(1, &mut RngRandomizer).unwrap();
        // Wrap to 0 -> check leaders -> 0 leads alone -> finished.
        assert!(g.finished);
    }
```

Replace with (the first `keep(0)` call's `next_player()` cascade rolls
exactly 3 dice for player 1's auto-start - script those 3 to brains so it
cannot bust; the second `keep(1)` call never rolls at all, because player 2
is skipped by the rolloff and the wrap straight to player 0 hits the
game-finished check before any `start_turn()`/roll would happen, so reusing
the same roller for both calls is safe and simpler than a second one):

```rust
    #[test]
    fn test_rolloff_resolves_when_unique_leader() {
        let mut g = Game::start(3).unwrap().0;
        g.scores = vec![WIN_SCORE, WIN_SCORE, 5];
        g.roll_off_players = vec![0, 1];
        // Player 0 keeps 1 brain, player 1 keeps 0 brains, then wrap to 0:
        // 0 has WIN_SCORE+1, 1 has WIN_SCORE -> unique leader, finished.
        g.current_turn = 0;
        g.round_brains = 1;
        // Scripted so player 1's post-skip auto-roll (triggered below)
        // cannot bust and bounce the turn away from 1.
        let mut roller = ScriptedRandomizer::new(vec![Face::Brain, Face::Brain, Face::Brain]);
        let _ = g.keep(0, &mut roller).unwrap();
        // Now player 1's turn.
        assert_eq!(1, g.current_turn);
        g.round_brains = 0;
        let _ = g.keep(1, &mut roller).unwrap();
        // Wrap to 0 -> check leaders -> 0 leads alone -> finished. (Player 2
        // is skipped by the rolloff and the wrap lands on the finished check
        // before any roll would happen, so no further scripted outcomes are
        // consumed here.)
        assert!(g.finished);
    }
```

**Verify:**
- `cd rust && cargo test -p zombie-dice-2 --lib
  test::test_rolloff_resolves_when_unique_leader -- --exact` passes.
- Run 20x in a loop to build confidence given this was a latent (not yet
  CI-observed) flake: `cd rust && for i in $(seq 1 20); do cargo test -p
  zombie-dice-2 --lib test::test_rolloff_resolves_when_unique_leader --
  --exact || break; done`.

---

### Task 7: Fix `test_start_initial_state`; document why `test_command_roll_and_keep` keeps its band-aid

**Files:** `rust/game/zombie-dice-2/src/lib.rs` (two test functions, no other
changes).

**Spec:**

`test_start_initial_state` calls `Game::start(2)`, which internally always
uses `RngRandomizer` (Task 3, step 5) - it cannot take an injected randomizer
because `start()` is the fixed `Gamer::start` trait method. Its
`assert_eq!(0, g.current_turn)` is a low-probability latent flake (a bust on
the very first roll of the game passes the turn to player 1). Replace only
that one assertion with a deterministic log-content check; leave every other
assertion in the test unchanged:

Before:
```rust
    #[test]
    fn test_start_initial_state() {
        let (g, logs) = Game::start(2).unwrap();
        assert_eq!(0, g.current_turn);
        assert!(!g.finished);
        assert!(g.round_shotguns < BUST_SHOTGUN_COUNT);
        // 3 dice were rolled; brains + shotguns are kept, footprints are runners.
        assert_eq!(ROLL_DICE_COUNT, g.kept.len() + g.current_roll.len());
        // Cup has 13 - 3 taken = 10 dice remaining.
        assert_eq!(10, g.cup.len());
        assert!(!logs.is_empty());
    }
```

After:
```rust
    #[test]
    fn test_start_initial_state() {
        let (g, logs) = Game::start(2).unwrap();
        // `Game::start` always uses the real `RngRandomizer` (it's the fixed
        // `Gamer::start` entry point, so no scripted randomizer can be injected
        // here). A 3-shotgun bust on this very first roll is rare but
        // possible and would legitimately advance the turn to player 1;
        // assert the deterministic invariant (player 0's roll is always
        // logged first, unconditionally, before any bust is even checked)
        // instead of assuming current_turn stays 0.
        let rendered: String = logs
            .iter()
            .map(|l| brdgme_markup::to_string(&l.content))
            .collect::<Vec<String>>()
            .join("");
        assert!(rendered.contains("{{player 0}} rolled"));
        assert!(!g.finished);
        assert!(g.round_shotguns < BUST_SHOTGUN_COUNT);
        // 3 dice were rolled; brains + shotguns are kept, footprints are runners.
        assert_eq!(ROLL_DICE_COUNT, g.kept.len() + g.current_roll.len());
        // Cup has 13 - 3 taken = 10 dice remaining.
        assert_eq!(10, g.cup.len());
        assert!(!logs.is_empty());
    }
```

For `test_command_roll_and_keep`, make no logic/assertion changes - add an
explanatory comment directly above the function so a future reader
understands this is a deliberate, permanent exception, not a leftover
band-aid:

```rust
    // NOTE: `command()` is the fixed `Gamer::command` entry point and always
    // builds its own `RngRandomizer` internally (see the module doc on
    // `brdgme_game::randomizer`), so this test cannot inject a `ScriptedRandomizer`.
    // It deliberately asserts on deterministic log content instead of exact
    // turn state, because the auto-roll for the next player can legitimately
    // bust and bounce the turn back to `keeper`.
    #[test]
    fn test_command_roll_and_keep() {
        // ... existing function body unchanged ...
    }
```

**Verify:**
- `cd rust && cargo test -p zombie-dice-2 --lib test::test_start_initial_state -- --exact` passes.
- `cd rust && cargo test -p zombie-dice-2 --lib test::test_command_roll_and_keep -- --exact` passes (unchanged behavior).
- `cd rust && for i in $(seq 1 20); do cargo test -p zombie-dice-2 --lib test::test_start_initial_state -- --exact || break; done`.

---

### Task 8: Full-suite verification and cleanup pass

**Files:** none (verification only; fix anything `cargo fmt`/`cargo clippy`
flags in the files touched by Tasks 1-7 if needed).

**Spec:**

1. `cd rust && cargo fmt --check -p brdgme_game -p zombie-dice-2` - if it
   reports diffs, run `cargo fmt -p brdgme_game -p zombie-dice-2` and review
   the diff is only whitespace/formatting.
2. `cd rust && cargo clippy -p brdgme_game -p zombie-dice-2 --all-targets
   --features test-support -- -D warnings` - fix any new warnings introduced
   by this change (do not touch pre-existing warnings in unrelated code).
3. `cd rust && for i in $(seq 1 10); do cargo test -p zombie-dice-2 --lib ||
   { echo "FAILED on run $i"; exit 1; }; done` - all 10 full-suite runs must
   pass green (this is the plan's overall acceptance criterion).
4. `cd rust && cargo test --workspace --lib` - confirm no other crate in the
   workspace regressed (only `brdgme_game`'s public surface changed, and only
   additively - new modules, no removed/changed existing items).
5. `cd rust && cargo test -p zombie-dice-2` (includes `tests/contract.rs`,
   the `assert_gamer_contract` integration test) - confirm it still passes;
   it exercises `Game` purely through the `Gamer` trait (real `RngRandomizer`
   throughout) and should be unaffected.

**Verify:** all five commands above exit 0.
