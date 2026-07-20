# Splendor

A 2-4 player engine-building game about collecting gems to buy development
cards. Bought cards give permanent gem bonuses that make future cards cheaper,
and some cards are worth prestige points. Attract wealthy nobles for bonus
prestige. First to 15 prestige (after the round finishes) wins.

## Components

**Development cards** - 90 cards across 3 levels (level 1 easiest/cheapest,
level 3 hardest/most valuable): 40 level 1, 30 level 2, 20 level 3. Each card
shows a gem resource (the permanent bonus it grants once bought) and a cost in
gems, e.g. a card might cost 2 Ruby + 1 Onyx and grant a permanent Emerald
bonus. Higher level cards are worth more prestige (0-1 at level 1, 1-3 at
level 2, 3-5 at level 3) - most level 1 cards are worth 0.

**Nobles** - 10 nobles available across all games, each worth 3 prestige. A
noble's cost is paid entirely in permanent card bonuses (never tokens) - 3 of
one gem type each at 3 gems, or 2 gem types at 4 each. Only `players + 1`
nobles are used each game (2p -> 3, 3p -> 4, 4p -> 5), drawn at random.

**Tokens** - 5 gem colours (Diamond, Sapphire, Emerald, Ruby, Onyx) plus Gold
(wildcard, substitutes for any single gem when paying). Gold is always 5
regardless of player count; gem supply depends on player count:

| Players | Each gem | Gold |
|---------|----------|------|
| 2 | 4 | 5 |
| 3 | 5 | 5 |
| 4 | 7 | 5 |

**Board** - 4 face-up cards per level, drawn from a shuffled deck of the
remaining cards in that level. When a face-up card is bought or reserved it is
immediately replaced from that level's deck (or the slot disappears once the
deck is empty, so the remaining cards in that level shift position).

## Turn Structure

Each turn has a Main phase, then an automatic Visit phase, then (if needed) a
Discard phase, before play passes to the next player.

### 1. Main phase

Take exactly one action:

- **Take tokens** - either 2 of the same gem (only if the bank has 4 or more
  of that gem left) or 3 different gems (only gems the bank still has at
  least 1 of). Gold cannot be taken this way.
  `take Ruby Ruby` (two of the same), `take Ruby Sapphire Diamond` (three
  different).
- **Buy a card** - from the board or from your own reserve, paying its cost
  in gem tokens, offset by your permanent bonuses from cards you already own.
  If a gem's cost still isn't covered by tokens, Gold substitutes
  automatically. `buy A1` (board, column A level 1), `buy C4` (your own
  reserved card in slot C - reserve slots always use row 4).
- **Reserve a card** - from the board only (not from another player's
  reserve), taking 1 Gold if the bank has any left. Maximum 3 reserved cards
  at once. `reserve B2`.

### 2. Visit phase (automatic)

After your Main action, the game checks which nobles you can now afford using
only your permanent card bonuses (tokens don't count towards a noble's cost):

- **0 affordable** - phase is skipped automatically.
- **Exactly 1 affordable** - you are visited automatically, no command
  needed.
- **2 or more affordable** - you must choose: `visit 2` (the noble numbered 2
  in the Nobles row). Note: the game does not re-check affordability at this
  point - you may pick any noble by number, even one you can no longer
  afford, and it will be granted anyway.

### 3. Discard phase (only if needed)

If you're holding more than 10 tokens total (gems + Gold) after your action,
you must discard down to 10 before your turn ends: `discard Onyx`,
`discard Onyx Gold` (discard multiple in one command). You can discard Gold.
If you're still over 10 after a partial discard, you remain in this phase and
must discard again.

Once at or below 10 tokens, play passes to the next player.

## Scoring

Prestige = sum of prestige on every development card you own + 3 for every
noble that has visited you. Tokens have no prestige value.

Worked example for one player:

| Source | Count | Prestige each | Subtotal |
|--------|-------|---------------|----------|
| Level 1 cards owned | 4 | 0 | 0 |
| Level 2 card (Emerald, cost Sapphire 5) | 1 | 2 | 2 |
| Level 3 card (Onyx, cost Ruby 7) | 1 | 4 | 4 |
| Nobles visited | 1 | 3 | 3 |
| **Total prestige** | | | **9** |

## Game End

The instant any player's prestige reaches 15 or more (checked after every
action), the end is triggered - but the game doesn't stop immediately. Every
other player still gets to finish out the current round, so the game only
actually ends the moment play would wrap back around to player 0. This means
all players get an equal number of turns in the final round, even if the
triggering player wasn't player 0.

## Winning

Whoever has the most prestige wins. If tied, the tiebreaker is the number of
development cards owned - **more cards wins the tie**, not fewer.

## Commands

| Command | Action | Example |
|---------|--------|---------|
| `take <token> <token>` | Take 2 of the same gem (bank needs 4+) | `take Ruby Ruby` |
| `take <token> <token> <token>` | Take 3 different gems | `take Ruby Sapphire Diamond` |
| `buy <loc>` | Buy a card from the board or your own reserve | `buy A1`, `buy A4` |
| `reserve <loc>` | Reserve a board card and take 1 Gold if available | `reserve B2` |
| `discard <token>...` | Discard one or more tokens (Gold allowed) down to 10 | `discard Onyx Gold` |
| `visit <number>` | Choose which affordable noble visits you (only when 2+ are affordable) | `visit 2` |
