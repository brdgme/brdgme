# Lost Cities Data Dictionary

## PubState (public information)

- `round` (usize): Current round number, 1 through 3.
- `is_finished` (bool): True when all 3 rounds are complete and the game is over.
- `phase` (Phase): Current turn phase. `PlayOrDiscard` means the active player must play or discard a card. `DrawOrTake` means the active player must draw from the deck or take from a discard pile.
- `deck_remaining` (usize): Number of cards left in the draw pile. When this hits 0, the round ends immediately.
- `discards` (HashMap<Expedition, Value>): The top (most recently discarded) card value on each expedition's shared discard pile. Only expeditions with at least one discarded card appear. A player may take the top card from any of these piles.
- `scores` (Vec<Vec<isize>>): Scores indexed by player (0 or 1), then by round. Each inner vec has one entry per completed round. Sum across rounds for the cumulative score.
- `expeditions` (Vec<Vec<Card>>): Cards played to expeditions, indexed by player (0 or 1). Each inner vec contains all cards that player has played across all five expeditions, in play order. Cards have an `expedition` field (Red, Green, White, Blue, Yellow) and a `value` field (Investment or N(2..10)).
- `current_player` (usize): Index (0 or 1) of the player whose turn it is.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0 or 1) this private state belongs to.
- `hand` (Vec<Card>): The cards currently in this player's hand. Each card has an `expedition` (Red, Green, White, Blue, Yellow) and a `value` (Investment, or N with a number 2 through 10). Cards are sorted by expedition then value.

## Card

- `expedition` (Expedition): One of Red, Green, White, Blue, Yellow.
- `value` (Value): Either `Investment` (wager card, shown as X) or `N(n)` where n is 2 through 10.

## Expedition enum

Red, Green, White, Blue, Yellow - the five expedition routes.

## Phase enum

- `PlayOrDiscard`: First phase of a turn. The active player must play a card to an expedition or discard it.
- `DrawOrTake`: Second phase of a turn. The active player must draw from the deck or take the top card from a discard pile.
