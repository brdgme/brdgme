# Sushizock

A 2-5 player dice and tile game. Roll dice to take blue tiles (positive points) or red tiles (negative points), or steal tiles from opponents with chopsticks. Highest score wins.

## Setup

- 12 blue tiles: two each valued 1 through 6, shuffled face down.
- 12 red tiles: five -1s, four -2s, two -3s, one -4, shuffled face down.
- 5 dice with 6 faces: 2 sushi, 2 bones, 1 blue chopsticks, 1 red chopsticks.
- Each player rolls 5 dice to start their turn.

## Turn

On your turn you roll the dice up to 3 times total (the initial roll plus 2 re-rolls). After each roll you may keep some dice and re-roll the rest.

- **roll `<die numbers>`** - re-roll the dice at the given positions (1-indexed). You must keep at least one die. Dice not listed are kept.

After your final roll (or when only one die remains un-kept), all dice are consolidated. You must then either take a tile or steal from an opponent. If you cannot, you are forced to take the worst available tile.

## Taking Tiles

- **take blue** - if your sushi dice count is N, take the Nth blue tile from the row (1-indexed from the left). Requires at least 1 sushi and enough blue tiles.
- **take red** - if your bones dice count is N, take the Nth red tile from the row. Requires at least 1 bones and enough red tiles.

## Stealing Tiles

With 3 or more chopsticks of one color, you can steal the top tile from an opponent's stack of that color:

- **steal `<player> blue** - steal the top blue tile from another player (requires 3+ blue chopsticks).
- **steal `<player> red** - steal the top red tile from another player (requires 3+ red chopsticks).

With 4 or more chopsticks, you can steal a specific tile from deeper in their stack:

- **steal `<player> blue `<n>** - steal the nth blue tile from the top (1 = top). Requires 4+ blue chopsticks.
- **steal `<player> red `<n>** - steal the nth red tile from the top. Requires 4+ red chopsticks.

## Forced Take

If after your final roll you cannot take or steal any tile, you are forced to take the most negative red tile (lowest value), or if no red tiles remain, the lowest blue tile.

## Scoring

Score = sum of all your red tile values + sum of your blue tile values, but only for the first N blue tiles where N is your number of red tiles. Extra blue tiles beyond your red tile count do not score.

Example: 3 red tiles and 5 blue tiles -> score = sum of 3 red values + sum of first 3 blue values.

## Game End

The game ends when both the blue and red tile piles are empty. Highest score wins. Ties share a place.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `roll <die numbers>` | Re-roll the dice at the given positions | `roll 1 3 5` |
| `take <blue\|red>` | Take a tile from the central row | `take blue` |
| `steal <player> <blue\|red> [n]` | Steal a tile from another player | `steal bj red 2` |
