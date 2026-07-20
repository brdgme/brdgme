# Seven Wonders Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in the game (3-7).
- `round` (u8): Current age (1, 2, or 3). The game has 3 ages, each with a card draft.
- `finished` (bool): True when all 3 ages are complete and the game is over.
- `discard_count` (usize): Number of cards in the shared discard pile. Cards are discarded for 3 coins or taken via DrawDiscard effects.
- `cards` (Vec<Vec<Card>>): Cards each player has built, indexed by player. Includes all card types (resources, military, science, civic, commercial, guilds, wonder stages).
- `coins` (Vec<i32>): Coins each player holds, indexed by player. Coins score 1 VP per 3 at game end.
- `victory_tokens` (Vec<i32>): Victory tokens per player, from military wins and VP-granting cards.
- `defeat_tokens` (Vec<i32>): Defeat tokens per player, from military losses. Each is -1 VP at game end.
- `cities` (Vec<City>): The wonder city assigned to each player. Determines starting resource and available wonder stages.
- `hand_sizes` (Vec<usize>): Number of cards in each player's current hand, indexed by player.
- `actions_chosen` (Vec<bool>): Whether each player has chosen their action for this hand, indexed by player. True means they are waiting for others.
- `to_resolve_player` (Option<usize>): If set, this player must resolve a DrawDiscard effect (take a card from the discard pile) before play continues.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player (0-indexed) this private state belongs to.
- `hand` (Vec<Card>): Cards in this player's current hand. Each card has a name, kind, cost, effects, and optional free-build prerequisites.

## Card

- `name` (String): Unique card name.
- `kind` (CardKind): One of Resource, Military, Science, Civic, Commercial, Guild, Wonder.
- `cost` (Cost<Good>): Resources and/or coins required to build.
- `free_with` (Vec<String>): Cards that allow free building of this card if already built.
- `effect` (CardEffect): The card's effect (goods production, military strength, science fields, VP, coins, etc.).

## City

- `name` (String): Wonder name (e.g., "Rhodes A", "Babylon B").
- `initial_resource` (Good): The starting resource this city produces.
- `wonder_stages` (Vec<String>): Names of the wonder stage cards, in build order.
