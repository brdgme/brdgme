# Farkle

A 2-6 player press-your-luck dice game. Roll six dice, set aside scoring combinations, and decide whether to keep rolling for more points or bank what you have. Roll no scoring dice and you lose your turn's points. First to 5000 ends the round; highest score wins.

## Setup

Each player takes zero points. A random first player is chosen and begins their turn by rolling all six dice.

## The Dice

Six ordinary dice, faces 1 to 6. Each face is shown in its own colour:

| Face | Colour |
|---|---|
| 1 | cyan |
| 2 | green |
| 3 | red |
| 4 | blue |
| 5 | yellow |
| 6 | purple |

## Turn Structure

On your turn you roll the dice, then repeatedly choose one of:

- **Score** - set aside a single scoring combination from the rolled dice (`score <dice>`, e.g. `score 1` or `score 5 5 5`). The scored dice are removed and the points added to your turn score. You must score at least once before you can roll again or bank.
- **Roll** - re-roll the remaining dice (`roll`). If you have scored all six dice, you re-roll all six again.
- **Done** - bank your turn score (`done`).

After a roll with no scoring dice at all, you bust: your turn score is lost and play passes to the next player.

## Scoring Combinations

| Combination | Points |
|---|---|
| Single 1 | 100 |
| Single 5 | 50 |
| Three 1s | 1000 |
| Three 2s | 200 |
| Three 3s | 300 |
| Three 4s | 400 |
| Three 5s | 500 |
| Three 6s | 600 |

Only these exact combinations score; any other selection of dice scores nothing. You score one combination per `score` command.

## Ending and Winning

The game ends when play returns to the first player and at least one player has 5000 or more points. Placings are by score, highest first; ties share a place.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `score <dice>` | Set aside a scoring combination | `score 1`, `score 5`, `score 1 1 1` |
| `roll` | Re-roll the remaining dice | `roll` |
| `done` | Bank your turn score | `done` |

## Reading the Display

- **Remaining dice** - the dice still in front of you this roll
- **Score this turn** - points accumulated this turn, banked on `done`
- **Player table** - each player's total score; the starting player is marked

## Strategy

- Single 1s (100) and 5s (50) make otherwise bad rolls worth something; grab them before re-rolling
- Three of a kind is the big scorer - watch for it on full re-rolls, especially three 1s for 1000
- Banking early locks in your points; pushing for one more roll risks busting and losing it all