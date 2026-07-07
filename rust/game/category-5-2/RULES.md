# Category 5

A 2-10 player card game (also known as 6 nimmt!). Players are dealt a hand of 10 cards numbered 1 to 104. Each round, everyone picks a card to play simultaneously. Cards are resolved from lowest to highest: each card goes into the row whose ending card is the highest below it. If a card is lower than every row's ending card, the player must choose a row to take. Taking a row scores its bullheads. When a row reaches 5 cards, the next card played into it takes the whole row. The game ends when someone reaches 66 bullheads; the player with the fewest bullheads wins.

## Card Bullheads

Every card has a bullhead value:

| Card | Bullheads |
|---|---|
| 55 | 7 |
| Multiples of 11 (11, 22, 33, 44, 66, 77, 88, 99) | 5 |
| Multiples of 10 (10, 20, 30, 40, 50, 60, 70, 80, 90, 100) | 3 |
| Multiples of 5 (5, 15, 25, 35, 45, 65, 75, 85, 95) but not above | 2 |
| All others | 1 |

The 104-card deck is the only deck used.

## Setup

Shuffle the 104-card deck. Place one card face up at the start of each of the 4 rows. Deal 10 cards to each player.

## Round

Each turn, every player simultaneously chooses one card from their hand to play. Once all have chosen, the cards are resolved lowest to highest:

- Find the row whose last card is the highest value still below the played card. The played card is appended to that row.
- If the played card is lower than every row's last card, that player chooses any of the 4 rows to take. The taken row's cards go to the player's taken pile, and the played card becomes the new start of that row.
- If a row already has 5 cards and a card would be appended as the 6th, the player takes the existing 5 cards and their played card becomes the new start of the row.

After all played cards are resolved, if players still have cards in hand, the next turn begins. When hands are down to one card, the last card is played automatically. When all hands are empty, the round ends: each player adds up the bullheads in their taken pile and adds it to their score. If no one has reached 66, a new round begins.

## Ending and Winning

The game ends at the end of a round where at least one player has 66 or more bullheads. The player with the fewest bullheads wins. Placings are by bullhead count, lowest first; ties share a place.

## Commands

| Command | Action |
|---|---|
| `play <card>` | Play a card from your hand |
| `choose <row>` | Choose a row to take (when your card is lower than all rows) |

Rows are numbered 1 to 4.

## Reading the Display

- **Board** - the 4 rows, each with up to 5 cards and a running bullhead total
- **Your hand** - the cards in your hand (only you see this)
- **Legend** - bullhead values by colour
- **Score table** - each player's taken card count and total bullheads
- **Points to end** - how many more bullheads the leader needs to trigger game end
