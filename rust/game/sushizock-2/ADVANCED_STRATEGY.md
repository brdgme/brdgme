# Sushizock - Advanced Strategy

Higher-level strategic considerations.

## Tile economy

- The scoring formula means blue tiles only score up to your red tile count. The optimal ratio is roughly equal blue and red tiles, with blue values exceeding the absolute red values.
- A -1 red tile unlocking a 6-value blue tile nets +5. A -4 red tile unlocking a 1-value blue tile nets -3. Evaluate the marginal value of each tile.
- Late in the game, tile availability constrains your options. Track which tiles remain and plan accordingly.

## Dice probability

- Each die has: 2/6 sushi, 2/6 bones, 1/6 blue chopsticks, 1/6 red chopsticks. Sushi and bones are twice as likely as chopsticks.
- Re-rolling 3 dice gives you a reasonable chance of improving, but re-rolling 4+ dice is high variance. Keep what you can use.
- If you need exactly 1 more sushi to reach a target tile, re-rolling 2 dice gives roughly a 56% chance of getting at least one sushi.

## Opponent interaction

- Stealing is the primary interaction. Watch opponents' tile stacks and target their most valuable tiles.
- Stealing a blue tile from an opponent with many red tiles hurts them most (they lose scoring potential). Stealing from an opponent with few red tiles is less impactful.
- Dumping a red tile on an opponent who has more blue tiles than red tiles is devastating - it reduces their scoring capacity.
- In multiplayer games, consider who is winning. Stealing from the leader is often better than optimizing your own tiles.

## Turn order awareness

- Tiles are taken in order from the row. If you need the 3rd blue tile, you need exactly 3 sushi. Having 4 sushi means you take the 4th tile instead.
- Plan your dice count to target specific tiles. Sometimes keeping fewer sushi is better if it lets you take the tile you want.
- The forced take gives you the worst tile. If you are forced to take, you get the most negative red (or lowest blue if no red remains). This is always bad; avoid it.

## Endgame

- When few tiles remain, every tile matters. Calculate the exact scoring impact before taking.
- If you are ahead, play conservatively. If you are behind, stealing becomes more important to close the gap.
- The game ends when both piles are empty. Track tile counts to know how many turns remain.
