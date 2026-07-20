# Modern Art Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in the game, 3 through 5.
- `round` (usize): Current round number, 0 through 3 (add 1 for the human-facing round number).
- `finished` (bool): True when all 4 rounds are complete and the game is over.
- `is_auction` (bool): True while an auction is in progress, false when the game is waiting for a player to play a card to start an auction.
- `current_player` (usize): Index of the player whose turn it is. During an auction this is the auctioneer (the player who played the card being sold).
- `auctioning` (Vec<Card>): The card or cards currently up for auction. One card normally, two for a Double auction once a second card has been added. Empty when not in an auction.
- `auction_type` (Option<Rank>): The auction type of the card being sold, which sets the bidding rules (Open, FixedPrice, Sealed, Double, or OnceAround). None when not in an auction. For a Double auction this becomes the type of the added card once one is added.
- `current_bid` (Option<(usize, i32)>): The current highest bid as (bidder index, amount). When nobody has bid yet this is the auctioneer at $0. None when not in an auction, or for Sealed auctions where bids stay secret until the auction settles.
- `purchases` (Vec<Vec<Card>>): Cards purchased this round, indexed by player. Each inner vec holds that player's bought cards, sorted by artist then auction type. Cleared at the start of each round.
- `value_board` (Vec<HashMap<Suit, i32>>): Artist value awarded per completed round. Each entry is one round's awards: a map of artist to the value it earned that round ($30 for 1st, $20 for 2nd, $10 for 3rd, omitted for artists that earned $0). Sum an artist's entries across rounds for its total value.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.
- `money` (i32): This player's current money. Private to the player until the end of the game.
- `hand` (Vec<Card>): The cards currently in this player's hand, sorted by artist then auction type.

## Card

- `suit` (Suit): The artist. One of LiteMetal, Yoko, ChristineP, KarlGitter, Krypto.
- `rank` (Rank): The auction type. One of Open, FixedPrice, Sealed, Double, OnceAround.

## Suit enum

LiteMetal, Yoko, ChristineP, KarlGitter, Krypto - the five artists.

## Rank enum

- `Open`: Any player may bid at any time; once you pass you are out. Ends when one bidder remains.
- `FixedPrice`: The auctioneer sets an asking price; others buy or pass in turn order.
- `Sealed`: Every player secretly bids once; highest bid wins. Bids are not revealed during the auction.
- `Double`: A second card of the same artist may be added before bidding; both sell for one payment.
- `OnceAround`: Each player after the auctioneer gets one chance to bid or pass, in turn order.
