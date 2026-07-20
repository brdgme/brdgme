# Roll Through the Ages Data Dictionary

This game has no hidden information: `PubState` and `PlayerState` both carry a
full clone of the game, so every field described below is visible to every
consumer (bot or spectator).

## PubState (public information)

- `game` (Game): The complete game state - all players' boards, the current dice, turn phase, turn supplies, and round/finish flags. See the Game section below.

## PlayerState (player-private information)

- `game` (Game): The complete game state, identical to the public state since there is no hidden information.
- `player` (usize): Which player index (0 through players-1) this state is being shown to.

## Game

- `players` (usize): Number of players in the game (2 through 4).
- `current_player` (usize): Index (0 through players-1) of the player whose turn it is.
- `phase` (Phase): The current turn phase for the active player. See the Phase enum below.
- `boards` (Vec<PlayerBoard>): One board per player, indexed by player number (0 through players-1). See the PlayerBoard section below.
- `rolled_dice` (Vec<Die>): The dice currently showing for the active player this turn, in 1-based position order (position 1 is index 0). Empty outside the roll phases. See the Die enum below.
- `kept_dice` (Vec<Die>): Dice the active player has locked in (kept) and will not reroll. Skulls are always kept automatically.
- `remaining_rolls` (i32): Number of reroll actions the active player still has this turn (starts at 2).
- `remaining_workers` (i32): Workers the active player has left to spend during the Build phase this turn.
- `remaining_ships` (i32): Ships the active player has left to spend during the Trade (swap) phase this turn.
- `remaining_coins` (i32): Coins the active player has left to spend during the Buy phase this turn.
- `final_round` (bool): True once the end-game condition has been triggered (a player reached 7 developments, or all 7 monuments were built). Play continues until player 0 finishes their turn in this round.
- `finished` (bool): True when the game is fully over and final scores/placings apply.
- `rng` (GameRng): Internal random number generator state. Not meaningful to consumers.

## PlayerBoard

- `city_progress` (i32): Progress spent growing the city, 0 through 18. Crossing each threshold (3, 7, 12, 18) increases the number of dice rolled and food needed per turn. The current city/dice count is 3 plus the number of thresholds reached.
- `developments` (HashSet<DevelopmentId>): The developments this player owns. Each development is bought at most once across the whole game. See the DevelopmentId enum below.
- `monuments` (HashMap<MonumentId, i32>): Workers spent so far on each monument this player is building. A monument scores only when its progress reaches its size. Monuments not started are absent from the map. See the MonumentId enum below.
- `monument_built_first` (HashSet<MonumentId>): The monuments this player was the first to fully complete. These score the higher "first" value; other completed monuments score the lower "subsequent" value.
- `food` (i32): Food currently stored. Each turn the city must be fed an amount equal to the current city/dice count; a shortfall causes famine (disaster points).
- `goods` (HashMap<Good, i32>): Count of each good held, keyed by good type. Each good has its own cap (wood 8, stone 7, pottery 6, cloth 5, spearhead 4). A player may hold at most 6 goods total unless they own Caravans. See the Good enum below.
- `disasters` (i32): Disaster points accumulated by this player. Subtracted directly from the final score.
- `ships` (i32): Ships built (with the Shipping development), capped at 5. Each ship lets the player swap one good for another during the Trade phase.

## Phase enum

The fixed sequence of phases a turn passes through. Phases with no legal action
are skipped automatically.

- `Preserve`: With Preservation, at least 1 pottery, and food > 0, the player may spend 1 pottery to double their food before rolling.
- `Roll`: Dice are rolled; the player may reroll selected dice (up to 2 reroll actions). Skulls are locked in.
- `ExtraRoll`: With Leadership, the player may reroll exactly one die.
- `Collect`: Dice results are tallied into food, workers, goods, and coins. Any kept food-or-workers dice must be assigned via `take`.
- `Resolve`: Automatic. The city is fed and any skulls trigger a disaster.
- `Invade`: Reached only via a 4-skull invasion when the player has Smithing; spend spearheads for extra damage to opponents.
- `Build`: Spend workers on city progress, monuments, or (with Shipping) ships; with Engineering, trade stone for workers.
- `Trade`: With Shipping and at least one ship and one good, swap one good type for another, 1 ship per unit.
- `Buy`: Spend coins and/or goods on one not-yet-owned development; with Granaries, sell food for coins.
- `Discard`: If holding more than 6 goods total (without Caravans), discard down to the limit.

## Die enum

The six faces of a die:

- `Food`: +3 food (+1 more with Agriculture).
- `Good`: +1 good.
- `Skull`: +2 goods, and counts toward disasters this turn. Skulls are locked in and can never be rerolled.
- `Workers`: +3 workers (+1 more with Masonry).
- `FoodOrWorkers`: the player's choice of +2 food (+1 with Agriculture) or +2 workers (+1 with Masonry).
- `Coins`: +7 coins (+5 more, 12 total, with Coinage).

## Good enum

Wood, Stone, Pottery, Cloth, Spearhead - the five goods types. Per-type caps are
8, 7, 6, 5, 4 respectively. A good's scored value is the triangular number of
units held (n(n+1)/2) times its multiplier (wood x1, stone x2, pottery x3, cloth
x4, spearhead x5).

## DevelopmentId enum

The 17 one-time developments. Cost / points / effect:

- `Leadership`: 10 / 2 / reroll 1 die after your last regular roll.
- `Irrigation`: 10 / 2 / drought disaster has no effect on you.
- `Agriculture`: 15 / 3 / +1 food per food die (food and food-or-workers dice).
- `Quarrying`: 15 / 3 / +1 stone the first time you collect stone in a turn.
- `Medicine`: 20 / 4 / pestilence disaster has no effect on you.
- `Preservation`: 20 / 4 / once per turn, spend 1 pottery to double your food before rolling.
- `Coinage`: 20 / 4 / coin die results are worth 12 instead of 7.
- `Caravans`: 20 / 4 / never need to discard excess goods.
- `Shipping`: 25 / 5 / build ships, and swap 1 good for a different type per ship.
- `Smithing`: 25 / 5 / invasion disasters hit all opponents instead of you, and lets you invade back with spearheads.
- `Religion`: 25 / 7 / revolt disasters strip goods from all opponents instead of you.
- `Granaries`: 30 / 6 / sell food for 6 coins each.
- `Masonry`: 30 / 6 / +1 worker per worker die (worker and food-or-workers dice).
- `Engineering`: 40 / 6 / trade stone for 3 workers each.
- `Commerce`: 40 / 8 / bonus: +1 point per good held (scored at game end).
- `Architecture`: 60 / 8 / bonus: +2 points per monument you have fully built.
- `Empire`: 70 / 10 / bonus: +1 point per city.

## MonumentId enum

The 7 shared monuments. Size (workers needed) / first points / subsequent points / effect:

- `StepPyramid`: 3 / 1 / 0 / none.
- `StoneCircle`: 5 / 2 / 1 / none.
- `Temple`: 7 / 4 / 3 / none.
- `Obelisk`: 9 / 6 / 4 / none.
- `HangingGardens`: 11 / 8 / 5 / none.
- `GreatWall` (displayed as "Wall"): 13 / 10 / 6 / invasion disasters have no effect on you.
- `GreatPyramid`: 15 / 12 / 8 / none.

## TakeAction enum

The choice for a kept food-or-workers die during the Collect phase:

- `Food`: the die becomes food.
- `Workers`: the die becomes workers.
