# Game Porting Plan (targets Rust)

All porting targets Rust game crates - see `docs/GAME_PORTING.md` for the
method and `docs/GO_VS_RUST_PORTING.md` for the Go-vs-Rust decision. Two
tracks:

- **Track A - new ports** from the old project (`~/Development/brdg.me/game/`):
  games never ported to this platform.
- **Track B - Rust conversions** of the 17 Go games already in `brdgme-go/`:
  each becomes a new `-2` edition (e.g. `age-of-war-1` -> `age-of-war-2`),
  following the existing `lost-cities-1` -> `lost-cities-2` precedent (deploy
  the `-2` GameVersion, mark the `-1` manifest `isDeprecated: true`). When all
  17 are converted, the entire `brdgme-go/` stack (Go toolchain, Dockerfile,
  Bazel files) can be retired.

Already done and out of scope: acquire-1, lost-cities-1/2 (Rust).

## Track A: old-project ports

| Game | Old size | Players | Effort | Notes |
|---|---|---|---|---|
| tic_tac_toe | 461 lines, 3 files | 2 | Small | Trivial; Rust-port warm-up |
| jaipur | 993 lines, 9 files | 2 | Medium | Hidden hands; goods enum |
| red7 | 1,143 lines, 11 files | 2-4 | Medium | Bitmask cards -> struct; eliminations |
| alhambra | 2,344 lines, 19 files | 2-6 | Large | Card enum; own square grid |
| starship_catan | 3,997 lines, 34 files | 2 | Very large | Card enum redesign; 20+ commands |
| seven_wonders | 4,310 lines, 39 files | 3-7 | Very large | Card enum redesign; simultaneous turns; needs cost lib |
| hive | 277 lines (stub) | 2 | Not a port | Old version unfinished; new development |
| chess | 744 lines (engine only) | 2 | Not a port | Move-gen library only; new development |

Suggested order: tic_tac_toe -> jaipur -> red7 -> alhambra -> starship_catan ->
seven_wonders. Defer hive and chess.

Prerequisite library work (Rust, shared with Track B):
- **cost/permutation module** before seven_wonders and splendor-2: port
  `libcost` (cost map + `Perm` permutation helpers, ~330 lines + tests) -
  `rust/lib` has no equivalent. Everything else needed already exists
  (`brdgme_game`, `brdgme_markup`, `brdgme_color`, parser combinators).
- **poker hand evaluation** before texas-holdem-2: port `libpoker`
  (~200 lines + tests).
- `libdie` (roll helpers) is trivial - inline per game or a tiny shared fn.
- No card/deck library is needed: per-game `enum Card` + `Vec<Card>` +
  `rand::seq::SliceRandom` covers every game (this replaces the Go libdeck
  design that the Rust decision made unnecessary). Games using `libcard`
  suit/rank cards (modern-art-2, for-sale-2, texas-holdem-2) get a local
  suit/rank `Card` struct or share one small module with the poker lib.

## Track A per-game plans

### tic_tac_toe (Small)

- 3 files. 2 players, no hidden info.
- One `play` command (coordinate parser); `PubState == PlayerState` in
  content (both render the same board).
- `gen_placings` on a single win metric; `points()` 1/0.
- Good first Rust port to establish rhythm against the lost-cities-1
  template.

### jaipur (Medium)

- 2 players. Goods/camels are int enums in the old code (`pieces.go`) ->
  `enum Good`. Commands: take, sell (+ trade variants inside take).
- Hidden info: hands. `PlayerState { public, player, hand }`; `PubState`
  carries opponent hand count only. Deck draws => `can_undo: false`.
- Round/point structure (bonus tokens, seals of excellence) -> metrics for
  `gen_placings` (seals, then points).
- Old `helper.MatchStringInStrings` goods matching -> `Enum` parser.

### red7 (Medium)

- 2-4 players. Old cards are int bitmasks (suit|rank flags) -> a
  `struct Card { suit: Suit, rank: u8 }` with the palette-comparison rules
  as methods; drop the bitmask cleverness.
- Commands: play, discard, done; combined play+discard turns rely on
  command chaining - `remaining` passthrough is load-bearing here.
- In-round eliminations -> `Status::Active.eliminated`.
- Hidden hands; deck draws => `can_undo: false`.
- Seven suit colors map to `brdgme_color` constants.

### alhambra (Large)

- 2-6 players (2p has a "Dirk" dummy - keep as non-seat state). 19 files.
- Old interface deck (`Card`/`ScoringCard` gob types) -> single
  `enum Card { Currency { currency: Currency, value: u8 }, Scoring { round: u8, ... } }`.
- Own square-grid/vector code (`grid.go`, `vect.go`) is self-contained ->
  small `board` module with `Loc` math; no shared lib needed.
- Commands: take, spend, place, remove, swap, done. Verify whether multiple
  players can act simultaneously during buying/scoring windows and reflect
  it in `whose_turn`.
- Tile-supply and card-pile draws => `can_undo: false`.

### starship_catan (Very large)

- 2 players. 34 files, ~4,000 lines; largest command surface in the old
  project (20+ commands: build, buy, trade, fight, found colony, sector
  navigation, transactions...).
- Old behaviour-interface cards (`Commander`, `Actioner`, `TradingPoster`,
  `VictoryPointer`, ...) -> one `enum AdventureCard`/`enum SectorCard` family
  with variant data (trade direction, price, max, resources, medals,
  diplomat points); behaviour = `match` on variants. This is the core
  redesign and the bulk of the effort.
- Sequential phases with sub-decisions (choose/complete/next/done) -> a
  `Phase` enum driving `command_parser`; port turn logic with the old tests.
- Hidden info: sector deck order, some opponent state; dice + draws => most
  commands `can_undo: false`.

### seven_wonders (Very large)

- 3-7 players. 39 files, ~4,300 lines; the biggest port.
- ~18 gob-registered card/action/resolver types -> `enum Card` with static
  data tables (cost via the new cost module, VP, military, science,
  commerce parameters) and a `Resolver`/`PendingAction` enum replacing the
  old resolver interface queue.
- **Simultaneous turns**: all players pick cards at once; `whose_turn`
  returns all unresolved players, `command_spec` is per-player, and undo is
  effectively always off during picks.
- Hidden hands during drafting -> `PlayerState` carries own hand only.
- Depends on the Rust cost/permutation module (see prerequisites).

### hive (defer - not a port)

- Old implementation is an unfinished stub (no commands, hardcoded demo
  render, `IsFinished` always false). New game development, in Rust, if ever
  prioritized; would need hex-grid representation and an ASCII hex renderer
  (nothing in `rust/lib` provides one yet).
- A partial Go bring-over exists in stash `wip-go-hive-chess-port` (libgrid +
  hex libs, chess skeleton); superseded by the Rust decision - do not build
  on it.

### chess (defer - not a port)

- Old code is a move-generation/board library with no game layer, never
  registered in the old project. If ever built: implement in Rust (piece
  logic ports naturally to enums + `match`), writing the game layer
  (turns, check/checkmate into `status`, draw/resign commands) from scratch.

## Track B: brdgme-go -> Rust `-2` conversions

These are substantially easier than Track A: the Go sources already use the
new platform's architecture (int players, returned `[]brdgme.Log`, parser
combinators, `GenPlacings`, JSON state, Pub/PlayerState split), so conversion
is mostly language translation plus idiomatic upgrades (int-const enums ->
Rust enums, typed states, `Renderer`). Rules knowledge is already encoded in
passing Go tests: **1:1 test porting is required** (every case, original
names, original assertions - see GAME_PORTING.md step 8); games with thin or
missing suites (zombie_dice: 0 lines, farkle: 16) additionally get baseline
command + scoring tests written during the port.

Versioning: new crate `rust/game/<name>-2`, deployed as `<name>-2`; the Go
`-1` GameVersion gets `isDeprecated: true` (existing games keep running until
finished). Retire the Go service once no active `-1` games remain.

Sizes are non-test Go lines (tests in parentheses):

| Game | Size | Lib needs | Notes |
|---|---|---|---|
| liars_dice | 467 (116) | die | |
| no_thanks | 434 (329) | - | Well-tested; easy |
| farkle | 515 (16) | die | |
| zombie_dice | 578 (0) | die | No tests - write basics during port |
| greed | 585 (28) | die | |
| category_5 | 671 (49) | - | (6 nimmt!) |
| battleship | 715 (53) | - | |
| for_sale | 732 (109) | card | |
| sushizock | 941 (242) | die | |
| texas_holdem | 922 (218) | card, poker | Needs poker hand eval module |
| sushi_go | 1,080 (403) | - | Simultaneous picks |
| age_of_war | 1,092 (28) | die | |
| modern_art | 1,123 (583) | card | Best-tested; auction phases |
| love_letter | 1,256 (128) | - | Hidden hands, eliminations |
| cathedral | 1,432 (278) | - | Own grid/shape placement code |
| splendor | 2,262 (53) | cost | Needs cost module |
| roll_through_the_ages | 2,806 (551) | die | Largest |

~17.6k lines of game code total. Suggested approach: start with a small dice
game (liars_dice or greed) to establish the translation rhythm, then order by
value/usage rather than by size. splendor-2 and texas-holdem-2 wait for their
library prerequisites.

**Done (Track B POC, 2026-07):** liars-dice-2 completed against the
lost-cities-1 template. 3 Go tests ported 1:1, `assert_gamer_contract` green,
clippy clean, fuzzed ~66k games with no panic. Reg wired: rust workspace,
Dockerfile, Tiltfile, k8s base/prod manifests; liars-dice-1 GameVersion marked
`isDeprecated: true`. Use it as a second reference alongside lost-cities-1,
especially for `Vec<Vec<u8>>` dice state, re-rolling round resets, and the
PubState-vs-render gap documented in GAME_PORTING.md step 6.

**Done (Track B, 2026-07):** no-thanks-2 ported. All 13 Go tests ported 1:1;
`test_winners` assertion adapted from `[1,1,2]` to `[1,1,3]` because the Rust
`gen_placings` helper uses standard-competition tie ranking (1224) while Go
`brdgme.GenPlacings` uses compact-ordinal (1223) - documented as a tracked
deviation per GAME_PORTING.md step 8. `assert_gamer_contract` green, clippy
clean, fuzzed ~33k games with no panic. Reg wired: workspace, Dockerfile,
Tiltfile, k8s base/prod manifests; no-thanks-1 GameVersion marked
`isDeprecated: true`. Track B progression offset by the placings-tie
gotcha documented for future -1/-2 ports.

**Done (Track B, 2026-07):** greed-2 ported. Both Go tests ported 1:1
(`TestGame`, `TestDoneTakesRemainingScoringDice`); the greed Go suite has no
placings/winners test so no tie assertion to adapt (baseline placings tests
added use Rust standard-competition semantics). `assert_gamer_contract` green,
clippy clean, fuzzed ~80k games / ~9.6M commands with no panic. Reg wired:
workspace, Dockerfile, CI matrix, Tiltfile, k8s base/prod manifests; greed-1
GameVersion marked `isDeprecated: true`. `libdie` inlined per the plan
(`dice_in_dice`/`dice_equals`/`available_scores` in `src/lib.rs`) - no shared
die lib needed. Note for future dice-game ports: `Token` parsing is
case-insensitive (UniCase), so Go faces that differ only by case (`E` vs `e`
in greed's E1/E2) collide in the `OneOf`; preserved 1:1 by keeping `Scores()`
priority order so the first-listed face wins, matching Go.

**Done (Track B, 2026-07):** farkle-2 ported. The Go farkle suite has only
`TestGame` (1 case, 16 lines) - it is ported 1:1 as `test_game`, and baseline
command/scoring/can_* /placings tests were added per step 8's thin-suite rule.
The farkle Go suite has no `Placings`/`Winners` test, so there is no tie
assertion to adapt; the added `test_finished_and_placings` pins Rust
standard-competition semantics (`[1, 1, 3]` for a two-way tie at the top).
`assert_gamer_contract` green, clippy clean, fuzzed ~1.1k games / ~12.5M
commands with no panic. Reg wired: workspace, Dockerfile, CI matrix, Tiltfile,
k8s base/prod manifests; farkle-1 GameVersion marked `isDeprecated: true`.

Farkle-specific port notes (vs greed-2):
- Dice are plain `u8` values 1..=6 (not named enum faces - farkle dice are
  genuinely numeric); per-face colours are a `match` on `u8` (1 cyan, 2 green,
  3 red, 4 blue, 5 yellow, 6 purple). `libdie` inlined (`dice_in_dice`/
  `dice_equals`/`available_scores` in `src/lib.rs`) - no shared die lib needed,
  same as greed-2.
- `score <dice>` parser is the faithful port: `Token("score")` + `AfterSpace` +
  `Many::some_spaced(Int::bounded(1, 6))` mapped to `Vec<u8>` - the selection
  is validated against the score table at action time (single 1=100, single
  5=50, three 1s=1000, three 2s=200, ... three 6s=600). This differs from
  greed-2's per-combo token sub-parsers because farkle dice are numbers, not
  named tokens.
- `done` does NOT auto-score leftover dice (unlike greed-2) - it only banks
  the accumulated `turn_score`. `can_done` requires `taken_this_roll` (you
  must have scored at least once this roll before banking), matching Go farkle.

farkle-2 and greed-2 duplicate the dice-multiset helpers (`dice_in_dice`/
`dice_equals`/`available_scores`) and the turn-engine structure
(`start_turn`/`bust`/`score`/`player_roll`/`done`/`placings`) almost verbatim,
differing mainly in `Die`'s representation (`u8` vs a named enum). This is the
accepted "inline per game or a tiny shared fn" tradeoff from the prereq
library guidance above, not an oversight. If a third dice-based game is
ported, revisit this: a shared module/crate generic over a die-index trait
should be considered at that point rather than pasting a third copy.

Library fix: `brdgme_game::command::parser::Space` was `struct Space {}`
(private), which made `Many::some_spaced`/`any_spaced`/`bounded_spaced` unusable
outside the `parser` module - their return type `Many<TP, Space>` contained a
private type parameter and could not cross crate boundaries. Made `Space`
`pub` (one-word change in `rust/lib/game/src/command/parser/mod.rs`). This
unblocks the documented `Many` combinator for all future ports (Track A red7's
chained play+discard turns depend on it).

Priority between tracks: Track A games are net-new content; Track B removes
the Go stack. Interleave as desired - both use the same method and any Track B
game is a low-risk filler task.
