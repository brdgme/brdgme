# Battleship Data Dictionary

## PubState (public information)
- `players` (usize): number of players in the game
- `phase` (Phase): current phase - Placing or Shooting
- `current_player` (usize): index of the player whose turn it is
- `boards` (Vec<Board>): each player's board as seen by opponents (hits, misses, unknown cells - no ship positions revealed until the game ends)
- `left_to_place_counts` (Vec<usize>): number of ships each player still needs to place
- `finished` (bool): whether the game has ended
- `placings` (Vec<usize>): final standings once finished (empty while game is active)

## PlayerState (player-private information)
- `public` (PubState): the full public game state
- `player` (usize): this player's seat index
- `board` (Board): this player's own board including ship positions (private)
- `left_to_place` (Vec<Ship>): ships this player still needs to place during the Placing phase
