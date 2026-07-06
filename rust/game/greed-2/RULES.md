# Greed

A 2-6 player press-your-luck dice game. Roll six custom dice, set aside scoring combinations, and decide whether to keep rolling for more points or bank what you have. Roll no scoring dice and you lose your turn's points. First to 5000 ends the round; highest score wins.

## Setup

Each player takes zero points. A random first player is chosen and begins their turn by rolling all six dice.

## The Dice

Six custom faces, each shown in its own colour:

| Face | Name | Colour |
|---|---|---|
| `$` | Dollar | grey |
| `G` | G | yellow |
| `R` | R | red |
| `E` | E | black |
| `e` | e | green |
| `D` | D | cyan |

## Turn Structure

On your turn you roll the dice, then repeatedly choose one of:

- **Score** - set aside a scoring combination from the rolled dice (`score <dice>`, e.g. `score $$$`). The scored dice are removed and the points added to your turn score. You must score at least once before you can roll again.
- **Roll** - re-roll the remaining dice (`roll`). If you have scored all six dice, you re-roll all six again.
- **Done** - bank your turn score (`done`). Before banking, every remaining scoring combination is taken automatically in priority order.

After a roll with no scoring dice at all, you bust: your turn score is lost and play passes to the next player.

## Scoring Combinations

| Combination | Points |
|---|---|
| Six of a kind (any face) | 5000 |
| Four `D` | 1000 |
| One of each face (`$ G R E e D`, a straight) | 1000 |
| Three `$` | 600 |
| Three `G` | 500 |
| Three `R` | 400 |
| Three `E` | 300 |
| Three `e` | 300 |
| One `D` | 100 |
| One `G` | 50 |

When several combinations are available, `done` takes them in the order above (six of a kind first, down to single dice).

## Ending and Winning

The game ends when play returns to the first player and at least one player has 5000 or more points. Placings are by score, highest first; ties share a place.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `score <dice>` | Set aside a scoring combination | `score $$$`, `score D`, `score $GREeD` |
| `roll` | Re-roll the remaining dice | `roll` |
| `done` | Bank your turn score (auto-scores the rest) | `done` |

## Reading the Display

- **Remaining dice** - the dice still in front of you this roll
- **Score this turn** - points accumulated this turn, banked on `done`
- **Player table** - each player's total score; the starting player is marked

## Strategy

- The single `D` (100) and single `G` (50) make otherwise bad rolls worth something; grab them before re-rolling
- Six of a kind and the straight both clear at 5000 or 1000 - watch for them on full re-rolls
- `done` auto-takes every remaining scoring combo, so banking late can yield more than you expect - but pushing for one more roll risks busting and losing it all
