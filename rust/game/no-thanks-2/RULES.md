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

