# Roll Through the Ages

## Overview

Roll Through the Ages is a 2-4 player dice-and-resource civilization game. Each turn you roll dice to gather food, workers, goods and coins, then spend them to grow your city, research developments, and build monuments, all while dodging disasters (or inflicting them on your rivals). The goal is to build the highest-scoring civilization before the game ends.

## Components

### Dice

Six 6-sided dice show these faces:

| Face | Symbol | Meaning |
|------|--------|---------|
| Food | `FFF` | +3 food (+1 with Agriculture) |
| Good | `G` | +1 good |
| Skull | `GXG` | +2 goods, but also counts toward disasters this turn |
| Workers | `WWW` | +3 workers (+1 with Masonry) |
| Food or Workers | `FF/WW` | your choice of +2 food (+1 with Agriculture) or +2 workers (+1 with Masonry) |
| Coins | `C` | +7 coins (+5 more, 12 total, with Coinage) |

You roll one die per city you have (starting at 3, more as your city grows - see Cities below).

### Cities

City progress runs from 0 to 18. Crossing each of these thresholds increases the number of dice you roll and the food your city needs each turn by 1: 3, 7, 12, 18. So a fresh city (0 progress) uses 3 dice/food, and a fully-grown city (18 progress) uses 7.

### Goods

Five goods types, each with its own holding cap and per-unit-count value (value = triangular number of units held, scaled by a per-good multiplier):

| Good | Cap | Value formula | Value at cap |
|------|-----|----------------|---------------|
| Wood | 8 | n(n+1)/2 x 1 | 36 |
| Stone | 7 | n(n+1)/2 x 2 | 56 |
| Pottery | 6 | n(n+1)/2 x 3 | 63 |
| Cloth | 5 | n(n+1)/2 x 4 | 60 |
| Spearhead | 4 | n(n+1)/2 x 5 | 50 |

Goods above the 6-goods-of-any-type limit must be discarded at the end of your turn (`discard 1 wood`), unless you have Caravans. You may hold at most 6 goods total across all types, but each type has its own separate per-type cap shown above.

### Developments

17 one-time purchases, each bought once (any development already owned by a player can't be bought again by anyone):

| Development | Cost | Points | Effect |
|---|---|---|---|
| Leadership | 10 | 2 | Reroll 1 die after your last regular roll |
| Irrigation | 10 | 2 | Drought disaster has no effect on you |
| Agriculture | 15 | 3 | +1 food per food die (food dice and food-or-workers dice) |
| Quarrying | 15 | 3 | +1 stone the first time you collect stone in a turn |
| Medicine | 20 | 4 | Pestilence disaster has no effect on you |
| Preservation | 20 | 4 | Once per turn, spend 1 pottery to double your food before rolling |
| Coinage | 20 | 4 | Coin die results are worth 12 instead of 7 |
| Caravans | 20 | 4 | Never need to discard excess goods |
| Shipping | 25 | 5 | Build ships, and swap 1 good for a different type per ship |
| Smithing | 25 | 5 | Invasion disasters hit all your opponents instead of you, and lets you invade back with spearheads |
| Religion | 25 | 7 | Revolt disasters strip goods from all your opponents instead of you |
| Granaries | 30 | 6 | Sell food for 6 coins each |
| Masonry | 30 | 6 | +1 worker per worker die (worker dice and food-or-workers dice) |
| Engineering | 40 | 6 | Trade stone for 3 workers each |
| Commerce | 40 | 8 | Bonus: +1 point per good held (scored at game end) |
| Architecture | 60 | 8 | Bonus: +2 points per monument you've fully built |
| Empire | 70 | 10 | Bonus: +1 point per city |

### Monuments

7 shared monuments, built with workers. Each is built by every player independently (your own workers, your own progress), but only the first player to fully complete a given monument scores its higher "first" value - everyone else who later completes it scores the lower "subsequent" value instead.

| Monument | Size (workers) | First / Subsequent points | Effect |
|---|---|---|---|
| Step Pyramid | 3 | 1 / 0 | - |
| Stone Circle | 5 | 2 / 1 | - |
| Temple | 7 | 4 / 3 | - |
| Obelisk | 9 | 6 / 4 | - |
| Hanging Gardens | 11 | 8 / 5 | - |
| Wall | 13 | 10 / 6 | Invasion disasters have no effect on you |
| Great Pyramid | 15 | 12 / 8 | - |

A monument only scores once it is fully built (progress >= size); partial progress scores nothing.

## Turn Structure

Your turn moves through a fixed sequence of phases. Many phases are skipped automatically if you have no legal action there, so a single `next` (or even a single `roll`) can cascade through several phases at once.

1. **Preserve** - if you have Preservation, at least 1 pottery, and food > 0, you may spend 1 pottery to double your current food before rolling (`preserve`). Otherwise this phase is skipped automatically.
2. **Roll** - dice equal to your current number of cities are rolled automatically. Any skulls rolled are locked in immediately (they can never be rerolled). You then get up to 2 reroll actions, each specifying which of your currently-rolled dice (1-based positions) to reroll; dice you don't list are kept as-is (`roll 1 3`, `roll 2`). Skip further rerolling early with `next`.
3. **Extra Roll** - if you have Leadership, you may reroll exactly one die from everything you're currently holding (`roll 4`), otherwise this phase is skipped automatically.
4. **Collect** - all dice results are tallied: food and worker dice add to your stock, good/skull dice add goods (round-robin across wood/stone/pottery/cloth/spearhead, starting at wood every time), and coin dice add coins. If you kept any food-or-workers dice, you must choose what each one becomes: `take food workers` (one choice per such die, in order).
5. **Resolve** - automatic. Your city is fed (famine gives you disaster points if you don't have enough food), then any skulls collected this turn trigger a disaster: 2 skulls = drought (2 disaster points, blocked by Irrigation), 3 skulls = pestilence (3 disaster points to every player, blocked per-player by Medicine), 4 skulls = invasion (4 disaster points - to you, blocked by the Wall monument, unless you have Smithing, in which case it hits all opponents instead and you enter the Invade phase), 5+ skulls = revolt (you lose all your goods, unless you have Religion, in which case all opponents lose all their goods instead).
6. **Invade** (only reached via a 4-skull invasion disaster when you have Smithing) - spend spearheads for extra damage: each spearhead deals 2 disaster points to every opponent not protected by the Wall (`invade 2`). Skip with `next`.
7. **Build** - spend workers on city progress (`build 3 city`), on any in-progress monument (`build 3 temple`), or (with Shipping, spending wood + cloth 1:1 instead of workers) on ships (`build 1 ship`, capped at 5 ships total). If you have Engineering and hold stone, you may also trade stone for workers here, 3 workers per stone (`trade 2`). Skip with `next` once you're done, or it's skipped automatically if you have no legal build/trade option.
8. **Trade** (ship-based good swapping, only if you have Shipping and at least 1 ship and 1 good) - spend ships to convert one good type into another, 1 ship per unit swapped, respecting the target good's cap (`swap 2 wood spearhead`). Skip with `next`.
9. **Buy** - spend coins and/or goods (by type - naming a good type spends your *entire* stack of that type, not a partial amount) on one not-yet-owned development (`buy leadership`, `buy engineering wood stone`, `buy shipping all` to spend every good type you hold). If you have Granaries, you may also sell food for 6 coins each here (`sell 3`). Skipped automatically if your total buying power (coins + goods value, plus food x6 with Granaries) is under 10, the cheapest development's cost.
10. **Discard** - if you're holding more than 6 goods total (and don't have Caravans), discard down to the limit one good type and amount at a time (`discard 2 stone`). This ends your turn once you're at or under the limit.

Once your turn fully resolves, play passes to the next player.

## Scoring

Score = development points + monument points (first-time value if you were the first to complete it, subsequent value otherwise, 0 if not fully built) + bonus points (Commerce: +1 per good currently held; Architecture: +2 per monument you've fully built; Empire: +1 per city) - disaster points (raw subtraction, not scaled).

Worked example, using the mid-game position below:

| Player | Developments | Monuments | Bonuses | Disasters | Score |
|---|---|---|---|---|---|
| Alice | Leadership (2) + Agriculture (3) = 5 | Step Pyramid built first: 1; Temple in progress (4/7, not built): 0 | none owned | -2 | 5 + 1 + 0 - 2 = **4** |
| Bob | Masonry (6) | none built | none owned | -4 | 6 - 4 = **2** |

## Game End

The final round is triggered the instant either (a) a player completes their 7th distinct development, or (b) all 7 monuments have been fully built (each by some player, not necessarily the same one). Once triggered, the game continues until the last player (player index 0) has completed their turn in that round, then the game finishes.

## Winning

The player with the highest score wins. Ties are broken using standard competition ranking (e.g. two tied players both take 1st, the next player takes 3rd) based purely on score - there is no goods-value or other tiebreaker in this version.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `next` | Advance to the next phase, skipping any remaining optional action | `next` |
| `roll <dice#>...` | Reroll the listed 1-based dice positions (others are kept) | `roll 1 3` |
| `take <food\|workers>...` | Choose food or workers for each kept food-or-workers die, in order | `take food workers` |
| `preserve` | Spend 1 pottery to double your food before rolling | `preserve` |
| `build <n> city` | Spend n workers on city progress | `build 3 city` |
| `build <n> <monument>` | Spend n workers on a monument | `build 4 temple` |
| `build <n> ship` | Spend n wood + n cloth to build ships (needs Shipping) | `build 1 ship` |
| `trade <n>` | Convert n stone into 3n workers (needs Engineering) | `trade 2` |
| `buy <development> [all\|<good>...]` | Buy a development using coins plus optionally-named good stacks | `buy engineering wood stone` |
| `sell <n>` | Sell n food for 6 coins each (needs Granaries) | `sell 3` |
| `swap <n> <from> <to>` | Swap n of one good type for n of another, 1 ship each (needs Shipping) | `swap 2 wood spearhead` |
| `discard <n> <good>` | Discard n of a good type to get under the goods limit | `discard 1 stone` |
| `invade <n>` | Spend n spearheads for 2n extra disaster points per unprotected opponent (needs Smithing) | `invade 2` |
