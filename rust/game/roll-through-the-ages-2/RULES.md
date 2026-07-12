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

## Reading the Display

The render below is a real mid-game position (2 players, Alice building her city/monuments, Bob holding goods). It shows:

- **Dice** - the current roll (empty here, since it's the Build phase and dice have already been resolved for the turn).
- **Turn supplies** - remaining workers and coins for the current player this turn. The Build/Buy variant shown here lists `Workers:` and `Coins: N (M including goods)` (M adds the coin value of your held goods, useful for judging buying power). The Trade-phase variant instead shows `Ships:` in place of `Workers:`.
- **Cities** - one row per player, showing a run of `X`/`x` markers (bold and in the current player's color for the current player's own city progress) for progress already spent, followed by `.` markers for progress remaining, in bands of 10 with a header row marking the 3/4/5/6/7 city-count thresholds; `(N left)` shows the remaining progress to the maximum.
- **Development** table - one row per development, in a fixed order, showing an `X` (bold, colored) under each player who owns it and a `.` if they don't, then Cost/Pts/Effect columns.
- **Monument** table - one row per monument. A cell shows `.` if a player hasn't started it, a plain number if partially built, or a bold `X` if that player fully completed it. The Pts column shows `first/subsequent` (e.g. `4/3` for the Temple).
- **Resource** table - one row per good (in reverse order: spearhead, cloth, pottery, stone, wood), showing each player's held count and its scored value in parentheses (e.g. `2 (6)`), a `total` row summing goods count and value, then food/ship/disaster/score rows. The current player's own cells are bold and in their color; other players' cells are plain-colored.

```brdgme
{{b}}Dice{{/b}} {{fg rgb(97,97,97)}}(F: food, W: worker, G: good, C: coin, X: skull){{/fg}}
{{table}}{{row}}{{/row}}{{row}}{{/row}}{{/table}}

{{table}}{{row}}{{cell left}}{{b}}Turn supplies{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}Workers:{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}4{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}Coins:{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}0 (15 including goods){{/cell}}{{/row}}{{/table}}

{{b}}Cities{{/b}} {{fg rgb(97,97,97)}}(number of dice and food used per turn){{/fg}}
{{table}}{{row}}{{cell left}}{{b}}Player{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}3{{/b}}     {{b}}4{{/b}}       {{b}}5{{/b}}         {{b}}6{{/b}}           {{b}}7{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{b}}{{fg player(0)}}X{{/fg}}{{/b}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}{{fg rgb(97,97,97)}}(10 left){{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{player 1}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg player(1)}}x{{/fg}} {{fg player(1)}}x{{/fg}} {{fg player(1)}}x{{/fg}} {{fg player(1)}}x{{/fg}} {{fg player(1)}}x{{/fg}} {{fg player(1)}}x{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{fg rgb(97,97,97)}}.{{/fg}} {{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}(13 left){{/fg}}{{/cell}}{{/row}}{{/table}}

{{table}}{{row}}{{cell left}}{{b}}Development{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{player 1}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Cost{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Pts{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Effect{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}Leadership{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}X{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 10{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 2{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}reroll 1 die (after last roll){{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Irrigation{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 10{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 2{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}drought has no effect{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Agriculture{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}X{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 15{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 3{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}+1 food / food die{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Quarrying{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 15{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 3{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}+1 stone if collecting stone{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Medicine{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 20{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 4{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}pestilence has no effect{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Preservation{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 20{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 4{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}food x2 before roll for 1 pottery{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Coinage{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 20{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 4{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}coin die results are worth 12{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Caravans{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 20{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 4{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}no need to discard goods{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Shipping{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 25{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 5{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}swap 1 good / ship{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Smithing{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 25{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 5{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}invasion affects opponents{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Religion{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 25{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 7{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}revolt affects opponents{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Granaries{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 30{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 6{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}sell food for 6 coins each{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Masonry{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}x{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 30{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 6{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}+1 worker / worker die{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Engineering{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 40{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 6{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}use stone for 3 workers each{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Commerce{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 40{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 8{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}bonus pts: 1 / good{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Architecture{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 60{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 8{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}bonus pts: 2 / monument{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Empire{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 70{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 10{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}bonus pts: 1 / city{{/fg}}{{/cell}}{{/row}}{{/table}}

{{table}}{{row}}{{cell left}}{{b}}Monument{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{player 1}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Size{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Pts{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}Effect{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}Step Pyramid{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}X{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 3{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}1{{/b}}/0{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Stone Circle{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 5{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}2{{/b}}/1{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Temple{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(0)}}4{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 7{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}4{{/b}}/3{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Obelisk{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 9{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}6{{/b}}/4{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Hanging Gardens{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 11{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}8{{/b}}/5{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Wall{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 13{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}10{{/b}}/6{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}invasion has no effect{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}Great Pyramid{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}} 15{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{b}}12{{/b}}/8{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}{{/fg}}{{/cell}}{{/row}}{{/table}}

{{table}}{{row}}{{cell left}}{{b}}Resource{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{player 0}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{player 1}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(251,192,45)}}spearhead{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}1 (5){{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}cloth{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}2 (12){{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}pottery{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}1 (3){{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(97,97,97)}}stone{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}2 (6){{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(123,31,162)}}wood{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}3 (6){{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}.{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}total{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}6 (15){{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}3 (17){{/fg}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(56,142,60)}}food{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}5{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}2{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(25,118,210)}}ship{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}1{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}0{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}{{fg rgb(211,47,47)}}disaster{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}2{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}4{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{b}}score{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg player(0)}}4{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg player(1)}}2{{/fg}}{{/cell}}{{/row}}{{/table}}
```

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

## Strategy Tips

No rulebook strategy tips are available yet; this section will be populated from the official rulebook or user-supplied advice.
