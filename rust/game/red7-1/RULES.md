# Red7

Red7 is a card game for 2-4 players where the rules change every turn. Play
cards to your palette to become the leader under the current rule, or discard
cards to change the rule in your favour. If you aren't winning at the end of
your turn, you're out of the round.

## Setup

Each player is dealt 7 cards and starts with 1 card in their palette. The
starting rule is **Highest card** (red).

## Commands

- `play ##` - play a card to your palette, eg. `play b4`
- `discard ##` - discard a card and set the new rule, eg. `discard b4`
- `done` - finish your turn

## Turn

On your turn you may:

1. **Play** a card from your hand to your palette (once per turn).
2. **Discard** a card from your hand to change the active rule. The discarded
   card's colour determines the new rule. You must be the leader under the new
   rule after discarding. If the discarded card's number is higher than the
   number of cards in your palette, you draw a card.
3. **Done** - end your turn. If you haven't played or discarded, you are
   eliminated.

At the end of your turn, if you are not the leader under the current rule, you
are eliminated.

## Rules (by colour)

| Colour | Rule |
|--------|------|
| Red | Highest card |
| Orange | Same number |
| Yellow | Same color |
| Green | Even cards |
| Blue | Most colors |
| Indigo | In a row |
| Violet | Below 4 |

## Scoring

When all but one player is eliminated in a round, the remaining player (the
leader) scores their palette cards. The first player to reach the target score
wins the game.

Target scores:
- 2 players: 40 points
- 3 players: 35 points
- 4 players: 30 points
