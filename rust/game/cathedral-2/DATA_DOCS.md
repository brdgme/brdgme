# Cathedral Data Dictionary

## PubState (public information)

- `players` (usize): Number of players (always 2).
- `board` (HashMap<String, Tile>): Board state keyed by location string (e.g. "A1" through "J10"). Each Tile has:
  - `player` (i32): Which player's piece occupies this cell (0, 1, or 2 for Cathedral). -1 (NO_PLAYER) means no piece.
  - `typ` (i32): Piece type number (1-15). 0 for empty cells.
  - `owner` (i32): Which player claims this cell as territory. -1 (NO_PLAYER) means unclaimed.
  - `text` (String): Display text (unused in game logic).
- `played_pieces` (Vec<Vec<bool>>): Which pieces have been played, indexed by [player][piece_index]. Player 0 has 14 pieces (indices 0-13), player 1 has 15 pieces (indices 0-14, where index 0 is the Cathedral).
- `current_player` (usize): Index of the player whose turn it is (0 or 1).
- `no_open_tiles` (bool): True when no open (unowned, unoccupied) tiles remain anywhere on the board. When true, both players may place pieces simultaneously.
- `finished` (bool): True when neither player can place any more pieces and the game is over.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this state belongs to.

## Tile

- `player` (i32): Occupying player. 0 = player 0's piece, 1 = player 1's piece, 2 = Cathedral (neutral), -1 = empty.
- `typ` (i32): Piece type number. 1-15 for actual pieces, 0 for empty cells.
- `owner` (i32): Territory owner. 0 = player 0's territory, 1 = player 1's territory, -1 = unclaimed.
- `text` (String): Display text (not used in game logic).

## Board coordinates

Columns 1-10 (left to right), rows A-J (top to bottom). Location "A1" is top-left, "J10" is bottom-right.

## Pieces

Player 0 has 14 pieces (sizes: 5, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 1, 1).
Player 1 has 15 pieces: the Cathedral (size 6, piece index 0) plus 14 pieces (sizes: 5, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 1, 1).
