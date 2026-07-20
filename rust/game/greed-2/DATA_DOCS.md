# Greed Data Dictionary

## PubState (public information)
- `players` (usize): number of players
- `current_player` (usize): index of the player currently taking their turn
- `first_player` (usize): index of the player who went first this game
- `scores` (Vec<i32>): banked score for each player
- `turn_score` (i32): points accumulated in the current turn (not yet banked)
- `remaining_dice` (Vec<Die>): dice still available to roll this turn (Die: Dollar, G, R, E1, E2, D)
- `finished` (bool): whether the game has ended
- `placings` (Vec<usize>): final standings once finished (empty while active)

## PlayerState (player-private information)
- `public` (PubState): the full public game state (Greed has no hidden information per player)
