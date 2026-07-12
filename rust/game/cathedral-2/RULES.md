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
Player 1 must place it before playing anything else. Once placed, it can never
be captured or moved, but the territory around and under it can still change
hands through captures like any other tile.

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

## Reading the Display

Real render captured from the `cathedral_2_cli` binary partway through a
2-player game (player 0 has placed pieces 1 and 2; player 1 has placed the
Cathedral plus pieces 2 and 3):

```brdgme
{{table}}{{row}}{{cell left}}{{b}}+{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}+{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A2{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A3{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A5{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A6{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}A9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}A10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B2{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B3{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B6{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}B9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}B10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}C1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}C2{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(2) | mono | inv}}{{bg player(2)}}+----+
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}C4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
+    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}C7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}C8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}C9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}C10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}D1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}D2{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(2) | mono | inv}}{{bg player(2)}}|    |
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}D4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}D5{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}D8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}D9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}D10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}E1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(2) | mono | inv}}{{bg player(2)}}+-----
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(2) | mono | inv}}{{bg player(2)}}+    +
      
+    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(2) | mono | inv}}{{bg player(2)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}E6{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}E7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}E8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}E9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}E10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}F1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}F2{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(2) | mono | inv}}{{bg player(2)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+-----
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+    |
     |
+    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}F6{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}F7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}F8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}F9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}F10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G2{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G3{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}G9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}G10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H3{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H6{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}H9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}H10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}I1{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
|    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}I4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
+    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}I7{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}I8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}I9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}I10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+-----
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+    |
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}J3{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}J4{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}J5{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}J8{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
  {{b}}{{fg rgb(97,97,97)}}J9{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}      
 {{b}}{{fg rgb(97,97,97)}}J10{{/fg}}{{/b}}  
{{/fg}}{{/cell}}{{cell left}}{{b}}|
|
|{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}+{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}------{{/b}}{{/cell}}{{cell left}}{{b}}+{{/b}}{{/cell}}{{/row}}{{/table}}

All pieces are shown in their {{b}}down{{/b}} position and pivot around the number.{{b}}

{{player 0}} remaining tiles:
{{/b}}
{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 3  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+-----
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+    +
      
+    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+-----
| 4   
|    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 5  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
|    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 6  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
+    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+-----
| 7   
|     {{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
     |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|     
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}     |
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 8  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 9  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 10 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 11 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 12 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 13 |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(0) | mono | inv}}{{bg player(0)}}+----+
| 14 |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{b}}

{{player 1}} remaining tiles:
{{/b}}
{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 4  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+-----
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+    +
      
+    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+-----
| 5   
|    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 6  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
|    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 7  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+-----
|     
|    +{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+    |
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+-----
| 8   
|     {{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
     |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|     
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}     |
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 9  |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 10 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 11 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    +
|     
+-----{{/bg}}{{/fg}}{{/b}}{{/cell}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}-----+
     |
-----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 12 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 13 |
|    |{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}|    |
|    |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 14 |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{b}}{{fg player(1) | mono | inv}}{{bg player(1)}}+----+
| 15 |
+----+{{/bg}}{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
```

The board is a 10x10 grid of `A1`-`J10` cells wrapped in bold box-drawing
borders. Each cell is a 6-wide, 3-tall block:

- **Empty, unclaimed cells** show their loc label (e.g. `A1`) centred in
  grey.
- **Empty, claimed cells** show their loc label centred and bold in the
  claiming player's colour - this is territory with no piece sitting on it.
- **Occupied cells** are filled with the owning player's colour as a solid
  background block. Adjoining cells belonging to the *same* piece (same
  player and same piece number) have their shared wall rendered as blank
  background instead of a box-drawing character, so a multi-cell piece
  reads as one continuous coloured shape rather than a grid of separate
  boxes. Cathedral tiles use a third, neutral colour.

Below the board, an instructional line reminds you that every piece shown in
the catalogues below is drawn in its default `down` orientation and rotates
around its numbered origin cell.

Each player then gets a "remaining tiles" catalogue (your own always
listed first, then your opponent's - both are always fully visible, since
Cathedral has no hidden information for anyone). Each unplayed piece is shown
as its own coloured shape with its 1-based piece number printed on its origin
cell, wrapped onto new rows once a row's total width would exceed 10 columns.
If a player has no unplayed pieces left, `None` is shown instead of a
catalogue.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `play <piece> <loc>` | Place a piece (default `down` orientation) | `play 3 c5` |
| `play <piece> <loc> <dir>` | Place a piece rotated `up`, `right`, `down` or `left` | `play 7 h2 right` |

## Strategy Tips

Tips will be added.
