# Lords of Vegas

## Overview

Players build casinos on the Las Vegas Strip, competing to control the
biggest, most valuable properties. This version of the game plays out the
building phase: everyone starts owning a couple of building lots, and takes
turns constructing casinos on the lots they own.

---

## The Strip

The board is six blocks, laid out left to right, top to bottom: **A**, **B**,
**C**, **D**, **E**, **F**. Each block is a grid of building lots, 3 lots
wide. Block C is the largest with 12 lots (4 rows); blocks A, B and E have 6
lots each (2 rows); blocks D and F have 9 lots each (3 rows). A lot is
identified by its block letter and lot number, e.g. **A1**, **C8**, **F9**.

Two lots are neighbours if they sit directly above/below or left/right of
each other within the same block (no diagonals, and blocks don't connect to
each other).

Every lot has a fixed **die value** (1-6) that never changes, and a build
cost and starting cash value that are entirely determined by that die value:

| Die | Build cost | Starting cash |
|-----|-----------|---------------|
| 1   | $8        | $9            |
| 2   | $6        | $8            |
| 3   | $9        | $7            |
| 4   | $12       | $6            |
| 5   | $15       | $5            |
| 6   | $20       | $4            |

Cheap-to-build, high-die lots (die 6) pay out the least starting cash;
expensive-to-build, low-die lots (die 1) pay out the most. Each lot is also
tagged with one of the five casinos below as its "home" casino (used only for
the casino card count on the display), or as a **strip** lot which isn't
tied to any casino.

---

## Casinos

There are five casinos, each with its own colour:

| Casino  | Colour        |
|---------|---------------|
| Albion  | Purple        |
| Sphinx  | Tan/olive     |
| Vega    | Green         |
| Tivoli  | Grey          |
| Pioneer | Brick red     |

A casino on the board is one contiguous group of built lots of the same
colour (regardless of who built them) at the same building height. Building
a lot of a colour next to an existing casino of that same colour joins it
into that casino - casinos can span lots owned and built by different
players.

---

## Setup

Each player is dealt 2 lots at random and takes ownership of them
immediately - no card is drawn or placed on the board yet. Their starting
cash is the sum of the starting cash values of their two lots. A random
player starts the game.

---

## Turn structure

On your turn you may **build** any number of times (once per lot you own),
then you must end your turn:

### Build

`build <lot> <casino>` - build a casino of the given colour on a lot you own
that hasn't been built on yet, e.g. `build C8 Albion`. You must have at least
the lot's build cost in cash; the cost is deducted immediately. The lot's
built tile is now marked with your fixed die value as its owner marker.

If your new tile joins one or more existing casinos of the same colour into
a single casino, and this creates a tie for the highest die value among the
tied players in that casino, all the tied tiles are **rerolled** (a fresh
random die 1-6 each) until there is a single highest value again - this can
cascade into further ties, which are resolved the same way.

### Done

`done` - end your turn once you're finished building. Play passes to the
next player in turn order.

---

## Implementation status

This version implements building and turn-passing only. It does not yet
implement sprawling a casino to an adjacent lot, remodelling a casino's
colour, reorganising (rerolling) a casino you have a die in, gambling at an
opponent's casino, raising a casino's height, drawing new lots during play,
scoring, or an end-of-game trigger - the game runs indefinitely and points
always show as 0. These will be added in a future version.

---

## Command reference

| Command                    | Action                                  | Example              |
|-----------------------------|------------------------------------------|-----------------------|
| `build <lot> <casino>`      | Build a casino on a lot you own          | `build C8 Albion`     |
| `done`                      | End your turn                            |                       |

Casino names can be abbreviated to a unique prefix (e.g. `Alb` for Albion,
`Sp` for Sphinx).
