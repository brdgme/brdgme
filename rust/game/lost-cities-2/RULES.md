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

## Reading the Display

**2-player layout:**

```brdgme
{{table}}{{row}}{{cell center}}Round {{b}}2{{/b}} of {{b}}3{{/b}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G9{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G8{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G7{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W10{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}B3{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}Y10{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G4{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W9{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}BX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}Y6{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell right}}{{player 1}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}GX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W6{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}BX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}Y5{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell right}}{{fg rgb(97,97,97)}}Discard{{/fg}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R3{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G3{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W8{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}B2{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}YX{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}   {{b}}7{{/b}} left{{/fg}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell right}}{{player 0}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R4{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G5{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}YX{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R5{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G10{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}Y3{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R6{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(56,142,60)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R8{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(56,142,60)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R10{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(56,142,60)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell center}}{{fg rgb(97,97,97)}}Your hand{{/fg}}{{/cell}}{{/row}}{{row}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}RX{{/fg}}{{/b}} {{b}}{{fg rgb(211,47,47)}}RX{{/fg}}{{/b}} {{b}}{{fg rgb(211,47,47)}}RX{{/fg}}{{/b}} {{b}}{{fg rgb(97,97,97)}}W3{{/fg}}{{/b}} {{b}}{{fg rgb(25,118,210)}}BX{{/fg}}{{/b}} {{b}}{{fg rgb(25,118,210)}}B6{{/fg}}{{/b}} {{b}}{{fg rgb(255,160,0)}}Y7{{/fg}}{{/b}} {{b}}{{fg rgb(255,160,0)}}Y8{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell center}}{{fg rgb(97,97,97)}}Scores{{/fg}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}R1{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}R2{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}R3{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}Tot{{/fg}}{{/cell}}{{/row}}{{row}}{{cell right}}{{player 0}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}33{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}33{{/cell}}{{/row}}{{row}}{{cell right}}{{player 1}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-1{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-1{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
```

**3-player layout:** Both opponents are shown side by side above the discard row, then your expeditions below.

```brdgme
{{table}}{{row}}{{cell center}}Round {{b}}2{{/b}} of {{b}}3{{/b}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell left}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G10{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{row}}{{cell right}}{{player 1}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R10{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G2{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}Y4{{/fg}}{{/b}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{cell left}}       {{/cell}}{{cell left}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(56,142,60)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W5{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{row}}{{cell right}}{{player 2}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R8{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}GX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}WX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(25,118,210)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}--{{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{/row}}{{row}}{{/row}}{{row}}{{cell right}}{{fg rgb(97,97,97)}}Discard{{/fg}}{{/cell}}{{cell left}}   {{/cell}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}R6{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}GX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W2{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}B6{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(255,160,0)}}Y9{{/fg}}{{/b}}{{/cell}}{{cell left}}{{fg rgb(97,97,97)}}   {{b}}21{{/b}} left{{/fg}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{/row}}{{row}}{{cell right}}{{player 0}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}--{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(56,142,60)}}G6{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(97,97,97)}}W7{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}BX{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}--{{/fg}}{{/cell}}{{/row}}{{row}}{{cell left}}{{/cell}}{{cell left}}   {{/cell}}{{cell left}}{{fg rgb(211,47,47)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(56,142,60)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(97,97,97)}}  {{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{b}}{{fg rgb(25,118,210)}}B9{{/fg}}{{/b}}{{/cell}}{{cell left}}  {{/cell}}{{cell left}}{{fg rgb(255,160,0)}}  {{/fg}}{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell center}}{{fg rgb(97,97,97)}}Your hand{{/fg}}{{/cell}}{{/row}}{{row}}{{cell center}}{{b}}{{fg rgb(211,47,47)}}RX{{/fg}}{{/b}} {{b}}{{fg rgb(56,142,60)}}G7{{/fg}}{{/b}} {{b}}{{fg rgb(97,97,97)}}WX{{/fg}}{{/b}} {{b}}{{fg rgb(97,97,97)}}W6{{/fg}}{{/b}} {{b}}{{fg rgb(255,160,0)}}YX{{/fg}}{{/b}} {{b}}{{fg rgb(255,160,0)}}Y7{{/fg}}{{/b}} {{b}}{{fg rgb(255,160,0)}}Y8{{/fg}}{{/b}}{{/cell}}{{/row}}{{row}}{{/row}}{{row}}{{cell center}}{{fg rgb(97,97,97)}}Scores{{/fg}}{{/cell}}{{/row}}{{row}}{{cell center}}{{table}}{{row}}{{cell left}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}R1{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}R2{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}R3{{/fg}}{{/cell}}{{cell left}}  {{/cell}}{{cell center}}{{fg rgb(97,97,97)}}Tot{{/fg}}{{/cell}}{{/row}}{{row}}{{cell right}}{{player 0}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-52{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-52{{/cell}}{{/row}}{{row}}{{cell right}}{{player 1}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-55{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-55{{/cell}}{{/row}}{{row}}{{cell right}}{{player 2}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-32{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}{{/cell}}{{cell left}}{{/cell}}{{cell center}}-32{{/cell}}{{/row}}{{/table}}{{/cell}}{{/row}}{{/table}}
```

- **Top section** - opponent expedition columns (in a 3-player game, both opponents are shown side by side, each in their own sub-table). Each column's earliest-played card sits at the bottom next to the player's name, closest to the discard row, with later cards extending upward and the most recent card at the top; `--` means expedition started but no cards yet, colored blank means not started
- **Discard row** - the top (takeable) card on each shared pile; `--` means empty. Deck count on the right
- **Bottom section** - your expeditions, built the mirror image of the top section: the earliest-played card sits at the top next to your name, closest to the discard row, with later cards extending downward
- **Your hand** - all cards sorted by expedition then value
- **Scores** - cumulative per round (R1, R2, R3) and running total; blank = not yet scored

The expedition columns are ordered **R G W B Y** left to right throughout the display.

## Strategy

- Avoid starting an expedition unless you can realistically turn a profit - the cost is punishing with few cards
- Wager cards are high-risk: they multiply both gains and losses. Only commit wagers when you hold several cards in that color and the round has plenty of turns remaining
