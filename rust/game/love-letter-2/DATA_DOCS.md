# Love Letter Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in the game, 2 through 4.
- `deck_remaining` (usize): Number of cards left in the draw pile. When this hits 0, the round ends after the current turn and the highest remaining hand wins.
- `discards` (Vec<Vec<Card>>): Cards each player has discarded (played) this round, indexed by player. Each inner vec lists that player's discards in the order they were played. Reveals which cards are no longer in play.
- `player_points` (Vec<usize>): Points accumulated toward winning, indexed by player. A player wins the game on reaching `end_score`.
- `current_player` (usize): Index of the player whose turn it is.
- `eliminated` (Vec<bool>): Whether each player is eliminated from the current round, indexed by player. Eliminated players take no further turns and cannot be targeted.
- `protected` (Vec<bool>): Whether each player is protected by the Handmaid until the start of their next turn, indexed by player. Protected players cannot be targeted by other players' cards.
- `end_score` (usize): Points required to win the game: 7 for 2 players, 5 for 3, 4 for 4.
- `leader_points` (usize): The highest point total held by any player. Compare against `end_score` to gauge how close the game is to ending.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.
- `hand` (Vec<Card>): The cards currently in this player's hand. Normally one card between turns, two on your own turn after drawing.

## Card enum

Cards are numbered 1 (lowest) to 8 (highest). The number decides round-end ties and Baron comparisons.

- `Guard` (1): Guess another player's card (not Guard) to eliminate them if correct. 5 copies.
- `Priest` (2): Look at another player's hand. 2 copies.
- `Baron` (3): Compare hands with another player; the lower card is eliminated. 2 copies.
- `Handmaid` (4): You are immune to other players' cards until your next turn. 2 copies.
- `Prince` (5): Choose a player (or yourself) to discard their hand and draw a new card. 2 copies.
- `King` (6): Trade your hand with another player. 1 copy.
- `Countess` (7): No effect on play, but must be discarded if you also hold the King or Prince. 1 copy.
- `Princess` (8): You are eliminated if you discard the Princess. 1 copy.
