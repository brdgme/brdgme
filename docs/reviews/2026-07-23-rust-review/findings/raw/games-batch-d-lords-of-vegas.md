# Raw findings — lords-of-vegas-1 (games batch D)

Snapshot reviewed: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/lords-of-vegas-1/`
(~1,980 LOC non-binary, 7 source files + 4 boilerplate binaries).

Important context: this crate is a deliberately partial implementation. `RULES.md`
("Implementation status" section) and in-code comments explicitly document that only
`build` and `done` are implemented — sprawl/remodel/reorg/gamble/raise, mid-game card
draws, payouts, scoring, and the endgame trigger are all stated as future work, and the
game "runs indefinitely and points always show as 0". The Go source is NOT in the
snapshot, so logic was judged against internal consistency and official Lords of Vegas
rules (noted per finding). Missing-feature items documented as WIP are listed under
"Cross-references (not findings)" at the bottom.

### unimplemented!() panic macros in runtime command dispatch
- severity: major
- category: quality
- location: game/lords-of-vegas-1/src/lib.rs:182-186
- finding: The `command()` dispatch maps `Command::Remodel`, `Command::Reorg`,
  `Command::Sprawl`, `Command::Gamble` and `Command::Raise` to `unimplemented!()`.
  Repo rules forbid `panic!`/`unreachable!`/`unimplemented!` in runtime paths reachable
  from player commands. Today these arms are unreachable only because
  `command_parser()` (command.rs:20-27) wires in just the build and done parsers — but
  the five other parsers (`sprawl_parser`, `remodel_action`, `reorg_parser`,
  `gamble_parser`, `raise_parser`, command.rs:49-152) are fully written, `pub`, and one
  line away from being added to the `OneOf`. The moment any is wired in, a valid player
  command panics the process instead of returning a `GameError`.
- recommendation: Replace each `unimplemented!()` arm with
  `Err(GameError::InvalidInput { message: "not yet implemented".into() })` (or
  `GameError::CommandNotSupported`-style) until the real implementation lands, so the
  dispatch can never panic regardless of parser wiring.

### Nondeterministic HashMap/HashSet iteration feeds RNG-dependent boss-tie resolution
- severity: major
- category: correctness
- location: game/lords-of-vegas-1/src/board.rs:248, game/lords-of-vegas-1/src/board.rs:278, game/lords-of-vegas-1/src/board.rs:314-344
- finding: `casino_at()` pops BFS candidates from a `HashSet<Loc>` via
  `queue.iter().next()` (board.rs:248) and `casinos()` iterates `TILES.keys()`
  (board.rs:278), a `HashMap` — both have per-process random iteration order
  (`RandomState`). `resolve_boss_ties()` then rerolls boss dice in that order
  (board.rs:330-332), consuming the seeded `GameRng` stream. Two processes replaying
  the same game from the same seed (or the same serialized state) can reroll the tied
  tiles in different orders, producing different dice and divergent game states. This
  breaks deterministic replay/audit and makes tie-resolution outcomes unrepeatable.
- recommendation: Make iteration deterministic: collect candidate locs into a `Vec`,
  `sort()` them (`Loc` derives `Ord`), and iterate that — both in the `casino_at` BFS
  queue and in `casinos()` (e.g. iterate `BLOCKS`/`max_lot` in order instead of
  `TILES.keys()`, or sort the keys). Alternatively switch `TILES`/`Board` to `BTreeMap`.

### resolve_boss_ties never populates its log output
- severity: minor
- category: correctness
- location: game/lords-of-vegas-1/src/board.rs:314-344
- finding: `resolve_boss_ties` builds a `logs: Vec<Log>` but nothing is ever pushed to
  it — the result of `self.reroll_at(&bt.loc, rng)` (which returns the new die value)
  is discarded at board.rs:331. When a boss tie occurs the function returns
  `Some(vec![])`; `build()` (lib.rs:308-311) extends its logs with an empty vec and
  sets `can_undo = false`. Net effect: dice are silently rerolled, the player sees no
  log of the tie or the new values, and undo is disabled for a change they were never
  told about.
- recommendation: Push a public log per reroll (e.g. "boss tie at <casino>, <player>'s
  die at <loc> rerolled to <n>") using the die returned by `reroll_at`, matching the
  cascade described in RULES.md.

### usize underflow panics in renderer when supplies are exceeded
- severity: minor
- category: correctness
- location: game/lords-of-vegas-1/src/render.rs:80, game/lords-of-vegas-1/src/render.rs:85, game/lords-of-vegas-1/src/render.rs:117
- finding: Three subtractions can underflow `usize` (panic in debug, huge number in
  release): `PLAYER_DICE - used.dice`, `PLAYER_OWNER_TOKENS - used.tokens`, and
  `CASINO_TILES - self.board.casino_tile_count(*casino)`. `build()` (lib.rs:251-313)
  never enforces the die supply (12/player), token supply (10/player), or the 9-tiles-
  per-casino physical supply — and the three strip lots (A6, D5, F8) can be built as
  any colour, so a colour can legitimately exceed 9 built tiles once more lots are
  ownable. Currently hard to reach (only 2 owned lots/player, no income), so latent.
- recommendation: Enforce supply limits in `build()` (return `GameError::InvalidInput`
  when out of dice/tokens/casino tiles), and/or use `saturating_sub` in the renderer.

### Loc::parse_str accepts out-of-range lots; neighbours() underflows on lot 0
- severity: minor
- category: quality
- location: game/lords-of-vegas-1/src/board.rs:80-91, game/lords-of-vegas-1/src/board.rs:106-108
- finding: `Loc::parse_str` (used by the `Deserialize` impl, i.e. on loaded game state)
  accepts any numeric lot — "A0", "A99" — without validating `1..=block.max_lot()`.
  `Loc::neighbours()` then computes `self.lot - 1` (board.rs:107) which panics on
  underflow for lot 0 in debug builds and wraps in release, and can emit nonexistent
  locs like `A99`'s neighbours. Not reachable from player commands today (the command
  parser uses `Enum::exact` over valid locs), only from crafted/corrupt state.
- recommendation: Validate the lot range in `parse_str`
  (`if lot < 1 || lot > block.max_lot() { return Err(...) }`).

### unreachable!() in starting-cash fold during game start
- severity: nit
- category: quality
- location: game/lords-of-vegas-1/src/lib.rs:118
- finding: `Card::GameEnd => unreachable!()` when summing starting cash. Provably
  unreachable — `shuffled_deck` inserts GameEnd at position `>= 48 - 9 = 39` while
  players drain at most 12 cards from the front — but it is still a panic macro in a
  game-start path, and the invariant lives in another file (`card.rs:31-33`).
- recommendation: Keep as-is but add a short comment stating the invariant, or return a
  `GameError`/`unreachable!` with a message explaining why GameEnd cannot be in a
  starting hand.

### lazy_static used for TILES instead of std OnceLock/once_cell
- severity: minor
- category: dependencies
- location: game/lords-of-vegas-1/src/tile.rs:3,23-25, game/lords-of-vegas-1/Cargo.toml:15
- finding: `lazy_static` is in maintenance mode; the ecosystem (and this review's
  dependency criteria) prefers `once_cell` or `std::sync::OnceLock`. Also, per the
  tile-data verification below, a `OnceLock<TileMap>` or a plain `static` built from a
  const-friendly structure would do. Separately, the 48-entry table built via 48
  `map.insert` calls is verbose but readable; not flagged beyond the dependency.
- recommendation: Replace with `static TILES: OnceLock<TileMap> = OnceLock::new()` +
  a getter, or `once_cell::sync::Lazy`, matching whatever the rest of the workspace
  settles on.

### serde_json is a runtime dependency but only used in tests
- severity: nit
- category: dependencies
- location: game/lords-of-vegas-1/Cargo.toml:18, game/lords-of-vegas-1/src/lib.rs:350
- finding: The only `serde_json` use in the crate is the `json_works` unit test. It is
  declared under `[dependencies]`, needlessly adding it to every dependent's build.
- recommendation: Move `serde_json` to `[dev-dependencies]`.

### Redundant `use std::iter::FromIterator` import
- severity: nit
- category: consistency
- location: game/lords-of-vegas-1/src/board.rs:3
- finding: `FromIterator` is in the prelude for edition 2024; the explicit import (used
  for `HashSet::from_iter` at board.rs:320) is redundant noise.
- recommendation: Delete the import and call `boss_tiles.iter()...collect::<HashSet<_>>()`
  or `HashSet::from_iter(...)` directly.

### Hardcoded literal 3 instead of BLOCK_WIDTH in renderer
- severity: nit
- category: consistency
- location: game/lords-of-vegas-1/src/render.rs:154-155
- finding: `render_block` computes tile coordinates with `(lot - 1) % 3` / `(lot - 1) / 3`
  while board.rs:16 defines `BLOCK_WIDTH: usize = 3` for exactly this. If the grid width
  ever changed, logic and rendering would silently disagree.
- recommendation: Use `BLOCK_WIDTH` (it is private to board.rs; re-export or duplicate the
  constant near the render code).

### Casino colours in code don't match RULES.md descriptions
- severity: nit
- category: consistency
- location: game/lords-of-vegas-1/src/casino.rs:28-34
- finding: RULES.md describes Sphinx as "Tan/olive" and Pioneer as "Brick red", but
  `Casino::color()` maps Sphinx to `NamedColor::Orange` and Pioneer to
  `NamedColor::Brown`. Presumably a palette limitation of `brdgme_color`, but the doc and
  the UI now tell the player different colour names.
- recommendation: Either adjust RULES.md wording to the actual rendered colours or note
  the approximation in the rules; no code change required.

### Player counts 2-6 deviate from official rules (2-4)
- severity: nit
- category: correctness
- location: game/lords-of-vegas-1/src/lib.rs:97-103, game/lords-of-vegas-1/src/lib.rs:225-227
- finding: Judged against official Lords of Vegas rules, the base game supports 2-4
  players; this implementation allows 2-6. (Judged against the Go port this may be a
  deliberate house extension — the Go source was not in the snapshot to confirm.) The
  deck math in `shuffled_deck` handles 6 players fine, so this is a rules-fidelity note,
  not a bug.
- recommendation: Confirm intended player-count range; if 2-6 is deliberate, mention it
  in RULES.md.

## Cross-references (not findings)

Documented-in-code WIP items (RULES.md "Implementation status", DATA_DOCS.md, and code
comments all state these explicitly; the game is a deliberately scoped "building phase"
port):

- No mid-game card draws, lot grants, or casino payouts — `next_player()` (lib.rs:330-333)
  only rotates `current_player`; the `deck`/`played` machinery, `Payout`, `strip` fields,
  and `casino_card_count` exist but are only exercised at setup.
- Scoring not implemented — `Player.points` always 0 (lib.rs:70 comment);
  `status()` placings (lib.rs:196-214) are effectively cash-order via
  `gen_placings(&[points, cash])`; `POINT_STOPS` only used for display.
- No endgame — `finished` is never set to `true`; GameEnd card is never drawn.
- Sprawl/remodel/reorg/gamble/raise not implemented — parsers exist but are unwired
  (command.rs:49-152); dispatch arms panic (reported above as a finding because of the
  panic-macro rule, not the missing feature).
- `unimplemented!()` arms mean `Command` variants for unimplemented actions are dead
  code today; likewise `money_parser`'s `as usize` cast (command.rs:165) is unreachable
  and harmless on 64-bit.
- `BoardTile::Built.owner` is `Option<TileOwner>` but is always `Some` at every
  construction site (lib.rs:291, board.rs:302) — the `None` case is speculative
  generality for future mechanics (e.g. raising), acceptable as scaffolding.
- `rng` serde migration shim comment (lib.rs:82-84) — deliberate, documented.
- The 4 binaries (`src/bin/lords_of_vegas_1_{cli,repl,fuzz,http}.rs`) are the standard
  boilerplate; no per-crate deviation observed. The resulting non-dev deps they pull in
  (`tokio` full, `brdgme_fuzz`) match the systemic pattern tracked in the dependencies
  unit.

## Verified-clean notes

- Tile data table (tile.rs) manually verified: 48 lots total, exactly 9 cards per casino
  (Albion/Sphinx/Vega/Tivoli/Pioneer) plus 3 strip lots (A6, D5, F8) — matches
  `CASINO_CARDS = 9`. Die values, build costs, and starting cash match the table in
  RULES.md (die n -> cost/cash pairs 1:8/9, 2:6/8, 3:9/7, 4:12/6, 5:15/5, 6:20/4).
- `Loc::neighbours` logic matches the 3-wide grid geometry for all block sizes
  (unit-tested at board.rs:391-402; boundary conditions verified by reading).
- `casino_at` flood-fill correctly restricts to same casino AND same height, orthogonal
  neighbours only (diagonal exclusion unit-tested).
- `boss_tiles()` correctly returns all tiles sharing the single highest die, and
  `resolve_boss_ties` rerolls exactly the tied tiles and recurses for cascades — matches
  RULES.md description of tie resolution.
- `build()` validation order and error paths all return `GameError::InvalidInput`; no
  panics on the build path itself.
- `shuffled_deck` places GameEnd in the last quarter of the post-deal deck; with 48
  cards and <=12 dealt, `quart_pile >= 9`, insert position 40-48 — cannot be dealt into
  a starting hand (see nit about the `unreachable!` arm).
- Command parsing: `build` restricts loc choices to the player's owned lots
  (`player_locs`), `Enum::partial` gives the documented prefix abbreviation for casino
  names; `command_spec` gates on `whose_turn`. Non-turn players get no spec.
- Serde: custom `Loc` string (de)serialization is symmetric and needed for JSON map
  keys (documented in-code); `Game` round-trips (unit-tested).
- No `.unwrap()`/`.expect()`/`panic!` in any player-reachable runtime path found other
  than the findings above (guarded `expect` at board.rs:248 is behind an
  `is_empty()` check; flagged only for its nondeterminism aspect).

## Module-level verdict

- `card.rs`, `casino.rs`: clean apart from items noted.
- `tile.rs`: clean data table; only the lazy_static dependency finding.
- `render.rs`: underflow findings above; otherwise consistent with project markup idioms.
- `command.rs`: clean; unwired parsers are documented WIP.
- `board.rs`: the determinism finding is the only substantive issue.
- `lib.rs`: unimplemented! dispatch finding; otherwise a clean, small Gamer impl.
- Crate overall: unusually well-documented WIP scoping; tests are thin (contract test +
  unit tests for neighbours/casino detection/JSON only — no build-flow or tie-resolution
  test), which may be worth a general observation to the Lead but is not reported as a
  per-crate finding since the crate is explicitly partial.
