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

**Done (Track B, 2026-07):** zombie-dice-2 ported. The Go zombie_dice suite
has zero tests (0 lines), so per step 8's absent-suite rule a full baseline
suite was written: player_counts, start state, dice/face counts,
take_dice/basic+refill+zero, roll distribution, keep banking+advancing,
can_roll/can_keep guards, leaders, finished-threshold (unique + below),
rolloff start/skip/resolve, placings (standard-competition ties incl.
three-way), command roll/keep/wrong-player/unknown/after-finished,
cup refill, pub_state field capture, plus `assert_gamer_contract`. clippy
clean, fuzzed ~10k games / ~500k commands with no panic. Reg wired:
workspace, Dockerfile, CI matrix, Tiltfile, k8s base/prod manifests;
zombie-dice-1 GameVersion marked `isDeprecated: true`.

**Done (Track B, 2026-07):** category-5-2 ported (6 nimmt!). All 3 Go tests
ported 1:1 (`TestGame_DrawCards` -> `test_game_draw_cards`,
`TestAutoPlayLastCard` -> `test_auto_play_last_card`,
`TestSortCards` -> `test_sort_cards`); the category_5 Go suite has no
placings/winners test so no tie assertion to adapt (baseline placings tests
added use Rust standard-competition semantics, with the lowest-score-wins
expectations category_5 requires - fewest bullheads places first). Per step
8's thin-suite rule baseline command/scoring/can_*/placings/pub_state tests
were added. `assert_gamer_contract` green, clippy clean, fuzzed ~2.5k games /
~530k commands with no panic. Reg wired: workspace, Dockerfile, CI matrix,
Tiltfile, k8s base/prod manifests; category-5-1 GameVersion marked
`isDeprecated: true`.

**Done (Track B, 2026-07):** battleship-2 ported. The Go battleship suite
has 1 test (`TestGame`, 53 lines) - ported 1:1 as `test_game`; per step 8's
thin-suite rule a full baseline suite was added (player_counts,
start_initial_state, ship_sizes, loc_display, can_place/can_shoot,
place removes/marks/logs/off-board/overlapping/already-placed/wrong-player,
shoot miss/hit/sunk/already-shot/wrong-player/before-placing/after-finished,
player_hits_remaining, player_ship_hits_remaining, finished conditions,
placings incl. standard-competition ties + three-way, points,
command unknown/after-finished, pub_state redacts ships + shows when
finished + captures fields, player_state includes own board + has ships,
alternating turns). `assert_gamer_contract` green, clippy clean, fuzzed
~246 games / ~137k commands with no panic. Reg wired: workspace, Dockerfile,
CI matrix, Tiltfile, k8s base/prod manifests; battleship-1 GameVersion marked
`isDeprecated: true`.

battleship-2-specific port notes (vs the card/dice-game Track B ports):
- Board is `[[Cell; 10]; 10]` indexed `[y][x]` where y=0..=9 (A..J), x=0..=9
  (1..=10); `Cell` enum has Empty/Carrier/Battleship/Cruiser/Submarine/Destroyer/Hit/Miss
  with `#[serde(rename_all = "lowercase")]`. Ship and Cruiser are both size 3
  (Go `shipSizes` has SHIP_SUBMARINE: 3, SHIP_CRUISER: 3).
- Structural redaction per GAME_PORTING.md gotcha: `PubState.boards` maps ship
  cells to `Cell::Empty` via `redact_board()` (not at render time) so the
  serialized state cannot leak ship positions; when finished the boards are
  not redacted (reveal at game end, matching Go's `tileOutputsSelf` switch).
  `PlayerState.board` carries the player's own full board (with ships).
- `Loc { y, x }` with `Display` -> `"B3"` etc; `Enum::exact(all_locations())`
  for the 100 locations (exact matching prevents "b1" matching "b10");
  `Enum::partial` for ships (prefix matching, "sub" -> Submarine) and
  directions ("r" -> Right). Go's `ParseShip` requires >= 3 chars but Rust
  `Enum::partial` accepts any unambiguous prefix - slightly more permissive,
  not a regression (all Go test commands still pass).
- Placings metric is `[player_hits_remaining(p)]` (higher = better, more ship
  cells remaining = you lost fewer ships); winner has the most remaining.
  Go suite has no placings test so no tie assertion to adapt; baseline
  placings tests use Rust standard-competition semantics.
- `can_undo: false` for both `place` and `shoot` (matches Go; placing is
  deterministic but Go returns false so the port preserves that).
- Placing phase is simultaneous: `whose_turn` returns all players with
  `left_to_place[p]` non-empty; shooting phase returns `[current_player]`.

category-5-2-specific port notes (vs the dice-game Track B ports):
- Cards are a `Card(u8)` newtype over the Go `Card int` (1..=104); `heads()`
  ports the Go precedence chain verbatim (`==55 -> 7`, `%11 == 0 -> 5`,
  `%10 == 0 -> 3`, `%5 == 0 -> 2`, else `1`). Note `%10` is checked before
  `%5`, so multiples of 10 (10, 50, 100) score 3, not 2. Colours map by
  heads value: 7 purple, 5 red, 3 yellow, 2 cyan, else grey.
- 4 rows (`[Vec<Card>; 4]`), each capped at 5 cards; a 6th append takes the
  row. `plays: Vec<Option<Card>>` replaces Go's `map[int]Card` with 0
  sentinel. `choose_player` + `resolving` gate the `choose <row>` command
  when a played card is below every row's last card.
- Simultaneous-play: `whose_turn` returns every player with `plays[p] ==
  None`; once all have played, `resolve_plays` resolves lowest-first,
  recursing through auto-play (hands len 1) and end_round/start_round
  (hands len 0) - faithful to Go's `ResolvePlays` switch.
- `can_undo: false` for both `play` (reveals chosen card) and `choose`
  (reveals row choice), matching Go. Hidden info: hands are private
  (`PlayerState.hand`), board/taken/points are public (`PubState`).
- Placings metric is `[-player_points[p]]` (lowest score wins); tie
  semantics use Rust standard-competition (`[2, 2, 1]` for two players
  tied at the higher score with a third lower).

zombie-dice-2-specific port notes (vs farkle-2/greed-2):
- Dice are not numeric values but a `Colour` enum (Green/Yellow/Red) with
  static `faces()` returning `&'static [Face]`. `Dice` serializes only the
  colour (faces are deterministic); `Dice::roll()` picks a random face. No
  `libdie` multiset helpers needed - this is the first Track B dice game
  where dice are not numeric values 1..=6.
- 13-dice cup composition (6 green / 4 yellow / 3 red) reconstructed as
  `all_dice()`. `take_dice` refills from `kept` (brains+shotguns) when the
  cup is short, matching Go `TakeDice`.
- Win threshold is 13 brains (not 5000 points like farkle-2). Rolloff logic:
  when `current_turn` wraps to 0 with multiple leaders at >=13, those
  leaders enter `roll_off_players: Vec<usize>` (empty = no rolloff, faithful
  to Go's `map[int]bool` nil check); non-rolloff players are skipped via
  recursive `next_player` until the rolloff resolves (unique leader on
  wrap-to-0).
- No random first-player selection: player 0 always starts, matching Go
  (`CurrentTurn` zero-value). This differs from farkle-2's randomized
  first-player but is the faithful port.
- `can_undo` is `false` for both `roll` (rng) and `keep` (advances turn),
  matching Go. There is no `done` command (Go zombie_dice has only
  roll/keep).

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
