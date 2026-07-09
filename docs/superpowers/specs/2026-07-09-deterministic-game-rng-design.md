# Deterministic Game RNG - Design

Date: 2026-07-09
Status: Approved design, pending implementation plan
Supersedes: `docs/superpowers/plans/zombie-dice-test-determinism.md` (the
scripted-`Randomizer` approach on branch `zombie-dice-test-determinism`)

## Problem

Game crates draw randomness from `rand::rng()` (thread-local, OS-seeded)
directly inside `Gamer::start()` and `Gamer::command()`. Outcomes are
unreproducible from outside, causing:

- CI flake: zombie-dice-2 tests fail ~7.5% of runs because a bust cascade
  during an auto-roll moves `current_turn` further than the test assumed
  (two prior reactive patches: `1b10f4e`, `a4bed83`).
- No fuzzer reproducibility: a fuzz failure cannot be replayed.
- No exact replay of a game from its inputs.

A prior fix attempt (scripted `Randomizer<T>` test doubles injected per
mechanic) was reviewed and rejected: game-specific, adds a test-double
surface per mechanic, cannot reach RNG inside the fixed `start()`/`command()`
entry points, and delivers no fuzzer reproducibility.

## RNG usage survey (basis for the design)

All game-mechanic randomness across the Rust crates (14 games) and Go games
(17 games, surveyed as a use-case forecast) reduces to five patterns:

1. Shuffle a deck/tiles/cup (most common; `start()` plus occasional mid-game
   reshuffles in `command()`)
2. Shuffle-then-take-subset (No Thanks, Splendor nobles)
3. Roll N uniform dice (Farkle, Greed, Liar's Dice, ...)
4. Pick uniformly from a custom face list - covers weighted dice via
   repeated faces (Zombie Dice, Sushizock)
5. Random starting player (random index in `[0, n)`)

No weighted sampling, no distributions, no other uses. Some games use no
randomness at all (Battleship, Cathedral). All RNG sits inside
`Gamer::start()` and `Gamer::command()`; every other trait method is pure.
There is no seed plumbing anywhere in the codebase.

## Design

One trait parameter, one field per game, one canonical RNG type. No new
traits.

### Shared crate: `brdgme_game`

- Remove the unused `Randomizer` trait / `randomizer` module added on branch
  `zombie-dice-test-determinism` (dead code; superseded by this design).
- New `brdgme_game::rng` module exporting one canonical RNG type:

  ```rust
  pub use rand_chacha::ChaCha8Rng as GameRng;
  ```

  Rationale: rust-random-maintained, output stream guaranteed portable and
  stable across crate versions (unlike `StdRng`/`SmallRng`, which explicitly
  are not, and which lost `Clone`/serde in rand 0.10), and full RNG state is
  serde-serializable. Fallback if `rand_chacha` compatibility with rand
  0.10's renamed traits proves awkward: `rand_pcg::Pcg64` (same guarantees).
  Verified at implementation time.
- `Gamer::start` gains a seed parameter:

  ```rust
  fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError>;
  ```

  One mechanical trait change, applied once across all game crates and
  callers.

### Per game

- Games with randomness add one serde-serialized field to their state:

  ```rust
  rng: GameRng,   // seeded via GameRng::seed_from_u64(seed) in start()
  ```

  Every `rand::rng()` call becomes `&mut self.rng`. Games keep using
  standard rand APIs (`shuffle`, `random_range`, `choose`) on that field -
  no new abstraction between the game and rand.
- Games without randomness ignore the parameter (`_seed`) and add no field.
- Because RNG state is part of serialized game state, determinism spans
  every `command()` (mid-turn rolls included) and survives save/reload.

### Callers

- `brdgme_cmd` `Request::New` gains `seed: Option<u64>`
  (serde-backward-compatible). `None` = generate from OS entropy - the
  production path is behaviorally unchanged. The generated seed is included
  in the response so callers can record it.
- Fuzzer generates an explicit seed per run and records
  `(seed, command list)`; any failure is fully reproducible.
- Bot / rand_bot harness RNG (choosing which command to fuzz with) is
  unrelated to game RNG and unchanged, except the fuzzer seeding above.

### Tests

- `G::start(players, FIXED_SEED)` makes an entire game deterministic; tests
  may assert exact post-roll state. The zombie-dice-2 cascade flake
  disappears.
- ChaCha's cross-version stability removes the "rand version bump silently
  changes outcomes" brittleness that ruled out seeded `StdRng`.
- Residual caveat: a seed does not self-document ("why does seed 42 bust?").
  Mitigation: keep using direct state construction where the roll is
  incidental; where the roll matters, a comment states the intended outcome
  the seed was chosen for. Tests may also overwrite `self.rng` mid-test with
  a fresh known seed to localize seed choice.
- Portability footnote: sampling `usize` ranges is word-size dependent. All
  targets are 64-bit; recorded as a doc note in `brdgme_game::rng`, not a
  mechanism.

### Documentation

- `docs/porting/GAME_PORTING.md`: update crate-layout deps and the "shuffle
  with `rand::seq::SliceRandom`" line; add a "Randomness" section: never
  call `rand::rng()` in game code; store a `GameRng` seeded from `start()`'s
  seed and draw everything from it; games without randomness ignore the
  seed. Point at a migrated reference game (zombie-dice-2).
- Rustdoc on `brdgme_game::rng` is the canonical explanation (why
  `ChaCha8Rng`, determinism guarantees, `usize` caveat); the porting guide
  links there rather than duplicating.

## Alternatives rejected

- **Scripted `Randomizer` injection (prior branch direction)**: tests only;
  per-mechanic test-double surface; cannot reach `start()`/`command()`
  internals; no fuzzer reproducibility.
- **Per-game `start_with_seed` convention, no trait change**: unenforced,
  and the seed is invisible to `brdgme_cmd`/fuzzer, losing reproducible
  fuzzing.
- **Injected `&mut impl Rng` parameter on `command()`**: callers own RNG
  lifetime, generics infect every signature, and save/reload resumability
  still requires serializing RNG state - strictly worse than storing it in
  state.
- **Seeded `StdRng`**: no cross-version output stability; no serde in rand
  0.10.

## Migration and rollout

- The trait change forces every game crate and caller to update in one
  commit (mechanical: signature + field + `rand::rng()` replacement).
- Serialized-state compatibility: adding the `rng` field changes each game's
  state schema. In-flight games without the field need a serde default
  (e.g. `#[serde(default = ...)]` seeding from OS entropy on first
  deserialize) or are accepted as broken per existing state-migration
  practice - decided per existing project policy at implementation time.
- zombie-dice-2's flaky tests are rewritten against fixed seeds as the
  proving case.

## Success criteria

- No game crate calls `rand::rng()` (or any ambient RNG).
- `zombie-dice-2` test suite passes repeatedly (e.g. 100 consecutive runs).
- Fuzzer failure output includes seed + commands and replays exactly.
- Same seed + same commands = byte-identical serialized game state.
