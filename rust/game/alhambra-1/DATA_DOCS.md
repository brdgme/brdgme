# Alhambra Data Dictionary

## PubState (public information)

- `human_players` (usize): Number of human players (2-6).
- `all_players` (usize): Total players including Dirk (the AI opponent in 2-player games). In games with 3+ players, this equals human_players.
- `current_player` (usize): Index of the player whose turn it is.
- `phase` (Phase): Current game phase. `Action` means the player must take money, buy a tile, or manage their reserve. `Place` means the player must place bought tiles. `FinalPlace` is the end-game placement phase. `End` means the game is over.
- `round` (i32): Current scoring round (1-3). Scoring is triggered when a scoring card is drawn from the deck.
- `boards` (Vec<PubBoard>): Public board state for each player, indexed by player position.
- `cards` (Vec<Card>): Money cards available in the market (up to 4). Each card has a currency (Red, Blue, Green, Yellow) and a value.
- `tiles` (Vec<Tile>): Building tiles available in the market (up to 4, one per currency slot). Each tile has a type, cost, and wall configuration.
- `tile_bag_len` (usize): Number of tiles remaining in the tile bag. When this hits 0, the game enters the final placement phase.

## PubBoard (per-player public board)

- `grid` (Grid): The player's placed tiles on their grid, as a map from coordinates to tiles. The grid must always contain a Fountain tile and be connected without gaps.
- `reserve` (Vec<Tile>): Tiles in the player's reserve. These can be placed on the grid or swapped with existing tiles during the Action phase.
- `card_count` (usize): Number of money cards in the player's hand. The actual cards are private.
- `place` (Vec<Tile>): Tiles the player has bought but not yet placed on their grid. Must be placed before the turn ends.
- `points` (i32): The player's current score from scoring rounds and walls.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0-indexed) this private state belongs to.
- `hand` (Vec<Card>): Money cards in this player's hand. Each card has a currency and value.

## Card

- `currency` (Currency): One of Red, Blue, Green, Yellow.
- `value` (i32): The monetary value of the card (1-9).

## Tile

- `tile_type` (TileType): One of Fountain, Pavillion, Seraglio, Arcades, Chambers, Garden, Tower.
- `cost` (i32): The purchase cost in the matching currency.
- `walls` (Vec<Dir>): Wall segments on the tile (Up, Down, Left, Right). Walls contribute to the longest external wall scoring.

## Phase enum

- `Action`: Player takes money cards, buys a tile, or manages reserve (place from reserve, swap, or remove).
- `Place`: Player places tiles they bought this turn onto their grid.
- `FinalPlace`: End-game phase where remaining tiles are distributed and placed.
- `End`: Game is over.
