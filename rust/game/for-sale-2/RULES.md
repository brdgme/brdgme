# For Sale

A 3-5 player property auction game. Two phases: buy buildings with chips, then sell them for cheques. Highest total (cheques + chips) wins.

## Setup

- 30 building cards numbered 1 to 30, shuffled. (20 are used here.)
- 30 cheques: two 0s, then 2..=20.
- Each player starts with 15 chips.
- For 3 players, two building and two cheque cards are removed.

## Buying Phase

Buildings are drawn (one per player, sorted low to high) and auctioned:

- **bid `<amount>`** - raise the bid above the current highest. You cannot bid more chips than you hold.
- **pass** - take the lowest building on the table, paying half your current bid (rounded down). You are out of this auction.

When only one bidder remains, they take the highest building and pay their full bid. A new set of buildings is then drawn. Buying ends when the building deck is empty.

## Selling Phase

Cheques are drawn (one per player, sorted low to high). Each player secretly selects one building to play:

- **play `<building>`** - play a building from your hand.

When all players have played, buildings are compared highest-to-lowest: the highest building takes the highest cheque, the next takes the next, and so on. A new set of cheques is drawn. When a player has only one building left, the final card is played automatically.

## Scoring

Final score = total of your cheques + your remaining chips. Highest score wins. Ties share a place.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `bid <amount>` | Raise the bid | `bid 5` |
| `pass` | Take the lowest building for half your bid | `pass` |
| `play <building>` | Play a building during the selling phase | `play 12` |

## Reading the Display

- **Buildings/Cheques available** - the cards on the table this round
- **Current bid** - the highest bid and who made it
- **Your bid** - your current bid (buying) or the building you're playing (selling)
- **Remaining players** - who is still in the auction
- **Your chips / buildings / cheques** - your hand
- **Rounds remaining** - how many buy/sell rounds are left
