# Acquire Data Dictionary

## PubState (public information)
- `phase` (Phase): current game phase - Play(usize), Found{player,at}, Buy{player,remaining}, ChooseMerger{player,at}, or SellOrTrade{player,corp,into,at,turn_player}
- `players` (Vec<PubPlayer>): public info for each player - money and share counts
- `board` (Board): the 9x12 tile grid showing placed tiles, corporation ownership, and empty/fixed status
- `shares` (HashMap<Corp, usize>): available (unowned) share count per corporation
- `remaining_tiles` (usize): number of tiles left in the draw bag
- `last_turn` (bool): whether the final round has been triggered
- `finished` (bool): whether the game has ended

## PubPlayer
- `money` (usize): player's current cash
- `shares` (HashMap<Corp, usize>): number of shares held per corporation

## PlayerState (player-private information)
- `public` (PubState): the full public game state
- `player` (usize): this player's seat index
- `tiles` (Vec<Loc>): the player's hand of tiles (positions on the board grid)
