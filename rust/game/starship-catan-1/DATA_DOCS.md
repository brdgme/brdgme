# Starship Catan Data Dictionary

## PubState (public information)

- `phase` (Phase): Current game phase. `ChooseModule` is the initial module selection. `Produce` is resource production. `ChooseSector` is selecting which sector to explore. `Flight` is the exploration phase. `TradeAndBuild` is the post-flight trade and build phase.
- `current_player` (usize): Index (0 or 1) of the player whose turn it is.
- `current_sector` (i32): The sector (1-4) currently being explored during flight.
- `player_boards` ([PlayerBoard; 2]): Full board state for both players. Each board contains resources, modules, colonies, trading posts, defeated pirates, completed adventures, and special cards.
- `flight_cards` (Vec<SectorCard>): Sector cards encountered during the current flight, in order of discovery.
- `trade_amount` (i32): Number of trades made at the current flight card's trade stop.
- `player_trade_amount` (i32): Number of "take" trades used this TradeAndBuild phase (limited by Trade module level).
- `yellow_dice` (i32): The production dice roll for this turn (1-3). Determines which colonies and modules produce.
- `flight_actions_used` (usize): Number of flight actions used so far this flight. Limited by Command module level (base 2).
- `card_finished` (bool): True when the current flight card has been fully resolved and the player can advance.
- `losing_module` (bool): True when the player must choose a module to lose after losing a pirate fight.
- `current_adventure_cards` (Vec<AdventureCard>): Adventure cards currently available to complete at adventure planets.
- `adventure_deck_len` (usize): Number of adventure cards remaining in the deck.
- `sector_pile_lens` (BTreeMap<i32, usize>): Number of cards remaining in each sector pile, keyed by sector number (1-4).
- `sector_draw_pile_len` (usize): Number of cards in the sector draw pile, used to replace cards removed during flight.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0 or 1) this private state belongs to.
- `peeking` (Vec<SectorCard>): Cards the player is peeking at via the Sensor module. Only visible to this player.

## PlayerBoard

- `player` (usize): Player index.
- `resources` (BTreeMap<Resource, i32>): Resource counts. Resources include Ore, Fuel, Food, Carbon, Trade, Science, Astro (money), ColonyShip, TradeShip, Booster, Cannon.
- `modules` (BTreeMap<Module, i32>): Module levels (0-2). Modules: Command, Logistics, Science, Trade, Sensor.
- `completed_adventures` (Vec<AdventureCard>): Adventure cards this player has completed.
- `colonies` (Vec<SectorCard>): Colony cards this player has founded.
- `trading_posts` (Vec<SectorCard>): Trading post cards this player has founded.
- `defeated_pirates` (Vec<SectorCard>): Pirate cards this player has defeated.
- `friend_of_the_people` (bool): True if this player holds the Friend of the People card (most diplomat points, >3).
- `hero_of_the_people` (bool): True if this player holds the Hero of the People card (most medals, >3).
- `last_sectors` (Vec<i32>): Recently visited sectors (most recent first).

## Resource enum

Ore, Fuel, Food, Carbon, Trade, Science, Astro (money), ColonyShip, TradeShip, Booster, Cannon.

## Module enum

Command (extra flight actions), Logistics (higher goods limit), Science (science production), Trade (trade production and take actions), Sensor (peek at sector cards before flight).
