# Lords of Vegas Data Dictionary

## PubState (public information)

- `players` (Vec<Player>): Per-player public info, indexed by player number. Each entry holds that player's `cash` and `points`.
- `current_player` (usize): Index of the player whose turn it is.
- `remaining_deck` (usize): Number of cards left in the draw deck. The hidden GameEnd card is included in this count but never revealed.
- `played` (Vec<Card>): Location cards that have been dealt so far. Each is a `Loc` card naming the lot it granted. Use this to work out which lots are still undealt and how many cards remain for each casino.
- `board` (Board): The state of every lot on the strip, keyed by location. See Board below.
- `finished` (bool): True when the game is over.

## PlayerState (player-private information)

- `player` (usize): Which player this private state belongs to.
- `state` (Option<Player>): This player's own cash and points. Present when the player is in the game.
- `pub_state` (PubState): The full public game state, as described above.

## Player

- `cash` (usize): Cash currently on hand, used to pay build costs.
- `points` (usize): Points index into the POINT_STOPS table. Currently always 0 because scoring is not yet implemented.

## Board

A map from `Loc` (a lot such as `C8`) to the `BoardTile` on that lot. Lots that have never been touched default to `Unowned`.

## BoardTile enum

- `Unowned`: The lot has no owner and nothing built on it.
- `Owned { player }`: The lot is owned by `player` but has not been built on yet. The owner may build a casino here.
- `Built { owner, casino, height }`: A casino has been built on the lot.
  - `owner` (Option<TileOwner>): The tile's owner marker (player and die value), if any.
  - `casino` (Casino): Which casino colour was built here.
  - `height` (usize): The building height. Starts at 1; raising is not yet implemented.

## TileOwner

- `player` (usize): The player who owns this built tile.
- `die` (usize): The die value shown on this tile (1-6). The highest die in a casino is the boss; ties are rerolled.

## Loc

- `block` (Block): The block the lot sits in, A through F.
- `lot` (usize): The lot number within the block. Blocks are 3 lots wide; A/B/E have lots 1-6, C has 1-12, D/F have 1-9.

## Block enum

A, B, C, D, E, F - the six blocks of the strip.

## Card enum

- `Loc { loc }`: A location card granting ownership of the lot `loc`.
- `GameEnd`: The hidden card that ends the game when drawn. It is never exposed in `played`.

## Casino enum

Albion, Sphinx, Vega, Tivoli, Pioneer - the five casino colours. A casino on the board is a contiguous group of built lots of the same colour at the same height.
