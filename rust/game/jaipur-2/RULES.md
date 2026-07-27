# Jaipur

Jaipur is a two-player game of trading goods in the markets of Rajasthan. Buy,
exchange and sell goods at the best moment, and keep the largest camel herd.

A match is played over up to three rounds. The first player to win two rounds
wins the match.

## Components

- A deck of 52 cards: 6 diamond, 6 gold, 6 silver, 8 cloth, 8 spice,
  10 leather and 8 camel.
- One goods token pile per trade good, highest value first:
  - diamond: 7 7 5 5 5
  - gold: 6 6 5 5 5
  - silver: 5 5 5 5 5
  - cloth: 5 3 3 2 2 1 1
  - spice: 5 3 3 2 2 1 1
  - leather: 4 3 2 1 1 1 1 1 1
- Three shuffled, face-down bonus token piles:
  - for 3-card sales: 3 3 2 2 2 1 1
  - for 4-card sales: 6 6 5 5 4 4
  - for sales of 5 or more cards: 10 10 9 8 8
- One camel token worth 5 points.

## Setup

Each round is set up from scratch:

- Three camels are placed directly into the market. They are **not** dealt from
  the deck, so 11 camels are in play in total: 8 in the 52-card deck plus these
  3 in the market.
- The deck is shuffled and two more cards are drawn into the market, so the
  market holds 5 cards.
- Each player is dealt 5 cards. Any camels dealt to a player go straight into
  that player's herd and do not count against the hand limit.
- 40 cards remain in the deck.

## Your turn

On your turn you either take cards or sell cards.

### Taking

- `take <good>` - take one good from the market into your hand. You may not put
  anything back.
- `take <good> <good> ... for <good> <good> ...` - exchange two or more cards.
  You must give back exactly as many cards as you take. The cards you give come
  from your hand or from your camel herd, and none of them may be the same type
  as any card you take.
- `take camel` - take **all** the camels in the market into your herd.

Your hand may never hold more than 7 goods. Camels live in your herd, not your
hand, so they are not limited.

After any take, the market is refilled from the deck back up to 5 cards.

### Selling

- `sell <n> <good>` or `sell <good> <good> ...` - sell cards from your hand.
- Every card in one sale must be the same type of good.
- Diamond, gold and silver require a minimum sale of 2 cards. Cloth, spice and
  leather can be sold one at a time.
- Camels can never be sold.
- Take one goods token per card sold, from the top (highest remaining value) of
  that good's pile. If the pile runs out you simply take fewer tokens.
- Selling 3 or more cards at once also earns one bonus token: from the 3-card
  pile for a 3-card sale, the 4-card pile for a 4-card sale, and the
  5-or-more pile for any sale of 5 or more cards. The value of the bonus token
  is shown privately to the seller.

## End of a round

A round ends immediately when either:

- three goods token piles have been exhausted, or
- the deck can no longer refill the market to 5 cards.

Then:

- The player with more camels takes the 5 point camel token. If both players
  have the same number of camels, nobody takes it.
- Each player adds up the value of every token collected during the round.
- The higher total wins the round. If the totals are equal, the round goes to
  the player with the most bonus tokens; if that is also equal, to the player
  with the most goods tokens. If everything is equal the round is replayed.

## Winning the match

The first player to win two rounds wins the match.
