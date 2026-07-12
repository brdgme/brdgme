# Modern Art

A 3-5 player game of art auctions. Play cards from your hand to auction off
paintings, bid to buy them from your opponents, and cash in at the end of each
round when the top three artists pay out - but only the cards you've bought
count toward your fortune.

## Cards

70 cards across five artists, each card also has an auction type:

| Code | Artist |
|------|--------|
| lm | Lite Metal |
| yo | Yoko |
| cp | Christine P |
| kg | Karl Gitter |
| kr | Krypto |

| Code | Auction type |
|------|--------------|
| op | Open |
| fp | Fixed Price |
| sl | Sealed |
| db | Double |
| oa | Once Around |

A card is written as artist + auction type, e.g. `lmop` (Lite Metal, Open
auction).

## Setup

Each player starts with $100. The deck is shuffled and cards are dealt at the
start of each round.

## Rounds

The game is played over 4 rounds. At the start of each round, cards are dealt
to each player's hand (no cards are dealt in round 4):

| Players | Round 1 | Round 2 | Round 3 | Round 4 |
|---------|---------|---------|---------|---------|
| 3 | 10 | 6 | 6 | 0 |
| 4 | 9 | 4 | 4 | 0 |
| 5 | 8 | 3 | 3 | 0 |

## Turn Structure

On your turn, play a card from your hand to start an auction (`play lmop`).
The auction type printed on the card determines how it's sold:

- **Open** - Any player (including the auctioneer) may call out a bid at any
  time (`bid 10`). Once a player passes, they're out for this auction. The
  auction ends when only one bidder remains (or nobody bids at all), and that
  player wins.
- **Fixed Price** - The auctioneer sets an asking price (`price 15`). Other
  players, in turn order, may buy at that price (`buy`) or pass. If everyone
  passes, the auctioneer buys their own painting at the price they set.
- **Sealed** - Every player (including the auctioneer) secretly bids a single
  amount (`bid 8`) or passes (bids of $0). Once everyone has responded, the
  highest bid wins; bids and passes are not revealed to other players as they
  happen.
- **Double** - Works like Open, Fixed Price, or Sealed depending on the second
  card added, with one exception: any player, including the auctioneer, may
  add a second card of the *same artist* (but not another Double) to the
  auction with `add <card>` before bidding starts. Both cards go to the
  winner for a single payment. Only one card can be added.
- **Once Around** - Starting with the player after the auctioneer, each
  player in turn gets exactly one chance to bid higher than the current
  highest bid, or pass. The auctioneer only gets to bid if nobody else has
  bid anything by the time it comes back around.

Whoever wins the auction pays their bid: to the auctioneer if someone else
wins, or to the bank if the auctioneer wins their own auction. The winner
adds the card(s) to their purchases (face-up, public knowledge) and it
becomes their turn next.

If a player has no cards in hand when it becomes their turn, they're skipped.

## End of Round

A round ends immediately when the fifth card of any single artist is played
into an auction (this 5th card does **not** get sold - the round ends before
bidding). It also ends naturally once all rounds' cards have been played out.

At the end of the round, the three artists with the most cards played this
round (counting only cards actually purchased by players, not any unsold 5th
card) are ranked and awarded value for this round:

- 1st: $30
- 2nd: $20
- 3rd: $10
- Any remaining artists: $0 this round

Ties are broken in artist order: Lite Metal, Yoko, Christine P, Karl Gitter,
Krypto.

An artist's total value is the sum of every round's value for that artist so
far (an artist who never places in the top 3 in a round contributes $0 for
that round, but keeps whatever it earned in previous rounds).

Every player is then paid, for **each** card they've purchased so far this
round, the *total* cumulative value of that card's artist - even if the
artist didn't place this round. Purchases are cleared at the start of the
next round.

## End of Game

After round 4 is scored, the game ends and final money is revealed. The
player with the most money wins.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `play <card>` | Play a card to start an auction | `play lmop` |
| `add <card>` | Add a second card to a Double auction | `add kgsl` |
| `bid <amount>` | Bid in an Open, Sealed, or Once Around auction | `bid 10` |
| `price <amount>` | Set the asking price in a Fixed Price auction | `price 15` |
| `buy` | Buy at the asking price in a Fixed Price auction | `buy` |
| `pass` | Pass on the current auction | `pass` |

## Reading the Display

While an auction is active, the display shows who is auctioning what, and
(except for Sealed auctions, where bids stay secret) the current highest bid
and bidder.

Below that, your money and hand are shown (money and hand contents are
private - only you can see them). A **Players / Purchases** table shows every
player's public purchases so far this round. An **Artist** table shows each
artist's value per round (`.` for rounds not yet scored) and running total.
