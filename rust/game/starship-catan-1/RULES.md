# Starship Catan

Starship Catan is a 2-player space exploration and economy game. You fly your
starship through four sectors of space, founding colonies and trading posts on
the planets you visit, fighting off pirates, completing adventures, and
building up your ship and modules. The first player to reach 10 victory points
wins.

## Components

### Resources

There are several resource types, shown in the display by their coloured name:

- **Goods** - `food` (red), `fuel` (grey), `carbon` (cyan), `ore` (default
  foreground), `trade` (yellow). Each good bay holds at most 2 by default; the
  logistics module raises this cap.
- `science` (purple) - held up to a maximum of 4.
- `astro` (green) - the money, rendered as `$N`.
- **Buildables** - `colony ship` and `trade ship` (you may hold at most 2 ships
  total), `booster` (up to 6), `cannon` (up to 6).

### Modules

Each player may own six module types at levels 0, 1 or 2. You start by choosing
one module at level 1 for free; modules are upgraded to level 2 (or bought at
level 1) during the trade and build phase.

| Module | Effect |
|--------|--------|
| `logistics` | Raise the per-good storage cap (2, 3, 4). |
| `command` | Raise the action limit during flight (2, 3, 4). |
| `sensor` | Peek at the top sector cards of a flight (0, 2, 3) and reorder them. |
| `trade` | Buy goods from your opponent for $2 each during trade and build. |
| `science` | Produce science on certain yellow-die rolls. |
| `production` | Produce trade goods on certain yellow-die rolls. |

### Sector cards

Each sector is a pile of face-down cards. There are six kinds:

- **Colony planet** - e.g. `Alioth VIII (colony planet, roll 1 for carbon)`.
  Found a colony here (costs a colony ship) to score 1 VP; the colony produces
  its resource whenever the yellow die matches its number.
- **Trade planet** - e.g. `Saiph VI (buy/sell food for $3 each, trading post)`.
  Buy/sell the listed resources at the listed price while landed. If it is a
  `trading post`, you may found a trading post here (costs a trade ship) for a
  persistent buy/sell price and 1 diplomat point. Some trade planets restrict
  the direction (`buy`/`sell`) and/or a maximum amount.
- **Pirate ship** - e.g. `pirate ship, asking a ransom of $3`. You must fight,
  pay the ransom, or (if you lose a fight that destroys a module) sacrifice a
  module. Defeating a pirate scores 1 medal.
- **Median** - `Median (2 diplomat points)`. You may found a trading post here
  (2 diplomat points).
- **Lost Planet** - `Lost Planet (empty space)`. Nothing happens.
- **Adventure planet** - one of `Hades`, `Pallas`, `Picasso`, `Poseidon`.
  Complete an adventure card matching this planet here.

### Adventure cards

Three adventure cards are face up at all times (the rest form a draw deck).
Each is tied to a planet and has a cost and a reward (resources, medals, or
victory points). When you land on the matching adventure planet you may
complete one of the face-up cards whose planet matches.

## Turn structure

The game begins in the choose-module phase, then each turn cycles through
produce, choose-sector, flight, and trade-and-build. Only the current player
acts, except during production (both players produce) and a pending resource
choice (the named player resolves it).

### 1. Choose module (first turn only)

Both players pick one starting module at level 1, for free. Either player may
choose first.

```
choose lo      # choose the logistics module (prefix match)
choose sensor  # full name also works
```

Module names are matched by unique prefix, case-insensitively: `lo` is
logistics, `se` is sensor, but `s` alone is ambiguous (science/sensor) and is
rejected.

### 2. Produce

The current player rolls the yellow die (a value of 1-3) and both players
produce from it, the current player first:

- A player whose trade module triggers on the roll gains 1 trade good.
- A player whose science module triggers on the roll gains 1 science.
- A player gains each distinct colony resource whose colony number equals the
  roll.

If a player can produce more than one resource and has room for several, they
are prompted to pick one with `gain`:

```
gain food      # take the food rather than another producible good
```

Production that cannot fit is lost. Once both players have produced, the phase
advances.

### 3. Choose sector

The current player picks which of the four sectors to fly through:

```
sector 3       # travel through sector 3
```

With a level-1 sensor module you peek at the top 2 cards of that sector; with
level 2 you peek at 3. Place each peeked card on the top or bottom of the pile
in any order, then the first card is drawn:

```
put 1 bottom   # put peeked card #1 on the bottom
put 2 top      # put peeked card #2 on the top
```

Without a sensor module the first card is drawn immediately.

### 4. Flight

Your ship travels the sector one card at a time. Your flight distance is the
yellow die plus your boosters; you also have a limited number of actions (2
plus your command module). At each planet you may take the action shown there,
then advance:

```
next           # draw the next sector card
end            # end the flight early and go to trade and build
```

You cannot `next` past a pirate you have not dealt with. When you run out of
moves or actions (or the sector pile empties), the flight ends automatically.
The planets you visited are shuffled back into the sector pile.

Planet actions during flight:

```
found          # on a colony planet: spend a colony ship, gain the colony (1 VP)
found          # on a trade/median planet that allows it: spend a trade ship,
               #   found a trading post
buy 2 food     # on a trade planet: buy 2 food at the planet's price
sell 1         # on a trade planet: sell 1 of the planet's resource
fight          # on a pirate: roll your cannon against the pirate's strength
pay            # on a pirate: pay its ransom in astro to pass
lose sensor    # after losing a module-destroying fight: pick the module lost
complete 1     # on an adventure planet: complete face-up adventure #1
```

Founding or trading replaces the planet you are on with a fresh card from the
draw pile (you keep your position). Fighting compares your roll plus cannon
against the pirate's roll plus strength; you win ties. Winning a fight takes
the pirate as a defeated pirate (1 medal) and draws a replacement; losing may
destroy a cannon or, for stronger pirates, force you to lose a module.

### 5. Trade and build

After the flight you may trade and build:

```
buy 2 carbon   # buy via one of your trading posts (up to 2 trades per phase)
sell 1 ore     # sell via one of your trading posts
take carbon    # buy 1 good from your opponent for $2 (trade module permits)
build colony   # build a colony ship (ore, fuel, food)
build trade    # build a trade ship (ore, fuel, trade)
build booster  # build a booster (2 fuel, plus 1 science once you have 3+)
build cannon   # build a cannon (2 carbon, plus 1 science once you have 3+)
upgrade trade  # upgrade a module one level (ore, carbon, and food)
done           # end your turn; the other player begins their produce phase
```

Trading-post prices come from the trading posts you have founded: you buy at
your cheapest buy price and sell at your best sell price for each resource. You
may make up to two trading-post trades per phase. `take` lets you buy a good
directly from your opponent for $2, a number of times equal to your trade
module level (they lose that good).

## Scoring

Victory points (VP) are the sum of:

- 1 VP for each colony you have founded.
- 1 VP for the `Epidemic` or `Monument` adventure if completed.
- 1 VP for each module at level 2.
- 1 VP if you are the hero of the people (medals leader, more than 3 medals).
- 1 VP if you are the friend of the people (diplomacy leader, more than 3
  diplomat points).

Medals come from defeated pirates (1 each) and several adventures. Diplomat
points come from trading posts (1 each) and the Median (2). The hero/friend
titles are awarded only while a player has strictly more than 3 of the relevant
total and strictly more than their opponent.

Worked example for one player:

| Source | Count | VP |
|--------|-------|----|
| Colonies founded | 5 | 5 |
| Trading posts founded | 2 | 0 (diplomat points, not VP) |
| Defeated pirates | 2 | 0 (medals, not VP) |
| `Monument` adventure completed | 1 | 1 |
| Modules at level 2 | 2 | 2 |
| Hero of the people (5 medals > 3, leads) | yes | 1 |
| Friend of the people (3 diplomat points, not > 3) | no | 0 |
| **Total** | | **9** |

## Game end and winning

There is no end-game action: the game ends the instant any player reaches 10 or
more VP. The player with the higher VP total wins; if both reach 10 VP and are
tied, the game is a draw.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `choose <module>` | Pick your free starting module (choose-module phase). | `choose lo` |
| `gain <resource>` | Take one of the resources you are producing. | `gain food` |
| `put <#> <top\|bottom>` | Place a peeked sensor card on the top or bottom of the pile. | `put 1 bottom` |
| `sector <1-4>` | Choose which sector to fly through. | `sector 3` |
| `next` | Advance to the next sector card during flight. | `next` |
| `end` | End the flight early. | `end` |
| `found` | Found a colony (colony planet) or trading post (trade/median planet). | `found` |
| `buy <n> [<resource>]` | Buy resources (flight trade planet or trade-and-build posts). | `buy 2 food` |
| `sell <n> [<resource>]` | Sell resources (flight trade planet or trade-and-build posts). | `sell 1` |
| `fight` | Fight the current pirate. | `fight` |
| `pay` | Pay the current pirate's ransom. | `pay` |
| `lose <module>` | Choose which module a pirate destroys. | `lose sensor` |
| `complete <#>` | Complete the numbered face-up adventure on a matching planet. | `complete 1` |
| `take <good>` | Buy one good from your opponent for $2 (trade-and-build). | `take carbon` |
| `build <item>` | Build a trade ship, colony ship, booster, or cannon. | `build colony` |
| `upgrade <module>` | Upgrade a module one level. | `upgrade trade` |
| `done` | End your turn. | `done` |

Resource and module arguments are matched by unique case-insensitive prefix
(`lo` = logistics, `se` = sensor, `colony` = colony ship). The resource on
`buy`/`sell` may be omitted when only one resource is tradable.
