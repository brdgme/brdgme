# Sushi Go Data Dictionary

## PubState (public information)

- `players` (usize): Number of real players in this game (2 to 5).
- `all_players` (usize): Total number of player slots including the dummy. In 2-player games this is 3 (two real players plus a dummy); otherwise it equals `players`.
- `round` (usize): Current round number, 1 through 3.
- `controller` (usize): Index of the player currently controlling the dummy. Only meaningful in 2-player games; alternates each hand.
- `played` (Vec<Vec<Card>>): Cards on each player's table, indexed by player slot. Pudding cards persist across rounds; all other cards are cleared between rounds.
- `player_points` (Vec<i32>): Cumulative points for each player slot across all completed rounds.
- `finished` (bool): True when all 3 rounds are complete and the game is over.
- `final_scores` (Vec<i32>): Final scores for each real player (length equals `players`). Only populated when `finished` is true; empty vec during play.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this private state belongs to.
- `hand` (Vec<Card>): Cards currently in this player's hand. Entries set to `Card::Played` mark slots already used this hand and cannot be played again.
- `playing` (Option<Vec<Card>>): The card(s) this player has chosen to play this hand. None means they have not yet submitted a play.
- `dummy_playing` (Option<Vec<Card>>): Cards assigned to the dummy this hand. Only visible to the controller in 2-player games; None otherwise.

## Card enum

- `Played`: Placeholder marking a used hand slot. Not a real card.
- `Tempura`: 2 tempura score 5 points.
- `Sashimi`: 3 sashimi score 10 points.
- `Dumpling`: Scores 1, 3, 6, 10, 15 for 1 through 5+ dumplings.
- `MakiRoll3`: Maki roll with 3 symbols.
- `MakiRoll2`: Maki roll with 2 symbols.
- `MakiRoll1`: Maki roll with 1 symbol.
- `SalmonNigiri`: Nigiri worth 2 points (6 with wasabi).
- `SquidNigiri`: Nigiri worth 3 points (9 with wasabi).
- `EggNigiri`: Nigiri worth 1 point (3 with wasabi).
- `Pudding`: Scored at end of round 3 only. Most puddings: +6, fewest: -6 (not in 2p).
- `Wasabi`: Triples the value of the next nigiri played after it.
- `Chopsticks`: Allows playing 2 cards in a future hand (returned to hand after use).
