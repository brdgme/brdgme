# Cathedral

## Overview

Cathedral is a 2-player abstract area-control game on a 10x10 board. Player 1
opens by placing the neutral Cathedral piece, then both players alternate
placing their remaining polyomino-shaped pieces onto open squares. Placing a
piece can seal off empty regions of the board, claiming them as territory or
even capturing an opponent's pieces if their shapes are fully enclosed. The
game ends once neither player can place anything at all, and whoever has the
fewest tiles left in hand wins - so every placement is a trade-off between
claiming space and keeping enough small pieces in reserve to keep fitting into
the shrinking board.

## Components

**Board.** A 10x10 grid, columns 1-10 left to right, rows A-J top to bottom
(e.g. `A1` is the top-left corner, `J10` the bottom-right).

**The Cathedral.** One neutral 6-cell piece, belonging to neither player.
Player 1 must place it before playing anything else. Once placed, it is never
returned to a hand (there is no hand for it), but the territory around and
under it can still change hands through captures like any other tile.

**Player pieces.** Each player has 14 of their own pieces (distinct
polyomino shapes), sized 5, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 1, 1 tiles. Every
piece has a fixed shape but can be placed in any of 4 rotations.

## Turn Structure

### 1. Player 1 places the Cathedral first

Before anything else can happen, player 1 must place the neutral Cathedral
piece somewhere on the board (`play 1 e5 down` - piece `1` is always the
Cathedral for player 1). No other piece of player 1's may be played until this
happens. Placing the Cathedral does not use up player 1's turn - player 1
immediately gets to place a normal piece as well, before play passes to
player 0.

### 2. Place a piece

On your turn, place one of your unplayed pieces with
`play <piece> <loc> [<dir>]`:

- `<piece>` is the piece's number (1-based, as listed in your remaining
  tiles).
- `<loc>` is the target square for the piece's origin cell, e.g. `e5`.
- `<dir>` is optional and rotates the piece: `up`, `right`, `down` or `left`.
  If omitted, the piece is placed in its default `down` orientation.

Examples: `play 3 c5`, `play 7 h2 right`.

Every cell the piece would occupy must be:
- On the board.
- Empty (not occupied by another piece).
- Either unclaimed, or already claimed as your own territory (you may build
  inside your own claimed area, but never inside the opponent's).

### 3. Captures resolve automatically

After you place a (non-Cathedral) piece, once the Cathedral has been placed,
the game checks whether your new piece has sealed off a region. Starting from
your new piece and walking outward through your own contiguous pieces, any
enclosed region not belonging to you that contains **at most one** distinct
piece identity is captured. The Cathedral counts as a piece identity for
this limit exactly like an opponent piece - a region containing the
Cathedral plus one opponent piece has two distinct identities and is NOT
captured.

When a region is captured:

- Every cell in that region becomes your claimed territory - including any
  Cathedral tiles, whose ownership flips like any other captured tile.
- A captured opponent piece is returned to its owner's hand, fully unplayed
  and free to be played again later in the game.
- The Cathedral's only special treatment is on this return step: it is
  never returned to a hand (there is no hand for it) and never counts
  toward the "pieces returned" totals in the capture log.

If the enclosed region contains **two or more** distinct piece identities
(counting the Cathedral), nothing is captured and the region is left
untouched.

### 4. Turn passes

Turns alternate between the two players, with one exception: if your
opponent has no legal move anywhere on the board, you keep taking turns
until they do. Once the whole board has been touched (no fully open, unowned
tile remains anywhere), both players may place pieces simultaneously
whenever they individually have a legal move, instead of strictly
alternating.

## Scoring

Your score is the total tile-count of every piece you still have unplayed in
hand - fewer is better. This is recalculated live throughout the game (it is
not accumulated separately from placements).

Worked example, partway through a game:

| Player | Unplayed pieces (sizes) | Remaining piece size |
|---|---|---|
| Player 0 (played sizes 5, 5 so far) | 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 1, 1 | **37** |
| Player 1 (played Cathedral size 6, and sizes 5, 5) | 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 1, 1 | **37** |

If the game ended here, both players would tie on remaining piece size.

## Rounds / Game End

There are no rounds - it is one continuous game. Two distinct phases happen
in sequence:

1. **Alternating phase.** Players alternate placing pieces (subject to the
   Cathedral-first and stuck-opponent rules above) until every tile on the
   board has been touched - either occupied by a piece or claimed as
   territory. At that point the game announces "No open tiles remain,
   players will play the rest of their pieces simultaneously" and switches
   to simultaneous mode.
2. **Simultaneous end-phase.** Both players may place pieces whenever they
   have a legal move, in any order, until neither player can place anything
   at all (every remaining piece is either played or has no legal spot
   left). At that point the game ends immediately and announces each
   player's final remaining piece size.

## Winning

The player with the **smallest** remaining piece size (fewest tiles left
unplayed) wins. Ties are shared.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `play <piece> <loc>` | Place a piece (default `down` orientation) | `play 3 c5` |
| `play <piece> <loc> <dir>` | Place a piece rotated `up`, `right`, `down` or `left` | `play 7 h2 right` |
