# Alhambra

Alhambra is a tile-laying game for 2-6 players (2-player mode adds a bot
named Dirk). Collect money cards, buy building tiles from the market, and
arrange them on your grid to score points across three scoring rounds.

## Setup

Each player starts with money cards and an empty grid with a fountain tile in
the centre. The market displays 4 tiles, one per currency.

## Commands

- `take <card> (<card>...)` - take money cards (multiple cards must total <= 5, or single card any value)
- `spend <card> (<card>...)` - spend cards of one currency to buy a tile
- `place <#> <coord>` - place a tile from your placeable/reserve tiles
- `swap <#> <coord>` - swap a reserve tile with a grid tile
- `remove <coord>` - remove a grid tile to your reserve
- `done` - end your turn, moving placeable tiles to reserve

## Turn

On your turn you perform one action:

1. **Take money** - take multiple cards whose combined value is <= 5, or a
   single card of any value. Cards come in 4 currencies (Blue, Green, Red,
   Yellow) with values 1-9.
2. **Buy a tile** - spend cards of a single currency to buy a tile from the
   market. If you pay exactly the tile's cost, you get another action;
   overpaying ends your turn. The bought tile goes to your place area.
3. **Manage reserve** - place tiles from reserve onto your grid, swap reserve
   tiles with grid tiles, or remove grid tiles to your reserve.

## Grid Rules

- Tiles must connect to the fountain.
- Walls must match between adjacent tiles.
- No enclosed gaps are allowed.

## Scoring

Scoring happens 3 times, triggered by scoring cards in the deck:

- Each of 6 tile types scores based on who has the most of that type (points
  from a per-type table).
- Longest external wall scores 1 point per wall segment.

After the tile bag is empty, remaining tiles go to players with the most money
of that currency. Final scoring occurs after all tiles are placed.

Highest total points wins.
