# Red7 Data Dictionary

## PubState (public information)

- `num_players` (usize): Number of players in the game (2-4).
- `current_player` (usize): Index of the player whose turn it is.
- `deck_len` (usize): Number of cards remaining in the draw deck.
- `discard_pile` (Vec<Card>): The discard pile. The suit of the top (last) card determines the current winning rule. If empty, the default rule is Red (highest card).
- `hand_sizes` (Vec<usize>): Number of cards in each player's hand, indexed by player position.
- `palettes` (Vec<Vec<Card>>): Cards each player has played to their palette this round, indexed by player. The current leader is determined by comparing palettes under the active rule.
- `scored_cards` (Vec<Vec<Card>>): Cards each player has accumulated from winning rounds, indexed by player. Points are the sum of card ranks.
- `eliminated` (Vec<bool>): Whether each player has been eliminated this round, indexed by player. Eliminated players cannot act until the next round.
- `finished` (bool): True when the game is over (a player reached the target score or the deck ran out).

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0-indexed) this private state belongs to.
- `hand` (Vec<Card>): The cards currently in this player's hand.

## Card

- `suit` (Suit): One of Red, Orange, Yellow, Green, Blue, Indigo, Violet.
- `rank` (u8): Number 1 through 7.

## Suit enum and rules

- Red: highest single card wins.
- Orange: most cards of one number wins.
- Yellow: most cards of one color (suit) wins.
- Green: most even cards wins (ties broken by highest even card).
- Blue: most cards of different colors wins.
- Indigo: most cards that form a run (consecutive ranks) wins.
- Violet: most cards below rank 4 wins.

Ties within a rule are broken by the highest card in the winning set, then by the highest card overall in the palette.
