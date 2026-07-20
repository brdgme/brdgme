# Jaipur Data Dictionary

## PubState (public information)

- `current_player` (usize): Index (0 or 1) of the player whose turn it is.
- `round_wins` ([u8; 2]): Number of rounds each player has won. First to 2 round wins takes the game.
- `market` (Vec<Good>): Goods currently available in the market (up to 5 cards). Includes camels and trade goods.
- `deck_len` (usize): Number of cards remaining in the draw deck. When the deck cannot replenish the market to 5, the round ends.
- `camels` ([u32; 2]): Number of camels each player holds, indexed by player. The player with more camels at round end gets a 5-point bonus.
- `hand_sizes` ([usize; 2]): Number of non-camel goods in each player's hand, indexed by player. Maximum hand size is 7.
- `token_counts` ([usize; 2]): Total number of tokens (good tokens + bonus tokens) each player has collected, indexed by player.
- `goods` (HashMap<Good, Vec<u32>>): Remaining token values for each trade good. Tokens are taken from the front of the vec (highest values first). When a good's tokens are exhausted, it contributes to the round-end trigger.
- `bonuses` (HashMap<usize, usize>): Number of bonus tokens remaining for each sale size (3, 4, or 5). Bonus tokens are awarded when selling 3+ of a good at once.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0 or 1) this private state belongs to.
- `hand` (Vec<Good>): Non-camel goods in this player's hand. Camels are tracked separately in `public.camels`.

## Good enum

- Diamond, Gold, Silver: Precious goods. Require minimum 2 to sell. Token values are high (5-7).
- Cloth, Spice: Common goods. Minimum 1 to sell. Token values are moderate (1-5).
- Leather: Common good. Minimum 1 to sell. Token values are low (1-4).
- Camel: Cannot be sold. Used for trading (swapping with market goods). Most camels at round end wins 5 bonus points.
