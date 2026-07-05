# Liar's Dice

A 2-6 player game of bluffing and dice. Each player rolls dice that only they can see, then takes turns making bids about the total number of dice showing a particular face across all players' cups. On your turn you must either raise the bid or call the previous bid a lie. Lose a die when you're wrong; the last player with dice wins.

## Setup

Each player starts with 5 dice. A random first player is chosen. All dice are rolled and kept hidden from other players.

## Turn Structure

On your turn the current bid is shown. You may either:

- **Bid** - raise the current bid (`bid <quantity> <value>`, e.g. `bid 2 5`)
- **Call** - claim the current bid is too high (`call`)

### Bidding Rules

A bid is a quantity and a face value - a claim that at least `<quantity>` dice showing `<value>` exist under all players' cups, counting the wild 1s.

To raise a bid you must either:
- Increase the quantity (any value), or
- Keep the same quantity and increase the value

You can never reduce the quantity. Value must be between 1 and 6. The first bid sets the starting bid; subsequent bids increase it.

### Wild Dice

Dice showing **1** are wild - they count as matching any bid value. (When the bid value itself is 1, only 1s count, as expected.)

### Calling

When you call, all dice are revealed. The number of dice matching the bid value (including wild 1s) is counted:

- If the actual count is **less than** the bid quantity, the **bidder** was wrong and loses a die.
- Otherwise the **caller** was wrong and loses a die.

The player who lost a die does not start the next round; the next active player (clockwise from the caller) starts.

## Elimination

A player with no dice remaining is eliminated and out of the round. Eliminated players' dice still count for revealing when a call is made - they simply no longer take turns.

## Winning

The game ends when fewer than two players have dice. The last player with dice remaining wins.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `bid <quantity> <value>` | Raise the bid | `bid 2 5`, `bid 6 1` |
| `call` | Call the current bid a lie | `call` |

## Reading the Display

- **Current bid** - the standing bid, or "first bid" if no bid yet
- **Your dice** - the dice under your cup (only you see this)
- **Player table** - how many dice each player has remaining

When a call is made, the reveal log shows every active player's dice with the dice matching the bid value (and wild 1s) highlighted.

## Strategy

- Track what dice you hold and infer probabilities - your own dice tell you which bids are likely safe or risky
- A high quantity bid puts pressure on the next player; a high value bid leverages wilds
- Don't call too eagerly early in a round - the odds often favour a high bid being correct, especially with many total dice in play