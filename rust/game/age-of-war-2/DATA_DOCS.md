# Age of War Data Dictionary

## PubState (public information)

- `players` (usize): Number of players in this game (2 to 6).
- `current_player` (usize): Index of the player whose turn it is.
- `conquered` (Vec<bool>): Whether each castle has been conquered, indexed by castle position (0-13). Castles are ordered by clan: Oda (0-3), Tokugawa (4-6), Uesugi (7-8), Mori (9-10), Chosokabe (11-12), Shimazu (13).
- `castle_owners` (Vec<Option<usize>>): Owner of each conquered castle (player index), or None if unconquered. Indexed the same as `conquered`.
- `currently_attacking` (Option<usize>): Index of the castle currently being attacked, or None if no attack has been declared this turn.
- `completed_lines` (Vec<usize>): Line indices (0-based) completed on the currently-attacked castle this turn. Empty when no attack is active.
- `current_roll` (Vec<Die>): Dice currently in the player's roll pool. Each die shows one of: Inf1, Inf2, Inf3, Archery, Cavalry, Daimyo.
- `scores` (Vec<u32>): Current scores for each player, recalculated from castle and clan ownership. If a player owns all castles in a clan, they score the clan bonus instead of individual castle points.

## PlayerState (player-private information)

- `public` (PubState): The full public game state, as described above.
- `player` (usize): Which player this state belongs to.

## Die enum

- `Inf1`: Infantry value 1.
- `Inf2`: Infantry value 2.
- `Inf3`: Infantry value 3.
- `Archery`: Archery symbol.
- `Cavalry`: Cavalry symbol.
- `Daimyo`: Daimyo symbol.

## Castle structure

Each castle belongs to a clan and has 1-4 lines. A line requires either specific symbols (Archery, Cavalry, Daimyo) or a fixed infantry total. Stealing a conquered castle adds an extra Daimyo line.

## Clans

| Clan | Castles | Clan bonus |
|------|---------|------------|
| Oda | Azuchi (3), Matsumoto (2), Odani (1), Gifu (1) | 10 |
| Tokugawa | Edo (3), Kiyosu (2), Inuyama (1) | 8 |
| Uesugi | Kasugayama (4), Kitanosho (3) | 8 |
| Mori | Gassantoda (2), Takahashi (2) | 5 |
| Chosokabe | Matsuyama (2), Marugame (1) | 4 |
| Shimazu | Kumamoto (3) | 3 |
