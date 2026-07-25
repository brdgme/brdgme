# WP-14: alhambra-1 core fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Fix the three mechanical defects in `rust/game/alhambra-1` — the critical money-duplication exploit in `take()` (b F16), the place-index divergence that inserts phantom Empty tiles into player grids (b F17), and the premature wall-walk termination in `grid_longest_ext_wall` (b F18) — and land the missing-test inventory from b F21 with them. Also apply the six verified nits in the same crate: expect()-named invariants (b F23), gap-check range symmetry (b F24), user-facing `{:?}` formatting (b F25), `tile_counts` duplication (b F26), column-header wrap (b F27), and Vec-as-queue / HashMap-as-set flood walks (b F28).

**Architecture — how alhambra-1 works (read this before editing):**

- One crate, `rust/game/alhambra-1` (package name `alhambra-1`, lib name `alhambra_1`): `src/lib.rs` (game state machine + `Gamer` impl + inline tests), `src/card.rs` (cards, tiles, `Grid` = `HashMap<Vect, Tile>`, grid validity + wall scoring), `src/command.rs` (command parsers), `src/render.rs` (markup rendering), `tests/contract.rs` (standard contract harness).
- Economy: a market of up to 4 money cards (`Game::cards`) and 4 building tiles (`Game::tiles`, one slot per `Currency`). On your Action turn you either `take` money cards from the market (multiple cards allowed only if their total value is ≤ 5) or `spend` same-currency cards from your hand to buy the tile in that currency's slot. Exact payment (`total == tile.cost`) grants an extra action (turn does not advance); overpayment advances to the Place phase. Bought tiles go into `PlayerBoard::place`; unplaced tiles move to `reserve` at end of turn.
- Placement: `place N coord` places the Nth tile (1-based in the UI, `Int::positive()` maps to 0-based `n`) from `reserve` (Action phase) or `place` (Place/FinalPlace phases) onto the player's grid. Every grid mutation is validated by `grid_is_valid` (fountain present, wall matching, walk-connectivity, no enclosed gaps) on a cloned test grid before committing — so every human grid reachable in play is valid.
- Scoring: 2 Scoring cards are injected into the money deck; drawing one triggers `score_round`, which awards tile-type majorities (`score_type`) and 1 point per segment of each player's longest external wall (`grid_longest_ext_wall`). `render_player_summary` also recomputes `grid_longest_ext_wall` on every render (the "Wall" column).
- 2-player games add Dirk (`DIRK = 2`), an AI board that draws tiles straight into its grid (`dirk_draw_tiles`) with NO validity check — Dirk's grid can be invalid, but `grid_longest_ext_wall` and wall scoring are never applied to it (`score_round` loops `0..human_players`; render shows "N/A" for Dirk's wall).
- Serialization: the whole `Game` is serde-serialized between requests. `Grid` is serialized via `grid_serde` as a `Vec<(Vect, Tile)>` (order = HashMap iteration order, content-stable). `PlayerBoard`/`Tile` shapes are persisted — **no fix in this package may change any serialized type**. All fixes below operate on local variables or pure logic only.

**Tech Stack:** Rust 1.97.0 workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `alhambra-1` (name confirmed from `rust/game/alhambra-1/Cargo.toml`). Tests: inline `#[cfg(test)] mod tests` in `src/lib.rs` (existing, line 1028) plus the untouched `tests/contract.rs`.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p alhambra-1`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- The two existing wall tests (`test_grid_longest_ext_wall`, expecting 5 and 12) and all other existing tests MUST keep passing unmodified. If a fix changes their result, the fix is wrong — stop and re-check.
- No serialized-shape changes: `Game`, `PlayerBoard`, `Tile`, `Grid`, `grid_serde` stay exactly as they are. Every collection changed in this package is a function-local.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).

**Non-Goals:**

- b F19 (Dirk excluded from final placings) and b F20 (reduced 2-player money deck) — rules-adjudication items owned by WP-16, blocked on decisions D-27/D-28. Do NOT touch `status()`/placings loops or `build_deck`.
- b F22 (is_finished() epilogue copy-pasted into six command arms) — owned by WP-08 (cross-crate epilogue-dup sweep). Do NOT refactor the six `command()` arms.
- `Tile::walls: HashMap<Dir, bool>` is a serialized field — out of scope even though it is also a "HashMap-as-set" (F28 covers only the function-local flood-walk collections).
- The serialized ORDER of `grid_serde` output (Vec built from HashMap iteration) is nondeterministic but content-equal after deserialize; not player-visible, not in scope.
- `rot_all`'s `panic!` (card.rs:209) — already carries a message naming its invariant; F23 verification accepted it. Leave it.

**Snapshot drift:** None. `diff -rq /home/beefsack/Development/brdgme-review-snapshot/rust/game/alhambra-1 /home/beefsack/Development/brdgme/rust/game/alhambra-1` is empty (verified 2026-07-25 against snapshot commit f8763a5). All line numbers below are live-file line numbers and match the findings' citations.

**Re-derivation note on F18 (read before Task 3):** The finding and its verification are correct that the `break` at card.rs:516 is unconditional and that, on the finding's example grid, the result depends on HashMap iteration order. However, re-derivation against `grid_is_valid` shows that **every grid reachable in live play is immune**: the three continuation candidates (outward-corner / straight / turn-back) are mutually exclusive on any grid satisfying wall-matching plus the no-gap rule, so the first non-Empty candidate failing implies no continuation exists and the break loses nothing. The truncation and the nondeterminism are only expressible on invalid grids (the finding's example violates wall matching; the diagonal-island pinch it needs is forbidden by the gap rule). The fix still lands: `grid_longest_ext_wall` is a pure helper that must not silently depend on validity invariants enforced elsewhere (Dirk-style unvalidated grids, future callers, unit tests all bypass them), and the fix is provably behavior-preserving on valid grids. Details and the proof sketch are in Task 3.

---

### Task 1: take() clone-and-verify — stop the duplicate-card mint (b F16, CRITICAL)

**Problem (restated):** `Game::take` (`rust/game/alhambra-1/src/lib.rs:550-584`). The availability pre-check at lib.rs:557-561 tests each requested card against the market independently:

```rust
for c in cards {
    if !self.cards.contains(c) {
        return Err(GameError::invalid_input(format!("{} is not available", c)));
    }
}
```

— no multiplicity accounting. The removal loop at lib.rs:570-575 then pushes the card into the player's hand **even when it was not found in the market**:

```rust
for c in cards {
    if let Some(pos) = self.cards.iter().position(|mc| mc == c) {
        self.cards.remove(pos);
    }
    self.boards[player].cards.push(*c);
}
```

**Exploit:** with a single `B1` in the market, the command `take b1 b1 b1 b1 b1` parses (the take arm uses `CardParser`, which accepts any letter+digits token — command.rs:149-160), passes the pre-check (`contains` is true for each copy), passes the multi-card value cap (total 5 ≤ 5), removes ONE `B1` from the market and pushes FIVE `B1`s into the hand — four money cards minted from nothing, every turn. Money is victory-relevant (buying power and the FinalPlace most-money distribution), so this is game-state corruption reachable from any player's crafted input. Contrast `spend()` (lib.rs:616-627), which clones the hand and removes-with-error — correct multiplicity handling; the finding's recommendation to mirror that pattern is confirmed correct against the live source.

**Fix (re-derived):** clone the market, remove each requested card from the clone (erroring on a miss — which now also catches over-requested duplicates), keep the existing value-cap check, and only then commit the clone and extend the hand. No mutation on any error path (same as today — all current errors fire before mutation).

**Edge cases:** duplicate request with the duplicate genuinely in the market (deck has 2-3 copies of each card, so two `B1`s in the market is legal) → succeeds, both removed, both gained; duplicate request exceeding market multiplicity → `"B1 is not available"` error, market and hand untouched; single card of any value → allowed (cap only applies to `cards.len() > 1`, unchanged); card not in market at all → same error as today; empty card list → existing guard unchanged; error-message precedence preserved (availability checked before the value cap, as today).

**Files:**
- Modify: `rust/game/alhambra-1/src/lib.rs` (`Game::take`, lines 557-575)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/alhambra-1/src/lib.rs`:

```rust
    #[test]
    fn take_cannot_mint_duplicate_cards() {
        // b F16: requesting the same market card twice must fail, not mint a
        // free copy into the hand.
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.cards = vec![Card::new(Currency::Blue, 1)];
        g.boards[0].cards = vec![];
        let result = g.command(0, "take b1 b1", &[]);
        assert!(
            result.is_err(),
            "taking one market card twice must be rejected"
        );
        assert_eq!(
            vec![Card::new(Currency::Blue, 1)],
            g.cards,
            "market must be unchanged after a failed take"
        );
        assert!(
            g.boards[0].cards.is_empty(),
            "hand must be unchanged after a failed take"
        );
    }

    #[test]
    fn take_allows_real_duplicates_in_market() {
        // The deck holds 2-3 copies of each card, so two B1s in the market is
        // legal and both may be taken in one command.
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.cards = vec![
            Card::new(Currency::Blue, 1),
            Card::new(Currency::Blue, 1),
            Card::new(Currency::Red, 9),
        ];
        g.boards[0].cards = vec![];
        g.command(0, "take b1 b1", &[]).unwrap();
        assert_eq!(
            vec![Card::new(Currency::Blue, 1), Card::new(Currency::Blue, 1)],
            g.boards[0].cards
        );
        assert!(
            !g.cards.contains(&Card::new(Currency::Blue, 1)),
            "both market B1s must have been removed"
        );
    }
```

  (Note: after a successful take, `next_phase` runs and the market refills from the deck — assert on the B1s specifically, never on `g.cards.len()`.)

- [ ] Run: `cargo test -p alhambra-1 take_cannot_mint` and `cargo test -p alhambra-1 take_allows_real`. Expected: `take_cannot_mint_duplicate_cards` FAILS on the first assert (`result.is_err()`) — the buggy take succeeds and mints the duplicate. `take_allows_real_duplicates_in_market` passes already (duplicates in market work today); it exists to lock in that the fix does not over-restrict.
- [ ] Implement. In `Game::take`, replace lines 557-575 (the pre-check loop, the value-cap block, and the removal loop) with:

```rust
        // Clone-and-verify (mirrors spend()): remove each requested card from
        // a market copy so duplicate requests are counted against market
        // multiplicity; commit only after all checks pass. A bare contains()
        // pre-check let `take b1 b1` mint cards that were never in the market.
        let mut market = self.cards.clone();
        for c in cards {
            match market.iter().position(|mc| mc == c) {
                Some(pos) => {
                    market.remove(pos);
                }
                None => {
                    return Err(GameError::invalid_input(format!("{} is not available", c)));
                }
            }
        }
        if cards.len() > 1 {
            let total: i32 = cards.iter().map(|c| c.value).sum();
            if total > 5 {
                return Err(GameError::invalid_input(
                    "can't take more than one card with a total value over 5",
                ));
            }
        }
        self.cards = market;
        self.boards[player].cards.extend(cards.iter().copied());
```

  Everything after (the log construction from line 576 and the `next_phase` call) is unchanged.

- [ ] Run: `cargo test -p alhambra-1` — both new tests PASS, full suite PASS (including `game_starts_and_take_works` and the contract test).
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/lib.rs` ; message: `fix(alhambra-1): count take() requests against market multiplicity (b F16, WP-14)`

---

### Task 2: place/swap index resolution over non-empty tiles (b F17, MAJOR)

**Problem (restated):** After a tile is placed during the Place/FinalPlace phases, `place()` leaves a `Tile::empty()` sentinel in `boards[player].place` (`rust/game/alhambra-1/src/lib.rs:694`) instead of removing the slot. But the render numbers only NON-empty tiles: `render_tile_set` (`rust/game/alhambra-1/src/render.rs:353-379`) filters via `not_empty` and labels the survivors `1..` (line 367), while `place()` indexes the RAW vec (lib.rs:664-669). Divergence walkthrough: buy two tiles so `place = [Pav, Ser]`; `place 1 <coord>` places Pav and leaves `[Empty, Ser]`; the render now shows "1: Ser", but `place 1 <coord2>` resolves raw index 0 = the Empty sentinel. The Empty tile passes `grid_is_valid` (Empty cells are skipped by the walks) and is inserted into the grid: the coordinate is permanently blocked ("coordinate is not empty" forever), "placed Empty tile" is logged, `grid_bounds` expand (coordinate labels shift), and a later `remove`/`swap` can push the phantom Empty into `reserve`, corrupting reserve indices the same way. The `FinalPlace` phase shares the same `_ =>` match arm. Verified CONFIRMED; the recommendation (resolve `n` against the non-empty subsequence) is correct against the live source and is what this task implements.

**Fix (re-derived):** resolve the 0-based command index against the subsequence of non-Empty tiles, in both `place()` and — hardening for the same class — the reserve lookups in `place()`'s Action arm and `swap()`. Rationale for including reserve/swap: `render_reserve` uses the same filtered numbering (render.rs:385-387), and although a clean game never puts an Empty in `reserve`, already-persisted games corrupted by this bug can have one (via `remove`/`swap` of a phantom Empty); filtered resolution makes those legacy states behave correctly instead of perpetuating the corruption. The sentinel write (`place[idx] = Tile::empty()`) is KEPT — removing it instead would also work for fresh games but would leave legacy sentinels mis-resolving raw indices; filtered resolution handles both, with the smaller diff.

**Edge cases:** `n` past the non-empty count (e.g. `place 3` with two live tiles, or any `n` into a sentinel-padded vec) → "invalid place tile index" / "invalid reserve tile index" error exactly as before; legacy state `[Empty, Ser]` → index 0 resolves to Ser (correct); all-sentinel vec → `can_place` already gates on `not_empty(...)`, and the resolver returns None → error (no panic); Action phase reserve behavior for clean states → byte-identical (no Empties → filter is the identity); parser is untouched (`Int::positive()` 1-based → 0-based mapping unchanged).

**Files:**
- Modify: `rust/game/alhambra-1/src/lib.rs` (`Game::place` lines 657-670 and 687-700, `Game::swap` lines 708-714; new private helper)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/alhambra-1/src/lib.rs`:

```rust
    #[test]
    fn place_index_matches_rendered_index_after_placement() {
        // b F17: after placing the first of two bought tiles, the render
        // shows the survivor as "1", so index 0 must resolve to it — not to
        // the Empty sentinel left in the raw vec.
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Place;
        g.boards[0].place = vec![
            Tile::new(TileType::Pavillion, 5, &[]),
            Tile::new(TileType::Seraglio, 3, &[]),
        ];
        // Fountain is at (0,0); orthogonally adjacent coords are valid.
        g.place(0, 0, Vect { x: 0, y: -1 }).unwrap();
        g.place(0, 0, Vect { x: 0, y: 1 }).unwrap();
        assert_eq!(
            TileType::Seraglio,
            g.boards[0].grid[&Vect { x: 0, y: 1 }].tile_type,
            "second `place 1` must place the remaining Seraglio tile"
        );
        assert!(
            g.boards[0]
                .grid
                .values()
                .all(|t| t.tile_type != TileType::Empty),
            "no Empty sentinel may ever be inserted into the grid"
        );
    }

    #[test]
    fn place_index_out_of_range_after_placement_errors() {
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Place;
        g.boards[0].place = vec![
            Tile::new(TileType::Pavillion, 5, &[]),
            Tile::new(TileType::Seraglio, 3, &[]),
        ];
        g.place(0, 0, Vect { x: 0, y: -1 }).unwrap();
        // Only one live tile remains; index 1 must be rejected even though
        // the raw vec still has two slots.
        assert!(g.place(0, 1, Vect { x: 0, y: 1 }).is_err());
    }

    #[test]
    fn swap_index_skips_empty_sentinels_in_reserve() {
        // Hardening for legacy states corrupted by the pre-fix bug: an Empty
        // in reserve must not be addressable; index 0 is the first live tile.
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.boards[0].reserve = vec![Tile::empty(), Tile::new(TileType::Pavillion, 5, &[])];
        g.boards[0]
            .grid
            .insert(Vect { x: 0, y: 1 }, Tile::new(TileType::Garden, 4, &[]));
        g.swap(0, 0, Vect { x: 0, y: 1 }).unwrap();
        assert_eq!(
            TileType::Pavillion,
            g.boards[0].grid[&Vect { x: 0, y: 1 }].tile_type,
            "swap 1 must use the first NON-empty reserve tile"
        );
        assert_eq!(TileType::Garden, g.boards[0].reserve[1].tile_type);
    }
```

- [ ] Run: `cargo test -p alhambra-1 place_index` and `cargo test -p alhambra-1 swap_index_skips`. Expected failures: `place_index_matches_rendered_index_after_placement` FAILS — the second `place` inserts the Empty sentinel, so the grid at `(0,1)` is `Empty`, failing the first assert (and the no-Empty-in-grid assert). `place_index_out_of_range_after_placement_errors` FAILS — raw index 1 (Ser) is in range so the call succeeds. `swap_index_skips_empty_sentinels_in_reserve` FAILS — raw index 0 swaps the Empty into the grid.
- [ ] Implement. In `rust/game/alhambra-1/src/lib.rs`:

  1. Add a private helper (place it just above `impl Game`, next to the other free items):

```rust
/// Raw index of the `n`th non-Empty tile, matching the 1-based numbering the
/// renderer shows (render_tile_set filters Empty sentinels before labeling).
fn nth_non_empty(tiles: &[Tile], n: usize) -> Option<usize> {
    tiles
        .iter()
        .enumerate()
        .filter(|(_, t)| t.tile_type != TileType::Empty)
        .nth(n)
        .map(|(i, _)| i)
}
```

  2. In `Game::place`, replace the tile-selection match (lines 657-670):

```rust
        let (tile, raw_idx) = match self.phase {
            Phase::Action => {
                let raw_idx = nth_non_empty(&self.boards[player].reserve, n)
                    .ok_or_else(|| GameError::invalid_input("invalid reserve tile index"))?;
                (self.boards[player].reserve[raw_idx].clone(), raw_idx)
            }
            _ => {
                let raw_idx = nth_non_empty(&self.boards[player].place, n)
                    .ok_or_else(|| GameError::invalid_input("invalid place tile index"))?;
                (self.boards[player].place[raw_idx].clone(), raw_idx)
            }
        };
```

  3. In the post-insert match (lines 687-700), use `raw_idx` instead of `n` in both arms: `self.boards[player].reserve.remove(raw_idx);` and `self.boards[player].place[raw_idx] = Tile::empty();`. Nothing else in the arms changes.

  4. In `Game::swap`, replace the bounds check and raw read (lines 708-714):

```rust
        let raw_idx = nth_non_empty(&self.boards[player].reserve, n)
            .ok_or_else(|| GameError::invalid_input("invalid reserve tile index"))?;
        ...
        let reserve_tile = self.boards[player].reserve[raw_idx].clone();
```

  and write back with `self.boards[player].reserve[raw_idx] = grid_tile.clone();` (line 725). The `grid.contains_key` check between them is unchanged.

- [ ] Run: `cargo test -p alhambra-1` — the three new tests PASS, full suite PASS.
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/lib.rs` ; message: `fix(alhambra-1): resolve place/swap indices over non-empty tiles (b F17, WP-14)`

---

### Task 3: complete the wall-walk candidate scan (b F18, MAJOR — impact re-derived)

**Problem (restated):** `grid_longest_ext_wall` (`rust/game/alhambra-1/src/card.rs:477-531`) walks each external wall run in both directions. At each step it generates up to three continuation candidates in priority order — outward corner (`rot_num 0`), straight (`rot_num 1`), turn-back on the same tile (`rot_num 2`) — but the `break` at card.rs:516 sits OUTSIDE the success `if`: the loop `continue`s past candidates whose tile is Empty, yet aborts at the first candidate whose tile exists but whose wall test fails (no wall / already visited / internal). Later candidates are never tried, so a run can be truncated, and because the outer loop iterates `g.iter()` (a `HashMap`), which segment starts each walk — and therefore how much of a run is counted before truncation — depends on hash iteration order. Wall points are awarded every scoring round and the "Wall" render column recalculates per render, so on an affected grid scores would be both undercounted and nondeterministic across deserialize/replay boundaries.

**Impact re-derivation (differs from the finding — read carefully):** on grids reachable in play the bug CANNOT fire. Every human grid passes `grid_is_valid` before commit, which enforces (a) wall matching between adjacent tiles and (b) no enclosed empty pockets. Under (a): if the outward-corner candidate's tile `d` exists, the straight candidate is necessarily internal (the straight wall would sit under `d`, forcing a matched pair); and if the straight candidate's tile exists, the turn-back candidate is internal for the same reason. Under (a)+(b): `d` existing with the turn-back as the true continuation would require both off-diagonal cells beside `d` to be empty, and any tile path connecting `d` to the fountain component then encloses one of those empty cells — an illegal gap. So on valid grids the first non-Empty candidate failing implies no continuation exists, the break loses nothing, every run is fully traversed, and the total is start-order-independent. The finding's example grid (T1 with an Up wall under a tile lacking the matching Down wall) violates wall matching and is unreachable. **Consequence:** live scoring is NOT currently wrong; the practical severity is below the assessed major. The fix lands anyway because (1) the function must not silently depend on invariants enforced by distant code (Dirk's grid is built with no validity check; unit tests pass arbitrary grids; future callers won't know), (2) the fix is provably behavior-preserving on valid grids (the extra candidates it tries cannot succeed there), and (3) it is a two-line change the grouping notes call uncontroversial. Record this as the disposition — do not report live games as having been mis-scored.

**Determinism / serialization decision (per package notes):** the nondeterminism source is `for (v, t) in g.iter()` over `Grid = HashMap<Vect, Tile>` (card.rs:481) — the local `visited` map is only probed, never iterated. Switching `Grid` to `BTreeMap` would fix ordering globally but changes a serialized type's in-memory representation and forces `Ord` on `Vect` plus a `grid_serde` audit — disproportionate, since a complete traversal is already order-independent on valid grids. Chosen fix: **sorted iteration inside this function only** (collect + sort by `(x, y)`), which is serialization-neutral (no field, no format, no migration) and pins the result even on invalid grids (tests, Dirk-style grids) as defense-in-depth for the exclusivity proof above.

**Edge cases:** closed perimeter loops (walk returns to the visited start → stops, counted once); single isolated wall segment (both directions find nothing → 1); grids with only internal walls (all candidates internal → each pair skipped at the outer loop); Empty candidate tiles (unchanged `continue`); invalid mismatched-wall grids (now fully traversed deterministically); the two existing test grids MUST still yield 5 and 12 (hand-verified: in the first grid the pinch at the (2,2)/(3,3) diagonal partitions differently under old/new walks but the maximum stays 5; the second grid's perimeter ring never hits the changed path).

**Files:**
- Modify: `rust/game/alhambra-1/src/card.rs` (`grid_longest_ext_wall`, lines 477-531)
- Test: `rust/game/alhambra-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/alhambra-1/src/lib.rs` (next to `test_grid_longest_ext_wall`). Note: this grid has deliberately mismatched walls (blocker tiles above lack Down walls), which `parse_grid`'s shared-character drawing cannot express — build it directly. It is not play-reachable (see the re-derivation note); it exercises the traversal's robustness on arbitrary grids and pins determinism:

```rust
    #[test]
    fn test_grid_longest_ext_wall_diagonal_blocker() {
        // b F18: a non-empty diagonal tile without the turning wall must not
        // abort the candidate scan before the straight continuation is tried.
        // Blockers sit above both walled tiles so BOTH walk directions hit
        // the truncating candidate first: the old code returns 1 from every
        // start segment; the correct answer is 2.
        let mut g: Grid = HashMap::new();
        g.insert(Vect { x: 0, y: 0 }, Tile::new(TileType::Arcades, 0, &[Dir::Up]));
        g.insert(Vect { x: 1, y: 0 }, Tile::new(TileType::Arcades, 0, &[Dir::Up]));
        g.insert(Vect { x: 0, y: -1 }, Tile::new(TileType::Arcades, 0, &[]));
        g.insert(Vect { x: 1, y: -1 }, Tile::new(TileType::Arcades, 0, &[]));
        assert_eq!(2, grid_longest_ext_wall(&g));
    }
```

  Why the old code deterministically returns 1: from `(0,0).Up` walking right, candidate 0 is the blocker `(1,-1)` (present, no Left wall) → break before the straight candidate `(1,0).Up`; from `(1,0).Up` walking left, candidate 0 is the blocker `(0,-1)` (present, no Right wall) → break before `(0,0).Up`. Both `Up` walls are external because the blockers have no Down walls (the internal-wall probe checks exactly that), so both starts count 1.

- [ ] Run: `cargo test -p alhambra-1 diagonal_blocker`. Expected: FAIL with `assertion `left == right` failed: left: 2, right: 1` — deterministically, from either start segment.
- [ ] Implement. In `grid_longest_ext_wall` in `rust/game/alhambra-1/src/card.rs`:

  1. Replace the outer iteration (line 481) with sorted iteration:

```rust
    // Iterate in coordinate order: Grid is a HashMap, and on grids that
    // violate the play invariants (mismatched walls) the walk result could
    // otherwise depend on which segment starts each traversal.
    let mut entries: Vec<(&Vect, &Tile)> = g.iter().collect();
    entries.sort_by_key(|(v, _)| (v.x, v.y));
    for (v, t) in entries {
```

  2. Move the `break` inside the success branch — replace the candidate test (lines 507-516):

```rust
                        if !visited.contains_key(&next_wall)
                            && grid_is_wall(g, next_wall)
                            && !grid_is_internal_wall(g, next_wall)
                        {
                            wall += 1;
                            visited.insert(next_wall, true);
                            found = true;
                            cur = next_wall;
                            break;
                        }
```

  (i.e. delete the standalone `break;` that followed the `if` block and add `break;` as the last statement inside it). The Empty-tile `continue` above it (lines 504-506) is unchanged. Nothing else in the function changes; `visited` stays a `HashMap` here (Task 10 converts it to a `HashSet` with the other flood-walk collections).

- [ ] Run: `cargo test -p alhambra-1 grid_longest_ext_wall` — the new test PASSES and both existing expectations (5 and 12) still hold. Then `cargo test -p alhambra-1` — full suite PASS. If 5 or 12 changed, the edit was wrong (most likely the `break` was removed entirely instead of moved, letting one step count two candidates) — revert and redo.
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/card.rs rust/game/alhambra-1/src/lib.rs` ; message: `fix(alhambra-1): try all wall-walk continuations, iterate deterministically (b F18, WP-14)`

---

### Task 4: remaining coverage from the F21 inventory (b F21, minor)

**Problem (restated):** F21 lists the untested risky paths. Three of them are the regression tests already landed in Tasks 1-3 (take multiplicity, place-index-after-placement, wall-walk diagonal blocker). Verification corrected the inventory: final-place distribution has a single-currency smoke test (`log_final_place`, lib.rs:1475) — the genuinely missing final-place coverage is the TIE path. Still missing: exact-payment extra action, overpay ending the turn, final-place tie, and 2-player/Dirk flows. These tests cover behavior that is believed CORRECT today — they should pass immediately; they exist to lock the behavior in. If any fails, stop and report (do not "fix" the game to match the test without investigating).

**F21 coverage map (which task carries which gap):**

| F21 gap | Covered by |
|---|---|
| `take` multiplicity | Task 1 (`take_cannot_mint_duplicate_cards`, `take_allows_real_duplicates_in_market`) |
| place-index after placement | Task 2 (`place_index_matches_rendered_index_after_placement`, `place_index_out_of_range_after_placement_errors`, `swap_index_skips_empty_sentinels_in_reserve`) |
| wall walk with diagonal blockers | Task 3 (`test_grid_longest_ext_wall_diagonal_blocker`) |
| exact-payment extra action | this task |
| overpay ending the turn | this task |
| final-place distribution ties | this task |
| 2-player / Dirk flows | this task |

**Files:**
- Test only: `rust/game/alhambra-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Add the tests:

```rust
    #[test]
    fn spend_exact_payment_grants_extra_action() {
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.boards[0].cards = vec![Card::new(Currency::Blue, 3)];
        g.tiles[0] = Tile::new(TileType::Tower, 3, &[]);
        g.command(0, "spend b3", &[]).unwrap();
        // total == cost: no phase advance — same player acts again.
        assert_eq!(Phase::Action, g.phase);
        assert_eq!(0, g.current_player);
        assert_eq!(TileType::Empty, g.tiles[0].tile_type);
        assert_eq!(1, not_empty(&g.boards[0].place).len());
    }

    #[test]
    fn spend_overpayment_ends_action_phase() {
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.boards[0].cards = vec![Card::new(Currency::Blue, 4)];
        g.tiles[0] = Tile::new(TileType::Tower, 3, &[]);
        g.command(0, "spend b4", &[]).unwrap();
        // Overpay: turn moves to the Place phase (the bought tile is
        // placeable, so the phase does not skip past it).
        assert_eq!(Phase::Place, g.phase);
        assert_eq!(0, g.current_player);
    }

    #[test]
    fn final_place_tie_distributes_no_tile() {
        let (mut g, _) = Game::start(3, 42).unwrap();
        g.phase = Phase::Place;
        g.current_player = 0;
        g.boards[0].place = vec![];
        g.boards[1].place = vec![];
        g.boards[2].place = vec![];
        g.tiles[0] = Tile::new(TileType::Pavillion, 5, &[]);
        g.tiles[1] = Tile::empty();
        g.tiles[2] = Tile::empty();
        g.tiles[3] = Tile::empty();
        g.boards[0].cards = vec![Card::new(Currency::Blue, 9)];
        g.boards[1].cards = vec![Card::new(Currency::Blue, 9)];
        g.boards[2].cards = vec![Card::new(Currency::Blue, 5)];
        let logs = g.final_place_phase();
        let combined: String = logs.iter().map(log_plain).collect::<Vec<_>>().join("\n");
        assert!(
            combined.contains("Nobody had the most money for blue"),
            "expected tie message in: {}",
            combined
        );
        assert!(
            g.boards.iter().all(|b| not_empty(&b.place).is_empty()),
            "tied tile must not be distributed to anyone"
        );
        assert_eq!(
            TileType::Pavillion,
            g.tiles[0].tile_type,
            "tied tile stays in the market slot"
        );
    }

    #[test]
    fn dirk_draws_six_tiles_at_start() {
        let (g, _) = Game::start(2, 42).unwrap();
        // Dirk's board: fountain + 6 drawn tiles.
        assert_eq!(7, g.boards[DIRK].grid.len());
    }

    #[test]
    fn dirk_draws_six_more_tiles_in_round_one_scoring() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        let before = g.boards[DIRK].grid.len();
        g.round = 1;
        g.score_round();
        assert_eq!(before + 6, g.boards[DIRK].grid.len());
        assert_eq!(2, g.round);
    }

    #[test]
    fn dirk_competes_in_tile_type_majorities() {
        let (mut g, _) = Game::start(2, 42).unwrap();
        // Human grids start with only the fountain; give Dirk the only
        // Pavillions so Dirk alone holds that majority.
        g.boards[0].grid = new_grid();
        g.boards[1].grid = new_grid();
        g.boards[DIRK].grid = new_grid();
        g.boards[DIRK]
            .grid
            .insert(Vect { x: 1, y: 0 }, Tile::new(TileType::Pavillion, 0, &[]));
        let scores = g.score_type(TileType::Pavillion, 1);
        assert_eq!(
            vec![RoundTypeScore {
                players: vec![DIRK],
                tile_count: 1,
                points: 1
            }],
            scores
        );
    }
```

- [ ] Run: `cargo test -p alhambra-1`. Expected: ALL PASS on the first run. If any fails, do not adjust production code — report the discrepancy (it would mean a live-behavior claim in this spec is wrong).
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/lib.rs` ; message: `test(alhambra-1): cover payment, final-place tie and Dirk flows (b F21, WP-14)`

---

### Task 5: name the invariants on runtime unwraps (b F23, nit)

**Problem (restated):** three invariant-guarded panic sites flagged for intent-documentation only (all verified unreachable from crafted input): `Currency::ALL.iter().position(|&c| c == currency).unwrap()` at `rust/game/alhambra-1/src/lib.rs:431` (`final_place_phase`) and the identical pattern at lib.rs:601 (`spend`, player-supplied currency — still guarded: `Currency::ALL` contains every enum variant); and `Card::parse(&s).unwrap()` at `rust/game/alhambra-1/src/command.rs:142` (the `spend` parser Map; `s` comes from `Enum::exact` over `Card::to_string()` values, which always re-parse). The `panic!` in `rot_all` (card.rs:209) already names its invariant — leave it (Non-Goals).

**Fix:** replace each `.unwrap()` with `.expect("...")` naming the invariant. No behavior change; no test can distinguish it.

**Files:**
- Modify: `rust/game/alhambra-1/src/lib.rs` (lines 431, 601), `rust/game/alhambra-1/src/command.rs` (line 142)

**Steps:**

- [ ] At lib.rs:431 and lib.rs:601 change `.unwrap()` to `.expect("Currency::ALL contains every Currency variant")`.
- [ ] At command.rs:142 change `Card::parse(&s).unwrap()` to `Card::parse(&s).expect("Enum::exact only emits hand-card strings, which always parse")`.
- [ ] Run: `cargo test -p alhambra-1` — full suite PASS (no behavioral assertion possible; the suite guards against typos).
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/lib.rs rust/game/alhambra-1/src/command.rs` ; message: `refactor(alhambra-1): name invariants on guarded unwraps (b F23, WP-14)`

---

### Task 6: symmetric gap-check ranges (b F24, nit)

**Problem (restated):** the gap-check loop in `grid_is_valid` (`rust/game/alhambra-1/src/card.rs:450-451`) is `for x in min.x..=max.x { for y in min.y..max.y {` — y exclusive, x inclusive. Verified provably harmless (any empty cell in row `max.y` borders the always-empty, always-flooded `max.y + 1` ring row, so including it can never newly fail), but it reads like an off-by-one.

**Fix:** use `..=` on both axes (no behavior change, per the proof) with a comment. This is strictly clearer than keeping the asymmetry and commenting why it is safe.

**Files:**
- Modify: `rust/game/alhambra-1/src/card.rs` (line 451)

**Steps:**

- [ ] Replace lines 450-451 with:

```rust
    // Inclusive on both axes for symmetry. Row max.y can never actually fail:
    // any empty cell there borders the empty max.y + 1 ring row, which the
    // outside flood always covers.
    for x in min.x..=max.x {
        for y in min.y..=max.y {
```

- [ ] Run: `cargo test -p alhambra-1` — full suite PASS, in particular `test_grid_is_valid_invalid_gap` and `test_grid_is_valid_valid` unchanged. A failure here means the harmlessness proof was violated — stop and report rather than adjusting the test.
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/card.rs` ; message: `refactor(alhambra-1): symmetric gap-check ranges (b F24, WP-14)`

---

### Task 7: replace Debug formatting in user-facing text (b F25, nit)

**Problem (restated):** five sites interpolate `{:?}` into player-visible strings (the two cited plus the three verification found): `rust/game/alhambra-1/src/lib.rs:603-606` error `"no tile available for {:?}"` (renders "Blue"); lib.rs:637-640 log `" spent {} on {:?} tile"` ("Tower"); lib.rs:684 log `" placed {:?} tile"`; lib.rs:730-731 log `" swapped {:?} with {:?} tile"`; lib.rs:762 log `" removed {:?} tile to reserve"`. Every other user-facing string in the crate uses `Currency::name()` ("blue") or `TileType::abbr().trim()` ("Tow" — see `reserve_tiles`' "added Pav, Ser", `score_round`'s "Scoring Pav", `final_place_phase`'s "got Tow"). Line numbers for the log sites may have shifted by a few lines after Tasks 1-2; locate by the format strings.

**Fix:** `currency.name()` for the error; `<tile_type>.abbr().trim()` for the four tile logs. Consistent with the existing convention; log-string changes are protocol-safe (logs are display-only).

**Files:**
- Modify: `rust/game/alhambra-1/src/lib.rs` (5 sites)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn logs_use_display_names_not_debug() {
        let (mut g, _) = Game::start(3, 0).unwrap();
        g.current_player = 0;
        g.phase = Phase::Action;
        g.boards[0].cards = vec![Card::new(Currency::Blue, 3)];
        g.tiles[0] = Tile::new(TileType::Tower, 3, &[]);
        let logs = g.spend(0, &[Card::new(Currency::Blue, 3)]).unwrap();
        let combined: String = logs.iter().map(log_plain).collect::<Vec<_>>().join("\n");
        assert!(
            combined.contains("spent B3 on Tow tile"),
            "expected abbr, not Debug name, in: {}",
            combined
        );

        // Error path: empty tile slot must name the currency in lowercase.
        g.phase = Phase::Action;
        g.current_player = 0;
        g.tiles[1] = Tile::empty();
        g.boards[0].cards = vec![Card::new(Currency::Green, 2)];
        let err = g
            .spend(0, &[Card::new(Currency::Green, 2)])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("no tile available for green"),
            "expected currency.name() in: {}",
            err
        );
    }
```

  (Note: the first `spend` is exact payment, so the phase stays Action for the second half of the test. `Currency::ALL[1]` is Green, so `tiles[1]` is the green slot.)

- [ ] Run: `cargo test -p alhambra-1 logs_use_display`. Expected: FAIL — the log says "spent B3 on Tower tile" and the error says "no tile available for Green".
- [ ] Implement, replacing only the format arguments:
  - lib.rs:603-606: `format!("no tile available for {}", currency.name())`
  - lib.rs:637-640: `format!(" spent {} on {} tile", card_strs.join(", "), tile.tile_type.abbr().trim())`
  - lib.rs:684: `format!(" placed {} tile", tile.tile_type.abbr().trim())`
  - lib.rs:730-731: `format!(" swapped {} with {} tile", reserve_tile.tile_type.abbr().trim(), grid_tile.tile_type.abbr().trim())`
  - lib.rs:762: `format!(" removed {} tile to reserve", tile.tile_type.abbr().trim())`
- [ ] Run: `cargo test -p alhambra-1` — new test PASSES, full suite PASS (no existing test asserts the old strings).
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/lib.rs` ; message: `fix(alhambra-1): display names instead of Debug in user-facing text (b F25, WP-14)`

---

### Task 8: deduplicate tile_counts (b F26, nit)

**Problem (restated):** `tile_counts` is byte-identical in `rust/game/alhambra-1/src/render.rs:69-77` (free fn over `&Grid`) and `rust/game/alhambra-1/src/card.rs:601-609` (`PlayerBoard::tile_counts`, used by `score_type` at lib.rs:322-326).

**Fix:** add `pub fn grid_tile_counts(g: &Grid) -> HashMap<TileType, i32>` in card.rs (next to the other `grid_*` free functions); make `PlayerBoard::tile_counts` delegate to it; delete render.rs's private copy (render.rs already glob-imports `crate::card::*`, so its call sites at render.rs:240 pick up the free fn — change the call to `grid_tile_counts(&board.grid)`).

**Files:**
- Modify: `rust/game/alhambra-1/src/card.rs`, `rust/game/alhambra-1/src/render.rs`

**Steps:**

- [ ] In card.rs add (near `grid_tile_at`):

```rust
pub fn grid_tile_counts(g: &Grid) -> HashMap<TileType, i32> {
    let mut counts = HashMap::new();
    for t in g.values() {
        if t.tile_type != TileType::Empty {
            *counts.entry(t.tile_type).or_insert(0) += 1;
        }
    }
    counts
}
```

  and reduce `PlayerBoard::tile_counts` to `grid_tile_counts(&self.grid)`.
- [ ] In render.rs delete the private `tile_counts` fn (lines 69-77) and change its one call site (`let counts = tile_counts(&board.grid);` in `render_player_summary`) to `let counts = grid_tile_counts(&board.grid);`. If `HashMap` is then unused in render.rs, drop the `use std::collections::HashMap;` import.
- [ ] Run: `cargo test -p alhambra-1` — full suite PASS (`test_game_score_type` exercises the delegating method).
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/card.rs rust/game/alhambra-1/src/render.rs` ; message: `refactor(alhambra-1): single grid_tile_counts helper (b F26, WP-14)`

---

### Task 9: clamp grid column headers (b F27, nit)

**Problem (restated):** `render_grid` (`rust/game/alhambra-1/src/render.rs:162-167`) computes `((x - x_start) as u8 + b'a') as char`, which wraps into punctuation past 26 columns. Practically unreachable (54 tiles max; verification adds that the coord parser cannot address past 'z' anyway, so it is cosmetic only).

**Fix:** clamp to 'z' with a comment. Aligns the display with the addressable coordinate space. No test — constructing a 27-column board is disproportionate for an unreachable cosmetic path; note this in the commit.

**Files:**
- Modify: `rust/game/alhambra-1/src/render.rs` (line 164)

**Steps:**

- [ ] Replace line 164:

```rust
        // Clamp: columns past 'z' are unaddressable via the coord parser
        // anyway; wrapping the u8 would render punctuation.
        let col_letter = char::from(((x - x_start).min(25)) as u8 + b'a');
```

- [ ] Run: `cargo test -p alhambra-1` — full suite PASS.
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/alhambra-1/src/render.rs` ; message: `fix(alhambra-1): clamp grid column headers at z (b F27, WP-14)`

---

### Task 10: idiomatic flood-walk collections (b F28, nit)

**Problem (restated):** both flood walks in `grid_is_valid` (`rust/game/alhambra-1/src/card.rs:379-448`) use a `Vec` as a queue (`walk_stack.remove(0)` — O(n) per pop) and `HashMap<Vect, bool>` as sets whose bool payloads are written but never read. `grid_longest_ext_wall`'s `visited: HashMap<VectDir, bool>` (card.rs:478 area, post-Task-3) is the same never-read-payload pattern. All are function-locals — zero serde impact.

**Fix:** `VecDeque::pop_front` + `HashSet`. Semantics note: in the current code an entry stays in `in_walk_stack` forever (queued with `true`, re-inserted with `false` on pop) and only `contains_key` is read — so plain `HashSet::insert` at the same points is behavior-identical.

**Files:**
- Modify: `rust/game/alhambra-1/src/card.rs`

**Steps:**

- [ ] In card.rs change the file-top import to `use std::collections::{HashMap, HashSet, VecDeque};` (HashMap is still needed by `Tile::walls`, `grid_tile_counts` and `Grid`).
- [ ] In `grid_is_valid`, for BOTH walks, mechanically apply:
  - `let mut walk_stack = vec![fv];` → `let mut walk_stack = VecDeque::from([fv]);` (second walk: `VecDeque::from([start])`)
  - `while let Some(next) = walk_stack.first().copied() { walk_stack.remove(0); ... }` → `while let Some(next) = walk_stack.pop_front() { ... }`
  - `in_walk_stack: HashMap<Vect, bool>` / `connected: HashMap<Vect, bool>` → `HashSet<Vect>`; every `.insert(v, _)` → `.insert(v)`; every `.contains_key(&v)` → `.contains(&v)`.
  - `walk_stack.push(dv)` → `walk_stack.push_back(dv)`.
- [ ] In `grid_longest_ext_wall`, change `visited` to `HashSet<VectDir>` (`VectDir` derives `Hash`/`Eq`): `.insert(vd, true)` → `.insert(vd)`, `.contains_key(&...)` → `.contains(&...)` (three probe sites after Task 3).
- [ ] Run: `cargo test -p alhambra-1` — full suite PASS (all four `test_grid_is_valid_*` tests plus all three wall tests are the behavioral lock).
- [ ] `cargo clippy -p alhambra-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/alhambra-1/src/card.rs` ; message: `refactor(alhambra-1): VecDeque/HashSet in flood walks (b F28, WP-14)`

---

## Findings disposition

| Finding | Severity | Disposition |
|---|---|---|
| b F16 take() duplicate mint | critical | Fixed (Task 1): clone-and-verify against market multiplicity, mirroring spend(); recommendation confirmed correct against live source. Exploit + regression tests added. |
| b F17 place-index divergence | major | Fixed (Task 2): indices resolved over the non-Empty subsequence in place() (both phases) AND swap()/reserve as hardening for legacy corrupted saves; Empty sentinel write kept deliberately (handles legacy states; smaller diff than compaction). |
| b F18 wall-walk premature break | major | Fixed (Task 3): break moved inside the success branch + sorted outer iteration. Impact re-derived: truncation/nondeterminism only occur on grids violating grid_is_valid invariants, which are unreachable in play — live scores were NOT wrong; fix lands for robustness. Chose sorted iteration over BTreeMap: serialization-neutral, no migration. |
| b F21 missing tests | minor | Fixed (Tasks 1-4): regression tests land with each fix; Task 4 adds exact-payment, overpay, final-place tie (per verification, the tie path was the real gap) and Dirk flows. Coverage map in Task 4. |
| b F23 invariant panics | nit | Fixed (Task 5): expect() naming the invariant at lib.rs:431/601 and command.rs:142; rot_all panic left (already named, per verification). |
| b F24 gap-check asymmetry | nit | Fixed (Task 6): inclusive on both axes + comment; provably no behavior change. |
| b F25 Debug in messages | nit | Fixed (Task 7): all 5 sites (2 cited + 3 from verification) use name()/abbr().trim(); log test added. |
| b F26 tile_counts duplication | nit | Fixed (Task 8): pub grid_tile_counts in card.rs; PlayerBoard method delegates; render copy deleted. |
| b F27 column-header wrap | nit | Fixed (Task 9): clamp at 'z' + comment; no test (unreachable cosmetic path). |
| b F28 Vec-as-queue / HashMap-as-set | nit | Fixed (Task 10): VecDeque + HashSet in both grid_is_valid walks and the wall-walk visited set; locals only, no serde impact. Tile::walls untouched (serialized — Non-Goal). |
