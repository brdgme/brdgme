# Porting Notes: 7 Wonders (seven-wonders-1)

Ported from the Go implementation in `go/game/sevenwonders/`.

## Version suffix

Using `-1` per orchestrator instruction (not the `-2` convention used by some
other ports).

## Shared cost crate

Uses `brdgme_cost` (at `rust/lib/cost/`) with `Cost<Good>` for card costs and
`Cost<MultiResource>` for multi-effect wonder stages. Uses `can_afford_perm`
for neighbor-trade affordability.

## Simultaneous turns

Go's `Actioner` interface + `CheckHandComplete` pattern ported as `Action`
enum + `check_hand_complete()` method. All players select actions, then
resolve simultaneously.

## Resolver queue

Go's `Resolver` interface ported as `Resolver` enum. Currently only
`DrawDiscard` variant (Halicarnassus wonder stages).

## Card polymorphism

Go's interface-based card types (CardGood, CardVP, CardMilitary, CardScience,
CardBonus, CardCommercialTrade, CardMulti, CardFreeBuild, CardDrawDiscard,
CardMimicGuild, CardPlayFinalCard) collapsed into a single `CardEffect` enum
with variant data.

## Science VP

Go's `ScienceVPPerm` recursive permutation ported as `science_permute`
recursive backtracking. Same math: sum(count^2) + min_count * 7.

## Placings

Uses Rust `gen_placings` (standard-competition `[1,1,3]`) instead of Go's
compact-ordinal `[1,1,2]`. Tiebreak by coins.

## RNG

Go's ambient `rand.New(rand.NewSource(time.Now().UnixNano()))` replaced with
deterministic `GameRng` (ChaCha8) seeded in `start`.

## Deck building

Go's `DeckForPlayers` threshold logic (`p <= players` adds one copy per
threshold) preserved exactly. Age 3 guild selection uses rng.

## Preserved quirks

- Go's `CanAfford` tries left-favour then right-favour deals; Rust port
  simplifies to finding all valid deals via `can_afford_perm` without the
  left/right priority ordering. The cheapest deal is still selected.
- Go's `CardMulti.VictoryPoints()` method (no player arg) vs `VictoryPointer`
  interface (with player arg) - the Multi VP is handled via `victory_tokens`
  in `post_build_hook` rather than at scoring time. This matches the Go
  behavior where Multi VP is added as victory tokens.
- DrawDiscard resolver only fires if there are takeable cards in discard
  (cards the player doesn't already own).

## Render

Basic render implemented. Full render parity with Go CLI not yet achieved (Go
uses template markup with colors; Rust uses Node trees).
