# Farkle Data Dictionary

## PubState (public information)
- `players` (usize): number of players in the game
- `current_player` (usize): index of the player currently taking their turn
- `first_player` (usize): index of the player who went first this game
- `scores` (Vec<i32>): banked score for each player
- `turn_score` (i32): points accumulated in the current turn, not yet banked
- `remaining_dice` (Vec<Die>): dice still available to roll this turn (Die values 1-6)
- `finished` (bool): whether the game has ended
- `placings` (Vec<usize>): final standings once finished (empty while active)

## PlayerState (player-private information)
- `public` (PubState): the full public game state (Farkle has no hidden information per player)
