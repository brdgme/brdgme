# Lost Cities

A 2-3 player card game of risk and reward. Build profitable expeditions across five routes, but every expedition you start costs points - so only commit when you can make it worth it.

## Cards

60 cards across five expeditions:

| Letter | Expedition |
|--------|------------|
| R | Red |
| G | Green |
| W | White |
| B | Blue |
| Y | Yellow |

Each expedition has:
- 3 wager cards (shown as **X**) - multipliers
- 9 numbered cards (2 through 10)

## Setup

Each player draws 8 cards (2 players) or 7 cards (3 players). The remaining cards form the draw pile. Five shared discard piles (one per expedition) start empty.

## Turn Structure

Each turn has two phases, in order:

### 1. Play or Discard

Choose one card from your hand:

- **Play** it face-up to that expedition on your side (`play g5`, `play rx`)
- **Discard** it face-up to the shared discard pile for that color (`discard y2`, `discard bx`)

**Expedition rules:**
- Cards within an expedition must be played in strictly ascending order
- Wager cards (X) must be played before any numbered card in that expedition
- Once a numbered card is played, no more wager cards can be added to that expedition
- You may play 1, 2, or 3 wager cards before your first numbered card

**Example:** Your Green expedition has G4. You can play G5, G6 ... G10, but not G2, G3, or GX.

### 2. Draw or Take

After playing or discarding, draw one card:

- **Draw** from the top of the draw pile (`draw`)
- **Take** the top card from any shared discard pile (`take g`, `take r`)

You cannot take a card you discarded in the same turn.

## Scoring

Each expedition you started is scored independently:

```
score = (sum of numbered cards - cost) x (wager cards + 1)
```

| Players | Expedition cost | Bonus threshold | Bonus |
|---------|----------------|-----------------|-------|
| 2 | 20 | 8+ cards | +20 |
| 3 | 15 | 7+ cards | +15 |

- **0 wagers** = x1, **1 wager** = x2, **2 wagers** = x3, **3 wagers** = x4
- The **bonus** applies to total cards in the expedition, including wagers
- Expeditions you never started score 0 - no cost, no reward

**Example (2-player):**

| Expedition | Cards Played | Wagers | Calculation | Score |
|------------|-------------|--------|-------------|-------|
| Red | R5, R8, R10 | 0 | (23 - 20) x 1 | **3** |
| Green | GX, G4, G7, G9 | 1 | (20 - 20) x 2 | **0** |
| Blue | BX, BX, B2, B3, B4, B5, B6, B7, B8 | 2 | (35 - 20) x 3 + 20 bonus | **65** |
| White | - | - | not started | **0** |
| Yellow | YX, Y3 | 1 | (3 - 20) x 2 | **-34** |

**Total: 34 points**

The Blue expedition scores big: 2 wagers triple the value, and 9 cards trigger the 20-point bonus. The Yellow expedition is a costly mistake - a wager card doubled the loss from starting an expedition with too few cards.

## Rounds

The game plays 3 rounds. A round ends immediately when the draw pile is exhausted.

After scoring, the player currently in the lead starts the next round. In a 3-player game, the leading player who is next in clockwise order from the current first player starts. On a tie for the lead, the first tied player clockwise starts.

## Winning

After 3 rounds, the player with the highest cumulative score wins.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `play <card>` | Play card to your expedition | `play g5`, `play rx`, `play b10` |
| `discard <card>` | Discard card to shared pile | `discard y2`, `discard wx` |
| `draw` | Draw from the deck | `draw` |
| `take <color>` | Take top card from discard pile | `take g`, `take r`, `take w` |

Cards are written as color letter + value: `G5` (Green 5), `RX` (Red wager), `B10` (Blue 10).
