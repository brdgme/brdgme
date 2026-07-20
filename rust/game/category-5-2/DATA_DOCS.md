# Category 5 Data Dictionary

## PubState (public information)
- `players` (usize): number of players in the game
- `board` ([Vec<Card>; 4]): the four rows of cards currently on the table, each row is a sequence of ascending cards
- `player_cards_counts` (Vec<usize>): number of cards in each player's taken pile
- `player_points` (Vec<i32>): accumulated bullhead points per player (lower is better)
- `finished` (bool): whether the game has ended
- `placings` (Vec<usize>): final standings once finished (1 = winner), empty while in progress

## PlayerState (player-private information)
- `public` (PubState): the full public game state
- `player` (usize): this player's seat index
- `hand` (Vec<Card>): cards in this player's hand (private until played)
