# Jaipur Rust Port (jaipur-2) - Design

Date: 2026-07-10
Status: Approved design, implementation pending
Parent: #23 Rust Game Ports

## Background

Port the Jaipur card game from the old-Go-project Go source
(`~/Development/brdg.me/game/jaipur/`) to a new Rust crate
`rust/game/jaipur-2`. All ports increment to the next version regardless of
source repository; the original old-project Go implementation counts as
version 1, so the new crate is `jaipur-2`. This is the third port in the #23 port track (after
`liars-dice-2` and `no-thanks-2`).

## Source and source analysis

The Go source implements the `game.Playable` interface from the old monolithic
Go server. It is feature-complete with two commands (`take` / `sell`), an
ASCII table render, round/match lifecycles, camel-bonus tie-breakers, and a
test suite (game start/decode, piece deck, command parsing, take/sell
validation, render output).

| Go file | Purpose |
|---|---|
| `pieces.go` | Good enum (int iota), static deck composition, token/bonus tables, good metadata |
| `game.go` | Game struct, `Start`, `StartRound`, `ReceiveCards`, `ReplenishMarket`, `EndRound`, `NextPlayer`, `Opponent`, `ParseGoodStr`, `IsFinished`/`Winners`/`WhoseTurn` |
| `take_command.go` | `TakeCommand` parser + `Take(allCamels)` / `TakeCamels` validation and mutation |
| `sell_command.go` | `SellCommand` parser + `Sell` validation, token/bonus collection, 3-goods-emptied end-of-round |
| `render.go` | `RenderForPlayer`: scores/leader, token table, bonus table, market/hand/camels for both players |

### Behavior and data flow

**Setup (exactly 2 players):**
1. `start()` validates `len(players) == 2`, then calls `StartRound()`.
2. `StartRound()`: shuffle deck (52 cards: 6 Diamond, 6 Gold, 6 Silver, 8 Cloth,
   8 Spice, 10 Leather, 8 Camel), seed market with 3 camels, `ReplenishMarket()`
   to 5. Deal 5 cards to each player. Initialise empty token piles per player,
   fresh token stacks per good, shuffled bonus piles for sale-sizes 3/4/5.
3. Cards received: camels go to `Camels[player]`, trade goods go to
   `Hands[player]`. Public log announces counts; private log shows exact cards
   to receiver, summary to opponent.

**Turns (`take` command):**
- Only the current player can act. `CanTake` = `g.CurrentPlayer == player`.
- Single-camel take: `take camel` (no `for` goods) -> `TakeCamels` removes all
  camels from market, adds them to `Camels[player]`, then `ReplenishMarket()`.
  Error if no camels in market.
- Single-good take: `take dia` -> 1 good from market to hand. No trade goods
  (`for`). Error if `for` goods specified.
- Multi-good take (trade): `take dia silv for lea lea` -> move N goods from
  hand to market, N goods from market to hand. Validations enforce: same count
  take vs for, no same-type trading, camels not taken in multi-goods, market
  has sufficient counts, player's hand (including camels) has sufficient
  counts, hand size after ≤ 7.
- `ReplenishMarket()`: draw enough cards to fill market to 5 spots. If deck
  empty, `EndRound()`. On success, `NextPlayer()`.

**Turns (`sell` command):**
- Only the current player can act. `CanSell` = `g.CurrentPlayer == player`.
- `sell N good`, e.g. `sell 2 dia` or `sell dia dia`.
- Validations: quantity ≥ `GoodMinSales[good]` (2 for rare goods
  Diamond/Gold/Silver, 1 for common), player has that many of that good in
  hand.
- Take up to `min(quantity, len(goods[good]))` tokens from the token stack for
  that good, add them to `Tokens[player]`, add count to `GoodTokens[player]`.
- If `Bonuses[quantity]` has remaining bonus tokens, take the first one.
- Remove sold cards from hand.
- Check for round end: if 3 or more good types have zero tokens remaining,
  call `EndRound()` and return (no `NextPlayer`). Otherwise `NextPlayer()`.

**Round end (`EndRound`):**
- Camel bonus: player with more camels gets 5 points. Tie -> no bonus.
- Sum per-player scores from `Tokens`.
- Tie-breakers in order: most tokens (score), most `BonusTokens`, most
  `GoodTokens`. If still tied, round is replayed (no `RoundWins` increment).
- Winner gets `RoundWins[winner]++`.
- If `RoundWins[p] == 2`, game is finished (best-of-3). Otherwise call
  `StartRound()`.

**Match completion:** first to 2 round wins. `IsFinished()` returns true.
`Winners()` returns the single winner.

**Hidden info:** a player sees their own hand contents exactly, opponent's
hand size only, and opponent's camel count only as "no" vs "some".

**Randomness:** deck shuffle at round start only (`helper.IntShuffle` in Go,
maps to `GameRng` + `shuffle` in Rust). No mid-round randomness.

**Undo:** `take` and `sell` are not undoable (`can_undo: false`) because the
round-start deck shuffle is a hidden random draw, and the replenish step
after most actions draws unknown cards.

## Architecture and components

### Crate layout

```
rust/game/jaipur-2/
  Cargo.toml
  RULES.md
  src/
    lib.rs          # Game struct, Gamer impl, all game logic (see module strategy below)
    render.rs       # Renderer impls for PubState/PlayerState
    bin/
      jaipur_2_http.rs
      jaipur_2_cli.rs
      jaipur_2_repl.rs
      jaipur_2_fuzz.rs
  tests/
    contract.rs     # assert_gamer_contract::<Game>();
```

### Module strategy

Lifecycle orchestration, state mutation, `Gamer` trait impl, command parsing,
and validation live in `lib.rs`. Per the task guidance, extract a module only
when a cohesive independently-understandable subset exists whose extraction
materially improves `lib.rs` readability; avoid tiny single-type modules.

`jaipur-2` has two natural module boundaries:
- `render.rs` (required): `PubState`/`PlayerState` types plus `Renderer`
  impls. Already justifies itself — the Renderer impl is a self-contained
  formatting concern with its own dep on `brdgme_markup`, structurally
  identical to `zombie-dice-2/render.rs`.
- Command parsing + the `Command` enum: the `command_parser` method, `Command`
  enum, and parser combinators. Whether to extract this into a `command.rs`
  module is a judgment call at implementation time. The Go source has two
  command files (take + sell) totalling ~315 lines; the Rust version will be
  concise. If extraction does not materially reduce `lib.rs` length or
  complexity, inline everything in `lib.rs`.

During implementation, add the module-extraction guidance to
`docs/authoring/GAME_DEVELOPMENT.md`: "Extract modules only for cohesive
independently-understandable subsets where doing so materially improves
`lib.rs` readability; avoid tiny modules."

### Porting rule

Preserve Go behavior by default. Every suspected source defect (inverted
conditions, off-by-one counts, missing validations) must be raised with the
user and approved before correction. During implementation, add this rule to
`docs/porting/GAME_PORTING.md`.

### Domain types

| Go int constant | Rust |
|---|---|
| `Good*` iota (0-6) | `enum Good` with 7 variants, `#[derive(Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]` |
| `TradeGoods` slice | `Good::trade_goods()` returning `&[Good; 6]` |
| `CardCounts`, `TradeGoodTokens`, `TradeBonuses`, `GoodMinSales` | `impl Good` methods returning `&[u32]` or equivalent; `const` arrays where feasible |
| `GoodStrings` / `GoodStringsPl` / `GoodColours` | methods on `Good`: `fn name(self) -> &'static str`, `fn plural(self) -> &'static str`, `fn color(self) -> color::Color` |

### Game struct

```rust
pub struct Game {
    pub current_player: usize,
    pub round_wins: [u8; 2],
    pub deck: Vec<Good>,
    pub hands: [Vec<Good>; 2],
    pub tokens: [Vec<u32>; 2],
    pub camels: [u32; 2],
    pub bonus_tokens: [u32; 2],
    pub good_tokens: [u32; 2],
    pub bonuses: HashMap<usize, Vec<u32>>,
    pub goods: HashMap<Good, Vec<u32>>,
    pub market: Vec<Good>,
    pub rng: GameRng,
}
```

**Key differences from Go:**
- `Players []string` → player-count only (indexed by `usize`).
- `Log *log.Log` field removed; mutating methods return `Vec<Log>`.
- `map[int]bool` sets → `Vec<bool>` or `HashSet<usize>`.
- Token values stored as `u32` (serde-compatible, matches Go `int` range).
- `GameRng` field for deterministic deck shuffle.

### PubState and PlayerState

```rust
pub struct PubState {
    pub current_player: usize,
    pub round_wins: [u8; 2],
    pub market: Vec<Good>,
    pub deck_len: usize,
    pub camels: [u32; 2],
    pub hand_sizes: [usize; 2],
    pub token_counts: [usize; 2],
    pub goods: HashMap<Good, Vec<u32>>,
    pub bonuses: HashMap<usize, usize>,  // remaining count per sale-size
}

pub struct PlayerState {
    pub public: PubState,
    pub player: usize,
    pub hand: Vec<Good>,
}
```

**Hidden info redaction:**
- `PubState` contains actual camel counts for both players — these are needed
  for camel-bonus determination at round end and are public knowledge
  (everyone sees how many camels each player collected from the market).
- The Renderer for `PubState` (spectator view) shows "no"/"some" for both
  players' camel counts rather than exact numbers.
- The Renderer for `PlayerState` shows exact camel count for the current
  player and "no"/"some" for the opponent. This matches Go's
  `RenderForPlayer` behavior: Go reads `g.Camels[opponentNum]` directly and
  formats as "no"/"some", never exposing the exact opponent camel count in
  the player render.

### Command enum and parsing

```rust
enum Command {
    Take { take: Vec<Good>, give: Vec<Good> },
    Sell { good: Good, quantity: usize },
}
```

Command parsing uses `brdgme_game::command::parser` combinators (same pattern
as `zombie-dice-2` and all other Rust games):
- `command_parser(&self, player) -> Option<Box<dyn Parser<T = Command> + '_>>`
  returns `None` when no commands are available (game finished or not player's
  turn).
- Take: parse good names (`Token` matching `Good::name()`), optional `for`
  keyword and second good list. Example: `dia silv for lea lea` →
  `Take { take: [Diamond, Silver], give: [Leather, Leather] }`. Single camel:
  `camel` → `Take { take: [Camel], give: [] }`.
- Sell: `Enum(Good::trade_goods())` for the good, then parse quantity
  (optional count prefix or count of repeated good names). Example: `2 dia` or
  `dia dia` → `Sell { good: Diamond, quantity: 2 }`.
- `Doc` descriptions mirror Go's `Usage()` strings.

### Gamer impl (lib.rs)

| Method | Behaviour |
|---|---|
| `start(players, seed)` | Validate `players == 2`, create initial state, seed RNG, shuffle deck, deal hands, return `(game, logs)` |
| `status()` | `Active { whose_turn: [current_player], eliminated: [] }` while not finished; `Finished { placings: [winner, loser], stats: [] }` when done |
| `pub_state()` / `player_state()` | Construct typed state structs (see above) |
| `command(player, input, players)` | Parse → dispatch `Take`/`Sell` → return `CommandResponse { logs, can_undo: false, remaining_input }` |
| `command_spec(player)` | Return parser spec for current player's available commands |
| `points()` | Running point totals from `Tokens` for each player |
| `player_counts()` | `vec![2]` |
| `player_count()` | `2` |
| `rules()` | `include_str!("../RULES.md").to_string()` |

### Errors

All validation errors use `GameError::invalid_input(...)` with descriptive
messages matching Go equivalents. No panics in runtime paths. Error cases:
- Wrong player count (`GameError::PlayerCount { min: 2, max: 2, given: N }`)
- Command from wrong player, or game already finished
- Take: no goods specified, single-good with `for`, multi-good count mismatch,
  camels in multi-good take, same-type trade, insufficient market stock,
  insufficient hand stock, hand size overflow
- Sell: wrong turn, below minimum sale quantity, insufficient copies in hand,
  not a trade good
- Camel take: no camels in market

### Integration and deployment

Follow the established checklist from `docs/porting/GAME_PORTING.md`:

1. `rust/Cargo.toml`: add `game/jaipur-2` to workspace `members`.
2. `rust/Dockerfile`: add final stage copying `target/release/jaipur_2_http`.
3. `.github/workflows/ci.yml`: add `jaipur-2` to `build-rust-games` job matrix.
4. `Tiltfile`: add `"jaipur-2"` to Rust games list.
5. `k8s/base/game/jaipur-2/`: `deployment.yaml`, `service.yaml`,
   `game-version.yaml`, `kustomization.yaml`; add dir to
   `k8s/base/game/kustomization.yaml`.
6. `k8s/prod/app/kustomization.yaml`: add `ghcr.io/brdgme/brdgme/jaipur-2`
   image override.

The `Cargo.toml` mirrors `zombie-dice-2` with identical dependency versions.
Binaries follow the 4-stub pattern (`_http`, `_cli`, `_repl`, `_fuzz`).

## Testing

### Port Go tests 1:1

| Go test | Rust equivalent assertion |
|---|---|
| `TestGame_Start` | After `start(2, seed)`: deck = 40, each player has 5 total cards, goods map has 6 entries, bonuses has 3 size tiers with correct lengths |
| `TestGame_Decode` | After `start`: serialize state, deserialize back, verify no errors (serde round-trip) |
| `TestParseGoodStr` | Port all 5 cases: single camel, "2 Camels", negative quantity, mixed-format input. `command()` parses and returns correct `Command` variant |
| `TestSellCommand_Call` | Start game, set hand to `[Gold]`, `sell 2 gold` errors, `sell 1 gold` errors (below min 2). Set hand to `[Gold, Leather, Gold]`, `sell 2 gold` succeeds: tokens = `[6, 6]`, goods for gold = `[5, 5, 5]`, hand = `[Leather]` |
| `TestDeck` | `Deck()` returns 52 cards |
| `TestGame_Render` | `start()`, call `pub_state()` / `player_state()` then render both; no errors |

### Additional Rust contract and behavioral tests

| Test | What it covers |
|---|---|
| `assert_gamer_contract` | `tests/contract.rs` - exercises the full `brdgme_cmd::test_support::assert_gamer_contract` matrix |
| Player counts | `start(1, 0)` errors, `start(3, 0)` errors, `start(2, 0)` succeeds |
| Serialization | State round-trips through serde JSON without data loss |
| Authorization | Commands from non-current player error; commands after game finished error; `command_spec` returns `None` when not player's turn |
| Hidden info | `pub_state` does not contain hand contents; `pub_state` render shows "no"/"some" for camel counts; `player_state` for player 0 emits own hand and exact own camels, opponent camels as "no"/"some" in render |
| Render | Both `PubState` and `PlayerState` render through `Renderer` without panicking; output contains expected text markers |
| Placings | Finished game returns correct placings; winner gets `[1, 2]`, loser gets `[2, 1]` |
| Take edge cases | Camel take with no camels in market, multi-take exceeding hand size, single-good with `for` goods, same-type trade rejection, insufficient market stock, insufficient hand stock |
| Sell edge cases | Below-minimum quantity per good type, selling more than held, camel cannot be sold, non-trade-good errors |
| Round completion | Selling the 3rd good type to depletion triggers `EndRound`; round-wins increment correctly |
| Match completion | After 2 round wins, `status()` returns `Finished`; `command()` errors for any player |
| Camel bonus and tie-breakers | Higher camel count gets 5pts; equal camels = no bonus. Score tie → BonusTokens tie-break → GoodTokens tie-break → replayed round |
| Camel bonus correctness | `camels[0] = 3, camels[1] = 1` → player 0 gets 5 camel-bonus token, round score includes it; `camels[0] = 2, camels[1] = 2` → no camel bonus |

**Regression tests** only for user-approved defect fixes discovered during
implementation. No speculative test cases for unconfirmed bugs.

**No comments, groupings, or references in test code pointing back to the
deprecated Go version.** Tests stand on their own; the 1:1 mapping is a
porting methodology, not a code convention.

### Verification

Run `cargo fmt` and `cargo clippy` after all work. The implementation must
update relevant documentation to make both mandatory CI steps for all Rust
changes. During implementation:
- Add "Run `cargo fmt --all -- --check` and `cargo clippy -p web --all-targets --features ssr -- -D warnings` (plus equivalent per-crate clippy for game crates) after all Rust changes" to `docs/authoring/GAME_DEVELOPMENT.md`.
- Add "After every port step or batch of changes, run `cargo fmt` and `cargo clippy` on the new crate" to `docs/porting/GAME_PORTING.md`.

## Documentation updates (during implementation)

- `docs/authoring/GAME_DEVELOPMENT.md`: add Randomness section (already done
  per deterministic-rng spec), add module-extraction guidance, add
  fmt/clippy requirements.
- `docs/porting/GAME_PORTING.md`: add preserve-Go-behavior rule, add
  fmt/clippy requirement.

## Constraints and success criteria

- Solitary new file: `docs/superpowers/specs/2026-07-10-jaipur-2-design.md`.
  No existing files modified. No staged/committed changes.
- Implementation must not start during this spec-writing session.
- All 6 Go tests ported with equivalent assertions.
- `cargo fmt` and `cargo clippy` pass clean.
- No panics in runtime paths; all errors are `GameError`.
- Game deploys and runs correctly under Tilt/Kind.
- No speculative features, refactors, or scope outside the specified task.
