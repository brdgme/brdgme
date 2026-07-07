# Battleship

A 2-player game of hidden ship placement and targeted shooting. Each player places 5 ships on their own 10x10 grid, then players take turns shooting at coordinates on the opponent's grid. The first player to sink all of the opponent's ships wins.

## Ships

| Ship | Size |
|---|---|
| Carrier | 5 |
| Battleship | 4 |
| Cruiser | 3 |
| Submarine | 3 |
| Destroyer | 2 |

## Setup

Both players place their 5 ships on their own 10x10 grid. Ships are placed by specifying the ship name, a starting location (e.g. B3), and a direction (up, down, left, right). Ships cannot overlap or go off the board. Both players place simultaneously - the game transitions to shooting once both have finished placing.

## Shooting

Players take turns shooting at a location on the opponent's board (e.g. "shoot B3"). A shot that hits a ship cell marks it as a hit; a shot at an empty cell marks it as a miss. When every cell of a single ship has been hit, that ship is sunk. You cannot shoot at the same location twice.

## Ending and Winning

The game ends when one player has all of their ship cells hit (zero remaining). The player with the most ship cells remaining wins. Placings are by ship cells remaining, highest first; ties share a place.

## Commands

| Command | Action |
|---|---|
| `place <ship> <location> <direction>` | Place a ship on your board during setup |
| `shoot <location>` | Shoot at a location on the opponent's board |

Ship names can be abbreviated to any unambiguous prefix (e.g. "sub" for submarine, "bat" for battleship). Locations are a letter A-J followed by a number 1-10 (e.g. B3, J10). Directions can be abbreviated (e.g. "u" for up, "r" for right).

## Reading the Display

- **Your board** - your own grid with ships shown as grey blocks, hits as red, misses as grey XX
- **Enemy board** - the opponent's grid with only hits and misses visible (ships hidden until the game ends)
- **Ships left to place** - during setup, the ships you still need to place with their sizes in brackets
