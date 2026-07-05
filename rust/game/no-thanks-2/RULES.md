# No Thanks!

A 3-5 player press-your-luck auction game. Cards 3-35 are dealt (9 removed), and players bid with chips to refuse the card on the table - or take it, along with all chips on it. Cards score in consecutive runs (only the lowest of each run counts), and unspent chips subtract from your score. Lowest score wins.

## Setup

- 33 cards numbered 3 to 35. 24 are dealt into the deck; the other 9 are removed unseen.
- Each player starts with 11 chips.
- A random first player is chosen.
- The top card of the deck is revealed as the current card.

## Turn Structure

On your turn you must either:

- **Pass** - spend one of your chips into the centre to pass the card (`pass`). You cannot pass if you have no chips left.
- **Take** - take the current card and all chips on it (`take`). Your turn continues; the next card from the deck is revealed and you face it again.

After a pass the turn passes to the next player. After a take the same player continues with the new card. The game ends when the deck is empty.

## Scoring

Cards score in runs of consecutive numbers - only the lowest card of each run counts against you. For example a hand of {3, 5, 6, 8, 9, 10, 15, 16} scores `3 + 5 + 8 + 15 = 31` (four runs: [3] [5,6] [8,9,10] [15,16]).

Final score = card score - chips remaining. Lowest final score wins. Ties share a place.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `pass` | Spend a chip to refuse the current card | `pass` |
| `take` | Take the current card and all chips on it | `take` |

## Reading the Display

- **Current card** - the card on the table, with the number of cards left in the deck
- **Current chips** - chips accumulated on the current card
- **Your hand** - your cards grouped into runs; cards one away from the current card are bolded
- **Your chips** - your remaining chips
- **Player table** - each player's cards (and final score, once the game ends)

## Strategy

- Cards one apart from each other are nearly free if joined - the adjacency bolding in your hand hints at which cards would complete or extend a run
- Don't pass a low card unless you can afford to fight for it; other players will gladly take a 3 with a single chip
- Chips are a hedge - a hand full of high singletons is far worse than the same hand with one joining card and a pile of chips