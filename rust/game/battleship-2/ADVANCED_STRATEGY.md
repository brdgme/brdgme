# Advanced Strategy

## Search Patterns
- Use a checkerboard search pattern (shoot only cells where row+col is even or odd) to find remaining ships efficiently - every ship of size 2+ must cover at least one checkerboard cell
- Space your initial shots based on the smallest remaining unsunk ship size

## Targeting After Hits
- After a hit, use probability-based targeting: cells adjacent to multiple hits or near known ship alignments have higher probability
- Once you have two hits in a line, continue along that axis until the ship is sunk
- When a direction from a hit yields a miss, try the opposite direction before abandoning the line
- Track which ships are sunk to know what sizes remain - don't waste shots in gaps too small for any remaining ship

## Placement Psychology
- Avoid common patterns: diagonal lines, the border, symmetrical arrangements
- Place small ships (destroyer, submarine) in open water away from larger ships
- Consider placing ships in the interior rather than edges, where opponents often search first
- Vary your placement style between games to avoid predictability

## Endgame Counting
- Track which ships have been sunk to know exactly what remains
- When few ships remain, switch to systematic sweeping of the checkerboard cells that can still contain them
- Count remaining cells vs remaining ship sizes to identify impossible placements and eliminate wasted shots
