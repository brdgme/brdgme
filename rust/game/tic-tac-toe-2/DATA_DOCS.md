# Tic-tac-toe Data Dictionary

## PubState (public information)

- `players` (usize): Number of players (always 2).
- `current_player` (usize): Index (0 or 1) of the player whose turn it is.
- `start_player` (usize): Index (0 or 1) of the player who goes first. The starting player plays as X.
- `board` (Board): The 3x3 game board, a 2D array of cells. Each cell is `Empty`, `X`, or `O`. Positions are labeled a-i in row-major order (a=top-left, i=bottom-right).

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0 or 1) this private state belongs to.

## Cell enum

- `Empty`: No mark in this cell.
- `X`: Marked by the starting player.
- `O`: Marked by the second player.

## Board layout

Positions are labeled a through i:
```
a | b | c
d | e | f
g | h | i
```
