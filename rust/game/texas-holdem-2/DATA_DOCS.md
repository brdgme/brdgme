# Texas Hold'em Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in the game, 2 through 9.
- `community_cards` (Deck): The shared community cards dealt so far. Empty pre-flop, 3 cards after the flop, 4 after the turn, 5 after the river. Every player uses these to build their best hand.
- `pot` (i32): Total money currently in the pot - the sum of all players' bets this hand.
- `current_dealer` (usize): Index of the player who is the dealer for this hand. The dealer position rotates each hand and determines blind order.
- `current_player` (usize): Index of the player whose turn it is to act.
- `player_money` (Vec<i32>): Money each player still has available (not currently bet), indexed by player. A player with $0 here and $0 bet is out.
- `bets` (Vec<i32>): Amount each player has bet in the current hand, indexed by player. The largest of these is the current bet that others must call. Bets are reset to 0 at the start of each new hand.
- `folded_players` (Vec<bool>): Whether each player has folded this hand, indexed by player. A folded player cannot win the pot but their bet stays in the pot.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.
- `hand` (Deck): This player's two private (hole) cards, visible only to them. Each card has a `suit` and a `rank`.

## Card

- `suit` (Suit): One of Clubs, Diamonds, Hearts, Spades.
- `rank` (u8): Numeric rank, 2 through 14. 11 = Jack, 12 = Queen, 13 = King, 14 = Ace (aces are high).

## Suit enum

Clubs, Diamonds, Hearts, Spades - the four card suits.

## Hand categories (best to worst)

Used at showdown to decide the winner. A player's best 5-card hand is chosen from their 2 hole cards plus the 5 community cards.

- Straight flush: five consecutive ranks, all the same suit.
- Four of a kind: four cards of the same rank.
- Full house: three of a kind plus a pair.
- Flush: five cards of the same suit, not consecutive.
- Straight: five consecutive ranks, mixed suits.
- Three of a kind: three cards of the same rank.
- Two pair: two separate pairs.
- One pair: two cards of the same rank.
- High card: none of the above; highest card wins.

When two players share a category, the higher relevant ranks win; if fully tied the pot is split.
