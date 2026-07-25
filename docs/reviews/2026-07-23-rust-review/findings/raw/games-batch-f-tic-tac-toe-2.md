# Findings: game/tic-tac-toe-2 (786 LOC, canonical minimal example)

Reviewed: `src/lib.rs`, `src/command.rs`, `src/render.rs`, `tests/contract.rs`, `Cargo.toml`, `RULES.md` (skimmed `DATA_DOCS.md`/strategy docs not in scope). The 4 `src/bin/` binaries were skipped per instructions (boilerplate, tracked systemically).

No Go source exists for this game; rules were judged against standard tic-tac-toe. **Core game logic is correct**: move validation (finished-game rejection, turn enforcement, bounds check, occupied-cell check — `src/lib.rs:88-100`), all 8 winning lines detected (`src/lib.rs:122-148`, plus a test enumerating all 8), draw detection via full board (`src/lib.rs:150-152`), placings/points match `RULES.md` (winner [1,2] + 1 pt; draw [1,1] + 0 pts). Test coverage is genuinely strong (contract test, serde round-trips, determinism, all-lines, draw/winner status+points). The findings below are all low-severity.

### `1 - start_player` underflows on crafted state
- severity: minor
- category: correctness
- location: game/tic-tac-toe-2/src/render.rs:34
- finding: `let o_player = 1 - start_player;` computes the other player by subtraction. `start_player` comes from `PubState`/`Game`, which the HTTP service deserializes from request JSON (`lib/cmd/src/requester/gamer.rs:28,37,41,45` — `serde_json::from_str(game)`). A crafted game state with `start_player >= 2` underflows the `usize`: panic in debug builds (overflow checks on), silent wrap to a garbage `N::Player(usize::MAX - k)` in release (workspace `Cargo.toml` sets no `overflow-checks` for release). Note `src/lib.rs:141` computes the same value safely as `(self.start_player + 1) % NUM_PLAYERS`, so the crate is internally inconsistent as well as fragile. Only reachable with a forged state (normal play keeps `start_player` in 0..2), hence minor.
- recommendation: use `(start_player + 1) % NUM_PLAYERS` here too, matching `lib.rs:141`.

### Dead, misleading `Cell::Empty` arm in `winner()`
- severity: nit
- category: quality
- location: game/tic-tac-toe-2/src/lib.rs:139-143
- finding: `line_winner.map(|cell| match cell { ... Cell::Empty => self.start_player })` — `matching_line` (`src/lib.rs:146-148`) never returns `Cell::Empty`, so this arm is unreachable and silently maps a hypothetical empty line to the start player, which would be a bug if it were ever reachable. In the canonical example crate this is exactly the kind of dead arm other authors copy.
- recommendation: have `matching_line` return a mark-only type, or replace the arm with `unreachable!("matching_line never returns Empty")` / filter `Empty` before the map.

### Mark casing inconsistent between logs and board render
- severity: nit
- category: consistency
- location: game/tic-tac-toe-2/src/lib.rs:107-118 and game/tic-tac-toe-2/src/render.rs:16-17
- finding: the play log renders the mark uppercase bold (`X`/`O`), the board render and the "is X / is O" label line use lowercase (`x`/`o`). `RULES.md` also documents the marks as lowercase `x`/`o` while the log says `X`. Cosmetic, but again this is the crate others copy.
- recommendation: pick one casing (uppercase matches tic-tac-toe convention and the label text) and use it in both the log and the renderer.

### Library `[dependencies]` include bin-only crates
- severity: minor
- category: dependencies
- location: game/tic-tac-toe-2/Cargo.toml:8-16
- finding: cross-reference of the known systemic "binary-only deps declared as library deps" issue — this crate is a clean example of it: the library code uses only `brdgme_game`, `brdgme_markup`, `brdgme_color`, `rand`, `serde`; `brdgme_cmd`, `brdgme_fuzz`, and `tokio` (with `features = ["full"]`) are used solely by the four `src/bin/` targets, and `brdgme_cmd` is additionally duplicated in `[dev-dependencies]` with the `test-support` feature for `tests/contract.rs`.
- recommendation: fix with the systemic resolution (move bin-only deps behind target-specific dependency sections or a shared bin support crate).

### Crafted `players` count drives unbounded allocation/iteration
- severity: minor
- category: correctness
- location: game/tic-tac-toe-2/src/lib.rs:154-160, 268-273
- finding: `placings()` and `points()` iterate `0..self.players`, and `status()` calls `placings()`; `lib/cmd` `renders()` also iterates `0..game.player_count()`. `players` is a plain `usize` field deserialized from request JSON with no validation, so a forged state with a huge `players` value causes a massive allocation (OOM/abort) or a near-infinite render loop. Systemic in that every game crate trusts the deserialized state, but noted here because the requester deserializes `Game` verbatim. Not reachable through normal play (state is server-generated).
- recommendation: systemic fix belongs in the requester/validation layer; at minimum games could clamp or validate `players == NUM_PLAYERS` on entry.

## Notes on things checked and found clean
- No `unwrap`/`expect`/`panic!`/indexing reachable from player move input: `play()` bounds-checks before indexing (`src/lib.rs:95-100`); the command parser only yields `Loc`s from `all_locations()` via `Enum::exact` (`src/command.rs:24`), so out-of-range coordinates can never reach the board index.
- Turn enforcement, occupied-cell rejection, and finished-game rejection are all covered both in logic and tests (`wrong_player_cannot_play`, `finished_game_rejects_play`, `test_play_same_cell`).
- `current_player` advancing after a game-ending move is deliberate, documented in `RULES.md:17`, and harmless (`Status::Finished` has no `whose_turn`).
- Serde on `Board` (`[[Cell; 3]; 3]`) rejects wrong shapes at deserialization time rather than panicking.
- `Loc::fmt` (`src/lib.rs:36-42`) would produce a garbage byte for out-of-range row/col, but `Loc` is never deserialized and the parser only constructs valid ones — not reachable.
- `PlayerState::render` is identical to `PubState::render` (no hidden info in tic-tac-toe) — appropriate.
- No start-of-game log announcing the starter/X assignment; the public render carries that info, so acceptable, but other games may want a start log — not a defect here.
