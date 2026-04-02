# Acquire

## Overview

Players invest in hotel chains on a 9x12 grid by placing tiles, founding
corporations, buying shares, and triggering mergers. The player with the most
cash at the end wins.

---

## Board

The board is a grid of 9 rows (A-I) by 12 columns (1-12). Each cell is
identified by its row letter then column number: **A1**, **C7**, **I12**, etc.

Each player holds 6 tiles in hand and places one per turn.

---

## Corporations

There are 7 corporations, each with 25 shares available at game start.

| Name        | Color      | Base value |
|-------------|------------|-----------|
| Worldwide   | Purple     | $200      |
| Sackson     | Deep orange| $200      |
| Festival    | Green      | $300      |
| Imperial    | Yellow     | $300      |
| American    | Blue       | $300      |
| Continental | Red        | $400      |
| Tower       | Black      | $400      |

### Share price

Price = base value + size bonus:

| Size    | Bonus |
|---------|-------|
| 2       | $0    |
| 3       | +$100 |
| 4       | +$200 |
| 5       | +$300 |
| 6-10    | +$400 |
| 11-20   | +$500 |
| 21-30   | +$600 |
| 31-40   | +$700 |
| 41+     | +$800 |

Example: American (base $300) with 6 tiles = $300 + $400 = **$700/share**.

### Stockholder bonuses

When a corporation is acquired or the game ends, bonuses are paid to the
top shareholders of that corporation:

- **Primary (major) bonus** = share price x 10
- **Secondary (minor) bonus** = share price x 5

In Tycoon Mode (3+ players), a tertiary bonus is also paid.

Ties: tied players combine their respective bonus and the next tier's bonus,
sum them, divide evenly, and round up to the nearest $100.

A sole shareholder receives both the primary and secondary bonuses.

---

## Turn structure

Each turn has three steps:

### 1. Place a tile

Play one tile from your hand onto its matching board location. One of four
things happens:

- **Independent**: the tile has no orthogonally adjacent tiles. Nothing more
  happens.
- **Extend**: the tile is adjacent to an existing corporation. That corporation
  grows by one.
- **Found**: the tile connects to one or more independent tiles (not part of
  any corporation). You must choose which of the available corporations to
  found. The chosen corporation's tiles are now active, and you receive one
  free share as the founder's bonus.
- **Merge**: the tile connects two or more existing corporations. See Mergers.

A tile is unplayable if it would merge two or more safe corporations (size 11+).
Unplayable tiles are discarded at the end of your turn and replaced.

A tile that would found a corporation when no corporation tokens are available
cannot be played yet (hold it for a future turn).

### 2. Buy shares (optional)

After placing (and resolving any merger), you may buy up to **3 shares** total
from any active corporations. You may split across corporations or buy multiple
in one. Pay the current share price for each.

You cannot buy shares in a corporation that is not currently active (size 0).

Type `done` to skip buying or finish early.

### 3. Draw a tile

Draw one tile from the pile to replenish your hand to 6. Discard any newly
unplayable tiles and draw replacements.

---

## Mergers

When your tile connects two or more corporations:

1. The **largest** corporation survives; smaller ones are acquired. If tied in
   size, you (the mergemaker) choose which survives using the `merge` command.
2. Stockholder bonuses are paid for each acquired corporation (largest acquired
   first if multiple).
3. Starting with you and going clockwise, each player with shares in an
   acquired corporation must choose what to do with each share:
   - `keep` - hold for if the corporation is re-founded later (worth nothing
     until then).
   - `sell <n>` - sell back to the bank at the current pre-merger price.
   - `trade <n>` - trade 2 acquired shares for 1 share in the surviving
     corporation (bank must have shares available).
   You may mix sell, trade, and keep on the same turn by issuing multiple
   commands. Type `keep` when finished.
4. The acquired corporation's tiles become part of the surviving corporation.

A **safe corporation** (11+ tiles) cannot be acquired. Two safe corporations
cannot be merged.

---

## Ending the game

The game can be ended voluntarily on your turn if either:
- At least one active corporation has 41+ tiles, OR
- All active corporations are safe (11+).

Type `end` to trigger the end. You still complete your full turn (including
buying shares) before the game finishes.

At game end:
1. Stockholder bonuses paid for all active corporations.
2. All shares sold back to the bank at current prices.
3. Highest total cash wins; ties are shared.

---

## 2-player special rule

A dummy shareholder participates in merger bonuses. When a merger occurs, roll
a six-sided die (D6): the result (1-6) is the dummy's share count in the
acquired corporation. Bonuses the dummy would receive stay in the bank. The
dummy competes for bonuses at game end the same way.

---

## Board rendering

The board is drawn as a grid. Each cell is 5 characters wide, 2 lines tall.

- **Empty cells**: alternating light grey shades (checkerboard pattern). The
  cell name (e.g. `C4`) is shown in dark grey text.
- **Your playable tiles**: highlighted in **pink**. The cell name is shown in
  bold. These are the tiles you can legally play this turn.
- **Unincorporated tiles** (placed but not yet part of a corporation): solid
  dark grey background, no label.
- **Corporation tiles**: filled with the corporation's color. The corporation
  name abbreviation and current share price are shown in contrasting text
  over the widest contiguous run of that corporation's tiles in each row.

Below the board:
- A status line showing whether the game end can be triggered.
- Remaining draw tiles count.
- A **corporation table**: Corporation | Size | Value | Shares (in bank) |
  Minor bonus | Major bonus. Size 0 means the corporation is not yet founded.
- A **player table**: Player | Cash | shares held per corporation (columns
  in corporation order: WO, SA, FE, IM, AM, CO, TO).

---

## Command reference

| Situation             | Command                          | Example                    |
|-----------------------|----------------------------------|----------------------------|
| Place tile            | `play <tile>`                    | `play C4`                  |
| Found corporation     | `found <corp>`                   | `found American`           |
| Buy shares            | `buy <n> <corp>`                 | `buy 3 Festival`           |
| Finish buying         | `done`                           |                            |
| Choose merger winner  | `merge <corp> into <corp>`       | `merge Imperial into Tower`|
| Sell acquired shares  | `sell <n>`                       | `sell 2`                   |
| Trade acquired shares | `trade <n>`                      | `trade 4`                  |
| Done with shares      | `keep`                           |                            |
| Trigger end of game   | `end`                            |                            |

Corporation names can be abbreviated to a unique prefix (e.g. `Am` for
American, `Co` for Continental, `To` for Tower, `Fe` for Festival, `Im` for
Imperial, `Wo` for Worldwide, `Sa` for Sackson).

Only the commands valid for the current game phase will be accepted.

---

## Strategy notes

- Owning the most shares in a corporation about to be acquired pays a large
  primary bonus — worth more than holding shares in a large safe corporation.
- Trading 2-for-1 into the surviving corporation after a merger is often
  better than selling, since you gain a share in a now-larger corporation at
  no cost.
- The corporation table shows how many shares remain in the bank. If only 1-2
  shares are left, others may be blocked from buying in.
- Safe corporations (11+ tiles) cannot be acquired, so their bonuses only pay
  at game end.
