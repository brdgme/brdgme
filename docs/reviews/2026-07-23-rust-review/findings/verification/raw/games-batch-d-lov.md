# Verification: games-batch-d, crate game/lords-of-vegas-1 (snapshot f8763a5)

Worker W1. Snapshot root: /home/beefsack/Development/brdgme-review-snapshot/rust/game/lords-of-vegas-1

Go source check: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go exists but contains no lords_of_vegas (or vegas-named) directory. All game-rule claims judged against the crate's own RULES.md and in-code comments; external-rules claims flagged.

## F1 unimplemented!() arms in command()
- verdict: CONFIRMED
- evidence: lib.rs:182-186 has `Command::Remodel {..} => unimplemented!()` and same for Reorg, Sprawl, Gamble, Raise. command.rs:20-27 `command_parser()` only pushes `build_parser` (guarded by can_build) and `done_parser` (can_done); no other parser is wired, so those Command variants can never be produced today. The five other parsers are fully written and `pub` at command.rs:49-152 (sprawl_parser:49, remodel_action:72, reorg_parser:95, gamble_parser:113, raise_parser:136). Wiring any into command_parser would make a syntactically valid player command hit unimplemented!() and panic, violating the repo convention (no panic macros in runtime paths reachable from player commands).
- severity: major/quality upheld. Latent but one-line-change away from a player-triggerable panic; the pub-but-unwired parsers are a footgun.

## F2 nondeterministic iteration order feeds seeded RNG
- verdict: CONFIRMED
- evidence: board.rs:248 `let next = *queue.iter().next().expect(...)` pops from `HashSet<Loc>` (std RandomState: per-process-random order). board.rs:278 `for loc in TILES.keys()` iterates a lazy_static `HashMap` (tile.rs:23-25), also per-process-random order. resolve_boss_ties (board.rs:314-344) walks `self.casinos()` and calls `reroll_at` (each call consumes one `roll(rng)` draw, board.rs:299) per boss tile at board.rs:330-332. Order IS RNG-relevant: (a) the order casinos are visited determines which tied casino's tiles consume which draws; (b) within a casino, `bc.tiles` order comes from the HashSet BFS pop order in casino_at, and boss_tiles preserves it (board.rs:361-376), so the mapping of rolled values to specific tiles varies run-to-run; (c) since rerolled values decide whether the recursive pass (board.rs:337) finds a new tie, even the total number of RNG draws can diverge. Replaying the same seed+commands in a new process can therefore produce different states. Loc derives Ord (board.rs:73 `#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, ...)]`), so sorting candidates / using BTreeMap-style ordering is feasible as the reviewer recommended (loc_parser already does `locs.sort()`, command.rs:155-157).
- severity: major/correctness upheld. Real seeded-replay divergence.

## F3 resolve_boss_ties logs always empty, undo disabled silently
- verdict: CONFIRMED
- evidence: board.rs:316 `let mut logs: Vec<Log> = vec![];` - nothing is ever pushed; the only mutation is `logs.extend(new_logs)` at board.rs:338 from the recursive call, which is itself always empty, so board.rs:340 `Some(logs)` is always `Some(vec![])`. board.rs:331 `self.reroll_at(&bt.loc, rng);` discards the `Option<usize>` rolled value. lib.rs:308-311: `if let Some(resolve_logs) = self.board.resolve_boss_ties(&mut self.rng) { logs.extend(resolve_logs); can_undo = false; }` - so when ties occur, dice are silently rerolled and undo is disabled with no player-visible record of what happened. RULES.md:85-88 explicitly describes the reroll as a player-facing event ("all the tied tiles are **rerolled**").
- severity: minor/correctness upheld (UX/observability defect, state itself is consistent).

## F4 usize underflow in render supply math
- verdict: ADJUSTED
- evidence: render.rs:80 `PLAYER_DICE - used.dice`, render.rs:85 `PLAYER_OWNER_TOKENS - used.tokens`, render.rs:117 `CASINO_TILES - self.board.casino_tile_count(*casino)`. build() (lib.rs:251-314) checks only turn, location validity, ownership, and cash - no die/token/casino-tile supply checks. Correction on "latent": the dice/token halves are indeed latent (players currently only ever own the 2 starting lots, sprawl is unimplemented, so used.dice/tokens <= 2, well under 12/10). But the CASINO_TILES half is reachable in normal play today: RULES.md:74 ("On your turn you may **build** ... once per lot you own") plus 5-6 player games (10-12 starting lots, lib.rs:97 allows up to 6) mean 10+ builds can all pick the same casino colour - nothing limits colour choice - pushing casino_tile_count past CASINO_TILES=9 (lib.rs:29). Then `9 - 10` panics in debug (subtract overflow) or renders a wrapped huge number in release, on every render of the game.
- severity: corrected minor -> major. Not purely latent: the casino-tile underflow is reachable from ordinary player commands in 5-6p games and breaks rendering. (Dice/token halves remain latent as originally stated.)

## F5 Loc::parse_str accepts out-of-range lots; lot 0 underflows neighbours()
- verdict: CONFIRMED
- evidence: board.rs:80-91 parse_str parses block char + any `usize` lot, no check against 1..=block.max_lot() (board.rs:31-37); it backs Deserialize via LocVisitor (board.rs:149-154). board.rs:106-107: `if self.lot % BLOCK_WIDTH != 1 { n.push((self.block, self.lot - 1).into()); }` - lot 0 gives 0 % 3 == 0 != 1, so `0 - 1` underflows usize (panic in debug). Player-command path is safe: loc_parser uses `Enum::exact(locs)` over valid locs only (command.rs:155-158, fed from board.player_locs / TILES.keys), so only crafted JSON / corrupt persisted state reaches it.
- severity: minor/quality-correctness upheld (deserialization hardening, not player-reachable).

## F6 lazy_static dependency for a single static
- verdict: CONFIRMED
- evidence: lazy_static used only at tile.rs:3 and tile.rs:23-25 (`pub static ref TILES: TileMap = tiles();`); Cargo.toml:15 `lazy_static = "1.5.0"`. grep confirms no other use in src/, tests/, or src/bin/. Crate is edition 2024 (Cargo.toml:6) so `std::sync::LazyLock` (or OnceLock) is available with no extra dependency.
- severity: nit/dependencies upheld.

## F7 Card::GameEnd => unreachable!() in starting-cash fold
- verdict: ADJUSTED
- evidence: lib.rs:118 `Card::GameEnd => unreachable!()` inside start()'s per-player draw fold. Provably unreachable: shuffled_deck (card.rs:20-35) builds 48 Loc cards (TILES has 6+6+12+9+6+9 = 48 entries) and inserts GameEnd at `cards_len - quart_pos` where quart_pos in 0..(48 - players*2)/4, i.e. position >= 38 (2p worst case: 48-10=38; 6p: >=40), while players drain at most 6*2 = 12 cards from the front. Two corrections: (1) the finding's ">= 39" bound is slightly off - the true minimum insert position is 38 (2 players), still far above 12, so the conclusion stands; (2) "invariant lives in card.rs:31-33 with no comment" is inaccurate - card.rs:27-29 has a comment explaining last-quarter insertion "taking into account the cards which will be drawn by the players". The genuine gap is that lib.rs:118 itself has no comment pointing at that invariant, and unreachable!() in start() is technically a runtime path (though start is not a player command).
- severity: nit/quality upheld.
- note: unreachable!() claim itself CONFIRMED; the location-of-invariant detail corrected.

## F8 serde_json in [dependencies] but only used in tests
- verdict: CONFIRMED
- evidence: Cargo.toml:18 `serde_json = "1.0.150"` under [dependencies]. Sole usage is lib.rs:350 inside `#[cfg(test)] mod tests` (json_works). grep over src/ (including src/bin/*) and tests/ finds no other serde_json reference. Belongs in [dev-dependencies]. (Incidental: tokio at Cargo.toml:19 is used by src/bin/lords_of_vegas_1_http.rs:6, so that one is legitimately a runtime dep.)
- severity: nit/dependencies upheld.

## F9 redundant use std::iter::FromIterator
- verdict: CONFIRMED
- evidence: board.rs:3 `use std::iter::FromIterator;`. Cargo.toml:6 `edition = "2024"`. FromIterator is in the prelude from edition 2021 onward, so the import is redundant (the trait is used at board.rs:320 `HashSet::from_iter(...)`, which resolves via the prelude without the import).
- severity: nit/consistency upheld.

## F10 render_block hardcodes 3 instead of BLOCK_WIDTH
- verdict: CONFIRMED
- evidence: render.rs:154-155 `let x = (lot - 1) % 3; let y = (lot - 1) / 3;` while board.rs:16 defines `const BLOCK_WIDTH: usize = 3;` (used consistently in Loc::neighbours, board.rs:103-113). BLOCK_WIDTH is module-private to board.rs, so using it from render.rs would need a visibility bump - trivial.
- severity: nit/consistency upheld.

## F11 casino colours diverge from RULES.md descriptions
- verdict: CONFIRMED
- evidence: RULES.md:48-54 colour table: Albion Purple, Sphinx "Tan/olive", Vega Green, Tivoli Grey, Pioneer "Brick red". casino.rs:27-35 maps Sphinx -> NamedColor::Orange and Pioneer -> NamedColor::Brown (Albion/Vega/Tivoli match). Likely constrained by the NamedColor palette, but doc and render disagree as stated.
- severity: nit/consistency upheld.

## F12 player count 2-6 vs official 2-4
- verdict: CONFIRMED (external-rules basis, flagged)
- evidence: lib.rs:97-103 `if !(2..=6).contains(&players)` with `GameError::PlayerCount { min: 2, max: 6 }`; lib.rs:225-227 `player_counts() -> (2..7).collect()`. The crate's own RULES.md nowhere states a supported player count (grep for player-count phrases finds none), so nothing internal contradicts 2-6; the deck math (card.rs:29-33) works for 6 players (48 tiles, 12 drawn). The "official base game is 2-4" claim rests solely on the original reviewer's external knowledge of the published Lords of Vegas rules (base 2-4; the Up! expansion adds 5-6) - no Go reference implementation exists in the snapshot to corroborate. Internally consistent claim; evidence basis is external.
- severity: nit/correctness upheld (design-intent question, not a code defect).
