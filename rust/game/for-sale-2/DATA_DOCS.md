# For Sale Data Dictionary

## PubState (public information)
- `players` (usize): number of players
- `phase` (Phase): current game phase - Buying, Selling, or Finished
- `finished` (bool): whether the game has ended
- `buy_rounds_remaining` (usize): property auctions left in the buying phase
- `sell_rounds_remaining` (usize): cheque rounds left in the selling phase
- `open_cards` (Vec<i32>): cards face up this round (buildings during buying, cheques during selling)
- `bidding_player` (usize): index of the player whose turn it is to bid/play
- `bids` (Vec<i32>): current bids from each player in the buying phase
- `finished_bidding` (Vec<bool>): which players have dropped out of the current auction

## PlayerState (player-private information)
- `public` (PubState): the full public game state
- `player` (usize): this player's seat index
- `chips` (i32): player's remaining money/chips
- `hand` (Vec<i32>): property cards in hand (values 1-30)
- `cheques` (Vec<i32>): cheque values collected for the selling phase
