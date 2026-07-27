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

On your turn, in this order:

1. **Play** at most one card from your hand to your palette (`play b4`). This
   does not end your turn.
2. **Discard** at most one card from your hand to change the active rule
   (`discard b4`). The discarded card's colour becomes the new rule, and you
   must be the leader under that new rule for the discard to be allowed. If the
   discarded card's number is higher than the number of cards in your palette,
   you draw a card. Discarding ends your turn immediately.
3. **Done** (`done`) - end your turn without discarding. If you have neither
   played nor discarded, you are eliminated.

Playing and then discarding in the same turn is allowed, and is usually the
strongest move: the played card strengthens your palette before the new rule is
judged. Because a discard ends your turn, you cannot play after discarding, and
`done` is only needed when you did not discard.

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

When all but one player is eliminated in a round, the remaining player scores
the cards in their palette that meet the current rule - not their whole palette.
Each card is worth its number, and the scored cards move out of the palette into
that player's score pile. The game ends as soon as any player reaches the target
score, and also ends if the deck no longer holds enough cards to deal the next
round. The player with the most points then wins; equal totals share the
placing.

Target scores:
- 2 players: 40 points
- 3 players: 35 points
- 4 players: 30 points
