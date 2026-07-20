# Sushizock Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in this game (2 to 5).
- `current_player` (usize): Index of the player whose turn it is.
- `blue_tiles` (Vec<Tile>): Blue tiles remaining in the central row, in order. Each tile has `kind: Blue` and a positive `value` (1 through 6). Two tiles of each value exist at game start.
- `red_tiles` (Vec<Tile>): Red tiles remaining in the central row, in order. Each tile has `kind: Red` and a negative `value` (-1 through -4). At game start: five -1s, four -2s, two -3s, one -4.
- `player_blue_tiles` (Vec<Vec<Tile>>): Blue tiles collected by each player, indexed by player. The order matters: stealing with 3 chopsticks takes the top (last) tile, stealing with 4+ chopsticks can target a specific position.
- `player_red_tiles` (Vec<Vec<Tile>>): Red tiles collected by each player, indexed by player. Same ordering rules as blue tiles.
- `rolled_dice` (Vec<DieFace>): Dice currently showing from the latest roll (not yet kept). Empty after the final roll when all dice are consolidated.
- `kept_dice` (Vec<DieFace>): Dice set aside during re-rolls. These are locked and will not be re-rolled.
- `remaining_rolls` (i32): Number of re-rolls remaining this turn. Starts at 2 (initial roll + 2 re-rolls = 3 total rolls).
- `finished` (bool): True when both the blue and red tile piles are empty and the game is over.
- `final_scores` (Vec<i32>): Final scores for each player. Only populated when `finished` is true; empty vec during play.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.

## Tile

- `kind` (TileType): Either `Blue` (positive) or `Red` (negative).
- `value` (i32): Point value. Blue tiles: 1 through 6. Red tiles: -1 through -4.

## DieFace enum

- `Sushi`: Used to take blue tiles. N sushi lets you take the Nth blue tile from the row.
- `Bones`: Used to take red tiles. N bones lets you take the Nth red tile from the row.
- `BlueChopsticks`: 3+ lets you steal a blue tile from an opponent. 4+ lets you steal a specific blue tile.
- `RedChopsticks`: 3+ lets you steal a red tile from an opponent. 4+ lets you steal a specific red tile.

## Scoring

Score = sum of red tile values + sum of blue tile values, but only the first N blue tiles score, where N is the number of red tiles you hold. Extra blue tiles beyond your red tile count do not score.
