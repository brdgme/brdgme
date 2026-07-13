# Texas Hold 'em

A 2-9 player game of poker. Each player starts with $100. Win by being the
last player left with money.

## Blinds

Each hand, the dealer position moves to the next active player. The player
after the dealer posts a small blind, and the next player after that posts a
big blind, forcing the betting started. In 2-player (heads-up) games the
dealer posts the small blind instead.

The minimum bet starts at $10 (so the small blind is $5, the big blind is
$10). The minimum bet doubles every 5 hands.

If a player doesn't have enough money to cover a blind, they go all in for
whatever they have.

## Playing a hand

Each player is dealt two private cards. Betting happens in rounds:

- Pre-flop, right after the blinds are posted.
- The flop: three community cards are dealt face up.
- The turn: a fourth community card is dealt.
- The river: a fifth and final community card is dealt.

On your turn you can:

- `check` - continue without betting more, only available if you're not
  behind the current bet.
- `call` - match the current bet.
- `raise <amount>` - increase the bet by the given amount above the current
  bet.
- `fold` - forfeit the hand and any money already bet.
- `allin` - bet all the money you have left.

If only one player is left after everyone else folds, they win the pot
immediately.

## Showdown

If two or more players remain after the river, hands are compared using
standard poker hand rankings (from best to worst): straight flush, four of a
kind, full house, flush, straight, three of a kind, two pair, one pair, high
card. Each player's best five-card hand is made from their two private cards
plus the five community cards.

If a player went all in for less than other players, side pots are formed:
each pot is contested only by the players who contributed to it, so an
all-in player can only win up to the size of the pot they're eligible for.

Ties split the pot evenly; if it doesn't divide evenly, the odd chip goes to
the first remaining player after the dealer.

## Winning

A player who runs out of money (and isn't owed anything from an in-progress
hand) is out. The game ends when only one player has money left - they win.
