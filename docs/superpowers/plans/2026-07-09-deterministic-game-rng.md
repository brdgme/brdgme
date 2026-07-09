# Deterministic Game RNG Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** All game randomness flows from a seed passed to `Gamer::start`, stored as a serde-serialized `GameRng` (ChaCha8) inside game state, making tests deterministic and fuzz failures reproducible.

**Architecture:** `brdgme_game::rng::GameRng` newtype over `rand_chacha::ChaCha8Rng`; `Gamer::start` gains a `seed: u64` parameter; `brdgme_cmd` `Request::New` gains `seed: Option<u64>` (None = entropy) and `Response::New` reports the seed used; each game stores `rng: GameRng` in state with a per-game serde-default migration shim.

**Tech Stack:** Rust workspace at `rust/` (run all cargo commands from `rust/`). rand 0.10.2, rand_chacha 0.10 (serde feature), serde.

**Spec:** `docs/superpowers/specs/2026-07-09-deterministic-game-rng-design.md`

## Global Constraints

- rand stays at `0.10.2`; new dep allowed: `rand_chacha = { version = "0.10", features = ["serde"] }` ONLY in `rust/lib/game`.
- Never call `rand::rng()`, `rand::random()`, or `rand::random_range()` inside game crates after their adoption task (harness crates bot.rs/fuzz keep their own `rand::rng()` for choosing actions - that is not game RNG).
- `GameRng` is the only RNG type games use. Games get it via `use brdgme_game::rng::GameRng;`.
- Migration shim on every EXISTING game crate's `rng` field: `#[serde(default = "GameRng::from_entropy")]` with comment `// Migration shim: pre-seed games get a fresh RNG on first load. Remove once no pre-RNG games remain active.`
- rand 0.10 API facts (verified): entropy seeding is `ChaCha8Rng::from_rng(&mut rand::rng())` (there is NO `from_os_rng`); the core trait is `rand::TryRng` (blanket impls give `Rng`/`RngExt`, so `.shuffle(&mut rng)`, `.random_range(..)`, `.choose(&mut rng)` all work on `GameRng`).
- Commit after each task with the trailer `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`. EXCEPTION: if executing in the session dated 2026-07-09 where the user said "don't commit or push for the rest of the session", skip all commit steps.
- Verify command after every task: `cargo build --workspace && cargo test --workspace` from `rust/` (or the narrower per-crate command given in the task).

---

### Task 1: `brdgme_game::rng` module (replaces `randomizer`)

**Files:**
- Create: `rust/lib/game/src/rng.rs`
- Delete: `rust/lib/game/src/randomizer.rs`
- Modify: `rust/lib/game/src/lib.rs:9` (`pub mod randomizer;` -> `pub mod rng;`)
- Modify: `rust/lib/game/Cargo.toml` (add rand_chacha)

**Interfaces:**
- Produces: `brdgme_game::rng::GameRng` with `GameRng::seed_from_u64(seed: u64) -> GameRng`, `GameRng::from_entropy() -> GameRng`, `Default` (seed 0), `Clone`, `Debug`, `PartialEq`, `Eq`, `Serialize`, `Deserialize`, and `rand::TryRng` (hence usable with `shuffle`/`random_range`/`choose`).

- [ ] **Step 1: Add dependency**

In `rust/lib/game/Cargo.toml` `[dependencies]` add:

```toml
rand_chacha = { version = "0.10", features = ["serde"] }
```

- [ ] **Step 2: Write `rust/lib/game/src/rng.rs` (module + inline tests)**

```rust
//! The single source of randomness for game mechanics.
//!
//! Every game stores a [`GameRng`] in its serialized state, seeded from the
//! `seed` passed to `Gamer::start`. All shuffles, dice rolls, and random
//! selections draw from that field - never from `rand::rng()` or any other
//! ambient source. Because the RNG state is part of game state, the same
//! seed and command sequence always reproduce the same game, across process
//! restarts and save/load cycles.
//!
//! `ChaCha8Rng` is used because rust-random guarantees its output stream is
//! portable and stable across crate versions (unlike `StdRng`/`SmallRng`,
//! which explicitly are not), and it serializes its full stream position.
//!
//! Portability note: avoid sampling `usize` ranges where cross-platform
//! reproducibility matters - `usize` sampling is word-size dependent. All
//! current targets are 64-bit, so in-repo games sample `usize` freely.

use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

/// A deterministic, serializable RNG owned by game state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRng(ChaCha8Rng);

impl GameRng {
    /// Seed deterministically; same seed = same stream, forever.
    pub fn seed_from_u64(seed: u64) -> Self {
        GameRng(ChaCha8Rng::seed_from_u64(seed))
    }

    /// Seed from OS entropy. Production path when no seed is supplied, and
    /// the serde default used as a migration shim for pre-seed game states.
    pub fn from_entropy() -> Self {
        GameRng(ChaCha8Rng::from_rng(&mut rand::rng()))
    }
}

/// Only so `#[derive(Default)]` game structs compile; `start()` must always
/// overwrite the field with a properly seeded value.
impl Default for GameRng {
    fn default() -> Self {
        GameRng::seed_from_u64(0)
    }
}

impl rand::TryRng for GameRng {
    type Error = std::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.0.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.0.next_u64())
    }

    fn try_fill_bytes(&mut self, dst: &mut [u8]) -> Result<(), Self::Error> {
        self.0.fill_bytes(dst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let draw = |seed| -> Vec<u8> {
            let mut r = GameRng::seed_from_u64(seed);
            (0..16).map(|_| r.random_range(0..100)).collect()
        };
        assert_eq!(draw(7), draw(7));
        assert_ne!(draw(7), draw(8));
    }

    #[test]
    fn serde_roundtrip_resumes_stream() {
        let mut r = GameRng::seed_from_u64(42);
        let _: u32 = r.random_range(0..100);
        let json = serde_json::to_string(&r).unwrap();
        let mut r2: GameRng = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
        assert_eq!(r.random_range(0..100u32), r2.random_range(0..100u32));
    }

    #[test]
    fn rand_ext_apis_work() {
        let mut r = GameRng::seed_from_u64(1);
        let mut v = vec![1, 2, 3, 4, 5];
        v.shuffle(&mut r);
        assert!(v.choose(&mut r).is_some());
    }
}
```

- [ ] **Step 3: Delete `rust/lib/game/src/randomizer.rs`; in `rust/lib/game/src/lib.rs` change `pub mod randomizer;` to `pub mod rng;`**

- [ ] **Step 4: Verify**

Run from `rust/`: `cargo test -p brdgme_game`
Expected: PASS including the three new rng tests.

- [ ] **Step 5: Commit** (skip if in the no-commit session)

```bash
git add rust/lib/game
git commit -m "Add brdgme_game::rng::GameRng, remove unused randomizer module"
```

---

### Task 2: Seed plumbing through `Gamer::start` and all callers (behavior unchanged)

Everything compiles and behaves exactly as before; games receive a seed and ignore it (adoption is per-game in later tasks).

**Files:**
- Modify: `rust/lib/game/src/game.rs:52` (trait signature)
- Modify: `rust/lib/game/src/bot.rs:117` (Fuzzer G::start call)
- Modify: `rust/lib/cmd/src/api.rs` (Request::New, Response::New)
- Modify: `rust/lib/cmd/src/requester/gamer.rs:25,81` (handle_new)
- Modify: `rust/lib/cmd/src/repl.rs:39`, `rust/lib/cmd/src/test_support.rs:44,58`
- Modify: `rust/lib/cmd/Cargo.toml` (add `rand = "0.10.2"`)
- Modify: `rust/tools/fuzz/src/lib.rs:146`
- Modify: `rust/api/src/controller/game.rs:54,652`
- Modify: `rust/web/src/game/server_fns.rs:265`, `rust/web/src/game/client.rs:92,127`, `rust/web/tests/ssr_pages.rs:337` (only if it uses `brdgme_cmd::api::Request`; its `GameRequest` may be a local mock type - adjust only real `Request`/`Response` uses)
- Modify: every `rust/game/*/src/lib.rs` `fn start` impl signature + every test call site of `Game::start(`

**Interfaces:**
- Produces: `fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError>` (trait); `Request::New { players: usize, seed: Option<u64> }` with `#[serde(default)]` on seed; `Response::New { game, logs, public_render, player_renders, seed: u64 }`.

- [ ] **Step 1: Trait change** in `rust/lib/game/src/game.rs`:

```rust
    fn start(players: usize, seed: u64) -> Result<(Self, Vec<Log>), GameError>;
```

- [ ] **Step 2: All 14 game crate impls**: change `fn start(players: usize)` to `fn start(players: usize, _seed: u64)` (underscore - unused until each game's adoption task). Crates: acquire-1, battleship-2, category-5-2, farkle-2, for-sale-2, greed-2, liars-dice-2, lords-of-vegas-1, lost-cities-1, lost-cities-2, no-thanks-2, sushi-go-2, sushizock-2, zombie-dice-2.

- [ ] **Step 3: All test call sites**: every `Game::start(N)` / `Game::start(N).unwrap()` in game crate test modules becomes `Game::start(N, 1)`. Mechanical:

```bash
cd rust && grep -rln "::start(" game/ lib/ | xargs sed -i -E 's/::start\(([0-9]+)\)/::start(\1, 1)/g'
```

Then `grep -rn "::start(" game/ lib/ | grep -v ", "` to catch non-literal-arg calls (e.g. `G::start(players)` in lib code) and fix by hand.

- [ ] **Step 4: `brdgme_cmd` API** in `rust/lib/cmd/src/api.rs`:

```rust
    New {
        players: usize,
        #[serde(default)]
        seed: Option<u64>,
    },
```

and in `Response::New` add field `seed: u64` (after `player_renders`).

- [ ] **Step 5: `handle_new`** in `rust/lib/cmd/src/requester/gamer.rs`; add `rand = "0.10.2"` to `rust/lib/cmd/Cargo.toml`:

```rust
            Request::New { players, seed } => Ok(handle_new::<G>(players, seed)),
```

```rust
fn handle_new<G: Gamer + Debug + Clone + Serialize + DeserializeOwned>(
    players: usize,
    seed: Option<u64>,
) -> Response {
    let seed = seed.unwrap_or_else(rand::random);
    match G::start(players, seed) {
        Ok((game, logs)) => GameResponse::from_gamer(&game)
            .map(|gs| {
                let (public_render, player_renders) = renders(&game);
                Response::New {
                    game: gs,
                    logs: CliLog::from_logs(&logs),
                    public_render,
                    player_renders,
                    seed,
                }
            })
            ...  // rest unchanged
```

- [ ] **Step 6: Update every `Request::New` constructor and `Response::New` pattern**:
  - `rust/lib/cmd/src/repl.rs:39`: add `seed: None,`; its `Response::New` match gains `..`.
  - `rust/lib/cmd/src/test_support.rs:44,58`: add `seed: None,`; `Response::New` matches gain `..` if not already.
  - `rust/tools/fuzz/src/lib.rs:146`: `&api::Request::New { players, seed: None }` (real seeding is Task 16).
  - `rust/api/src/controller/game.rs:54,652`: add `seed: None,`; the two `cli::Response::New` destructures gain `..` if they list fields exhaustively.
  - `rust/web/src/game/server_fns.rs:265`: add `seed: None,`; `rust/web/src/game/client.rs:92` (mock match): `Request::New { players, .. }`; `client.rs:127`: add `seed: None,`; fix any `Response::New` literals in web tests to include `seed: 0`.
  - `rust/lib/game/src/bot.rs:117`: `G::start(self.player_count, rand::random())`.

- [ ] **Step 7: Verify**

Run from `rust/`: `cargo build --workspace && cargo test --workspace`
Expected: builds clean; all existing tests pass (behavior unchanged - games still use `rand::rng()` internally).

- [ ] **Step 8: Commit** (skip if in the no-commit session)

```bash
git add -A rust/
git commit -m "Thread RNG seed through Gamer::start and brdgme_cmd New request"
```

---

### Task 3: zombie-dice-2 adoption + deterministic tests (proving case)

**Files:**
- Modify: `rust/game/zombie-dice-2/src/lib.rs`

**Interfaces:**
- Consumes: `brdgme_game::rng::GameRng` (Task 1), seeded `start` (Task 2).

- [ ] **Step 1: Add the field** to `Game` (`lib.rs:162-178`):

```rust
    // Migration shim: pre-seed games get a fresh RNG on first load.
    // Remove once no pre-RNG games remain active.
    #[serde(default = "GameRng::from_entropy")]
    pub rng: GameRng,
```

with `use brdgme_game::rng::GameRng;` at the top.

- [ ] **Step 2: Seed it in `start`** (`lib.rs:390`): after constructing `g`, before `g.start_turn(...)`/any roll:

```rust
        let mut g = Game {
            players,
            scores: vec![0; players],
            rng: GameRng::seed_from_u64(seed),
            ..Game::default()
        };
```

and rename `_seed` back to `seed`.

- [ ] **Step 3: Replace the two RNG call sites**:
  - `Colour::roll` (`lib.rs:76-80`) becomes:

```rust
    pub fn roll(self, rng: &mut GameRng) -> Face {
        let faces = self.faces();
        let i = rng.random_range(0..faces.len());
        faces[i]
    }
```

    Update its callers to pass `&mut self.rng`.
  - `shake_cup` (`lib.rs:216`): `self.cup.shuffle(&mut rand::rng());` -> `self.cup.shuffle(&mut self.rng);` (`shake_cup` takes `&mut self`; disjoint field borrows are fine).
  - Confirm `grep -n "rand::rng" src/lib.rs` returns nothing; drop `use rand::prelude::*;` if now unused (keep if `shuffle`/`random_range` ext traits still need it - they do, keep it).

- [ ] **Step 4: Rewrite the flake-prone tests** (`test_rolloff_skips_non_rolloff_players` at ~`lib.rs:741`, `test_rolloff_resolves_when_unique_leader`, and any test that asserts exact `current_turn`/`finished` after an auto-roll cascade). Pattern:

```rust
    /// Find a seed whose next roll from this state produces no shotguns, so
    /// the auto-roll cannot bust-cascade past the expected player.
    fn seed_where(g: &Game, pred: impl Fn(&Game) -> bool) -> u64 {
        (0..10_000)
            .find(|&s| {
                let mut probe = g.clone();
                probe.rng = GameRng::seed_from_u64(s);
                // trigger the same action the test is about to take
                pred(&probe)
            })
            .expect("no suitable seed in 10k")
    }
```

Each rewritten test: build the state as today, then `g.rng = GameRng::seed_from_u64(FOUND_SEED);` immediately before the action, with a comment stating the property the seed was searched for (e.g. `// seed 17: player 0's auto-roll shows no shotguns (searched via seed_where)`). Assert the exact post-action state. Hardcode the found constant; keep `seed_where` in the test module so the constant can be re-derived.

- [ ] **Step 5: Verify determinism**

```bash
cd rust && cargo test -p zombie-dice-2
for i in $(seq 100); do cargo test -p zombie-dice-2 --lib -q >/dev/null || { echo "FLAKE run $i"; break; }; done; echo done
```

Expected: 100 clean runs.

- [ ] **Step 6: Commit** (skip if in the no-commit session)

```bash
git add rust/game/zombie-dice-2
git commit -m "zombie-dice-2: seeded GameRng in state, deterministic tests"
```

---

### Tasks 4-15: adopt GameRng in the remaining 12 game crates

Each task: same shape as Task 3 (field + shim + seed in `start` + replace call sites + `grep -n "rand::" src/` must show only ext-trait `use` lines + run crate tests + commit). One task per crate; exact call sites below. In `start`, when RNG is needed before the struct exists, create `let mut rng = GameRng::seed_from_u64(seed);` first, draw from it, then move it into the struct (`rng,` field). Tests comparing whole `Game` values with `assert_eq!` may need `expected.rng = actual.rng.clone();` first - or compare fields instead.

**Task 4: farkle-2** (`rust/game/farkle-2/src/lib.rs`)
- `random_dice(n)` (`:181-184`) -> `fn random_dice(rng: &mut GameRng, n: usize) -> Vec<Die>` using `rng.random_range(1..=6u8)`; callers pass `&mut self.rng` (or `&mut rng` inside `start`).
- `start` (`:323`): `let mut rng = GameRng::seed_from_u64(seed); let current_player = rng.random_range(0..players);` and `rng,` in the struct literal.
- Test: `cargo test -p farkle-2`.

**Task 5: greed-2** (`rust/game/greed-2/src/lib.rs`)
- `random_dice` (`:230-235`): same treatment as farkle-2, drawing `DIE_FACES[rng.random_range(0..6)]`.
- `start` (`:380`): same seeded-local-rng pattern.
- Test: `cargo test -p greed-2`.

**Task 6: liars-dice-2** (`rust/game/liars-dice-2/src/lib.rs`)
- `roll_dice` (`:86-91`): `self.player_dice[p][d] = self.rng.random_range(1u8..=6);` - note `self.rng` borrow vs `self.player_dice` index assignment conflicts inside the same statement on older borrow rules; if the borrow checker objects, draw into a local first: `let v = self.rng.random_range(1u8..=6); self.player_dice[p][d] = v;`.
- `start` (`:226`): seeded local rng for `current_player`, then `rng,` into struct (struct uses `..Game::default()`, so list `rng` explicitly).
- Test: `cargo test -p liars-dice-2`.

**Task 7: lost-cities-1** (`rust/game/lost-cities-1/src/lib.rs`)
- `start_round` (`:101`): `deck.shuffle(&mut self.rng);`.
- `start` (`:492`): set `rng: GameRng::seed_from_u64(seed)` in the struct literal (uses `..Game::default()`).
- Test: `cargo test -p lost-cities-1`.

**Task 8: lost-cities-2** (`rust/game/lost-cities-2/src/lib.rs`)
- `start_round` (`:126`): `deck.as_mut_slice().shuffle(&mut self.rng);`.
- `start` (`:487`): as lost-cities-1.
- Test: `cargo test -p lost-cities-2`.

**Task 9: no-thanks-2** (`rust/game/no-thanks-2/src/lib.rs`)
- Pool shuffle (`:60`, helper building `remaining_cards`): give the helper a `rng: &mut GameRng` parameter; caller in `start` passes the seeded local rng.
- `start` (`:208`): `g.currently_moving = g.rng.random_range(0..players);` after `g.rng = GameRng::seed_from_u64(seed);` (or seeded-local pattern).
- Test: `cargo test -p no-thanks-2`.

**Task 10: sushi-go-2** (`rust/game/sushi-go-2/src/lib.rs`)
- `start_hand` dummy draw (`:296`): `let i = self.rng.random_range(0..self.hands[DUMMY].len());` (draw the bound into a local before indexing if borrows conflict).
- `start` (`:733`): `g.deck.shuffle(&mut g.rng);` (disjoint fields, fine).
- `:1079` is inside a test (`test_shuffle`) - seed a `GameRng` locally there.
- Test: `cargo test -p sushi-go-2`.

**Task 11: sushizock-2** (`rust/game/sushizock-2/src/lib.rs`)
- `roll_dice(n)` (`:131-136`) -> `fn roll_dice(rng: &mut GameRng, n: usize) -> Vec<DieFace>` with `*DIE_FACES.choose(rng).unwrap()`; callers pass `&mut self.rng`.
- `start` (`:635-636`): `g.blue_tiles.shuffle(&mut g.rng); g.red_tiles.shuffle(&mut g.rng);` after seeding `g.rng`.
- Test: `cargo test -p sushizock-2`.

**Task 12: category-5-2** (`rust/game/category-5-2/src/lib.rs`)
- `shuffle(cards)` (`:66-69`) -> `fn shuffle(mut cards: Vec<Card>, rng: &mut GameRng) -> Vec<Card>`.
- `start` (`:335`): seeded local rng; `deck: shuffle(deck(), &mut rng),` then `rng,` in the literal.
- Mid-game reshuffle (search `shuffle(` around `:219`-equivalent in current source): pass `&mut self.rng`, using `std::mem::take` on the discard pile if the same `self` borrow is needed twice.
- Test: `cargo test -p category-5-2`.

**Task 13: for-sale-2** (`rust/game/for-sale-2/src/lib.rs`)
- `start` (`:358-359`): `g.building_deck.shuffle(&mut g.rng); g.cheque_deck.shuffle(&mut g.rng);` after seeding.
- Test: `cargo test -p for-sale-2`.

**Task 14: lords-of-vegas-1** (`rust/game/lords-of-vegas-1/src/lib.rs`, `src/card.rs`)
- `roll()` (`lib.rs:71-73`) -> `pub fn roll(rng: &mut GameRng) -> usize { rng.random_range(DIE_MIN..=DIE_MAX) }`; update all callers (`grep -n "roll()" src/`) to pass `&mut self.rng` - if any caller only has `&self`, restructure to take `&mut self` (rolls only happen during command processing).
- `shuffled_deck(players)` (`card.rs:19-26`) -> add `rng: &mut GameRng` parameter.
- `start` (`lib.rs:79-91`): seeded local rng; pass to `shuffled_deck`; `current_player` from it; `rng,` into struct. Note this struct derives no `PartialEq` and has `Default` - keep both as-is.
- Test: `cargo test -p lords-of-vegas-1`.

**Task 15: acquire-1** (`rust/game/acquire-1/src/lib.rs`)
- `start` (`:177`): `tiles.as_mut_slice().shuffle(&mut g.rng);` after `g.rng = GameRng::seed_from_u64(seed);`.
- `start` (`:194`): `let start_player = g.rng.random_range(0..players);` (replaces the modulo-biased `next_u32() % players`; distribution change is fine, it was biased before).
- `:863` (`dummy_shares = rand::random_range(1..=5)` in `bonus_players`): INVESTIGATE FIRST - find `bonus_players`' receiver and callers (`grep -n "fn bonus_players\|bonus_players(" src/lib.rs`). If it is `&self` or called from a pure path (`points()`, render), drawing from `self.rng` needs `&mut self`; if a pure path genuinely calls it, draw the dummy share count once during command processing, store it in state, and have `bonus_players` read the stored value. Do NOT leave randomness in a `&self`/render path.
- Test: `cargo test -p acquire-1`.

Commit message per crate (skip if in the no-commit session): `"<crate>: seeded GameRng in state"`.

---

### Task 16: Fuzzer seed recording

**Files:**
- Modify: `rust/tools/fuzz/src/lib.rs`

**Interfaces:**
- Consumes: `Request::New { players, seed }`, `Response::New { .., seed }` (Task 2).

- [ ] **Step 1:** Add to `Fuzzer` struct (`:113-119`): `seed: Option<u64>,` and `command_log: Vec<String>,` (init `None` / `vec![]` in `try_new`).

- [ ] **Step 2:** In `new_game` (`:140`): generate and send a seed, reset the log:

```rust
        let seed: u64 = self.rng.random();
        match self.client.request(&api::Request::New { players, seed: Some(seed) })? {
            api::Response::New { game, player_renders, .. } => {
                self.seed = Some(seed);
                self.command_log.clear();
                ...
```

- [ ] **Step 3:** Record executed commands. `exec_rand_command` (`:281`) generates the command internally; make it return the command string alongside the result (e.g. change `CommandResponse::Ok(FuzzGame)` handling or have `exec_rand_command` return `(String, CommandResponse)`), and push every attempted command (including user-errored ones) onto `self.command_log` in `Fuzzer::command`.

- [ ] **Step 4:** Extend `FuzzStep::Error` (`:219-223`) with `seed: Option<u64>` and `commands: Vec<String>`; populate from `self.seed`/`self.command_log.clone()` at both construction sites (`:248`, `:256`); print them in the error arm (`:67-75`):

```rust
                println!(
                    "\nError detected: {}\n\nSeed: {:?}\nCommands: {:#?}\n\nCommand: {}\n\nGame: {:?}",
                    error, seed, commands, ...
                );
```

- [ ] **Step 5: Verify**: `cargo build -p brdgme_fuzz` (crate name per `rust/tools/fuzz/Cargo.toml`), then run the fuzzer briefly against one game binary if a target is documented in `docs/DEV.md`; otherwise build-only is acceptable.

- [ ] **Step 6: Commit** (skip if in the no-commit session): `"fuzz: record seed and command log for reproducible failures"`.

---

### Task 17: Documentation + retire superseded plan

**Files:**
- Create: `docs/authoring/GAME_DEVELOPMENT.md`
- Modify: `docs/porting/GAME_PORTING.md:36,77`
- Delete: `docs/superpowers/plans/zombie-dice-test-determinism.md`

- [ ] **Step 1: Create `docs/authoring/GAME_DEVELOPMENT.md`:**

```markdown
# Game Development Guidelines

Best practices for building game crates. The porting guide
(`docs/porting/GAME_PORTING.md`) covers Go-to-Rust conversion mechanics and
retires when porting finishes; this document is permanent.

## Randomness

All game randomness must be deterministic given the seed passed to
`Gamer::start`. See the `brdgme_game::rng` module docs for design rationale.

- Never call `rand::rng()`, `rand::random()`, or `rand::random_range()` in
  game code.
- Store one `rng: brdgme_game::rng::GameRng` field in your `Game` struct
  (it serializes with the rest of the state) and seed it first thing in
  `start()` with `GameRng::seed_from_u64(seed)`.
- Draw everything from that field: `deck.shuffle(&mut self.rng)`,
  `self.rng.random_range(1..=6)`, `faces.choose(&mut self.rng)`. Helper
  functions take `rng: &mut GameRng` as a parameter.
- Games with no randomness (e.g. Battleship) name the parameter `_seed`
  and add no field.
- Randomness only ever happens while processing `start()` or `command()`.
  Never draw from the RNG in `pub_state`, `player_state`, `status`,
  `points`, or render code - those must stay pure.
- Reference implementation: `rust/game/zombie-dice-2`.

Tests seed explicitly (`Game::start(3, 42)`) and may re-seed mid-test
(`g.rng = GameRng::seed_from_u64(n)`) to make a specific outcome happen;
comment what property the seed was chosen for. Same seed + same commands =
identical game, so exact assertions are safe.
```

- [ ] **Step 2:** `docs/porting/GAME_PORTING.md`: line 77 "Decks/hands are `Vec<Card>`; shuffle with `rand::seq::SliceRandom`." -> "Decks/hands are `Vec<Card>`; shuffle with the game's `rng` field - see `docs/authoring/GAME_DEVELOPMENT.md` (Randomness)." Add a pointer to the new doc near the top ("General best practices: `docs/authoring/GAME_DEVELOPMENT.md`").

- [ ] **Step 3:** Delete `docs/superpowers/plans/zombie-dice-test-determinism.md` (superseded; the design spec records why).

- [ ] **Step 4: Final verify**: from `rust/`: `cargo build --workspace && cargo test --workspace && cargo clippy --workspace` plus `grep -rn "rand::rng()" rust/game/` (expect no hits).

- [ ] **Step 5: Commit** (skip if in the no-commit session): `"Document seeded RNG guidelines; retire superseded zombie-dice plan"`.
