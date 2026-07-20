# Porting notes: starship_catan (Go) -> starship-catan-1 (Rust)

## Bug fixes (authorized divergences from Go)

1. **`Winners()` always returned player 0 (the headline fix).** Go
   `starship_catan.go:178-191` reads:

   ```go
   switch {
   case p1 > p2:
       return []string{g.Players[0]}
   case p2 > p1:
       return []string{g.Players[0]} // BUG: should be Players[1]
   default:
       return g.Players
   }
   ```

   Both non-tie branches return `Players[0]`, so in Go player 0 always wins
   unless the VP totals tie. The Rust port implements the correct logic in
   `Game::status` (`src/lib.rs`) via `gen_placings`: `p1 > p2` => player 0
   (placings `[1, 2]`), `p2 > p1` => player 1 (placings `[2, 1]`), tie => both
   (`[1, 1]`). This fix was explicitly authorized; no Go test covered
   `Winners()`, so nothing forced the old behaviour. Regression-tested by
   `tests::winner_when_player_ahead`, `tests::winner_when_player_b_ahead`, and
   `tests::tie_game`.

2. **`Transaction::Lose()` returned the whole transaction.** Go
   `transaction.go:94-102` builds a `lose` map of the negative entries but then
   `return t` (line 101) - returning the original transaction, not the filtered
   map. The Rust `Transaction::lose` (`src/lib.rs`) returns only the negative
   entries, which is what `PlayerBoard::can_afford` (via `can_fit(t.lose())`)
   relies on. Go happened to work anyway because `can_fit`/`fit_transaction`
   floors every resource at 0, so the spurious positive entries were neutralised
   downstream; the correct filter is ported regardless.

## RNG and dice

- **Die range.** Go rolls `(r.Int() % 3) + 1` (`starship_catan.go:330` for the
  yellow die, `fight_command.go:56,58` for the pirate and player fight rolls),
  giving a value in {1, 2, 3}. Rust uses `GameRng::random_range(1..=3)`.
- **The RNG is entirely different.** Go draws from its ambient `math/rand`
  source; Rust uses a seeded `GameRng` (ChaCha8) stored on `Game` and seeded
  from the `start` seed. Intra-game determinism is preserved (every
  shuffle/roll draws from `self.rng` in the same order as Go), but the actual
  random values are NOT comparable across the port. Per the render-parity rule,
  no test asserts on random values and no render is compared against Go output.

## Naming and representation

- **Crate name `starship-catan-1`.** Named `-1` per explicit orchestrator
  instruction. The porting survey recommended `-2` (counting the Go game as
  v1); `-1` was directed, so this doc and the crate use it.
- **`Resource::Ore` colour.** Go colours ore "black"; `brdgme_color::NamedColor`
  has no `Black`, so `Foreground` is used (`src/card.rs`). Go's "magenta" for
  science maps to `NamedColor::Purple`. Neither is a render-parity blocker.
- **`TradeDir` is a plain enum, not an int discriminant.** Go uses
  `TradeDirBoth=0, TradeDirBuy=1, TradeDirSell=-1`. Rust stores
  `TradeDir::{Both, Buy, Sell}` and computes the sign via `TradeDir::sign`
  (`src/card.rs`), avoiding a negative serde discriminant.

## Command-surface folding

- **Dynamic per-card commands.** Go's `Commander` interface (each of
  `ColonyCard`/`TradeCard`/`PirateCard`/`MedianCard`/`AdventurePlanetCard`
  exposes its own `Commands()`) is folded into `Game::command_parser`
  (`src/command.rs`): a `OneOf` whose first entries are gated on
  `flight_cards.last()` (the current planet) and the relevant `can_*` guard,
  followed by the static command parsers in Go's order.
- **Shared-name commands collapsed.** Go has distinct command structs that share
  a name - `found` (FoundColony + FoundTrade), `buy` (flight Buy +
  TradePhaseBuy), `sell` (flight Sell + TradePhaseSell). Rust collapses each to
  a single `Command` variant: `Found` dispatches on the current card
  (`can_found_colony` vs `can_found_trading_post`), and `Buy`/`Sell` are offered
  once, gated by `can_buy`/`can_sell` whose `can_trade` already spans both the
  flight and trade-and-build phases.

## Other decisions

- **Placings ties.** `gen_placings` yields standard-competition placings
  (`[1, 1, 3]` for a two-way tie among three) where Go's compact-ordinal
  `Placings()` would give `[1, 1, 2]`. This game only ever has two players, so a
  tie is `[1, 1]` under both schemes; no Go test asserted a tie, so no
  adaptation was forced. Noted for completeness.
- **`lose` removes the zeroed module key.** `Game::lose` (`src/lib.rs`) deletes
  a module from the `BTreeMap` when its level drops to 0, whereas Go keeps the
  key with value 0. Functionally identical - `module()` treats an absent key as
  level 0 - and keeps the map tidy for the `modules.is_empty()` choose-phase
  check.
- **`PubState`/`PlayerState` are structural redactions.** `PubState`
  (`src/render.rs`) carries only public information plus count-only fields
  (`adventure_deck_len`, `sector_pile_lens`, `sector_draw_pile_len`); the sector
  pile contents, the sector draw pile, the adventure deck beyond the public
  top-3, and the sensor `peeking` cards are absent fields (peeking lives only in
  `PlayerState`). Serialization therefore cannot leak them - verified by
  `tests::pub_state_does_not_leak_hidden_info`, which walks the serialized JSON
  keys and asserts the hidden fields are absent and the count fields are
  numbers.
- **Logs returned, not pushed.** Every action returns a `Vec<Log>` that bubbles
  up through `command()`, mirroring Go's `g.Log.Add(...)` ordering (action ->
  consequences -> next turn) using `Node` trees instead of `{{b}}`/`{{c}}`
  string markup.
- **`render.Table(_, 0, 2)` -> `table_with_gap(&rows, 2)`.** Go's `colSpacing=2`
  is reproduced with `brdgme_markup::table_with_gap`, the known category-5-2
  gotcha.

## Tests

All Go test cases are ported 1:1 with snake_cased names: `start`,
`choose_module` (`starship_catan_test.go`); the deck-count tests
`sector_base_cards`, `sector1_cards`..`sector4_cards`, `shuffled_sector_cards`,
`adventure1_cards`..`adventure4_cards`, `shuffled_adventure_cards`
(`card_test.go` - the Go "is an Adventurer" type assertion is meaningless for a
Rust enum and is dropped, keeping the count); and `parse_module`
(`module_test.go`, exercised through the `choose` command parser's
`Enum::partial(Module::ALL)` prefix matching). Added per the brief: the three
scoring/winner tests (including the `winner_when_player_b_ahead` regression test
for the `Winners()` bug) and `pub_state_does_not_leak_hidden_info`. The
`tests/contract.rs` gamer-contract test passes.
