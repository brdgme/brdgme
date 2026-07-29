# Hanamikoji - Porting Notes

Notes on the from-scratch rules interpretation for `hanamikoji-1`. This is a new
implementation of Hanamikoji (EmperorS4), written from the rules rather than
ported from existing code. Where the prose in `RULES.md` and the code could
disagree, the code is authoritative.

## Decisions

- **D1 - automatic draw.** Drawing at the start of your turn is mandatory in the
  rules, so it happens automatically on entering a player's turn
  (`Game::begin_turn` in `src/lib.rs`). There is no `draw` command; a turn is
  just one action.
- **D2 - multiset hand.** The three item cards of a geisha are indistinguishable,
  so a hand is a multiset of geisha types and commands reference cards by geisha
  name (`secret tea`, `gift fan fan shamisen`), not by unique card ids.
- **D3 - secret/trade-off redaction.** A player's secret and trade-off cards are
  visible to their owner and hidden from the opponent. `PubState` carries only
  counts and presence flags (`hand_counts`, `has_secret`, `traded_counts`); the
  identities live only in `PlayerState`.
- **D4 - public pending cards.** The cards offered by a gift or competition are
  face-up, so `PubState.pending` exposes them in full (the three gift cards, or
  the two competition pairs).
- **D5 - automatic scoring and update.** Scoring and the round reset are
  automatic transitions with no player-facing phase or command (`score_round`).
  Only a win stops the game.
- **D6 - ASCII geisha names.** The real game uses Japanese art names and colours.
  The names here (Flute, Koto, Fan, Shamisen, Umbrella, Taiko, Tea) are a
  cosmetic ASCII-friendly convention. Only the charm multiset {2,2,2,3,3,4,5}
  and three-copies-per-type are mechanically meaningful; the name-to-value
  mapping is decorative.
- **D7 - charm beats geisha.** If one player reaches four geisha and the other
  reaches eleven charm in the same scoring, the eleven-charm player wins
  (`Game::decide_winner`). Only one player can hold eleven charm (charm sums to
  21), so the tiebreak is unambiguous.

## Command syntax

- `secret <geisha>`, `trade <geisha> <geisha>`, `gift <geisha> <geisha> <geisha>`.
- `compete <a> <b> <c> <d>` is two positional pairs: (a,b) is set 1 and (c,d) is
  set 2. There is no separator token between the pairs.
- The opponent resolves a gift with `choose <geisha>` (one of the offered cards)
  and a competition with `choose 1` or `choose 2` (which set to take).
- Geisha names parse case-insensitively.

## Deployment status

This crate mirrors `lords-of-vegas-1`: it is complete and tested but not
deployed. Registration is a single line in the `rust/Cargo.toml` workspace
members list (alphabetical, between `game/greed-2` and `game/jaipur-2`). There
is no Dockerfile, docker-bake entry, Tiltfile target or k8s manifest, and no
GameVersion CRD. The interface version is orthogonal to the `-1` crate suffix;
if it were ever deployed it would declare `interfaceVersion: 2`.
