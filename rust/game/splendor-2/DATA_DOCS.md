# Splendor Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in the game, 2 through 4.
- `board` (Vec<Vec<Card>>): Face-up development cards available to buy or reserve, indexed by level (0 = level 1, 1 = level 2, 2 = level 3). Each level holds up to 4 cards; when a card is bought or reserved it is refilled from that level's deck, or the slot disappears once the deck is empty (so column positions can shift).
- `nobles` (Vec<Noble>): Nobles currently available to visit, each worth 3 prestige. A noble's cost is paid entirely in permanent card bonuses, never tokens.
- `tokens` (Cost): The bank's remaining supply of each gem (Diamond, Sapphire, Emerald, Ruby, Onyx) and Gold.
- `player_boards` (Vec<PubPlayer>): Public info for each player, indexed by player number (0-based). See PubPlayer below.
- `current_player` (usize): Index (0-based) of the player whose turn it is.
- `phase` (Phase): Current turn phase: `Main`, `Visit`, or `Discard`.
- `finished` (bool): True once the game has ended.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0-based) this private state belongs to.
- `reserve` (Vec<Card>): This player's own reserved cards, up to 3. The only hidden information in the game - other players see only the reserve count, never these cards.

## PubPlayer

- `bonuses` (Cost): Permanent gem bonuses from development cards this player owns, one per owned card of that gem. These discount card costs and are what attract nobles.
- `tokens` (Cost): Gem and Gold tokens this player is currently holding.
- `nobles` (Vec<Noble>): Nobles that have visited this player, each worth 3 prestige.
- `card_count` (usize): Number of development cards this player has bought.
- `reserve_count` (usize): Number of cards this player has reserved. The reserved cards themselves are hidden.
- `prestige` (i32): This player's current prestige score.

## Card

- `resource` (Resource): The permanent gem bonus this card grants once bought (Diamond, Sapphire, Emerald, Ruby, or Onyx).
- `prestige` (i32): Prestige points this card is worth, 0 for most level 1 cards, higher for level 2 and 3.
- `cost` (Cost): The gem cost to buy this card, before any permanent bonuses are applied.

## Noble

- `prestige` (i32): Prestige this noble is worth, always 3.
- `cost` (Cost): The permanent-bonus cost required to attract this noble. Paid in card bonuses only, never tokens - either 3 of three gem types or 4 of two gem types.

## Cost

A map from `Resource` to count (`HashMap<Resource, i32>`). Used for the bank's token supply, a player's held tokens, a player's permanent bonuses, and card/noble costs. Absent resources count as 0.

## Resource enum

- `Diamond`, `Sapphire`, `Emerald`, `Ruby`, `Onyx`: The five gem colours. Used as tokens, as card bonuses, and as costs.
- `Gold`: Wildcard token. Substitutes for any single gem when paying a card cost. Cannot be taken with the `take` action; gained by reserving.
- `Prestige`: Not a token - only used as a label for prestige values in rendering.

## Phase enum

- `Main`: The active player takes exactly one action - take tokens, buy a card, or reserve a card.
- `Visit`: Noble visit phase. Reached automatically after the Main action. Skipped if no noble is affordable, resolved automatically if exactly one is, and requires a `visit` choice if two or more are.
- `Discard`: Reached only if the active player holds more than 10 tokens. They must discard down to 10 before the turn passes.
