# Tic-tac-toe

## Overview

Tic-tac-toe is a 2-player game played on a 3x3 board. Players alternate placing marks, aiming to make a horizontal, vertical, or diagonal line of three.

## Components

- One 3x3 board with squares labelled `a` through `i` in row-major order.
- Two marks: `x` and `o`.
- The randomly selected starting player uses `x`; the other player uses `o`.

## Turn Structure

1. The game randomly selects the starting player, who plays as `x`.
2. On your turn, place your mark in any empty square with `play <square>` (for example, `play a` or `play e`). Square letters are case-insensitive.
3. Play passes to the other player after every successful move, including a move that ends the game.

You cannot play outside squares `a` through `i`, play in an occupied square, or play when it is not your turn.

## Game End

The game ends immediately when one player has three matching marks in a horizontal, vertical, or diagonal line. It also ends as a draw when all nine squares are occupied without a winning line.

## Winning

The player with a line of three places first and receives 1 point. The other player places second and receives 0 points. In a draw, both players share first place and receive 0 points.

## Commands

| Command | Action | Example |
|---|---|---|
| `play <square>` | Place your mark in an empty square from `a` through `i` | `play e` |
