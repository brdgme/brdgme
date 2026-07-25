# WP-22: lords-of-vegas-1 fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Remove the five `unimplemented!()` panic arms from command dispatch (d F1), make boss-tie reroll order deterministic per seed (d F2), stop the reachable render underflow when the 9-tile casino supply is exceeded in 5-6 player games (d F4, verification-upgraded to major), make boss-tie rerolls visible in the log (d F3), and land the mechanical riders: lazy_static -> std LazyLock (d F6), invariant comment on the starting-cash `unreachable!()` (d F7), serde_json to dev-dependencies (d F8), dead `FromIterator` import (d F9), `BLOCK_WIDTH` instead of literal `3` in the renderer (d F10), RULES.md colour wording (d F11).

**Architecture — how lords-of-vegas-1 works (read this before editing):**

- One crate, `rust/game/lords-of-vegas-1` (package name `lords-of-vegas-1`, confirmed from `Cargo.toml:2`; lib name `lords_of_vegas_1`). Deliberately partial port: RULES.md "Implementation status" (RULES.md:98-105) states only `build`/`done` are implemented — no card draws, payouts, scoring, or endgame; the game runs indefinitely.
- `src/lib.rs`: `Game` (serde-persisted, all fields `pub`, seeded `GameRng` field with a `#[serde(default = "GameRng::from_entropy")]` migration shim at lib.rs:82-85), the `Gamer` impl, `build()`/`done()`, and an inline `#[cfg(test)] mod tests` (lib.rs:336-352, currently 2 tests). Test module name is `tests`.
- `src/board.rs`: `Loc` (block A-F + lot number; derives `Ord`, board.rs:73; custom string serde so it can key JSON maps, board.rs:129-164), `Board(HashMap<Loc, BoardTile>)` (board.rs:186-187), flood-fill `casino_at()` (board.rs:236-273), `casinos()` (board.rs:275-288), `reroll_at()` (board.rs:290-312), `resolve_boss_ties()` (board.rs:314-344), plus an inline `mod tests` (board.rs:379-539, 3 tests).
- `src/tile.rs`: static `TILES: HashMap<Loc, Tile>` via `lazy_static!` (tile.rs:23-25) — the 48-lot data table (die value, build cost, starting cash, payout casino, strip flag). Never serialized; pure static data.
- `src/command.rs` (private module `mod command;`, lib.rs:21): `Command` enum and parsers. Only the build and done parsers are wired into `command_parser()` (command.rs:19-28); the sprawl/remodel/reorg/gamble/raise parsers (command.rs:49-152) are written and `pub` but NOT wired.
- `src/render.rs`: markup renderer. `render_player_table` subtracts used dice/tokens from the `PLAYER_DICE`/`PLAYER_OWNER_TOKENS` supplies (render.rs:78-86); `render_casino_table` subtracts built-tile counts from `CASINO_TILES` (render.rs:112-119). `render_block` hardcodes grid width 3 (render.rs:154-155).
- `src/card.rs`: deck of 48 `Card::Loc` + 1 `Card::GameEnd`, inserted in the last quarter of the deck (card.rs:27-33; the invariant comment lives at card.rs:27-29).
- Turn/game flow: `start()` deals each player 2 lot cards (ownership + starting cash), `build <lot> <casino>` converts an `Owned` lot to `Built` (any colour, cost deducted, owner die = the lot's fixed die value), building can trigger `resolve_boss_ties` (reroll all tied boss dice, recursively), `done` passes the turn. `status()` never finishes (`finished` is only ever false in practice).
- Serialization: `Game`/`PubState`/`PlayerState` round-trip through the DB as serde JSON. **No fix in this package may change any serialized type or field.** `Board` stays `HashMap<Loc, BoardTile>` (serializes as a JSON object via `Loc`'s string serde; key order is irrelevant to round-tripping). All determinism fixes below use transient locals only.
- Bins (`src/bin/*.rs`) are the 4 standard boilerplate binaries (cli/fuzz/http/repl); none uses `serde_json` or `lazy_static` directly. `tests/contract.rs` is the standard `assert_gamer_contract::<Game>()` harness.

**Tech Stack:** Rust 1.97.0 (edition 2024) workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `lords-of-vegas-1`. `std::sync::LazyLock`, `BTreeSet::pop_first`, and let-chains are all available on this toolchain.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p lords-of-vegas-1`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- All 6 existing tests (`player_counts_works`, `json_works` in lib.rs; `loc_neighbours_works`, `test_board_casino_at_works`, `test_board_casinos_works` in board.rs; `game_contract` in tests/contract.rs) MUST keep passing unmodified. None constrains any fix below: the board tests compare sorted neighbour lists and casino counts (order-independent), and the contract test only exercises start/serde/rules plumbing.
- Line numbers cited are LIVE-file numbers as of the drift check below. Task 1 shifts lib.rs numbering below line ~195 and Task 2 shifts board.rs numbering below line ~247 — later tasks locate by symbol name where noted.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).

**Non-Goals:**

- d F5 (`Loc::parse_str` accepts out-of-range lots; `neighbours()` underflows on lot 0) — owned by WP-09 (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36). Do NOT add validation to `Loc::parse_str` (board.rs:80-91) or touch `neighbours()`' arithmetic (board.rs:100-116).
- d F12 (player counts 2-6 vs official 2-4) — owned by WP-26 (batch-d rules adjudication, BLOCKED-ON-DECISION). Do NOT change `player_counts()`/`start()` bounds (lib.rs:97-103, 225-227) and do NOT add player-count wording to RULES.md.
- Implementing any of the five missing actions (sprawl/remodel/reorg/gamble/raise), card draws, payouts, or scoring — the crate is a documented partial port; Task 1 only makes the dispatch non-panicking.
- Enforcing the per-player die (12) and owner-token (10) supplies in `build()` — unreachable today (each player owns at most the 2 dealt lots, so at most 2 dice/2 tokens are ever used; re-derived below under Task 3). Becomes relevant only when card draws are implemented; the renderer saturation covers display safety meanwhile.
- Restricting which casino colour may be built where (any lot can currently be built any colour) — gameplay-fidelity question for the future full implementation, not a review finding.
- The unused `thiserror` dependency (Cargo.toml:14) — observed during spec-writing, not a batch-d finding; leave it (workspace manifest hygiene is WP-65/WP-64 territory).

**Snapshot drift:** None. `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/lords-of-vegas-1 /home/beefsack/Development/brdgme/rust/game/lords-of-vegas-1` is empty (verified 2026-07-25 against snapshot commit f8763a5). All line numbers below are live-file numbers and match the findings' citations.

**Re-derivation notes (verified against live source):**

- **F1 (unimplemented!() dispatch):** `command()` (lib.rs:172-194) maps `Command::Remodel`/`Reorg`/`Sprawl`/`Gamble`/`Raise` to `unimplemented!()` (lib.rs:182-186), under `#[allow(unused_variables)]` (lib.rs:172) because the arms bind fields they never use. Today the arms are unreachable ONLY because `command_parser()` (command.rs:19-28) wires just `build_parser`/`done_parser`; the other five parsers (command.rs:49-152) are complete and `pub` — one `parsers.push(...)` away from a valid player command panicking the game process (each game crate serves HTTP via `brdgme_cmd`; a panic kills the request/worker). The finding's recommendation (return `GameError::InvalidInput`) is correct; the arms are not testable through `command()` (the parser cannot produce those variants), so the fix extracts the match into a private `dispatch(&mut self, player, cmd)` helper that a unit test can drive directly with constructed `Command` values.
- **F2 (RNG-order nondeterminism):** three iteration-order sources feed the seeded reroll stream:
  1. `casino_at()` pops BFS candidates via `queue.iter().next()` on a `HashSet<Loc>` (board.rs:247-250). The queue `HashSet` is constructed fresh per call, and std `RandomState` is per-instance, so the visit order — and therefore the order of `tiles` in the returned `BoardCasino` — varies even between two calls in the SAME process.
  2. `casinos()` iterates `TILES.keys()` (board.rs:278), a `HashMap` — stable within one process (single static instance) but random across processes.
  3. `resolve_boss_ties()` (board.rs:314-344) iterates `self.casinos()` and, per tied casino, rerolls `boss_tiles()` in `tiles` order (board.rs:330-332), consuming `GameRng` draws; the recursive re-tie pass (board.rs:337) means the ORDER also changes how many draws occur. Two replays of the same state+seed can therefore diverge.
  The `bosses: HashSet<usize>` (board.rs:320-324) is only `len()`-checked — no order dependence, leave it. Fix: BFS pops the smallest pending `Loc` via a transient `BTreeSet` (`Loc` derives `Ord`, board.rs:73), and `casinos()` iterates a sorted `Vec` of TILES keys. The finding's alternative "switch `TILES`/`Board` to `BTreeMap`" is NOT taken: `TILES` is static data that only needs sorted iteration in one place, and re-typing the serialized `Board` field is exactly the class of change WP-13 flagged for shape caution — unnecessary here since both fixes are transient locals (for the record, `HashMap<Loc, _>` and `BTreeMap<Loc, _>` would serialize to the same JSON-object shape because `Loc` serializes as a string, but no serialized type is touched at all).
- **F4 (render underflow, verification-upgraded to major):** `build()` (lib.rs:251-314) enforces ownership, not-already-built, and cash — NO supply limits and no colour restriction. `render_casino_table` computes `CASINO_TILES - self.board.casino_tile_count(*casino)` (render.rs:117) with `CASINO_TILES = 9` (lib.rs:29). With 5-6 players there are 10-12 owned lots and every one may be built the SAME colour, so the 10th same-colour build makes every subsequent render underflow `usize` — panic in debug, absurd number in release. The dice/token halves (render.rs:80, 85) are latent: each player owns at most 2 lots (dealt in `start()`, no draw mechanism), so `used.dice <= 2 < 12` and `used.tokens <= 2 < 10`. The `CASINO_CARDS - casino_card_count(...)` subtraction (render.rs:109) cannot underflow (at most 9 lot cards per casino exist, verified in the tile table). Fix is two-sided: `build()` rejects building a colour whose 9-tile supply is exhausted (the actual game rule, and it stops the state from going bad), AND the renderer uses `saturating_sub` on all three cited subtractions — required regardless of the build guard, because already-saved games may ALREADY hold >9 tiles of one colour and must keep rendering.
- **F3 (silent rerolls):** `resolve_boss_ties` builds `logs: Vec<Log>` but only ever extends it with the recursive result — which is also always empty — and discards `reroll_at`'s returned die (board.rs:331). On a tie, `build()` (lib.rs:307-311) extends the empty vec and sets `can_undo = false`: dice change silently, undo is disabled for a change the player was never shown. RULES.md:85-89 describes rerolls as player-facing. `reroll_at` returns `Some(die)` for every boss tile (boss_tiles only contains owner-Some tiles, board.rs:361-376), so the new value is available to log.
- **F7 detail (verification-adjusted):** the `Card::GameEnd => unreachable!()` at lib.rs:118 is provably unreachable: `shuffled_deck` (card.rs:20-35) inserts GameEnd at `cards_len - quart_pos` where `quart_pos < (48 - 2*players)/4`; the minimum insert position is 48 - 10 = **38** (2 players), while starting hands drain at most 6*2 = 12 cards from the front. The invariant comment exists at card.rs:27-29; the missing breadcrumb is at lib.rs:118 itself.

---

### Task 1: replace `unimplemented!()` dispatch arms with errors (d F1, major)

**Problem (restated):** lib.rs:182-186 maps five parseable-in-principle commands to `unimplemented!()`. Repo rules forbid panic macros on runtime paths reachable from player commands; these arms are one parser-wiring line away from being reachable, and a panic kills the serving process instead of returning a `GameError`.

**Fix (re-derived):** extract the dispatch match into a private helper so it is unit-testable, then replace each `unimplemented!()` with `Err(GameError::InvalidInput { message: "<verb> is not implemented yet".to_string() })`, matching the crate's existing struct-literal error style (`build()` lib.rs:258-260). Use `..` patterns so the unused field bindings — and with them the `#[allow(unused_variables)]` at lib.rs:172 — go away.

**Edge cases:**
- The `Gamble` arm currently binds `player`, shadowing the `command()` parameter (lib.rs:185); the `..` pattern removes the shadow hazard.
- `Command::Build`/`Command::Done` behavior must be byte-identical: same calls, same `?` propagation, same `CommandResponse` fields.
- `output.value` is moved into `dispatch` while `output.remaining` is still read afterwards — this partial move is exactly what the current code does inline; keep the field access order.
- `mod command` is private, but `crate::command::Command` is visible to the crate-internal `tests` module, so the test can construct variants directly.

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/lib.rs` (`command()` lines 172-194, new `dispatch` in `impl Game`, tests)

**Steps:**

- [ ] Mechanical extraction (behavior-preserving, panics kept for the moment). Replace `command()` (lib.rs:172-194) with:

```rust
    fn command(
        &mut self,
        player: usize,
        input: &str,
        players: &[String],
    ) -> Result<CommandResponse, GameError> {
        let output = self.command_parser(player).parse(input, players)?;
        let (logs, can_undo) = self.dispatch(player, output.value)?;
        Ok(CommandResponse {
            logs,
            can_undo,
            remaining_input: output.remaining.to_string(),
        })
    }
```

  (note: the `#[allow(unused_variables)]` attribute at lib.rs:172 is deleted) and add to `impl Game` (the block starting lib.rs:246), next to `can_build`:

```rust
    fn dispatch(&mut self, player: usize, cmd: Command) -> Result<(Vec<Log>, bool), GameError> {
        match cmd {
            Command::Build { loc, casino } => self.build(player, &loc, casino),
            Command::Remodel { loc, casino } => unimplemented!(),
            Command::Reorg { loc } => unimplemented!(),
            Command::Sprawl { from, to } => unimplemented!(),
            Command::Gamble { player, amount } => unimplemented!(),
            Command::Raise { loc } => unimplemented!(),
            Command::Done => self.done(player),
        }
    }
```

  For this intermediate step keep `#[allow(unused_variables)]` on `dispatch` (it moves rather than disappears; it is deleted in the fix step below). Run `cargo test -p lords-of-vegas-1` — all 6 existing tests PASS (pure extraction).
- [ ] Write the failing test. Add to `mod tests` in `rust/game/lords-of-vegas-1/src/lib.rs`:

```rust
    #[test]
    fn unimplemented_commands_error_instead_of_panicking() {
        // d F1: the five unwired command arms must return a GameError, not
        // panic the process, so that wiring their parsers in later can never
        // turn a valid player command into a crash.
        use crate::board::Block;
        use crate::command::Command;

        let mut g = Game::start(2, 1).expect("could not start game").0;
        let p = g.current_player;
        let loc: Loc = (Block::A, 1).into();
        let cmds = vec![
            Command::Remodel {
                loc,
                casino: Casino::Albion,
            },
            Command::Reorg { loc },
            Command::Sprawl {
                from: loc,
                to: (Block::A, 2).into(),
            },
            Command::Gamble {
                player: (p + 1) % 2,
                amount: 5,
            },
            Command::Raise { loc },
        ];
        for cmd in cmds {
            match g.dispatch(p, cmd) {
                Err(GameError::InvalidInput { message }) => assert!(
                    message.contains("not implemented"),
                    "unexpected message: {}",
                    message
                ),
                Ok(_) => panic!("expected InvalidInput error, got Ok"),
                Err(e) => panic!("expected InvalidInput error, got: {}", e),
            }
        }
    }
```

- [ ] Run: `cargo test -p lords-of-vegas-1 unimplemented_commands` — expected FAIL: the test panics with `not implemented` (the `unimplemented!()` macro message) on the first `dispatch` call.
- [ ] Implement: replace the five panic arms in `dispatch` (and delete its temporary `#[allow(unused_variables)]`):

```rust
    fn dispatch(&mut self, player: usize, cmd: Command) -> Result<(Vec<Log>, bool), GameError> {
        match cmd {
            Command::Build { loc, casino } => self.build(player, &loc, casino),
            Command::Remodel { .. } => Err(GameError::InvalidInput {
                message: "remodel is not implemented yet".to_string(),
            }),
            Command::Reorg { .. } => Err(GameError::InvalidInput {
                message: "reorg is not implemented yet".to_string(),
            }),
            Command::Sprawl { .. } => Err(GameError::InvalidInput {
                message: "sprawl is not implemented yet".to_string(),
            }),
            Command::Gamble { .. } => Err(GameError::InvalidInput {
                message: "gamble is not implemented yet".to_string(),
            }),
            Command::Raise { .. } => Err(GameError::InvalidInput {
                message: "raise is not implemented yet".to_string(),
            }),
            Command::Done => self.done(player),
        }
    }
```

- [ ] Run: `cargo test -p lords-of-vegas-1` — new test PASSES, all 6 existing tests PASS.
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/lib.rs` ; message: `fix(lords-of-vegas-1): return errors from unimplemented command arms (d F1, WP-22)`

NOTE: this task shifts lib.rs line numbers below ~line 195 by roughly +25; later tasks cite `build()` and the tests module by symbol.

---

### Task 2: deterministic iteration for boss-tie rerolls (d F2, major)

**Problem (restated):** `casino_at()`'s BFS pops from a per-call `HashSet` (board.rs:247-250) and `casinos()` iterates the `TILES` `HashMap` (board.rs:278). `resolve_boss_ties()` consumes the seeded `GameRng` in that order (and, via the recursive re-tie pass at board.rs:337, the order changes how MANY draws occur), so two replays of the same state+seed diverge. Breaks deterministic replay/audit; the per-call `HashSet` even makes two identical calls in the same process diverge.

**Fix (re-derived):** make both iterations ordered using transient locals only — no serialized type changes:

- `casino_at()`: queue becomes a `BTreeSet<Loc>` popped with `pop_first()` (smallest pending `Loc` first; `Loc` derives `Ord`). This also removes the `expect("queue shouldn't be empty")` at board.rs:248. `visited` stays a `HashSet` (only `contains`/`insert`, no iteration).
- `casinos()`: collect `TILES.keys()` into a `Vec<Loc>`, sort, iterate.

Downstream, `resolve_boss_ties` needs no change: `casinos()` order and each `BoardCasino.tiles` order (hence `boss_tiles()` order, which preserves tiles order) are now fully determined by board content.

**Edge cases:**
- `BoardCasino.tiles` order changes from "random" to "BFS by smallest loc". `tiles` is never serialized (`BoardCasino` is a transient struct) and the only order-sensitive consumer is the reroll loop this fix is canonicalizing. The existing board tests compare full `tiles` vectors (`test_board_casino_at_works`, board.rs:420-496) — their expected vectors are listed in ascending loc order (A1, A2, A5), which is exactly the BFS-from-A1 pop order (A1 -> {A2, A4dne, A5...}), so they keep passing; if any ordering assertion were to fail it means the implementation deviated from smallest-first, not that the test needs changing.
- Existing saved mid-game states: no serialized data changes; on replay they now take one canonical reroll order. Old replays were nondeterministic anyway, so no stored state becomes invalid — determinism starts at the next command.
- `player_locs()` (board.rs:226-234) also iterates the board `HashMap`, but its output feeds `loc_parser`, which sorts (command.rs:155-158); no RNG contact. Out of scope.
- `used_resources`/`casino_tile_count` are order-independent folds. Untouched.

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/board.rs` (imports line 1, `casino_at` lines 236-273, `casinos` lines 275-288, tests)

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/lords-of-vegas-1/src/board.rs`:

```rust
    #[test]
    fn resolve_boss_ties_is_deterministic_per_seed() {
        // d F2: identical board + identical seed must produce identical
        // reroll outcomes on every run. Pre-fix, the BFS queue is a fresh
        // HashSet per call whose iteration order varies per instance, so the
        // reroll order (and via re-tie cascades, the draw count) diverges.
        use self::Block::*;

        let locs: Vec<Loc> = vec![(A, 1).into(), (A, 2).into(), (A, 3).into(), (A, 6).into()];
        let dice_at = |b: &Board| -> Vec<Option<usize>> {
            locs.iter()
                .map(|l| match b.get(l) {
                    BoardTile::Built {
                        owner: Some(TileOwner { die, .. }),
                        ..
                    } => Some(die),
                    _ => None,
                })
                .collect()
        };

        let mut outcomes: Vec<Vec<Option<usize>>> = vec![];
        for _ in 0..100 {
            let mut b = Board::default();
            for (i, l) in locs.iter().enumerate() {
                // A1-A2-A3-A6 is one contiguous Albion casino (A3 and A6 are
                // vertical neighbours); four distinct players tied on die 4.
                b.set(
                    *l,
                    BoardTile::Built {
                        casino: Casino::Albion,
                        owner: Some(TileOwner { die: 4, player: i }),
                        height: 1,
                    },
                );
            }
            let mut rng = GameRng::seed_from_u64(7);
            assert!(
                b.resolve_boss_ties(&mut rng).is_some(),
                "a four-way boss tie must trigger resolution"
            );
            outcomes.push(dice_at(&b));
        }
        for o in &outcomes[1..] {
            assert_eq!(
                &outcomes[0], o,
                "same board and seed must produce identical rerolls (d F2)"
            );
        }
    }

    #[test]
    fn casinos_iterates_in_sorted_loc_order() {
        // Lock-in for d F2: casinos() must scan the board in Loc order so the
        // reroll pass is deterministic across processes (TILES is a HashMap
        // whose key order differs per process).
        let mut b = Board::default();
        b.set(
            (Block::F, 9).into(),
            BoardTile::Built {
                casino: Casino::Tivoli,
                owner: None,
                height: 1,
            },
        );
        b.set(
            (Block::A, 1).into(),
            BoardTile::Built {
                casino: Casino::Albion,
                owner: None,
                height: 1,
            },
        );
        let cs = b.casinos();
        assert_eq!(2, cs.len());
        assert_eq!(Casino::Albion, cs[0].casino, "A1 casino must come first");
        assert_eq!(Casino::Tivoli, cs[1].casino);
    }
```

- [ ] Run: `cargo test -p lords-of-vegas-1 resolve_boss_ties_is_deterministic` — expected FAIL: the `assert_eq!(&outcomes[0], o, ...)` fires (100 runs over a 4-tile tie make at least one divergent order overwhelmingly likely; if this red run ever passes, re-run — do NOT skip the red confirmation). `casinos_iterates_in_sorted_loc_order` may pass pre-fix within a single process (TILES order is process-stable); it is a cross-process lock-in, not the red test.
- [ ] Implement, in `rust/game/lords-of-vegas-1/src/board.rs`:
  1. Line 1: `use std::collections::{BTreeSet, HashMap, HashSet};`
  2. In `casino_at()` (board.rs:236-273), replace the queue setup and loop:

```rust
        let mut queue: BTreeSet<Loc> = BTreeSet::new();
        queue.insert(*loc);
        let mut visited: HashSet<Loc> = HashSet::new();
        let mut tiles: Vec<CasinoTile> = vec![];

        while let Some(next) = queue.pop_first() {
            visited.insert(next);
            match self.get(&next) {
                BoardTile::Built {
                    casino: c,
                    owner,
                    height: h,
                } if c == casino && h == height => {
                    tiles.push(CasinoTile { loc: next, owner });
                    for n in next.neighbours() {
                        if !visited.contains(&n) {
                            queue.insert(n);
                        }
                    }
                }
                _ => {}
            }
        }
```

  (the `queue.iter().next().expect(...)` and `queue.remove(&next)` lines are gone.)
  3. In `casinos()` (board.rs:275-288), replace the `for loc in TILES.keys()` loop header with:

```rust
        let mut locs: Vec<Loc> = TILES.keys().cloned().collect();
        locs.sort();
        for loc in &locs {
```

  (loop body unchanged: the `visited.contains(loc)` / `casino_at(loc)` calls already take `&Loc`.)
- [ ] Run: `cargo test -p lords-of-vegas-1` — both new tests PASS, all existing tests PASS (see edge-case note on `test_board_casino_at_works`' tile ordering above).
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/board.rs` ; message: `fix(lords-of-vegas-1): make boss-tie reroll order deterministic (d F2, WP-22)`

NOTE: this task shifts board.rs line numbers below ~line 247; later tasks cite `resolve_boss_ties` by symbol.

---

### Task 3: enforce the casino tile supply and saturate renderer subtractions (d F4, minor -> MAJOR per verification)

**Problem (restated):** `build()` imposes no supply limits and no colour restriction, so in 5-6 player games (10-12 owned lots) a 10th same-colour build exceeds `CASINO_TILES = 9` and `CASINO_TILES - casino_tile_count(...)` (render.rs:117) underflows `usize` — every subsequent render panics in debug / prints a huge number in release. The dice/token subtractions (render.rs:80, 85) are the same pattern but latent (max 2 dice/tokens used per player today, see re-derivation notes).

**Fix (re-derived, two-sided):**
1. `build()` rejects building a colour whose tile supply is exhausted — this is the real game rule (9 tiles per casino) and prevents new states from going bad.
2. The renderer uses `saturating_sub` on all three cited subtractions — REQUIRED even with (1), because already-persisted games may already hold >9 tiles of one colour and must keep rendering (serialized states are immutable history; a build-side guard cannot repair them).

**Edge cases:**
- 9th tile of a colour must still be allowed (`>=` guard, checked BEFORE deducting cash so a failed build has no side effects).
- Legacy state with 10+ tiles of a colour: `build()` of that colour now errors; render shows `0` tiles left. Both acceptable.
- `CASINO_CARDS - casino_card_count(...)` (render.rs:109) cannot underflow (max 9 lot cards per casino in the deck data) — left alone, matching the finding's cited lines.
- Error message uses `Casino`'s `Display` impl (casino.rs:46-60), e.g. "there are no Albion tiles remaining".

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/lib.rs` (`build()` — locate by symbol; the cash-check block is at live lines 281-285 pre-Task-1), `rust/game/lords-of-vegas-1/src/render.rs` (lines 80, 85, 117)
- Test: `rust/game/lords-of-vegas-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/lords-of-vegas-1/src/lib.rs`:

```rust
    #[test]
    fn build_rejects_when_casino_tile_supply_exhausted() {
        // d F4: there are only 9 tiles per casino colour; the 10th build of a
        // colour must be rejected instead of corrupting the board.
        use crate::board::Block;

        let mut board = Board::default();
        for loc in [
            (Block::A, 1),
            (Block::A, 2),
            (Block::A, 3),
            (Block::A, 4),
            (Block::A, 5),
            (Block::A, 6),
            (Block::B, 1),
            (Block::B, 2),
            (Block::B, 3),
        ] {
            board.set(
                loc.into(),
                BoardTile::Built {
                    casino: Casino::Albion,
                    owner: None,
                    height: 1,
                },
            );
        }
        board.set((Block::F, 5).into(), BoardTile::Owned { player: 0 });
        let mut g = Game {
            players: vec![
                Player {
                    cash: 100,
                    points: 0,
                },
                Player {
                    cash: 100,
                    points: 0,
                },
            ],
            current_player: 0,
            deck: vec![],
            played: vec![],
            board,
            finished: false,
            rng: GameRng::seed_from_u64(1),
        };
        match g.build(0, &(Block::F, 5).into(), Casino::Albion) {
            Err(GameError::InvalidInput { message }) => assert!(
                message.contains("no Albion tiles remaining"),
                "unexpected message: {}",
                message
            ),
            Ok(_) => panic!("10th Albion build must fail: the supply is 9 tiles"),
            Err(e) => panic!("expected InvalidInput, got: {}", e),
        }
        // A different colour still has tiles and must build fine.
        assert!(
            g.build(0, &(Block::F, 5).into(), Casino::Vega).is_ok(),
            "other colours must be unaffected by an exhausted Albion supply"
        );
    }

    #[test]
    fn render_saturates_when_supplies_exceeded() {
        // d F4: legacy saved states can already exceed the supplies; the
        // renderer must saturate at zero instead of underflowing usize.
        // 13 built Albion tiles owned by player 0 exceed both the 9-tile
        // casino supply and the 12-die player supply.
        use crate::board::Block;

        let mut board = Board::default();
        let mut locs: Vec<Loc> = vec![(Block::C, 1).into()];
        for lot in 1..=6 {
            locs.push((Block::A, lot).into());
            locs.push((Block::B, lot).into());
        }
        for loc in &locs {
            board.set(
                *loc,
                BoardTile::Built {
                    casino: Casino::Albion,
                    owner: Some(TileOwner { die: 1, player: 0 }),
                    height: 1,
                },
            );
        }
        let ps = PubState {
            players: vec![Player::default()],
            current_player: 0,
            remaining_deck: 0,
            played: vec![],
            board,
            finished: false,
        };
        // Pre-fix both of these panic in debug builds with
        // "attempt to subtract with overflow".
        let player_table = ps.render_player_table(0);
        let casino_table = ps.render_casino_table();
        assert!(matches!(player_table, N::Table(_)));
        assert!(matches!(casino_table, N::Table(_)));
    }
```

- [ ] Run: `cargo test -p lords-of-vegas-1 supply` — expected FAILURES: `build_rejects_when_casino_tile_supply_exhausted` fails on `Ok(_) => panic!("10th Albion build must fail...")` (build currently succeeds), and `render_saturates_when_supplies_exceeded` fails with the debug-build panic `attempt to subtract with overflow` (cargo test builds in debug).
- [ ] Implement:
  1. In `build()` in `rust/game/lords-of-vegas-1/src/lib.rs`, directly after the cash check block (`if self.players[p].cash < TILES[loc].build_cost { ... }`) and before the `self.players[p].cash -= ...` deduction, insert:

```rust
        if self.board.casino_tile_count(casino) >= CASINO_TILES {
            return Err(GameError::InvalidInput {
                message: format!("there are no {} tiles remaining", casino),
            });
        }
```

  2. In `rust/game/lords-of-vegas-1/src/render.rs`:
     - line 80: `vec![N::text(format!("{}", PLAYER_DICE.saturating_sub(used.dice)))],`
     - line 85: `vec![N::text(format!("{}", PLAYER_OWNER_TOKENS.saturating_sub(used.tokens)))],`
     - line 117 (inside the existing `format!`): `CASINO_TILES.saturating_sub(self.board.casino_tile_count(*casino))`
- [ ] Run: `cargo test -p lords-of-vegas-1` — both new tests PASS, all previous tests PASS.
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/lib.rs rust/game/lords-of-vegas-1/src/render.rs` ; message: `fix(lords-of-vegas-1): enforce casino tile supply and saturate render math (d F4, WP-22)`

---

### Task 4: log every boss-tie reroll (d F3, minor)

**Problem (restated):** `resolve_boss_ties` (board.rs, locate by symbol; pre-Task-2 lines 314-344) never pushes to its `logs` vec and discards the die returned by `reroll_at`. On a tie it returns `Some(vec![])`; `build()` extends the empty vec and sets `can_undo = false` — dice are silently rerolled, the player sees nothing, and undo is disabled for an invisible change.

**Fix (re-derived):** log one public entry per reroll inside the existing loop, using `reroll_at`'s returned die and the tile's owner (always `Some` for boss tiles — `boss_tiles()` only collects owner-Some tiles). Recursion already extends `logs` with cascade-pass entries, so cascaded rerolls are logged in order.

**Edge cases:**
- Multiple tied tiles of the SAME player (same die value twice in one casino) each get their own log line — correct, both dice are rerolled.
- Cascade: the recursive pass's logs are appended after the current pass's (existing `logs.extend(new_logs)` at the tail) — chronological order preserved.
- `reroll_at` returning `None` cannot happen for a boss tile; the `if let Some(die) = ...` guard simply skips logging in that impossible case rather than unwrapping.
- The `Some(vec![])`-vs-`None` contract of the function is unchanged; `build()`'s `can_undo = false` behavior is unchanged (now justified by visible logs).

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/board.rs` (`resolve_boss_ties`, tests)

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/lords-of-vegas-1/src/board.rs`:

```rust
    #[test]
    fn resolve_boss_ties_logs_each_reroll() {
        // d F3: rerolls are player-facing (RULES.md) and disable undo, so
        // every rerolled die must appear in the logs.
        let mut b = Board::default();
        for (lot, player) in [(1, 0), (2, 1)] {
            b.set(
                (Block::A, lot).into(),
                BoardTile::Built {
                    casino: Casino::Albion,
                    owner: Some(TileOwner { die: 5, player }),
                    height: 1,
                },
            );
        }
        let mut rng = GameRng::seed_from_u64(1);
        let logs = b
            .resolve_boss_ties(&mut rng)
            .expect("a two-way boss tie must trigger resolution");
        assert!(
            logs.len() >= 2,
            "each rerolled die must be logged, got {} logs",
            logs.len()
        );
        let text: String = logs
            .iter()
            .map(|l| brdgme_markup::plain(&brdgme_markup::transform(&l.content, &[])))
            .collect::<Vec<String>>()
            .join("\n");
        assert!(text.contains("rerolled"), "got: {}", text);
        assert!(
            text.contains("A1") && text.contains("A2"),
            "both rerolled locations must be named, got: {}",
            text
        );
    }
```

- [ ] Run: `cargo test -p lords-of-vegas-1 resolve_boss_ties_logs` — expected FAIL on `logs.len() >= 2` (pre-fix the vec is always empty).
- [ ] Implement: in `resolve_boss_ties`, replace the reroll loop body

```rust
            for bt in &boss_tiles {
                self.reroll_at(&bt.loc, rng);
            }
```

  with

```rust
            for bt in &boss_tiles {
                if let Some(die) = self.reroll_at(&bt.loc, rng)
                    && let Some(TileOwner { player, .. }) = bt.owner
                {
                    logs.push(Log::public(vec![
                        N::text("Boss tie at "),
                        bc.casino.render(),
                        N::text(": "),
                        N::Player(player),
                        N::text("'s die at "),
                        bt.loc.render(),
                        N::text(" rerolled to "),
                        N::Bold(vec![N::text(die.to_string())]),
                    ]));
                }
            }
```

  (`bc` is the loop variable of the enclosing `for bc in self.casinos()`; `Log` and `N` are already imported at board.rs:8-10; `Casino::render`/`Loc::render` are existing pub methods.)
- [ ] Run: `cargo test -p lords-of-vegas-1` — new test PASSES, all previous tests PASS (including Task 2's determinism test — logging does not touch the RNG stream).
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/board.rs` ; message: `fix(lords-of-vegas-1): log boss-tie rerolls (d F3, WP-22)`

---

### Task 5: replace lazy_static with std::sync::LazyLock (d F6, minor)

**Problem (restated):** `lazy_static` (tile.rs:3, 23-25; Cargo.toml:15) is in maintenance mode and `TILES` is its only use in the crate; the toolchain (1.97.0) has `std::sync::LazyLock`.

**Fix (re-derived):** `pub static TILES: LazyLock<TileMap> = LazyLock::new(tiles);` — verification and the toolchain both point at std `LazyLock`, not the finding's `OnceLock`+getter or `once_cell` alternatives (no new dependency, no call-site changes). `LazyLock<TileMap>` derefs to `HashMap`, so every existing usage (`TILES[&loc]`, `TILES.contains_key(loc)`, `TILES.keys()`) compiles unchanged via auto-deref.

**Edge cases:** none — static data, no serialization, no behavior change. Existing tests (`json_works`, the board tests, `game_contract`) all force `TILES` initialization and cover the swap.

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/tile.rs` (lines 1-25), `rust/game/lords-of-vegas-1/Cargo.toml` (line 15)

**Steps:**

- [ ] In `tile.rs`: replace `use lazy_static::lazy_static;` (line 3) with `use std::sync::LazyLock;`, and replace

```rust
lazy_static! {
    pub static ref TILES: TileMap = tiles();
}
```

  with

```rust
pub static TILES: LazyLock<TileMap> = LazyLock::new(tiles);
```

- [ ] In `Cargo.toml`: delete the line `lazy_static = "1.5.0"` (line 15).
- [ ] Run: `cargo test -p lords-of-vegas-1` — full suite PASSES (no new test: no observable behavior; the whole suite exercises `TILES`). `Cargo.lock` will drop the crate's lazy_static edge — include it in the commit if it changed.
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/tile.rs rust/game/lords-of-vegas-1/Cargo.toml rust/Cargo.lock` ; message: `refactor(lords-of-vegas-1): replace lazy_static with std LazyLock (d F6, WP-22)`

---

### Task 6: move serde_json to dev-dependencies (d F8, nit)

**Problem (restated):** `serde_json` is declared under `[dependencies]` (Cargo.toml:18) but its only use in the crate is the `json_works` unit test (lib.rs:350, inside `#[cfg(test)]`). Verified during spec-writing: none of the four bins nor any non-test module uses it.

**Fix:** move the line `serde_json = "1.0.150"` from `[dependencies]` to the existing `[dev-dependencies]` section (Cargo.toml:21-23).

**Files:**
- Modify: `rust/game/lords-of-vegas-1/Cargo.toml`

**Steps:**

- [ ] Move `serde_json = "1.0.150"` into `[dev-dependencies]`.
- [ ] Run: `cargo build -p lords-of-vegas-1` — compiles (proves no non-test use), then `cargo test -p lords-of-vegas-1` — full suite PASSES (`json_works` still compiles as a dev-dependency user).
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/Cargo.toml` ; message: `refactor(lords-of-vegas-1): serde_json is test-only, move to dev-dependencies (d F8, WP-22)`

---

### Task 7: invariant breadcrumb on the starting-cash unreachable!() (d F7, nit)

**Problem (restated):** `Card::GameEnd => unreachable!()` (lib.rs:118, inside the starting-cash fold in `start()` — locate by symbol after Task 1) is provably unreachable, but the proof lives in card.rs (the insert-position comment at card.rs:27-29); the arm itself explains nothing.

**Fix:** comment plus message at the arm — no logic change (per the finding's recommendation; converting this to an error is NOT wanted: the invariant is real and local to `shuffled_deck`, and `start()` has no sensible recovery). Verification-corrected numbers: minimum insert position is 38 (2 players), maximum starting-hand drain is 12 cards (6 players x 2).

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/lib.rs` (the `Card::GameEnd` arm in `start()`)

**Steps:**

- [ ] Replace the arm with:

```rust
                    // shuffled_deck inserts GameEnd in the last quarter of the
                    // deck (position >= 38 even for 2 players; see card.rs),
                    // while starting hands drain at most 12 cards from the
                    // front, so GameEnd can never be dealt here.
                    Card::GameEnd => unreachable!("GameEnd cannot be in a starting hand"),
```

- [ ] Run: `cargo test -p lords-of-vegas-1` — full suite PASSES.
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/lib.rs` ; message: `docs(lords-of-vegas-1): explain the GameEnd unreachable invariant (d F7, WP-22)`

---

### Task 8: code nits — dead FromIterator import, BLOCK_WIDTH in renderer (d F9 + d F10, nits)

**Problem (restated):**
- d F9 — `use std::iter::FromIterator;` (board.rs:3) is redundant on edition 2024 (prelude); the sole user is `HashSet::from_iter` in `resolve_boss_ties`, which the prelude covers.
- d F10 — `render_block` (render.rs:154-155) computes `(lot - 1) % 3` / `(lot - 1) / 3` while board.rs:16 defines `BLOCK_WIDTH: usize = 3` for exactly this; `Loc::neighbours` already uses the constant, so a width change would silently desynchronize logic and rendering. `BLOCK_WIDTH` is module-private — bump visibility and import it (the finding's duplicate-constant alternative is rejected: it would recreate the same divergence risk one file over).

**Fix:** two mechanical edits, behavior-identical, covered by the existing suite (`loc_neighbours_works` + the render path in `game_contract`). No new tests — nothing observable to assert.

**Files:**
- Modify: `rust/game/lords-of-vegas-1/src/board.rs` (line 3, line 16), `rust/game/lords-of-vegas-1/src/render.rs` (line 13 import, lines 154-155)

**Steps:**

- [ ] board.rs: delete `use std::iter::FromIterator;` (line 3) and change line 16 to `pub const BLOCK_WIDTH: usize = 3;`.
- [ ] render.rs: extend the import at line 13 to `use crate::board::{BLOCK_WIDTH, BLOCKS, Block, Board, BoardTile, Loc, TileOwner};` and in `render_block` change:

```rust
            let x = (lot - 1) % BLOCK_WIDTH;
            let y = (lot - 1) / BLOCK_WIDTH;
```

- [ ] Run: `cargo test -p lords-of-vegas-1` — full suite PASSES.
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/src/board.rs rust/game/lords-of-vegas-1/src/render.rs` ; message: `refactor(lords-of-vegas-1): drop dead import, use BLOCK_WIDTH in renderer (d F9 d F10, WP-22)`

---

### Task 9: RULES.md colour wording (d F11, nit, doc-only) + final gate

**Problem (restated):** RULES.md:48-54 describes Sphinx as "Tan/olive" and Pioneer as "Brick red", but `Casino::color()` (casino.rs:27-35) renders Sphinx as Orange and Pioneer as Brown (a `brdgme_color` palette approximation). Doc and UI disagree on colour names. Per the finding: adjust the wording; no code change.

**Fix:** update the two table cells to the rendered colours and note the approximation once.

**Files:**
- Modify: `rust/game/lords-of-vegas-1/RULES.md` (lines 48-54)

**Steps:**

- [ ] In the colour table, change the Sphinx row's colour cell from `Tan/olive` to `Orange` and the Pioneer row's from `Brick red` to `Brown`. Directly under the table add the line:

  `(Colours approximate the physical game's tan and brick-red tiles.)`

  Do NOT touch any other RULES.md content — in particular nothing about player counts (WP-26 owns d F12).
- [ ] Run: `cargo test -p lords-of-vegas-1` — full suite PASSES (`rules()` is `include_str!`; a rebuild re-embed is the only effect).
- [ ] `cargo clippy -p lords-of-vegas-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/lords-of-vegas-1/RULES.md` ; message: `docs(lords-of-vegas-1): match casino colour names to the renderer (d F11, WP-22)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| d F1 unimplemented!() dispatch arms | major | Replace each arm with `Err(GameError::InvalidInput { message: "not yet implemented" })` | CONFIRMED (implementation adjusted) | Rec is correct; fix additionally extracts a private `dispatch` helper because the arms are untestable through `command()` (parser cannot produce them), uses per-verb messages, and `..` patterns to drop the `#[allow(unused_variables)]` and the `player` shadow in the Gamble arm (Task 1). |
| d F2 HashMap/HashSet order feeds boss-tie RNG | major | Collect candidate locs into a sorted `Vec` in both sites, or switch `TILES`/`Board` to `BTreeMap` | ADJUSTED | Nondeterminism re-derived and worse than stated in one respect: the BFS queue is a fresh `HashSet` per call, so even same-process calls diverge (verification also noted the draw COUNT varies via the re-tie recursion). Fix uses a transient `BTreeSet::pop_first` BFS + sorted `Vec` in `casinos()`. The `BTreeMap` alternative is rejected: touching the serialized `Board` type is needless risk (WP-13 shape-caution precedent) when transient locals suffice; `TILES` is static and only needs sorted iteration once (Task 2). |
| d F3 resolve_boss_ties never logs | minor | Push a public log per reroll using the value from `reroll_at` | CONFIRMED | Exactly as recommended; log names casino, player, loc, and the new die; cascade passes append in order (Task 4). |
| d F4 usize underflow in renderer supply math | minor -> MAJOR (verification upgrade) | Enforce supply limits in `build()` (dice/tokens/casino tiles) and/or `saturating_sub` in the renderer | ADJUSTED | The CASINO_TILES half is reachable in ordinary 5-6p play (re-derived: 10-12 owned lots, no colour limit) — enforced in `build()`. The dice/token supplies are UNREACHABLE today (players own at most 2 lots, so at most 2 dice/tokens used) — not enforced in `build()` (dead checks; revisit when card draws land). `saturating_sub` applied to all three cited render sites, which is mandatory regardless of the build guard because already-persisted states may already exceed the supply (Task 3). |
| d F5 parse_str unvalidated / lot-0 underflow | minor | Validate lot range in `parse_str` | OUT OF SCOPE | Owned by WP-09 (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36). Not touched here. |
| d F6 lazy_static for TILES | minor | `OnceLock` + getter, or `once_cell::sync::Lazy`, matching workspace direction | ADJUSTED | `std::sync::LazyLock` (verification's own suggestion) beats both offered options: no getter boilerplate, no new dependency, zero call-site changes via Deref (Task 5). Removes the crate's lazy_static dependency entirely. |
| d F7 unreachable!() in starting-cash fold | nit | Comment stating the invariant, or an `unreachable!` message | CONFIRMED (details verification-adjusted) | Both comment and message added; numbers corrected per verification (min insert position 38, not 39; card.rs already carries the source-side comment) (Task 7). |
| d F8 serde_json runtime dep, test-only use | nit | Move to `[dev-dependencies]` | CONFIRMED | Re-verified: sole use is the `json_works` test; no bin uses it (Task 6). |
| d F9 redundant FromIterator import | nit | Delete the import | CONFIRMED | Edition 2024 prelude covers `HashSet::from_iter` (Task 8). |
| d F10 hardcoded 3 vs BLOCK_WIDTH | nit | Use `BLOCK_WIDTH` (re-export or duplicate the constant) | ADJUSTED | Constant made `pub` and imported; the duplicate-constant alternative is rejected because it would reintroduce the exact divergence risk the finding describes (Task 8). |
| d F11 casino colours vs RULES.md | nit | Adjust RULES.md wording to rendered colours; no code change | CONFIRMED | Doc-only edit plus an approximation note (Task 9). |
| d F12 player counts 2-6 vs official 2-4 | nit | Confirm intent; document if deliberate | OUT OF SCOPE | Owned by WP-26 (batch-d rules adjudication, blocked on decisions). Not touched here; `player_counts_works` continues to lock 2-6. |

## Cross-package coordination points

- **WP-09 (BLOCKED-ON-DECISION D-36)** owns d F5: `Loc::parse_str` range validation and any deserialized-state hardening for this crate. This spec deliberately leaves `parse_str`/`neighbours` untouched; if WP-09 later validates lots in `parse_str`, nothing here conflicts (Task 2's `BTreeSet` BFS only iterates locs already on the board or in `TILES`).
- **WP-26 (BLOCKED-ON-DECISION)** owns d F12 (2-6 vs 2-4 players) and may edit this crate's `player_counts()`/RULES.md. Task 9's RULES.md edit is confined to the colour table (lines 48-54) plus one added line, far from any player-count text — no textual conflict expected.
- **WP-65 (workspace hygiene)** lists a lazy_static -> LazyLock sweep and touches `game/lords-of-vegas-1/Cargo.toml` for stale-template cleanup. Task 5 here removes this crate's lazy_static use and dependency first (WP-22 is READY, WP-65 is sequenced "best after WP-64"); WP-65's sweep should find nothing left to do in this crate but may still rewrite adjacent Cargo.toml lines — trivial rebase either way. Same note for Task 6's serde_json move.
- **WP-08 (epilogue dedup sweep)** does not include lords-of-vegas-1 (the crate has no finish epilogue — games never finish); Task 1's `command()` restructure is unrelated to that sweep's shape.
- **Unfindinged observations flagged, not fixed:** (1) `thiserror` (Cargo.toml:14) appears unused anywhere in the crate — same class as d F8 but not a filed finding; left for workspace manifest work. (2) `build()` performs no adjacency/colour legality checks beyond ownership (any owned lot, any colour) — consistent with the documented partial implementation, noted for whoever implements the full rules. (3) `status()` can never reach `Finished` (no endgame trigger) — documented behavior (RULES.md:98-105), not a defect.
