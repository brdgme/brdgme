# Raw findings: game/acquire-1 (~2,520 LOC)

Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/acquire-1/`
Note: there is NO Go source for acquire — where behavior is judged, it is judged
against the official Acquire rulebook, and those findings say so explicitly.

## Findings

### player_counts() excludes 6 players despite MAX_PLAYERS = 6
- severity: major
- category: correctness
- location: game/acquire-1/src/lib.rs:313
- finding: `fn player_counts()` returns `(2..6).collect()`, i.e. `[2, 3, 4, 5]`.
  The half-open range excludes 6, but `MAX_PLAYERS` is 6 (lib.rs:25) and
  `start()` accepts 6 (`(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)`,
  lib.rs:186). `player_counts()` is the `Gamer` trait's advertised set of
  supported player counts, so the lobby/service layer will never offer or
  allow a 6-player acquire game even though the engine fully supports it.
  Acquire is a 3-6 player game (2 with the dummy variant), so 6 is the
  headline player count.
- recommendation: Change to `(MIN_PLAYERS..=MAX_PLAYERS).collect()`.

### 2-player dummy shareholder die roll can never be a 6
- severity: major
- category: correctness
- location: game/acquire-1/src/lib.rs:902
- finding: In 2-player games `bonus_players()` rolls the dummy shareholder's
  holding with `self.rng.random_range(1..=5)`, i.e. a uniform 1-5. The game's
  own start log says "A dice (D6) is rolled to determine the dummy player's
  shares" (lib.rs:221-223), and the official Acquire 2-player variant rolls
  one standard six-sided die per chain when bonuses are calculated. As
  written the dummy is systematically weaker than the rules state (never
  holds 6 shares), which shifts majority/minority outcomes in 2-player games.
  Judged against the official rulebook (no Go port exists).
- recommendation: Use `self.rng.random_range(1..=6)`.

### panic! in pay_bonuses on empty major-bonus list
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:841
- finding: `pay_bonuses()` does `if major_len == 0 { panic!("expected some
  major bonus players") }`. This is a runtime path executed during merges and
  game end; a panic in the game service kills the HTTP worker, and the
  project style rule forbids `panic!` in request-reachable paths. In practice
  it appears unreachable (a corp on the board always has its founder holding
  >=1 share at bonus time, and 2-player games always push the dummy into
  `major`), but "appears unreachable" is exactly what `GameError::Internal`
  is for.
- recommendation: Return `Result` and surface `GameError::Internal { message:
  "no major bonus players" }` instead of panicking.

### expect() cluster panics on legacy/corrupt state missing HashMap keys
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:683 (also 1006, 1026, 1076, 1087, 1122,
  1139); game/acquire-1/src/command.rs:59 and :163; game/acquire-1/src/render.rs:78
- finding: Numerous `.expect("could not get player shares")` /
  `.expect("could not get corp share count")` calls on
  `players[p].shares.get(&corp)` and `shares.get(&corp)`. Fresh games
  pre-populate all 7 corp keys (`corp_hash_map`, lib.rs:1224), so these only
  fire on deserialized states that lack a key — but serde happily
  deserializes a HashMap with missing keys, and this crate already carries a
  migration shim for legacy games (`rng` field, lib.rs:156-159), so legacy
  states are a real concern. render.rs:78
  (`self.shares.get(c).expect("expected corp to have shares")`) is in the
  render path, where a panic is even less acceptable. The code is also
  internally inconsistent: `handle_buy_command` (lib.rs:616) and the render
  player table (render.rs:138) use `.get(&corp).cloned().unwrap_or(0)` for
  the same lookups. Also note the typo "could not et player shares" at
  command.rs:163.
- recommendation: Standardize on `.get(&corp).cloned().unwrap_or(0)` (or
  `.copied().unwrap_or_default()`) everywhere; fix the typo.

### "Trades" stat reports the merge count
- severity: minor
- category: correctness
- location: game/acquire-1/src/stats.rs:46
- finding: `s.insert("Trades".to_string(), Stat::Int(self.merges as i32));`
  — copy-paste of the line above; should be `self.trades`. `stats.trades` is
  maintained in `handle_trade_command` (lib.rs:1095) but never surfaced
  correctly.
- recommendation: Use `self.trades as i32`.

### Stats are tracked but never surfaced (dead code)
- severity: minor
- category: quality
- location: game/acquire-1/src/lib.rs:238; game/acquire-1/src/stats.rs:27
- finding: `status()` returns `Status::Finished { placings, stats: vec![] }`,
  and `Stats::to_brdgme_stats()` has no callers anywhere in the workspace
  (grep confirms only its definition). The entire per-player stats
  bookkeeping (`Stats` struct, ~15 fields updated across lib.rs) is
  write-only dead weight as shipped. Either wire it into `status()` stats or
  delete it; as-is it also hides the "Trades" bug above.
- recommendation: Return the stats from `status()` via `to_brdgme_stats()`,
  or remove the stats machinery.

### Start player chosen randomly instead of by initial tile draw
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:213
- finding: Official Acquire setup: each player draws one tile and places it
  on the board; the player whose tile is closest to 1-A (row letter then
  number) plays first. The code places one random tile per player
  (lib.rs:200-202) but then picks the start player with
  `g.rng.random_range(0..players)`. A common digital simplification, but it
  is a deviation from the rulebook. Judged against the official rulebook (no
  Go port exists).
- recommendation: Either derive the start player from the initially placed
  tiles (lowest row, then lowest col), or note the deviation in RULES.md.

### Full-hand redraw discards temporarily-unplayable tiles
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:693-735; game/acquire-1/src/board.rs:130-142
- finding: `start_turn()` treats a tile as unplayable via
  `assert_loc_playable()`, which rejects both (a) tiles merging two safe
  corps (permanently unplayable per the rulebook) and (b) tiles that would
  found a chain when all 7 corps are on the board (only *temporarily*
  unplayable — they become legal again after any merger frees a corp). If
  every tile in hand is unplayable, `redraw_hand()` permanently discards the
  whole hand (`set_discarded`) and redraws, including type-(b) tiles. The
  end-of-turn discard in `draw_replacement_tiles()` (lib.rs:377-380)
  correctly discards only type-(a) tiles. The wholesale redraw rule itself
  exists in later Hasbro editions but not the classic 3M/AH rulebooks, so
  this is an edition choice worth confirming. Judged against the official
  rulebook (no Go port exists).
- recommendation: Confirm the intended edition; if keeping the redraw,
  consider only discarding permanently-unplayable tiles and redrawing the
  rest, or document the house rule in RULES.md.

### Tile-bag exhaustion ends the game immediately
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:403-408
- finding: `draw_replacement_tiles()` calls `self.end()` when the bag can't
  refill the hand to 6, ending the game mid-turn. The official rulebook ends
  the game by player declaration once end conditions are met; behavior on bag
  exhaustion differs by edition (some end the game, some continue without
  drawing). Worth an explicit decision since there is no Go port to match.
- recommendation: Verify against the chosen rulebook edition; document in
  RULES.md.

### Unused thiserror dependency
- severity: minor
- category: dependencies
- location: game/acquire-1/Cargo.toml:14
- finding: `thiserror = "2.0.18"` is declared but never used — grep for
  `thiserror` across the crate matches only Cargo.toml. It is also beyond the
  standard game-crate dep set.
- recommendation: Remove the dependency.

### can_undo in handle_found_command is a tautology
- severity: nit
- category: simplicity
- location: game/acquire-1/src/lib.rs:586
- finding: Returns `matches!(self.phase, Phase::Buy { .. })` immediately
  after `self.buy_phase(player)` unconditionally set the phase to
  `Phase::Buy` — always `true`.
- recommendation: Return `true` (or restructure so the value is meaningful).

### unwrap() on single-element neighbouring_corps set
- severity: nit
- category: consistency
- location: game/acquire-1/src/lib.rs:466
- finding: `neighbouring_corps.iter().next().unwrap()` in the `1 =>` match
  arm. Safe by construction (arm guarded on `len() == 1`) but the project
  idiom forbids `.unwrap()` in runtime paths; a
  `let Some(n_corp) = ... else { return Err(GameError::Internal{..}) }` or
  iterating the set costs nothing.
- recommendation: Replace with a fallible extraction returning
  `GameError::Internal`.

### unwrap() in board render row-run logic
- severity: nit
- category: consistency
- location: game/acquire-1/src/render.rs:268-270
- finding: `start.unwrap()` (twice) inside the corp-text width scan. Safe by
  construction (`start` set to `Some(col)` earlier in the same branch) but
  sits in the render path; `if let Some(s) = start` expresses it without
  panic paths.
- recommendation: Restructure with `if let`.

### Full-game clone for can_end checks
- severity: nit
- category: quality
- location: game/acquire-1/src/lib.rs:1201 (also 1184); lib.rs:259
- finding: `player_can_end()` calls `self.pub_state().can_end()`, and
  `pub_state()` is `self.to_owned().into()` — a deep clone of the entire game
  (players, board, maps) just to compute three integers. This runs on every
  `command_parser()` build (i.e. every command/spec request). The logic in
  `PubState::can_end` only needs `board`, `finished`, and `last_turn`.
- recommendation: Move `can_end` onto a shared helper taking
  `(&Board, finished, last_turn)` and call it directly from `Game`.

### Nondeterministic corp ordering in found parser
- severity: nit
- category: consistency
- location: game/acquire-1/src/command.rs:34
- finding: `self.board.available_corps()` returns a `HashSet<Corp>` whose
  iteration order feeds `found_parser(Enum::partial(...))`, so suggestion /
  spec ordering of foundable corps varies run to run. Cosmetic only.
- recommendation: Sort by `CORPS` order before building the parser.

## Worker summary

Covered: full read of `src/lib.rs` (all 1,370 lines), `src/board.rs`,
`src/command.rs`, `src/render.rs`, `src/corp.rs`, `src/stats.rs`,
`Cargo.toml`, `tests/contract.rs`, and the 4 `src/bin/` binaries. Cross-
checked the `Gamer` trait contract (lib/game/src/game.rs), confirmed
`to_brdgme_stats` has no callers workspace-wide, and confirmed `thiserror` is
unused. Tests were not executed (review-only task).

Judged hardest against the official Acquire rulebook (no Go port exists):
- Merger resolution (largest-wins, tie broken by tile-placer, multi-chain
  sequential resolution via `choose_merger_phase` recursion, safe-corp merge
  ban, safe corp auto-survives) — verified correct, including the
  [4,4,2] tie-for-largest case which correctly resolves the third chain in a
  follow-up merge.
- Majority/minority bonuses: `bonus_players()` traced through ties
  ([5,5,3], [5,3,3], [3,5,5], [5,3], sole-shareholder-gets-both,
  tie-with-dummy splits) — verified correct. Split bonuses are rounded UP to
  the nearest $100 (lib.rs:851, 863), which matches the official rulebook;
  note the review task text said "rounded down" — flagging the discrepancy,
  I believe the code is right.
- Stock pricing tiers by size (corp.rs `additional_value`: 3/4/5/6-10/
  11-20/21-30/31-40/41+), per-corp base values, SAFE_SIZE=11,
  GAME_END_SIZE=41 — verified correct.
- Founder free share only when bank stock remains; 3-share buy limit; 25
  shares per corp; defunct sell/trade/keep with 2-for-1 trades bounded by
  bank stock; final scoring (bonuses + redeem all stock at current value);
  end conditions (41+ or all-on-board-safe, player-declared); end-of-turn
  discard of permanently-unplayable tiles only — verified correct.
- Tile privacy (only own hand in PlayerState; bag count only in PubState) —
  clean.
- Binaries: standard boilerplate, no deviations.

Findings count: 2 major, 8 minor, 5 nit (0 critical).
