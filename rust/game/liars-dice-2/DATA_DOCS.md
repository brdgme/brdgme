# Liar's Dice Data Dictionary

## PubState (public information)
- `players` (usize): total number of players in the game (including eliminated)
- `current_player` (usize): index of the player whose turn it is to bid or call
- `bid_quantity` (i32): quantity (number of dice) in the current bid; 0 if no bid yet this round
- `bid_value` (i32): face value (1-6) in the current bid; 0 if no bid yet this round
- `bid_player` (usize): index of the player who made the current bid
- `remaining_dice` (Vec<usize>): number of dice each player still has, indexed by seat

## PlayerState (player-private information)
- `public` (PubState): the full public game state visible to all players
- `player` (usize): this player's seat index
- `dice` (Vec<u8>): values of this player's dice (private, each 1-6; 1s are wild)
