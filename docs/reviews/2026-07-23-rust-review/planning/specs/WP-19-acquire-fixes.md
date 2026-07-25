# WP-19: acquire-1 fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Make 6-player Acquire games offerable at all (c F7 — `player_counts()` advertises `[2,3,4,5]` while the engine fully supports 6), make the 2-player dummy shareholder die a real D6 (c F8 — it can never roll a 6 today, contradicting both the crate's own RULES.md and its own start log), remove the panic macros and `expect()` cluster from command-, render- and endgame-reachable paths (c F9, c F10, c F18, c F19), fix the `Trades` stat that reports the merge count (c F11), drop the unused `thiserror` dependency (c F16), stop deep-cloning the whole game to compute three integers on every parser build (c F20), make foundable/mergeable corporation ordering deterministic (c F21), and collapse the always-true `can_undo` expression in `handle_found_command` (c F17).

**Architecture — how acquire-1 works (read this before editing):**

- One crate, `rust/game/acquire-1` (package name `acquire-1`, confirmed from `Cargo.toml:2`; lib name `acquire_1`). No Go port exists — this is a fresh Rust implementation, so rules questions are judged against the crate's own `RULES.md` first and the official rulebook second.
- `src/lib.rs` (1370 lines): constants (`MIN_PLAYERS = 2`, `MAX_PLAYERS = 6`, `STARTING_MONEY`, `STARTING_SHARES = 25`, `TILE_HAND_SIZE = 6`, `BONUS_ROUNDING = 100`, `DUMMY_PLAYER_OFFSET = 999`, lib.rs:24-30), the serialized `Phase` enum (lib.rs:32-54), `PubState` (lib.rs:84-100), `PlayerState` (lib.rs:137-145), `Game` (lib.rs:147-160, with a `#[serde(default = "GameRng::from_entropy")]` migration shim on `rng` at lib.rs:156-159), the `Gamer` impl (lib.rs:177-341), the `CanEnd`/`CanEndFalse` end-condition types (lib.rs:343-362), the private `BonusPlayers` carrier (lib.rs:364-368), all command handlers and turn flow in `impl Game` (lib.rs:370-1203), `Player`/`PubPlayer` (lib.rs:1205-1261), the `#[cfg(test)] From<&str> for Game` board-literal helper (lib.rs:1263-1285), and `#[cfg(test)] mod tests` (lib.rs:1287-1370, 4 tests).
- `src/board.rs`: `Board(pub Vec<Tile>)` — a flat 108-cell `Vec` (`WIDTH = 12`, `HEIGHT = 9`, `SIZE = 108`, board.rs:13-15) indexed by `Loc { row, col }` through `From<&Loc> for usize` (board.rs:283-293). `get_tile` is bounds-safe (`.get(..).cloned().unwrap_or_default()`, board.rs:30-32) and `set_tile` grows the vec (board.rs:34-42) — the board has no indexing-panic surface. Playability rules: `assert_loc_playable` (board.rs:130-142), `loc_founds` (board.rs:144-154), `loc_neighbours_multiple_safe_corps` (board.rs:156-167). Inline `mod tests` at board.rs:295-347 (6 tests).
- `src/corp.rs`: `Corp` (7 variants), `CORPS: [Corp; 7]` (corp.rs:24-32) — **the canonical corporation order**, `Corp::iter()` over it (corp.rs:49-51), pricing tiers, `SAFE_SIZE = 11`, `GAME_END_SIZE = 41`, `MINOR_MULT = 5`, `MAJOR_MULT = 10`.
- `src/command.rs` (private `mod command;`, lib.rs:19): the `Command` enum (command.rs:9-19) and `Game::command_parser` (command.rs:22-76), which builds a phase-specific `OneOf` and returns `None` when it is not the player's turn or the game is finished. All parsers are wired.
- `src/render.rs`: markup renderer for `PubState`/`PlayerState` — `corp_table` (render.rs:56-90), `player_table`/`player_row` (render.rs:102-143), `Board::render` canvas with the corp-name width scan (render.rs:197-306).
- `src/stats.rs` (private `mod stats;`, lib.rs:22): serialized per-player `Stats` struct (stats.rs:9-24) and `to_brdgme_stats()` (stats.rs:26-85). `to_brdgme_stats` has **zero callers workspace-wide** — `status()` returns `stats: vec![]` (lib.rs:238). Wiring-or-deleting that machinery is c F12, owned by WP-20; this package only fixes the wrong field in it (c F11).
- Turn/game flow: `start()` shuffles `Loc::all()`, places one tile per player on the board, deals 6 tiles each, picks a random start player. `Phase::Play` -> `play <tile>` -> (found / extend / merge) -> `Phase::Buy` (3 shares max) -> `done` -> `draw_replacement_tiles` -> next player's `start_turn`. Mergers run `pay_bonuses(from)` then a `Phase::SellOrTrade` round-robin (`sell`/`trade`/`keep`), then `convert_corp` and a re-check for chained mergers. `end` sets `last_turn`; the following `done` runs `end()`, which pays bonuses for every live corp and liquidates all shares.
- Serialization: `Game`/`PubState`/`PlayerState` round-trip through the DB as serde JSON. **No fix in this package may change any serialized type, field name, or shape.** `shares: HashMap<Corp, usize>` on `Game`/`Player`/`PubState`/`PubPlayer` stays a `HashMap` (a `BTreeMap` re-typing of serialized fields has been rejected in earlier specs in this review); every determinism fix below uses transient sorted locals derived from `CORPS`.
- Bins (`src/bin/*.rs`) are the 4 standard boilerplate binaries (cli/fuzz/http/repl), verified byte-identical to the standard pattern by the review. `tests/contract.rs` is the standard `assert_gamer_contract::<Game>()` harness.

**Tech Stack:** Rust 1.97.0 (edition 2024) workspace at `/home/beefsack/Development/brdgme/rust`. Single crate touched: `acquire-1`. `GameRng` is `ChaCha8Rng` (`rust/lib/game/src/rng.rs:11-30`) — portable, stable across crate versions, serialized with full stream position, so seeded tests are reproducible forever. Let-chains and `Option::get_or_insert` are available on this toolchain.

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p acquire-1`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean. Both are clean at baseline (measured 2026-07-25).
- **Measured green baseline: 11 tests passing** — `cargo test -p acquire-1` gives 10 unit tests in the lib (`board::tests::{usize_into_loc_works, loc_into_usize_works, board_get_tile_works, board_indexing_by_loc_works, board_set_tile_works, board_corp_size_works}`, `tests::{game_from_str_is_deterministic, play_works, found_works, merge_works}`) plus 1 integration test (`tests/contract.rs::game_contract`); the four bins and doc-tests contribute 0. All 11 MUST keep passing unmodified. None constrains any fix below — see the per-task edge cases for `merge_works` (the only test that walks a merge and therefore the F9/F10 code) and `game_contract` (the only test that exercises `player_counts()`).
- Line numbers cited are LIVE-file numbers as of the drift check below. Tasks 3 and 4 shift `lib.rs` numbering below ~line 675; later tasks locate by symbol name.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script provides the containers).
- Never replace a silent wrong answer with a panic, and never add a panic macro or an indexing/`unwrap`/`expect` panic on a path reachable from a player command or from rendering.

**Non-Goals (owned elsewhere — do NOT touch these):**

- **c F13** (start player chosen by `g.rng.random_range(0..players)` at lib.rs:213 instead of by initial tile draw) — WP-20, BLOCKED-ON-DECISION. Do not change `start()`'s start-player selection or the initial tile placement loop (lib.rs:199-214).
- **c F14** (full-hand redraw discards temporarily-unplayable tiles: `start_turn` lib.rs:693-708 + `redraw_hand` lib.rs:710-735 vs `assert_loc_playable` board.rs:130-142) — WP-20, BLOCKED-ON-DECISION. Do not touch `start_turn`, `redraw_hand`, or `assert_loc_playable`.
- **c F15** (tile-bag exhaustion ends the game immediately, lib.rs:403-408) — WP-20, BLOCKED-ON-DECISION. Do not touch `draw_replacement_tiles`' end trigger.
- **c F12** (stats tracked but never surfaced — `status()` returns `stats: vec![]` at lib.rs:238; `to_brdgme_stats` has no callers) — WP-20, keep-or-drop decision. Task 5 fixes ONE wrong field inside `to_brdgme_stats` and MUST NOT wire it into `status()` and MUST NOT delete the module. If WP-20 later decides "delete", Task 5's edit and its test disappear with it — that is fine and expected.
- **c F2** (texas-holdem-2 `MAX_PLAYERS` 8 vs Go 9) — WP-20, different crate.
- The **edition/player-cap adjudication itself**: this package changes only the *internal inconsistency* between `player_counts()` and `MAX_PLAYERS`/`start()` (see the F7 re-derivation, which proves the fix is separable from any edition question). Do NOT add player-count wording to `RULES.md`, and do NOT change `MIN_PLAYERS`/`MAX_PLAYERS` or `start()`'s bounds check.
- The `use std::iter::FromIterator;` import at board.rs:3 (prelude-redundant since edition 2021) — not a filed finding here and clippy at `-D warnings` does not flag it; see "Cross-package / newly discovered".
- The two `panic!("must be Phase::SellOrTrade")` sites at lib.rs:951 and lib.rs:980 — NOT filed findings, provably unreachable, reported in "Cross-package / newly discovered" for routing rather than fixed here.

**Snapshot drift:** None. `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/acquire-1 /home/beefsack/Development/brdgme/rust/game/acquire-1` is empty (exit 0, verified 2026-07-25 against snapshot commit `f8763a5`). All line numbers below are live-file numbers and match the findings' and verification's citations exactly.

**Re-derivation notes (verified against live source):**

- **F7 (6-player games are never offered) — HEADLINE, cause re-derived and fix confirmed separable.** `player_counts()` is `(2..6).collect()` (lib.rs:312-314) — a half-open range yielding `[2, 3, 4, 5]`. Everything else in the crate says 6 is supported: `MAX_PLAYERS = 6` (lib.rs:25) and `start()` gates on `(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)` (lib.rs:186), i.e. it *accepts* 6. `player_counts()` is the `Gamer` trait's advertised set (`rust/lib/game/src/game.rs:64`); it is served over the wire by `handle_player_counts` (`rust/lib/cmd/src/requester/gamer.rs:56-58`), persisted into `game_types.player_counts` by the web service (`rust/web/src/game/mod.rs:478`), and the new-game UI both filters the game list by it and populates the player-count selector from it (`rust/web/src/new_game.rs:71`, `:244`, `:470`). Consequence: 6-player acquire is unreachable through the product even though the engine handles it. `rand_bot`/`bot` also derive their player-name padding from it (`rust/lib/game/src/bot.rs:66-72`). **Capacity re-derived, not assumed:** 6 players consume `players + players * TILE_HAND_SIZE` = 6 + 36 = 42 of the 108 tiles in `Loc::all()` at setup, leaving 66 in the bag; bank stock is 25 per corp independent of player count; `bonus_players`/`pay_bonuses` have no player-count ceiling (only the `== 2` dummy branch); the renderer's player table iterates `self.players` (render.rs:102-112) with no fixed width. **Verified empirically** during spec writing with a throwaway integration test (since deleted): `Game::start(n, 1)` succeeds for n in 2..=6 and every seat's `player_state(p)` + `command_spec(p)` builds without panic; `start(1)` and `start(7)` correctly error. So the fix is a one-line range correction with no edition question attached: the resulting advertised set is **`[2, 3, 4, 5, 6]`**, exactly matching `MIN_PLAYERS..=MAX_PLAYERS`, and no rules decision (2-player dummy variant validity, official 3-6 range, edition trio F13/F14/F15) is touched or presupposed. This finding is **not** blocked on WP-20.
- **F8 (dummy die can never roll 6) — HEADLINE, exact bug located.** `bonus_players()` at lib.rs:897-905:

```rust
        if self.players.len() == 2 {
            dummy_shares = self.rng.random_range(1..=5);
```

  `1..=5` is an inclusive range whose top is 5, so the "D6" yields a uniform 1-5 and 6 is unreachable. This contradicts **two in-repo sources** (not just external rules): the start log the game itself prints, "A dice (D6) is rolled to determine the dummy player's shares" (lib.rs:219-224), and `RULES.md:151-155`, "roll a six-sided die (D6): the result (1-6) is the dummy's share count in the acquired corporation". The effect is not cosmetic: `dummy_shares` seeds `major_count` (lib.rs:904), so the dummy is systematically weaker than specified and 2-player majority/minority splits are biased toward the human players. Fix is `1..=6`. Note this is a live-behavior change for in-flight 2-player games, which is the point; no serialized data changes and the `rng` stream advances identically (one `random_range` call either way).
- **F9 (`panic!` in `pay_bonuses`) — reachability proven latent-but-request-adjacent.** lib.rs:838-842 does `if major_len == 0 { panic!("expected some major bonus players") }`. `pay_bonuses` is called from exactly two places, both request-reachable: `handle_merge_command` (lib.rs:805, reached from `command()` via `Command::Merge`, and from `choose_merger_phase`'s auto-merge at lib.rs:542) and `end()` (lib.rs:678, reached from `handle_done_command` -> `end_turn` when `last_turn`, and from `draw_replacement_tiles`' bag-exhaustion path). **Under game invariants `major` cannot be empty:** for 2 players the dummy is unconditionally pushed (lib.rs:903), and for 3+ players every corp present on the board was founded through `handle_found_command`, which hands the founder a free share (lib.rs:571-578); a player can only dispose of shares in a corp that is *being merged away* (`sell`/`trade` are gated on `Phase::SellOrTrade`'s `corp`), and `pay_bonuses(from)` runs *before* that phase opens (lib.rs:805-812), while `end()` pays a corp's bonuses before liquidating that corp's shares (lib.rs:674-687). The one gap is founding when the bank has 0 shares of that corp (lib.rs:573 `if *corp_shares > 0`), which requires all 25 to be player-held — in which case `major` is non-empty anyway. So: **latent, not reachable from valid state; reachable from a deserialized/legacy/corrupt state**, which is exactly what `GameError::Internal` exists for. Fix: `pay_bonuses` returns `Result<Vec<Log>, GameError>`; both call sites already sit in `Result` functions. Error-path state mutation is safe: `bonus_players` has already rolled the dummy die by then, but `rust/lib/cmd/src/requester/gamer.rs:88-107` returns `SystemError`/`UserError` *without* a game payload on `Err`, so the mutated state is discarded, not persisted.
- **F10 (`expect()` cluster on `HashMap` keys) — all 10 sites verified live.** `.expect(...)` on share-map lookups at lib.rs:680-683 (`end()`), lib.rs:1003-1007 (`handle_sell_command`), lib.rs:1023-1026 (`sell`), lib.rs:1072-1076 (`handle_trade_command`), lib.rs:1083-1087 (`self.shares` for the `into` corp), lib.rs:1119-1122 (`take_shares`), lib.rs:1136-1139 (`return_shares`), command.rs:56-60 (`command_parser`, `Phase::SellOrTrade` arm), command.rs:157-164 (`player_shares_parser`, including the typo `"could not et player shares"`), and render.rs:74-79 (`corp_table`). Fresh games pre-populate all 7 keys via `corp_hash_map` (lib.rs:1224-1230, used by `Game::default` and `Player::default`), so these only fire on a deserialized state missing a key — and serde deserializes a `HashMap` with missing keys happily, while this crate already ships a legacy-state migration shim (the `rng` `#[serde(default)]` at lib.rs:156-159), so legacy states are a live concern. The crate is already internally inconsistent about this: lib.rs:616, lib.rs:813, lib.rs:909, lib.rs:965 and render.rs:138 use `.get(&corp).cloned().unwrap_or(0)` for the *same* lookups. **Every site re-derived for "does 0 give a correct answer, not a silent wrong one?":** lib.rs:683 -> `p_shares == 0` skips the liquidation sell (correct); lib.rs:1006 -> treated as "player is done", advances the round-robin (correct, and in fact unreachable-with-missing-key because the preceding `sell()` inserts the key via `entry()` in `return_shares`); lib.rs:1026 -> `n > 0 == player_shares` yields `InvalidInput "you don't have that many shares"` (correct); lib.rs:1076 -> `InvalidInput "you only have 0 <Corp>"` (correct); lib.rs:1087 -> `InvalidInput "<Corp> only has 0 remaining"` (correct); lib.rs:1122 -> `InvalidInput "<Corp> only has 0 left"` (correct, and it returns *before* the `entry(corp).or_insert(STARTING_SHARES)` at lib.rs:1130 could resurrect the key with 25 and underflow); lib.rs:1139 -> `InvalidInput "only has 0 left"` (correct, same ordering argument); command.rs:59 -> no `trade` parser offered (correct — you cannot 2-for-1 with 0 shares); command.rs:163 -> `Int::bounded(1, 0)`, which is *not* a panic and not UB: `Int::parse` (`rust/lib/game/src/command/parser/mod.rs:119-175`) checks `min`/`max` independently and rejects every input with `"N is too high"`, and `to_spec` just emits `Spec::Int { min: Some(1), max: Some(0) }` (correct: a player with no shares can offer no amount); render.rs:78 -> prints `"0 left"` (correct). No site trades a panic for a wrong answer.
- **F11 (`Trades` stat):** stats.rs:45-46 is a literal copy-paste — `s.insert("Merges", Stat::Int(self.merges as i32)); s.insert("Trades", Stat::Int(self.merges as i32));`. `stats.trades` is maintained (incremented by `receive`, i.e. shares *gained*, at lib.rs:1095) but never surfaced. One-token fix.
- **F16 (`thiserror`):** `Cargo.toml:14` declares `thiserror = "2.0.18"`; `grep -rn thiserror rust/game/acquire-1/` (excluding `Cargo.lock`) matches only that line — no `use`, no `#[derive(Error)]`, in `src/`, `src/bin/` or `tests/`. The crate defines no error type of its own; it returns `brdgme_game::errors::GameError`.
- **F17 (tautological `can_undo`):** `handle_found_command` returns `matches!(self.phase, Phase::Buy { .. })` (lib.rs:586) immediately after `self.buy_phase(player)` (lib.rs:579), and `buy_phase` unconditionally assigns `Phase::Buy { player, remaining: 3 }` (lib.rs:516-521). Always `true`. `true` is also the *correct* value: founding consumes no randomness and reveals no hidden information, so it is undoable.
- **F18 (`unwrap()` on the 1-element set):** lib.rs:466 `neighbouring_corps.iter().next().unwrap()` inside the `1 =>` arm of `match neighbouring_corps.len()` (lib.rs:464). Safe by construction; `Corp` is `Copy`, so a `let ... else` with `GameError::Internal` costs nothing.
- **F19 (`unwrap()` in the render row-run scan):** render.rs:263-271 sets `start = Some(col)` when `start.is_none()`, then reads `start.unwrap()` twice on the last-column branch. Safe by construction but on the render path. `let s = *start.get_or_insert(col);` expresses the same thing with no panic operator.
- **F20 (full-game clone per parser build):** `player_can_end` (lib.rs:1200-1202) and `handle_end_command` (lib.rs:1184) both call `self.pub_state().can_end()`, and `pub_state()` is `self.to_owned().into()` (lib.rs:258-260) — a deep clone of the whole `Game` (108-element board `Vec`, every player's `HashMap<Corp, usize>` + `Vec<Loc>` hand + `Stats`, the bank `HashMap`, the `ChaCha8Rng`) discarded immediately. `player_can_end` runs inside `command_parser` (command.rs:67), which is built on **every** `command()` and `command_spec()` call, and `command_spec` is called once per seat per request by `renders()` (`rust/lib/cmd/src/requester/gamer.rs:70-77`) — so a 6-player game pays 6+ full-game clones per request. `PubState::can_end` (lib.rs:102-135) reads only `self.board`, `self.finished` and `self.last_turn`, and `From<Game> for PubState` (lib.rs:1232-1243) copies those three fields verbatim, so a shared helper over `(&Board, finished, last_turn)` is exactly equivalent. `PubState::can_end` must stay `pub` — render.rs:30 calls it.
- **F21 (nondeterministic corp ordering in the found parser):** command.rs:34 feeds `self.board.available_corps()` — a `HashSet<Corp>` (board.rs:55-63) — straight into `Enum::partial`, so `Spec::Enum { values }` ordering varies per call. `std`'s `RandomState` is per-instance and `available_corps()` builds a fresh `HashSet` each call, so **the order differs between two calls in the same process**, not merely across processes. Measured during spec writing: 50 consecutive `command_spec(0)` calls in one process on a `Phase::Found` state produced **50 distinct** `Spec` debug strings. The same defect exists one arm down at command.rs:43-52, where `neighbouring_corps(&at)` (also a `HashSet<Corp>`, board.rs:65-73) feeds the merge parser's two `Enum::partial` calls — the finding cites only command.rs:34, but it is the identical defect in the identical shape, so Task 8 fixes both and says so out loud (see the disposition table and the newly-discovered section). Fix: filter `CORPS` (corp.rs:24-32, already imported at command.rs:7) by set membership, which yields the canonical corporation order with no new types and no serialized change.

---

### Task 1: advertise 6-player games (c F7, major)

**Problem (restated):** `player_counts()` returns `(2..6).collect()` = `[2, 3, 4, 5]` (lib.rs:312-314) while `MAX_PLAYERS = 6` (lib.rs:25) and `start()` accepts 6 (lib.rs:186). Because `player_counts()` is what the service persists into `game_types.player_counts` and what the new-game UI offers, 6-player Acquire — the game's headline player count — can never be created, despite the engine supporting it end to end.

**Fix (re-derived):** replace the half-open literal range with the inclusive constant range so the advertised set is derived from the same constants `start()` validates against: `(MIN_PLAYERS..=MAX_PLAYERS).collect()`. This is the finding's recommendation verbatim, and re-derivation confirms it: capacity holds (42 of 108 tiles used at 6-player setup, 66 left in the bag; bank stock is per-corp, not per-player; the renderer is player-count agnostic) and no rules/edition question is entangled (`MIN_PLAYERS`/`MAX_PLAYERS` are unchanged). Using the constants rather than `(2..=6)` is deliberate: it makes the advertised set and the accepted set impossible to desynchronize again.

**Edge cases:**
- `tests/contract.rs::game_contract` drives `assert_gamer_contract` (`rust/lib/cmd/src/test_support.rs:20-154`), which (a) picks the smallest count in `0..=max+1` that is *not* advertised and asserts `New` fails for it — that is `0` both before and after this change, so the assertion still holds — and (b) runs `New` + `Status` + a garbage `Play` for **every** advertised count. After this change the contract test additionally exercises a real 6-player game's renders and specs; verified passing by construction during spec writing (a throwaway test ran `start(2..=6)` plus every seat's `player_state`/`command_spec`).
- Existing saved games are unaffected: `player_counts()` is a static function, not serialized state.
- The web service caches `player_counts` per game *version* in the `game_types` table (`rust/web/src/game/mod.rs:478`); a redeploy of acquire-1 registers a new row. No migration is needed from this crate, and nothing here touches the DB.
- 2-player games remain advertised (the dummy-shareholder variant is implemented, RULES.md:151-155); this task does not adjudicate whether 2 *should* be offered — it only stops dropping 6.

**Files:**
- Modify: `rust/game/acquire-1/src/lib.rs` (`player_counts`, lines 312-314; `mod tests`)

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/acquire-1/src/lib.rs`:

```rust
    #[test]
    fn player_counts_covers_min_to_max_players() {
        // c F7: player_counts() is the platform's advertised set. It must match
        // what start() actually accepts, or supported player counts silently
        // become unreachable through the product (6 was being dropped).
        assert_eq!(vec![2, 3, 4, 5, 6], Game::player_counts());
        for n in Game::player_counts() {
            let (g, _logs) = Game::start(n, 1)
                .unwrap_or_else(|e| panic!("advertised count {} must start: {}", n, e));
            assert_eq!(n, g.player_count());
            // Every seat must render and produce a command spec.
            for p in 0..n {
                let _ = g.player_state(p);
                let _ = g.command_spec(p);
            }
        }
        assert!(
            Game::start(MIN_PLAYERS - 1, 1).is_err(),
            "counts below MIN_PLAYERS must be rejected"
        );
        assert!(
            Game::start(MAX_PLAYERS + 1, 1).is_err(),
            "counts above MAX_PLAYERS must be rejected"
        );
    }
```

- [ ] Run: `cargo test -p acquire-1 player_counts_covers` — expected FAIL on the first `assert_eq!` (`left: [2, 3, 4, 5, 6]`, `right: [2, 3, 4, 5]`).
- [ ] Implement: in `rust/game/acquire-1/src/lib.rs`, replace `player_counts` (lines 312-314) with:

```rust
    fn player_counts() -> Vec<usize> {
        (MIN_PLAYERS..=MAX_PLAYERS).collect()
    }
```

- [ ] Run: `cargo test -p acquire-1` — new test PASSES, all 11 baseline tests PASS (`game_contract` now also drives a 6-player game).
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/lib.rs` ; message: `fix(acquire-1): advertise 6-player games in player_counts (c F7, WP-19)`

---

### Task 2: the dummy shareholder rolls a real D6 (c F8, major)

**Problem (restated):** lib.rs:902 rolls the 2-player dummy shareholder's holding with `self.rng.random_range(1..=5)` — a uniform 1-5. Six is unreachable. Both the game's own start log ("A dice (D6) is rolled to determine the dummy player's shares", lib.rs:219-224) and the crate's `RULES.md:151-155` ("roll a six-sided die (D6): the result (1-6)") specify a D6. `dummy_shares` becomes `major_count` (lib.rs:904), so the dummy loses majority contests it should win and 2-player bonus payouts are biased.

**Fix (re-derived):** `self.rng.random_range(1..=6)`. Confirmed as the finding recommends; no alternative is defensible given two in-repo specifications of a D6.

**Edge cases:**
- One `random_range` call either way, so the `GameRng` stream advances by the same number of *calls*; the concrete bytes consumed and hence all subsequent draws in a live 2-player game change. That is inherent to fixing a wrong distribution and affects no serialized shape.
- `dummy_shares == 6` flows through the same code as 1-5: `major = [DUMMY_PLAYER_OFFSET]`, `major_count = 6`; the payout loops skip `DUMMY_PLAYER_OFFSET` (lib.rs:854, 866) so no `self.players[999]` indexing occurs, and `bonus_log` renders it as "dummy player" (lib.rs:888-890). A dummy holding 6 can now tie a human on 6 shares, which correctly lands both in `major` (lib.rs:919-920) — the tie logic is share-count based and needs no change.
- `handle_merge_command` already refuses undo in 2-player games because a die is rolled (`can_undo && self.players.len() > 2`, lib.rs:820) — unchanged.
- The test must not be flaky. It uses a fixed seed (`ChaCha8Rng`, portable and stable across crate versions per `rust/lib/game/src/rng.rs:11-16`) and 1000 draws, so it is fully deterministic, not probabilistic. Measured during spec writing at several stream offsets: with seed 42 all six faces appear, and the *first* 6 lands within the first 16 draws at every offset tried — 1000 draws is enormous headroom, and because the seed is fixed the result cannot vary between runs.

**Files:**
- Modify: `rust/game/acquire-1/src/lib.rs` (`bonus_players`, line 902; `mod tests`)

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/acquire-1/src/lib.rs`:

```rust
    #[test]
    fn dummy_shareholder_rolls_a_full_d6() {
        // c F8: RULES.md:151-155 and the start log both specify a D6 (1-6).
        // Seeded and deterministic: 1000 rolls from a fixed ChaCha8 stream, so
        // this test can never flake - it either covers 1-6 or the range is wrong.
        let mut g = Game::start(2, 42).expect("expected 2 player game").0;
        let mut seen = [0usize; 7];
        for _ in 0..1000 {
            let n = g.bonus_players(Corp::Worldwide).dummy_shares;
            assert!(
                (1..=6).contains(&n),
                "dummy roll {} is outside a six-sided die",
                n
            );
            seen[n] += 1;
        }
        for face in 1..=6 {
            assert!(
                seen[face] > 0,
                "die face {} never appeared in 1000 rolls (counts: {:?})",
                face,
                seen
            );
        }
    }
```

- [ ] Run: `cargo test -p acquire-1 dummy_shareholder_rolls` — expected FAIL on `die face 6 never appeared in 1000 rolls`.
- [ ] Implement: in `bonus_players` (`rust/game/acquire-1/src/lib.rs`, line 902), change `self.rng.random_range(1..=5)` to `self.rng.random_range(1..=6)`.
- [ ] Run: `cargo test -p acquire-1` — new test PASSES, all previous tests PASS. `merge_works` (lib.rs:1336-1369) is a **2-player** merge test and asserts exact money totals — verify it still passes: it seeds via `Game::start(2, 1)` in the `From<&str>` helper (lib.rs:1278) and its assertions are about major/minor splits where both players hold 8-9 shares, far above any 1-6 dummy roll, so the dummy lands in neither `major` nor `minor` regardless of the face. If it *does* fail, STOP and report — that would mean the assertions were tuned to a specific die value.
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/lib.rs` ; message: `fix(acquire-1): dummy shareholder rolls a full D6 (c F8, WP-19)`

---

### Task 3: `pay_bonuses` returns an error instead of panicking (c F9, minor)

**Problem (restated):** `pay_bonuses` (lib.rs:823-875) does `if major_len == 0 { panic!("expected some major bonus players") }` (lib.rs:838-842). It is called from `handle_merge_command` (lib.rs:805) and `end()` (lib.rs:678), both reachable from `Gamer::command`. Under game invariants `major` is never empty (proof in the re-derivation notes), but a deserialized/legacy state whose share maps lost keys reaches it — and a panic in the game binary kills the serving request/worker instead of returning a `GameError`. Repo rules forbid panic macros on command-reachable paths.

**Fix (re-derived):** change the signature to `fn pay_bonuses(&mut self, corp: Corp) -> Result<Vec<Log>, GameError>` and return `GameError::Internal { message: "no major bonus players" }`. Both call sites already return `Result`, so they just gain `?`. `GameError::Internal` is the right variant: `rust/lib/cmd/src/requester/gamer.rs:103` maps it to `Response::SystemError` (an operator-visible bug signal) while ordinary rule violations stay `UserError` — and crucially the error response carries **no game payload**, so the partially-mutated state (the dummy die has already been rolled by `bonus_players`) is discarded rather than persisted. This is the finding's recommendation, adopted as written.

**Edge cases:**
- Do NOT "fix" this by paying nothing and continuing: that would silently swallow a shareholder bonus, converting a loud bug into a wrong game state. Erroring out is the correct failure mode.
- `end()` (lib.rs:667-691) is also called from `draw_replacement_tiles`' bag-exhaustion branch (lib.rs:406) and `end_turn` (lib.rs:740); both propagate `Result` already.
- `choose_merger_phase` -> `handle_merge_command` (lib.rs:542) auto-merge path is covered by the same `?`.
- `bonus_players` stays infallible — it cannot fail, it just may return an empty `major`.
- Message wording matters only for humans; keep it short and non-templated so log greps stay stable.

**Files:**
- Modify: `rust/game/acquire-1/src/lib.rs` (`pay_bonuses` lines 823-875, `end()` line 678, `handle_merge_command` line 805; `mod tests`)

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/game/acquire-1/src/lib.rs`:

```rust
    #[test]
    fn pay_bonuses_with_no_shareholders_errors_instead_of_panicking() {
        // c F9: a corp on the board with no shareholders cannot happen in a
        // valid game, but a deserialized/legacy state can present one. That must
        // surface as GameError::Internal, not kill the serving process.
        // Three players, so no dummy shareholder is injected.
        let mut g: Game = "AA012".into();
        assert_eq!(3, g.players.len());
        assert_eq!(2, g.board.corp_size(Corp::American));
        for p in &mut g.players {
            p.shares.insert(Corp::American, 0);
        }
        match g.pay_bonuses(Corp::American) {
            Err(GameError::Internal { message }) => {
                assert!(message.contains("major bonus"), "unexpected: {}", message)
            }
            Ok(_) => panic!("expected an Internal error for a corp with no shareholders"),
            Err(e) => panic!("expected GameError::Internal, got: {}", e),
        }
    }
```

- [ ] Run: `cargo test -p acquire-1 pay_bonuses_with_no_shareholders` — expected FAIL: the test aborts with the panic `expected some major bonus players`.
- [ ] Implement, in `rust/game/acquire-1/src/lib.rs`:
  1. Change the `pay_bonuses` signature (line 823) to `fn pay_bonuses(&mut self, corp: Corp) -> Result<Vec<Log>, GameError> {`.
  2. Replace the panic block (lines 838-842) with:

```rust
        let major_len = major.len();
        let minor_len = minor.len();
        if major_len == 0 {
            // Unreachable in a valid game: every corp on the board was founded
            // by a player who received a free share, and shares can only be
            // disposed of while the corp is being merged away - after this
            // point. Reachable only from a corrupt/legacy deserialized state.
            return Err(GameError::Internal {
                message: "no major bonus players".to_string(),
            });
        }
```

  3. Change the tail of the function (line 874) from `logs` to `Ok(logs)`.
  4. `end()` (line 678): `logs.extend(self.pay_bonuses(*corp)?);`
  5. `handle_merge_command` (line 805): `logs.extend(self.pay_bonuses(from)?);`
- [ ] Run: `cargo test -p acquire-1` — new test PASSES, all previous tests PASS (`merge_works` walks `handle_merge_command` -> `pay_bonuses` and must be unaffected).
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/lib.rs` ; message: `fix(acquire-1): pay_bonuses returns Internal instead of panicking (c F9, WP-19)`

NOTE: this task shifts `lib.rs` line numbers below ~line 838 by a few lines; Task 4 locates its edits by function name and by the `expect` string.

---

### Task 4: tolerate missing share-map keys everywhere (c F10, minor)

**Problem (restated):** ten `.expect(...)` calls read `HashMap<Corp, usize>` entries that a deserialized state may not contain: lib.rs:683, 1006, 1026, 1076, 1087, 1122, 1139, command.rs:59, command.rs:163 (with the typo `"could not et player shares"`), render.rs:78. Fresh games pre-populate all 7 corp keys (`corp_hash_map`, lib.rs:1224-1230), but serde accepts a `HashMap` with missing keys and this crate already carries a legacy-state shim (lib.rs:156-159). render.rs:78 is on the **render** path, where a panic breaks even reading the game. The crate is already inconsistent: lib.rs:616, 813, 909, 965 and render.rs:138 use `.get(&corp).cloned().unwrap_or(0)` for identical lookups.

**Fix (re-derived):** standardize on `.get(&corp).copied().unwrap_or_default()` (`usize`, so `0`) at all ten sites, as the finding recommends. Each site was individually checked (see the F10 re-derivation notes) to confirm 0 produces a *correct* outcome — a rejection, a skip, or a `"0"` in the render — never a silently wrong payout. Two ordering details make this safe rather than merely quiet: in `take_shares` and `return_shares` the zero-read causes an early `InvalidInput` return *before* the `entry(corp).or_insert(...)` lines that would otherwise resurrect the key with a different value and underflow. The typo is deleted along with its `expect`. Use `.copied()` (not `.cloned()`) at the new sites since `usize: Copy`; leave the pre-existing `.cloned().unwrap_or(0)` sites alone — rewriting them is churn outside the finding.

**Edge cases:**
- command.rs:163 becomes `Int::bounded(1, 0)` for a keyless player. Verified non-panicking: `Int::parse` (`rust/lib/game/src/command/parser/mod.rs:119-175`) evaluates `min` and `max` independently and rejects all input with `"N is too high"`; `to_spec` emits `Spec::Int { min: Some(1), max: Some(0) }`. In a *valid* state this cannot arise: `next_player_sell_trade` (lib.rs:965) skips players holding 0 shares of the merging corp.
- lib.rs:1006 is unreachable-with-missing-key (the preceding `sell()` -> `return_shares` inserts the key via `entry()`), but is standardized anyway so no site is left as the odd one out.
- `player_shares_parser` keeps its `.get(&corp)` on an `Option<&Player>` from `self.players.get(player)`: the whole chain becomes `.and_then(|p| p.shares.get(&corp).copied()).unwrap_or_default()`, which also removes the *player*-index panic risk in the same expression. Do not switch it to `self.players[player]`.
- No serialized type changes; no key is written where one was absent.

**Files:**
- Modify: `rust/game/acquire-1/src/lib.rs` (`end`, `handle_sell_command`, `sell`, `handle_trade_command`, `take_shares`, `return_shares`), `rust/game/acquire-1/src/command.rs` (lines 56-60, 157-164), `rust/game/acquire-1/src/render.rs` (lines 74-79)
- Test: `rust/game/acquire-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/acquire-1/src/lib.rs`:

```rust
    /// c F10 helper: a legacy-shaped state whose share maps are missing the
    /// `American` key entirely, as serde would deserialize from an older save.
    fn game_missing_american_key() -> Game {
        let mut g: Game = "AA01".into();
        for p in &mut g.players {
            p.shares.remove(&Corp::American);
        }
        g.shares.remove(&Corp::American);
        g
    }

    #[test]
    fn missing_share_key_does_not_panic_render_or_spec() {
        // c F10: render.rs and the parser builder must survive a state whose
        // share maps lack a corp key - a panic here makes the game unreadable.
        use brdgme_game::Renderer;

        let mut g = game_missing_american_key();
        let _ = g.pub_state().render();
        let _ = g.player_state(0).render();
        g.phase = Phase::SellOrTrade {
            player: 0,
            corp: Corp::American,
            into: Corp::Festival,
            at: Loc { row: 0, col: 2 },
            turn_player: 0,
        };
        let spec = g.command_spec(0);
        assert!(spec.is_some(), "a spec must still be produced");
    }

    #[test]
    fn missing_share_key_errors_instead_of_panicking() {
        // c F10: share-count reads on a keyless map must behave as zero, which
        // means a clean InvalidInput rejection - never a panic, and never a
        // payout computed from a resurrected default.
        let mut g = game_missing_american_key();
        assert!(g.sell(0, 1, Corp::American).is_err());
        assert!(g.take_shares(0, 1, Corp::American).is_err());
        assert!(g.return_shares(0, 1, Corp::American).is_err());
        g.phase = Phase::SellOrTrade {
            player: 0,
            corp: Corp::American,
            into: Corp::Festival,
            at: Loc { row: 0, col: 2 },
            turn_player: 0,
        };
        assert!(g.handle_trade_command(0, 2).is_err());
        // Now give the player shares but drop the bank key for the target corp,
        // exercising the "into shares" lookup on the trade path.
        g.players[0].shares.insert(Corp::American, 4);
        g.shares.remove(&Corp::Festival);
        assert!(g.handle_trade_command(0, 2).is_err());
        // Bank/player maps must not have gained a resurrected 25-share entry.
        assert_eq!(None, g.shares.get(&Corp::Festival));
    }

    #[test]
    fn end_tolerates_missing_share_keys() {
        // c F10: game-end liquidation walks every player's share map for every
        // corp on the board; one missing key must not abort scoring.
        let mut g = game_missing_american_key();
        g.players[0].shares.insert(Corp::American, 3);
        // player 1 still has no American key at all.
        let logs = g.end().expect("game end must not panic or error");
        assert!(!logs.is_empty(), "ending the game must log bonus payouts");
        assert!(g.finished);
    }
```

- [ ] Run: `cargo test -p acquire-1 missing_share_key` and `cargo test -p acquire-1 end_tolerates` — expected FAILURES, each aborting on an `expect` panic: `expected corp to have shares` (render), `could not et player shares` (spec), `could not get player shares` (`sell`/trade/`end`), `could not get corp share count` (`take_shares`), `could not get player share count` (`return_shares`).
- [ ] Implement — replace each `expect` with the standard form. In `rust/game/acquire-1/src/lib.rs`:
  1. `end()` (the `p_shares` binding, was lines 680-683):

```rust
                    let p_shares = self.players[player]
                        .shares
                        .get(corp)
                        .copied()
                        .unwrap_or_default();
```

  2. `handle_sell_command` (was lines 1003-1007):

```rust
        if self.players[player]
            .shares
            .get(&corp)
            .copied()
            .unwrap_or_default()
            == 0
        {
```

  3. `sell()` (was lines 1023-1026):

```rust
        let player_shares = self.players[player]
            .shares
            .get(&corp)
            .copied()
            .unwrap_or_default();
```

  4. `handle_trade_command` (was lines 1072-1076): `let corp_shares = self.players[player].shares.get(&corp).copied().unwrap_or_default();`
  5. `handle_trade_command` (was lines 1083-1087): `let into_shares = self.shares.get(&into).copied().unwrap_or_default();`
  6. `take_shares` (was lines 1119-1122): `let corp_shares = self.shares.get(&corp).copied().unwrap_or_default();`
  7. `return_shares` (was lines 1136-1139): `let player_shares = self.players[player].shares.get(&corp).copied().unwrap_or_default();`
  
  In `rust/game/acquire-1/src/command.rs`:
  
  8. Lines 56-60 become:

```rust
                    if self.players[player]
                        .shares
                        .get(&corp)
                        .copied()
                        .unwrap_or_default()
                        >= 2
                    {
```

  9. `player_shares_parser` (lines 157-165) becomes:

```rust
    fn player_shares_parser(&self, player: usize, corp: Corp) -> impl Parser<T = i32> {
        Int::bounded(
            1,
            self.players
                .get(player)
                .and_then(|p| p.shares.get(&corp).copied())
                .unwrap_or_default() as i32,
        )
    }
```

  In `rust/game/acquire-1/src/render.rs`:
  
  10. Lines 74-80 — the `"{} left"` cell becomes `self.shares.get(c).copied().unwrap_or_default()`.
- [ ] Run: `cargo test -p acquire-1` — the four new tests PASS, all previous tests PASS.
- [ ] Confirm no `expect(` remains on a share lookup: `grep -n "expect(\"c\|expect(\"e" rust/game/acquire-1/src/*.rs` must return nothing.
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/lib.rs rust/game/acquire-1/src/command.rs rust/game/acquire-1/src/render.rs` ; message: `fix(acquire-1): treat missing share-map keys as zero instead of panicking (c F10, WP-19)`

---

### Task 5: the `Trades` stat reports trades (c F11, minor)

**Problem (restated):** stats.rs:46 is `s.insert("Trades".to_string(), Stat::Int(self.merges as i32));` — a copy-paste of the `Merges` line above it (stats.rs:45). `Stats::trades` is maintained (lib.rs:1095) but never surfaced correctly.

**Fix (re-derived):** use `self.trades`. Confirmed as recommended. Scope discipline: this fixes the wrong field **only**. Wiring `to_brdgme_stats()` into `status()` (or deleting the whole stats module) is c F12, owned by WP-20 — do not touch lib.rs:238.

**Edge cases:**
- `Stats::trades` counts shares *received* (`receive = n / 2`, lib.rs:1095), not trade actions. That is a defensible definition and matching the label "Trades" to it is not this package's call; the test asserts the field is plumbed through, not what the field means.
- `stats.rs` has no test module today; add one. `Stats` is `pub` inside the private `mod stats`, so the test must live in-crate (inline in `stats.rs` is the tightest placement).
- `Stat` derives `PartialEq` + `Debug` (`rust/lib/game/src/game.rs:12-18`), so `assert_eq!` on the map value works.

**Files:**
- Modify: `rust/game/acquire-1/src/stats.rs` (line 46; new inline `mod tests`)

**Steps:**

- [ ] Write the failing test. Append to `rust/game/acquire-1/src/stats.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trades_and_merges_are_reported_separately() {
        // c F11: stats.rs:46 was a copy-paste of the Merges line, so the Trades
        // stat reported the merge count.
        let stats = Stats {
            merges: 3,
            trades: 7,
            ..Stats::default()
        };
        let s = stats.to_brdgme_stats();
        assert_eq!(Some(&Stat::Int(3)), s.get("Merges"));
        assert_eq!(Some(&Stat::Int(7)), s.get("Trades"));
    }
}
```

- [ ] Run: `cargo test -p acquire-1 trades_and_merges` — expected FAIL: `Trades` is `Stat::Int(3)`.
- [ ] Implement: in `rust/game/acquire-1/src/stats.rs` line 46, change `Stat::Int(self.merges as i32)` to `Stat::Int(self.trades as i32)`.
- [ ] Run: `cargo test -p acquire-1` — new test PASSES, all previous tests PASS.
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/stats.rs` ; message: `fix(acquire-1): Trades stat reports trades, not merges (c F11, WP-19)`

---

### Task 6: remove the unused `thiserror` dependency (c F16, minor)

**Problem (restated):** `Cargo.toml:14` declares `thiserror = "2.0.18"`. Nothing in the crate uses it — no `use thiserror`, no `#[derive(Error)]`, in `src/`, `src/bin/` or `tests/`; the crate returns `brdgme_game::errors::GameError` throughout. It is also outside the standard game-crate dependency set.

**Fix:** delete the line.

**Edge cases:** none. `cargo build -p acquire-1` proves no non-test use and `cargo test -p acquire-1` proves no test use. `Cargo.lock` loses this crate's `thiserror` edge (the package itself stays in the lock file if any other workspace member uses it) — include `rust/Cargo.lock` in the commit if it changed.

**Files:**
- Modify: `rust/game/acquire-1/Cargo.toml` (line 14), possibly `rust/Cargo.lock`

**Steps:**

- [ ] Delete `thiserror = "2.0.18"` from `[dependencies]` in `rust/game/acquire-1/Cargo.toml`.
- [ ] Run: `cargo build -p acquire-1` — compiles. Then `cargo test -p acquire-1` — all tests PASS.
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/Cargo.toml rust/Cargo.lock` ; message: `refactor(acquire-1): drop unused thiserror dependency (c F16, WP-19)`

---

### Task 7: compute `can_end` without cloning the game (c F20, nit)

**Problem (restated):** `player_can_end` (lib.rs:1200-1202) and `handle_end_command` (lib.rs:1184) call `self.pub_state().can_end()`, and `pub_state()` is `self.to_owned().into()` (lib.rs:258-260) — a deep clone of the entire `Game` (board vec, per-player share maps, hands, stats, bank map, RNG) thrown away immediately. `player_can_end` runs inside `command_parser` (command.rs:67), which is rebuilt on every `command()` and every `command_spec()`; `renders()` (`rust/lib/cmd/src/requester/gamer.rs:70-77`) calls `command_spec` once per seat per request, so a 6-player game pays 6+ whole-game clones per request. `PubState::can_end` (lib.rs:102-135) needs only `board`, `finished` and `last_turn`.

**Fix (re-derived):** extract the body into a free function over the three inputs and have both `PubState::can_end` and a new `Game::can_end` delegate to it, as the finding recommends. Equivalence is exact because `From<Game> for PubState` (lib.rs:1232-1243) moves `board`, `last_turn` and `finished` across verbatim. `PubState::can_end` stays `pub` — render.rs:30 calls it on a `PubState` that the harness has already built.

**Edge cases:**
- Do **not** delete or narrow `PubState::can_end`; it is part of the render path and (being on a `pub` type) is effectively public API for the view layer.
- `Game::can_end` is a private helper — no new public surface, no serialized change.
- The `CanEnd`/`CanEndFalse` types and `PartialEq` comparisons in `handle_end_command`/`player_can_end` are unchanged, so behavior is bit-identical; the only observable difference is the absent allocation.
- Keep the free function private to the crate and place it next to `CanEndFalse` (lib.rs:343-362) so the end-condition logic stays in one region of the file.

**Files:**
- Modify: `rust/game/acquire-1/src/lib.rs` (`PubState::can_end` lines 102-135, new free fn, `handle_end_command` line 1184, `player_can_end` lines 1200-1202; `mod tests`)

**Steps:**

- [ ] Write the equivalence test (this is a refactor with no observable behavior change; the test locks the equivalence rather than driving a fix). Add to `mod tests` in `rust/game/acquire-1/src/lib.rs`:

```rust
    #[test]
    fn game_can_end_matches_pub_state_can_end() {
        // c F20: Game::can_end must stay exactly equivalent to the PubState
        // view's answer - it exists only to avoid cloning the whole game.
        let mut g: Game = "AA01".into();
        assert_eq!(g.pub_state().can_end(), g.can_end());
        g.last_turn = true;
        assert_eq!(g.pub_state().can_end(), g.can_end());
        g.finished = true;
        assert_eq!(g.pub_state().can_end(), g.can_end());
    }
```

- [ ] Run: `cargo test -p acquire-1 game_can_end_matches` — expected FAIL to COMPILE (`no method named can_end found for struct Game`). That compile failure is the red state for this refactor.
- [ ] Implement, in `rust/game/acquire-1/src/lib.rs`:
  1. Replace `impl PubState { pub fn can_end(&self) -> CanEnd { ... } }` (lines 102-135) with:

```rust
impl PubState {
    pub fn can_end(&self) -> CanEnd {
        can_end(&self.board, self.finished, self.last_turn)
    }
}
```

  2. Add the extracted logic as a private free function immediately after the `CanEnd` enum (after line 362), body copied verbatim from the old method with `self.board` -> `board`, `self.finished` -> `finished`, `self.last_turn` -> `last_turn`:

```rust
/// Shared end-condition check. Takes the three inputs it actually needs so
/// `Game` can answer without cloning itself into a `PubState` (c F20).
fn can_end(board: &Board, finished: bool, last_turn: bool) -> CanEnd {
    if finished {
        return CanEnd::Finished;
    }
    if last_turn {
        return CanEnd::Triggered;
    }
    let mut largest: usize = 0;
    let mut has_safe: bool = false;
    let mut unsafe_count: usize = 0;
    for corp in Corp::iter() {
        let size = board.corp_size(*corp);
        if size > largest {
            largest = size;
        }
        if size >= corp::SAFE_SIZE {
            has_safe = true;
        }
        if size > 0 && size < corp::SAFE_SIZE {
            unsafe_count += 1;
        }
    }
    if largest >= corp::GAME_END_SIZE || has_safe && unsafe_count == 0 {
        return CanEnd::True;
    }
    CanEndFalse {
        largest,
        has_safe,
        unsafe_count,
    }
    .into()
}
```

  3. Add to `impl Game` (next to `player_can_end`):

```rust
    fn can_end(&self) -> CanEnd {
        can_end(&self.board, self.finished, self.last_turn)
    }
```

  4. `handle_end_command` (line 1184): `if self.can_end() != CanEnd::True {`
  5. `player_can_end` (lines 1200-1202): `self.phase.main_turn_player() == player && self.can_end() == CanEnd::True`
- [ ] Run: `cargo test -p acquire-1` — new test PASSES, all previous tests PASS.
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/lib.rs` ; message: `perf(acquire-1): compute can_end without cloning the game (c F20, WP-19)`

---

### Task 8: deterministic corporation ordering in the found and merge parsers (c F21, nit)

**Problem (restated):** command.rs:34 pipes `self.board.available_corps()` — a `HashSet<Corp>` (board.rs:55-63) — into `Enum::partial`, so the `Spec::Enum { values }` order of foundable corporations varies. `std`'s `RandomState` is per-instance and the set is rebuilt on every call, so the order differs **between two calls in the same process**: measured 50 distinct `Spec` debug strings from 50 consecutive `command_spec(0)` calls on a `Phase::Found` state. Suggestions and the command spec sent to clients (and to the bot) therefore shuffle between requests. The finding cites only command.rs:34; command.rs:43-52 has the identical defect, feeding `neighbouring_corps(&at)` (also a `HashSet<Corp>`, board.rs:65-73) into the merge parser's two `Enum::partial` calls.

**Fix (re-derived):** filter the canonical `CORPS` array (corp.rs:24-32, already imported at command.rs:7) by membership in the set, producing a `Vec<Corp>` in canonical order. Transient local only: no serialized type changes, `available_corps`/`neighbouring_corps` keep returning `HashSet<Corp>` (their other callers — lib.rs:463-511, 564, 481, board.rs:85, 158 — are membership/length tests that do not care about order). Both sites are fixed; leaving the merge parser random while fixing the found parser would be an arbitrary half-fix of one defect.

**Edge cases:**
- `available_corps()` empty: cannot reach the `Phase::Found` arm (`handle_play_command` rejects founding with no corps available, lib.rs:481-485), and `Enum::partial(vec![])` is not newly reachable through this change.
- Post-fix order is `CORPS` order: Worldwide, Sackson, Festival, Imperial, American, Continental, Tower. That is the same order `corp_table` (render.rs:65) and the player table header (render.rs:119) use, so the spec now matches the UI's column order — a small usability win, and the reason to sort by `CORPS` rather than alphabetically.
- `Enum::partial` prefix matching is order-sensitive only in *presentation*; parse results are unchanged for every input (the corp set is identical, only its sequence changes).
- `merge_parser` takes `&[Corp]` and builds two `Enum::partial(corps.to_owned())` calls from the same slice, so one sorted vec fixes both.

**Files:**
- Modify: `rust/game/acquire-1/src/command.rs` (`command_parser`, lines 32-53)
- Test: `rust/game/acquire-1/src/lib.rs`, inline `mod tests`

**Steps:**

- [ ] Write the failing tests. Add to `mod tests` in `rust/game/acquire-1/src/lib.rs`:

```rust
    #[test]
    fn found_parser_corp_order_is_canonical_and_stable() {
        // c F21: available_corps() is a HashSet, so its iteration order - and
        // hence the command spec sent to clients - varied per call, even within
        // one process. Order must be the canonical CORPS order.
        let players = vec!["mick".to_string(), "steve".to_string()];
        let mut g: Game = "...
                           #0.
                           ..."
        .into();
        g.command(0, "play b2", &players)
            .expect("expected playing tile to work");
        assert!(matches!(g.phase, Phase::Found { .. }));
        let first = format!("{:?}", g.command_spec(0));
        for _ in 0..50 {
            assert_eq!(
                first,
                format!("{:?}", g.command_spec(0)),
                "command spec must not vary between builds"
            );
        }
        for name in ["Worldwide", "Sackson", "Festival"] {
            assert!(first.contains(name), "spec should list {}: {}", name, first);
        }
        assert!(
            first.find("Worldwide") < first.find("Sackson")
                && first.find("Sackson") < first.find("Festival"),
            "corps must be listed in CORPS order: {}",
            first
        );
    }

    #[test]
    fn merge_parser_corp_order_is_stable() {
        // c F21 (same defect, second site): neighbouring_corps() is also a
        // HashSet feeding Enum::partial for the merge parser.
        let players = vec!["mick".to_string(), "steve".to_string()];
        let mut g: Game = "FF0
                           ..A
                           ..A"
        .into();
        g.command(0, "play a3", &players)
            .expect("expected 'play a3' to work");
        assert!(matches!(g.phase, Phase::ChooseMerger { .. }));
        let first = format!("{:?}", g.command_spec(0));
        for _ in 0..50 {
            assert_eq!(
                first,
                format!("{:?}", g.command_spec(0)),
                "merge spec must not vary between builds"
            );
        }
        assert!(
            first.find("Festival") < first.find("American"),
            "corps must be listed in CORPS order: {}",
            first
        );
    }
```

- [ ] Run: `cargo test -p acquire-1 parser_corp_order` — expected FAIL on the `assert_eq!` inside the 50-iteration loop (measured: 50/50 distinct orders pre-fix, so the red run is reliable, not probabilistic; if it ever passes, re-run — do NOT skip the red confirmation).
- [ ] Implement, in `rust/game/acquire-1/src/command.rs`:
  1. The `Phase::Found` arm (lines 32-36) becomes:

```rust
                Phase::Found { .. } => {
                    // CORPS order, not HashSet order: the spec is sent to
                    // clients and bots, and must not shuffle per request.
                    let available = self.board.available_corps();
                    parsers.push(Box::new(self.found_parser(
                        CORPS.iter().copied().filter(|c| available.contains(c)).collect(),
                    )));
                }
```

  2. The `Phase::ChooseMerger` arm (lines 43-53) becomes:

```rust
                Phase::ChooseMerger { at, .. } => {
                    let neighbouring = self.board.neighbouring_corps(&at);
                    let corps: Vec<Corp> = CORPS
                        .iter()
                        .copied()
                        .filter(|c| neighbouring.contains(c))
                        .collect();
                    parsers.push(Box::new(self.merge_parser(&corps)));
                }
```

- [ ] Run: `cargo test -p acquire-1` — both new tests PASS, all previous tests PASS (`merge_works` issues `merge am into fe`, which parses regardless of ordering).
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/acquire-1/src/command.rs` ; message: `fix(acquire-1): canonical corp ordering in found and merge parsers (c F21, WP-19)`

---

### Task 9: remove the remaining panic operators and the tautology (c F17 + c F18 + c F19, nits) + final gate

**Problem (restated):**
- c F18 — lib.rs:466 `neighbouring_corps.iter().next().unwrap()` in the `1 =>` arm of `handle_play_command`. Safe by construction (the arm is guarded on `len() == 1`), but `.unwrap()` sits on a path reached directly from `play <tile>`.
- c F19 — render.rs:263-271 calls `start.unwrap()` twice in the corp-name width scan. Safe by construction (`start` is set to `Some(col)` earlier in the same branch), but it is on the render path.
- c F17 — lib.rs:586 returns `matches!(self.phase, Phase::Buy { .. })` right after `buy_phase(player)` unconditionally set `Phase::Buy` (lib.rs:516-521). Always `true`; the expression reads as a meaningful check and is not one.

**Fix (re-derived):** three mechanical, behavior-identical edits:
1. F18: `let Some(n_corp) = neighbouring_corps.iter().copied().next() else { return Err(GameError::Internal { .. }) };` — `Corp: Copy`, so this also removes the later `*n_corp` derefs. `GameError::Internal` (not `InvalidInput`): reaching it would mean `HashSet::len() == 1` disagreed with `iter().next()`, which is an engine bug, not player error.
2. F19: `let s = *start.get_or_insert(col);` replaces the `if start.is_none() { start = Some(col); }` + two `unwrap()`s. Same value, no panic operator, one fewer branch.
3. F17: return `true` with a comment naming the invariant. `true` is the correct value on its own merits — founding consumes no randomness and reveals nothing hidden, so it is undoable.

**Edge cases:**
- F18: the `1 =>` arm then uses `n_corp` by value in `extend_corp(loc, n_corp)`, `n_corp.render()` and `corp_size(n_corp)` — drop the `*` sigils at lib.rs:467-471. Behavior with a 1-element set is unchanged.
- F19: `get_or_insert` returns `&mut usize`; dereference immediately so `start` is not borrowed across the `if` (the closure also assigns `start = None` in the `_ =>` arm — that arm is untouched). The emitted `(x, y, w)` tuples are byte-identical, so the rendered canvas is unchanged; `found_works`/`merge_works` render nothing, but `game_contract` renders every state it builds and covers this path.
- F17: `handle_found_command`'s `Ok` tuple otherwise unchanged; no caller inspects the discriminant differently.
- No test accompanies these three: F18 and F19 are unreachable-by-construction (a test would have to construct a state the type system already excludes), and F17 changes no value. The existing suite covers all three code paths (`play_works`/`found_works` walk the F18 arm and F17's return; `game_contract` renders through F19).

**Files:**
- Modify: `rust/game/acquire-1/src/lib.rs` (`handle_play_command` lines 465-472, `handle_found_command` line 586 — locate by symbol after Tasks 3/4/7), `rust/game/acquire-1/src/render.rs` (lines 259-284)

**Steps:**

- [ ] F18 — in `handle_play_command`, replace the `1 =>` arm head (lines 465-472):

```rust
            1 => {
                let Some(n_corp) = neighbouring_corps.iter().copied().next() else {
                    // Unreachable: this arm is guarded on len() == 1.
                    return Err(GameError::Internal {
                        message: "neighbouring corp set was unexpectedly empty".to_string(),
                    });
                };
                self.board.extend_corp(loc, n_corp);
                logs.push(Log::public(vec![
                    n_corp.render(),
                    N::text(" increased in size to "),
                    N::Bold(vec![N::text(format!("{}", self.board.corp_size(n_corp)))]),
                ]));
                self.buy_phase(player);
            }
```

- [ ] F19 — in `rust/game/acquire-1/src/render.rs`, replace the `Tile::Corp` match arm of the width scan (lines 261-275) with:

```rust
                                    match self.get_tile(l) {
                                        Tile::Corp(tc) if tc == *c => {
                                            let s = *start.get_or_insert(col);
                                            if col == board::WIDTH - 1 {
                                                Some((s, row, col - s + 1))
                                            } else {
                                                None
                                            }
                                        }
```

  (the `_ =>` arm at lines 276-283 is unchanged.)
- [ ] F17 — in `handle_found_command`, replace the final `matches!(self.phase, Phase::Buy { .. })` (line 586) with:

```rust
            // buy_phase() above unconditionally sets Phase::Buy, and founding
            // consumes no randomness and reveals nothing hidden, so this is
            // always undoable.
            true,
```

- [ ] Run: `cargo test -p acquire-1` — all tests PASS (no new tests; see edge cases for why).
- [ ] Confirm no panic operators remain outside `#[cfg(test)]` code: `grep -n "unwrap()\|expect(\|panic!\|unreachable!\|unimplemented!" rust/game/acquire-1/src/*.rs` — the only expected remaining hits are the two `panic!("must be Phase::SellOrTrade")` sites (lib.rs, `next_player_sell_trade` and `end_sell_trade_phase`), which are **out of scope** (see "Cross-package / newly discovered"), plus any hits inside `#[cfg(test)]` blocks. If anything else remains, STOP and report.
- [ ] `cargo clippy -p acquire-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final package commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add rust/game/acquire-1/src/lib.rs rust/game/acquire-1/src/render.rs` ; message: `refactor(acquire-1): drop unwraps on play and render paths, simplify can_undo (c F17 c F18 c F19, WP-19)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| c F7 `player_counts()` excludes 6 | major | Change to `(MIN_PLAYERS..=MAX_PLAYERS).collect()` | CONFIRMED | Re-derived end to end: `(2..6)` yields `[2,3,4,5]` (lib.rs:313) while `MAX_PLAYERS = 6` and `start()` accepts 6; the value is served by `gamer.rs:56-58`, persisted to `game_types.player_counts` and drives the new-game UI's filter and selector (`web/src/new_game.rs:71,244,470`). Capacity independently verified (42/108 tiles at 6p) and a throwaway test confirmed `start(2..=6)` plus every seat's `player_state`/`command_spec` work. Resulting set `[2,3,4,5,6]`. **Separable from WP-20** — `MIN_PLAYERS`/`MAX_PLAYERS`, `start()`'s bounds and RULES.md are all untouched, so no edition/player-cap decision is presupposed (Task 1). |
| c F8 dummy die never rolls 6 | major | `self.rng.random_range(1..=6)` | CONFIRMED | `random_range(1..=5)` at lib.rs:902 contradicts the crate's own RULES.md:151-155 and its own start log (lib.rs:219-224) — two in-repo sources, so no rules adjudication is needed. Fix is the recommended one; test is seeded (`ChaCha8Rng`, seed 42, 1000 draws) so it is deterministic rather than statistical (Task 2). |
| c F9 `panic!` in `pay_bonuses` | minor | Return `Result` with `GameError::Internal` | CONFIRMED | Adopted as written. Reachability re-derived: both call sites (lib.rs:678, 805) are command-reachable, but `major` cannot be empty under game invariants (founder free share + shares only disposable while the corp is merged away, after bonuses are paid) — so **latent**, reachable only from a legacy/corrupt deserialized state. Error-path mutation is safe because `gamer.rs:88-107` returns no game payload on `Err` (Task 3). |
| c F10 `expect()` cluster on `HashMap` keys | minor | Standardize on `.get(&corp).copied().unwrap_or_default()`; fix the typo | CONFIRMED | All 10 sites verified live and each individually re-derived to show 0 yields a correct rejection/skip/`"0"` and never a silent wrong payout; in `take_shares`/`return_shares` the zero-read returns *before* the `entry().or_insert()` lines that would resurrect the key and underflow. `Int::bounded(1, 0)` (command.rs:163 with a missing key) confirmed non-panicking against `parser/mod.rs:119-175`. Typo removed with its `expect` (Task 4). |
| c F11 `Trades` stat reports merges | minor | Use `self.trades as i32` | CONFIRMED | One-token fix at stats.rs:46, plus the crate's first `stats.rs` test. Explicitly does NOT wire `to_brdgme_stats` into `status()` — that is c F12/WP-20 (Task 5). |
| c F12 stats tracked but never surfaced | minor | Wire into `status()` or delete | OUT OF SCOPE | WP-20 owns the keep-or-drop decision. Task 5 edits one line inside the dead machinery and nothing else. |
| c F13 random start player vs tile draw | minor | Derive from placed tiles, or document the deviation | OUT OF SCOPE | WP-20 (acquire edition trio, BLOCKED-ON-DECISION D-30/D-31/D-40). `start()`'s start-player selection untouched. |
| c F14 full-hand redraw discards temp-unplayable tiles | minor | Confirm edition; discard only permanently-unplayable, or document | OUT OF SCOPE | WP-20. `start_turn`/`redraw_hand`/`assert_loc_playable` untouched. |
| c F15 bag exhaustion ends the game | minor | Verify against the chosen edition; document | OUT OF SCOPE | WP-20. `draw_replacement_tiles`' end trigger untouched. |
| c F16 unused `thiserror` dependency | minor | Remove the dependency | CONFIRMED | Re-verified: the only match in the whole crate (excluding `Cargo.lock`) is `Cargo.toml:14`; the crate defines no error type (Task 6). |
| c F17 `can_undo` tautology | nit | Return `true` (or restructure so the value is meaningful) | CONFIRMED (first option) | `buy_phase` unconditionally assigns `Phase::Buy` (lib.rs:516-521), so the `matches!` is always `true`; `true` is also correct on the merits (no randomness, no hidden info revealed), so restructuring would add machinery for nothing. Comment records the invariant (Task 9). |
| c F18 `unwrap()` on 1-element set | nit | Fallible extraction returning `GameError::Internal` | CONFIRMED | Implemented as `let ... else` with `GameError::Internal`; `.copied()` also removes the downstream `*n_corp` derefs. `Internal` rather than `InvalidInput` because reaching it means `len()` disagreed with `iter().next()` (Task 9). |
| c F19 `unwrap()` in board render row-run | nit | Restructure with `if let` | ADJUSTED | Same intent, better mechanics: `let s = *start.get_or_insert(col);` replaces the `is_none()` set + two `unwrap()`s in one line, whereas the suggested `if let Some(s) = start` would need the `is_none()` pre-assignment kept *and* a fallback arm for the impossible `None`. Emitted tuples are byte-identical (Task 9). |
| c F20 full-game clone for `can_end` | nit | Move `can_end` to a shared helper over `(&Board, finished, last_turn)` and call it from `Game` | CONFIRMED | Implemented exactly as recommended; cost re-derived as worse than stated — `command_spec` is called once per seat per request by `renders()` (`gamer.rs:70-77`), so a 6-player game pays 6+ full-game clones per request. `PubState::can_end` stays `pub` for render.rs:30 (Task 7). |
| c F21 nondeterministic corp order in found parser | nit | Sort by `CORPS` order before building the parser | ADJUSTED (scope extended) | Sorting by `CORPS` adopted. Two adjustments: (a) severity of the *symptom* is worse than "varies run to run" — measured 50/50 distinct specs from 50 consecutive calls in ONE process, because `available_corps()` builds a fresh `HashSet` per call; (b) the fix is applied to the merge parser at command.rs:43-52 as well, which has the identical `HashSet<Corp>` -> `Enum::partial` defect but was not cited (Task 8). |

## Test plan summary

Run everything from `/home/beefsack/Development/brdgme/rust`.

- Baseline (measured 2026-07-25): `cargo test -p acquire-1` = **11 passing** (10 lib unit tests + `tests/contract.rs::game_contract`), `cargo clippy -p acquire-1 --all-targets -- -D warnings` clean, `cargo fmt --all -- --check` clean.
- New tests, 12 in total, all inline (`mod tests` in `src/lib.rs` unless noted) — final expected count **23 passing**:
  1. `player_counts_covers_min_to_max_players` (Task 1) — red: advertised set is `[2,3,4,5]`.
  2. `dummy_shareholder_rolls_a_full_d6` (Task 2) — red: face 6 absent from 1000 seeded rolls.
  3. `pay_bonuses_with_no_shareholders_errors_instead_of_panicking` (Task 3) — red: `panic!("expected some major bonus players")`.
  4. `missing_share_key_does_not_panic_render_or_spec` (Task 4) — red: `expect("expected corp to have shares")` / `expect("could not et player shares")`.
  5. `missing_share_key_errors_instead_of_panicking` (Task 4) — red: the `sell`/`take_shares`/`return_shares`/trade `expect`s.
  6. `end_tolerates_missing_share_keys` (Task 4) — red: `expect("could not get player shares")` in `end()`.
  7. `game_missing_american_key` (Task 4) — helper, not a test.
  8. `trades_and_merges_are_reported_separately` (Task 5, in `src/stats.rs`) — red: `Trades` is `Int(3)`.
  9. `game_can_end_matches_pub_state_can_end` (Task 7) — red: fails to compile until `Game::can_end` exists.
  10. `found_parser_corp_order_is_canonical_and_stable` (Task 8) — red: spec differs between builds.
  11. `merge_parser_corp_order_is_stable` (Task 8) — red: same.
  (Tasks 6 and 9 add no tests — dependency removal and unreachable-by-construction/no-op changes; both are covered by the existing suite plus `cargo build -p acquire-1`.)
- Per-task gate: `cargo test -p acquire-1` then `cargo clippy -p acquire-1 --all-targets -- -D warnings` then `cargo fmt --all -- --check`.
- Targeted runs: `cargo test -p acquire-1 player_counts_covers`, `... dummy_shareholder_rolls`, `... pay_bonuses_with_no_shareholders`, `... missing_share_key`, `... end_tolerates`, `... trades_and_merges`, `... game_can_end_matches`, `... parser_corp_order`.
- Package gate before the final commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh`.
- NEVER run workspace-wide `cargo build`/`cargo test`/`cargo clippy` on a dev machine (AGENTS.md).

## Cross-package coordination points

- **WP-20 (BLOCKED-ON-DECISION D-30/D-31/D-40)** owns the acquire edition trio (c F13 start-player draw, c F14 full-hand redraw, c F15 bag exhaustion) and the stats keep-or-drop question (c F12), and will edit `src/lib.rs`, `src/board.rs`, `src/stats.rs` in this crate. Conflict surface: Task 3 and Task 4 both touch `end()` and Task 4 touches `sell`/`take_shares`/`return_shares`; WP-20's F15 work touches `draw_replacement_tiles` (lib.rs:403-408) and F14 touches `start_turn`/`redraw_hand` — adjacent functions, no shared lines. Task 5 changes stats.rs:46 only; if WP-20 decides "delete the stats machinery", Task 5's line and its test vanish with the module, which is fine. WP-19 should land first (it is READY, WP-20 is blocked).
- **WP-08 (finish/placings epilogue dedup sweep)** does not list acquire-1, but acquire-1 has the same copy-pasted epilogue shape in `command()`'s `Command::Done` arm (lib.rs:286-294: `is_finished()` -> per-player scores -> `placings_log`). Whoever writes WP-08's spec should decide whether acquire-1 joins that sweep; nothing in WP-19 touches that block, so either answer rebases trivially.
- **WP-09 (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36)** covers the systemic "deserialized state trusted verbatim" class. Task 4 is the acquire-specific instance for share maps and is filed as its own finding (c F10), so it lands here; it does not pre-empt WP-09's requester-boundary decision, and a bounds check at `gamer.rs` would be complementary. For the record, acquire-1 needs **no** `player_state` bounds fix of the e F18/e F36 kind: `player_state` (lib.rs:262-268) indexes `self.players[player]`, but `renders()` only ever calls it with `0..player_count()` (`gamer.rs:70-77`), and `command()` rejects out-of-range players before any indexing (`command_parser` returns `None` because `phase.whose_turn() != player`, command.rs:27).
- **WP-03 (lib-game parser mechanical fixes)** touches `rust/lib/game/src/command/{parser,suggest,doc}`. Task 8's tests assert on `Spec` debug output; if WP-03 changes `Spec`'s shape or `Doc` rendering, those two assertions may need their substring checks refreshed (the ordering property they test is unaffected).
- **Workspace manifest hygiene (WP-64/WP-65)** may also edit `game/acquire-1/Cargo.toml`. Task 6 removes `thiserror` first; note that `tokio = { features = ["full"] }` (Cargo.toml:17) and `brdgme_fuzz` as a non-dev dependency (Cargo.toml:10) are part of the standard game-crate template here and are deliberately left alone by this package.

## Cross-package / newly discovered

Found while writing this spec. **None of these are fixed by this package** (except where noted), and none is baked into a test.

1. **Two `panic!("must be Phase::SellOrTrade")` sites — lib.rs:951 (`next_player_sell_trade`) and lib.rs:980 (`end_sell_trade_phase`).** Same class as c F9 (panic macro in a function reachable from `Gamer::command` via `sell`/`trade`/`keep`/`merge`) but not filed by the review. Both are provably unreachable today: every caller either has just matched `Phase::SellOrTrade` itself (`handle_sell_command` lib.rs:994, `handle_trade_command` lib.rs:1054, `handle_keep_command` lib.rs:1155) or has just assigned that phase (`handle_merge_command` lib.rs:806-812), and `end_sell_trade_phase` is called only from `next_player_sell_trade` at lib.rs:956 *before* the phase is reassigned. A legacy/corrupt state cannot reach them either, because the phase is re-matched on entry to every handler. **RESOLVED by the unit-3 Lead: ROUTED TO WP-09 (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36) as an added item; do NOT fold it into WP-19.** Rationale: WP-19 is a fixed-scope package of filed findings, and quietly absorbing an unfiled fix is exactly the drift this review's spec discipline forbids; WP-09 is the package whose whole subject is "internal invariants that only a bad state can violate", and it already converts this class of panic across ~12 game crates. Note for WP-09's spec writer: `acquire-1` is **not** currently in WP-09's crate list (work-packages.md WP-09 paths) and must be added. The change itself is ~6 lines mirroring this spec's Task 3 (`GameError::Internal`), and Task 9's panic-grep step here deliberately whitelists these two sites so the two packages do not fight.
2. **`RULES.md:60` documents a tertiary bonus that does not exist:** "In Tycoon Mode (3+ players), a tertiary bonus is also paid." The implementation pays only major and minor bonuses (`pay_bonuses`, lib.rs:823-875; `MINOR_MULT`/`MAJOR_MULT` only, corp.rs:10-11) and there is no mode selection anywhere in the crate. So either the doc overstates the implementation or a rule is missing. This is an **edition/rules question in exactly WP-20's territory** (the acquire edition trio) and should be added to that package's scope; it must not be "fixed" by silently editing RULES.md here, because which way it resolves (implement the bonus vs delete the sentence) is a rules decision.
3. **`command.rs:43-52` merge-parser corp ordering** — the same `HashSet<Corp>` -> `Enum::partial` nondeterminism as c F21 but at an uncited location. This one **is** fixed, in Task 8, and is called out in the disposition table as a deliberate scope extension of F21 rather than a silent change.
4. **`use std::iter::FromIterator;` at board.rs:3** is redundant under the edition-2021+ prelude (its only user is `HashSet::from_iter` at board.rs:56). Cosmetic; identical to WP-22's filed `d F9` for lords-of-vegas-1 but not filed for acquire-1, and clippy at `-D warnings` does not flag it. Left untouched; route to workspace hygiene (WP-64/WP-65) if anyone wants the sweep.
