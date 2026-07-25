# battleship-2 review findings

Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/battleship-2/` (1,198 LOC Rust + docs).
Go original: `brdgme-go/battleship_1/` (`battleship.go`, `command.go`).

Overall: this is a clean, faithful port. Core logic (placement bounds/overlap
validation, hit/miss/sunk resolution, duplicate-shot rejection, simultaneous
placing phase, alternating shooting turns, win detection, placings, pub-state
redaction of unhit ships) is correct and matches the Go original. Tests are
unusually thorough for these ports (30+ unit tests plus the gamer-contract
test). No critical or major findings.

### shoot() drops Go's bounds validation; direct indexing can panic on out-of-range Loc
- severity: minor
- category: correctness
- location: game/battleship-2/src/lib.rs:308-348 (indexing at :317, :328, :332)
- finding: Go's `Shoot` validated `IsValidLocation(y, x)` and returned an error
  for off-board shots (battleship.go:378-380). The Rust port omits this check
  and indexes `self.boards[op][y][x]` directly. `Loc { y, x }` has public
  fields and no validity invariant, so any caller that bypasses the command
  parser can panic the process with e.g. `Loc { y: 99, x: 0 }`. In the HTTP
  flow this is unreachable because `loc` only ever comes from
  `Enum::exact(all_locations())` (command.rs:24), which constrains coordinates
  to 0..10 — so this is defense-in-depth, not an exploitable input path.
  (`place_ship` does validate bounds via `is_valid_location`, lib.rs:277-282.)
- recommendation: either add an explicit bounds check at the top of `shoot`
  returning `GameError::invalid_input("that is not a valid location on the
  board")` (restoring Go parity), or make `Loc` construction validated and
  fields private.

### Indexing trusts self.players/self.current_player against Vec lengths; inconsistent with defensive .get() elsewhere
- severity: minor
- category: correctness
- location: game/battleship-2/src/lib.rs:378-380 (is_finished via player_hits_remaining :354), :423-425 (status), :387-389 (placings), :270/:283/:290 (place_ship)
- finding: Several methods index `self.boards[p]` / `self.left_to_place[p]`
  for `p in 0..self.players`, panicking if a deserialized/persisted `Game`
  ever has `players`, `boards.len()`, and `left_to_place.len()` out of sync.
  Meanwhile `can_place` (:249), `player_state` (:460-461), and `place_parser`
  (command.rs:51) all use `.get()` defensively — the crate is internally
  inconsistent about the invariant. Not reachable from crafted *command* input
  (state comes from the store, and `start` keeps lengths consistent), so this
  is a robustness/consistency concern only. `can_shoot` also never verifies
  `player < boards.len()`, though with NUM_PLAYERS=2 `other_player` keeps the
  indexed opponent in range.
- recommendation: pick one invariant strategy — ideally validate state once on
  load/deserialize, or use `.get()` consistently in `status`, `is_finished`,
  and `placings` as is already done in `can_place`.

### expect("cell is a ship") in shoot sunk-detection branch
- severity: nit
- category: quality
- location: game/battleship-2/src/lib.rs:331
- finding: `ship_cell.to_ship().expect("cell is a ship")` — provably
  unreachable (the `Cell::Hit | Cell::Miss` and `Cell::Empty` arms above leave
  only the five ship variants), but an `expect` on the logic hot path is a
  needless panic-shaped construct; a future `Cell` variant added without
  updating `to_ship` would turn a logic error into a request-killing panic.
- recommendation: restructure to `match ship_cell.to_ship() { Some(ship) => ..., None => unreachable-with-error }` or handle the ship cells explicitly in the match arms so the ship is bound by the pattern.

### Ship::all() and Direction::all() have inconsistent return types
- severity: nit
- category: consistency
- location: game/battleship-2/src/lib.rs:64 vs :118
- finding: `Ship::all() -> &'static [Ship]` while `Direction::all() -> Vec<Direction>`.
  Both are fixed 4/5-element sets; the allocation in `Direction::all()` is
  gratuitous and the asymmetry is untidy.
- recommendation: make both return `&'static [T]`; call sites already only
  iterate (`.to_vec()`/direct iteration both work with slices).

### Hit-count helpers return i32 for non-negative counts
- severity: nit
- category: quality
- location: game/battleship-2/src/lib.rs:350, :362
- finding: `player_hits_remaining` and `player_ship_hits_remaining` return
  `i32` though the value is a count that can never be negative (kept as i32 to
  feed `gen_placings` metrics, matching Go). Minor type smell; fine as-is.
- recommendation: optional — return `usize`/`u32` and cast at the
  `gen_placings`/points call sites.

### Binary-only dependencies declared as library dependencies (systemic cross-reference)
- severity: nit
- category: dependencies
- location: game/battleship-2/Cargo.toml:8-19
- finding: Known systemic issue (tracked elsewhere): `brdgme_cmd`,
  `brdgme_fuzz`, `rand`, and `tokio` are only used by the `src/bin/` targets,
  not the library; `rand` and `tokio` are not used by this crate's library
  code at all. `brdgme_cmd` also appears in `[dev-dependencies]` with the
  `test-support` feature for tests/contract.rs. Noted for completeness only.
- recommendation: covered by the systemic cleanup of binary deps across the 27
  game crates.

### Divergences from the Go original — all benign improvements (cross-reference, not fresh findings)
- severity: nit
- category: consistency
- location: game/battleship-2/src/command.rs:50-73
- finding: (1) Go's `ParseShip` required >= 3 characters for prefix matching;
  the Rust `Enum::partial` accepts any unambiguous prefix (e.g. "su" for
  submarine). RULES.md:34 documents the new behavior, and ambiguous prefixes
  ("c" for carrier/cruiser) correctly error via the parser's ambiguity path.
  (2) Go's `ParseDirection` iterated a map, making ambiguous-prefix resolution
  nondeterministic; the Rust port is deterministic. (3) Go's `Shoot` bounds
  check is dropped — see the separate minor finding above. No Go quirk that
  deviates from official Battleship rules was found needing preservation; the
  port matches the official ship set/lengths (5/4/3/3/2), allows adjacent
  ships (standard Hasbro rules), announces sunk ships publicly, and forbids
  repeat shots.
- recommendation: none.

## Things checked and found clean
- Placement validation: bounds, overlap, duplicate ship type, wrong player,
  wrong phase all error correctly; footprint via `locations_in_direction`
  includes the origin cell and matches ship sizes.
- Attack resolution: hit/miss/sunk transitions correct; duplicate shots on
  Hit/Miss cells rejected; turn alternates exactly once per successful shot;
  game-end placings and `placings_log` emitted on the winning shot.
- Win condition: `is_finished` only in Shooting phase; both-zero tie state is
  unreachable in real play (only one board changes per shot and the game ends
  immediately).
- Phase transitions: simultaneous placing (both players in `whose_turn`),
  Shooting begins only when both have placed; `current_player` stays 0 so the
  first placer doesn't gain/l advantage — matches Go.
- Coordinate parsing: all coordinates come from `Enum::exact(all_locations())`
  (100 fixed values), parsed case-insensitively; no manual arithmetic parsing, so
  no out-of-bounds or non-ASCII arithmetic hazards beyond the known systemic
  core-parser issues. No case/non-ASCII bug found in this crate's own code.
- Privacy: `pub_state` redacts unhit ship cells until finished; only counts of
  ships-left-to-place leak (public info in real play); `player_state` exposes
  only the requesting player's own board.
- No `unwrap`/`panic`/`unreachable` reachable from player command input; array
  indexing is parser-constrained (see minor findings for the residual
  non-parser paths).
- Rendering matches Go byte-for-byte in layout (header/footer, row letters,
  checkerboard Cyan/Blue, XX hit/miss glyphs); per-row `N::Table` use is
  documented in a comment as intentional.
