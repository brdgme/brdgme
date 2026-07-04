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
value/usage rather than size. splendor-2 and texas-holdem-2 wait for their
library prerequisites.

Priority between tracks: Track A games are net-new content; Track B removes
the Go stack. Interleave as desired - both use the same method and any Track B
game is a low-risk filler task.
