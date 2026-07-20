# Tic-tac-toe - Basic Strategy

Hard rules to avoid obviously bad moves.

## Opening

- Always take the center (e) if available on your first move. The center is part of 4 winning lines.
- If the center is taken, take a corner (a, c, g, or i). Corners are part of 3 winning lines each.
- Do not open on an edge (b, d, f, or h) unless all corners and the center are taken. Edges are part of only 2 winning lines.

## Blocking

- If your opponent has two in a row with an empty third cell, you must block unless you have an immediate win yourself.
- Check all 8 winning lines (3 rows, 3 columns, 2 diagonals) for threats every turn.

## Winning

- If you have two in a row with an empty third cell, take the win immediately.
- Always check for an immediate win before blocking.

## General

- Do not play in an occupied cell.
- The game is a draw if all 9 cells are filled with no winner. With perfect play, tic-tac-toe always draws.
