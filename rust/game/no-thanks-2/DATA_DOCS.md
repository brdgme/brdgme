# No Thanks! Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in this game (3 to 5).
- `finished` (bool): True when the deck is empty and the game is over.
- `current_card` (Option<i32>): The current card on the table (3 to 35), or None if the game is finished.
- `remaining_after` (usize): Number of cards remaining in the deck after the current card. When 0, taking the current card ends the game.
- `centre_chips` (i32): Number of chips accumulated on the current card from players passing.
- `hands` (Vec<Vec<i32>>): Cards collected by each player, indexed by player. Cards are numbered 3 to 35. Consecutive cards form runs where only the lowest counts for scoring.
- `chips` (Vec<i32>): Chips held by each player, indexed by player. Only populated when the game is finished; empty vec during play (chip counts are hidden).
- `final_scores` (Vec<i32>): Final scores for each player (card score minus chips). Only populated when the game is finished; empty vec during play. Lower is better.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.
- `chips` (i32): Number of chips this player currently holds. This is always visible to the player, unlike the public `chips` field which is hidden until game end.
