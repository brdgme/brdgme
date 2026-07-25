# Verification: game/seven-wonders-1 (games-batch-b, F1-F15)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5.
All paths relative to snapshot root unless absolute.

## F1 - Halicarnassus B wonder-stage VP never scored (major)

Verdict: CONFIRMED. Severity major: correct.

- `player_vp` match, game/seven-wonders-1/src/lib.rs:706-722: arms are
  `CardEffect::VP`, `CardEffect::Bonus { .. } if *bonus_vp > 0`,
  `CardEffect::MimicGuild`, then `_ => {}`. No `DrawDiscard` arm, so its
  `vp` payload is dropped.
- card.rs:1269 `CardEffect::DrawDiscard { vp: 2 }` (Halicarnassus B Stage 1),
  card.rs:1277 `{ vp: 1 }` (Stage 2), card.rs:1285 `{ vp: 0 }` (Stage 3).
  Halicarnassus A Stage 2 (card.rs:1252) is `{ vp: 0 }` so A side is
  unaffected in score.
- Line refs exact. A Halicarnassus B player building stages 1+2 loses 3 VP.
- Rules premise (official 2/1/0) is external (RULES.md is silent on
  per-stage VP), but the code encodes vp values it then never scores -
  internally inconsistent regardless of official rules.

## F2 - Reachable permanent soft-lock in DrawDiscard resolver (major)

Verdict: CONFIRMED. Severity major: correct.

- Queue condition, lib.rs:410: `CardEffect::DrawDiscard { .. } if
  !self.discard.is_empty()` - no ownership filter.
- `take_from_discard`, lib.rs:908-940: lib.rs:921-923 rejects with
  "already own this card" if `self.cards[player]` contains a card of the
  same name.
- Parser while resolver pending, command.rs:21-37: only the `take` parser is
  returned for the resolving player; all other players get `None`
  (command.rs:22-24). No pass/skip command exists.
- `status()`, lib.rs:1152-1156: while `to_resolve` is non-empty it returns
  `whose_turn: vec![*player]` unconditionally.
- `command()` (lib.rs:997-1004) routes everything through the parser, so the
  only accepted verb is `take N`, and every `take N` errors when all discard
  cards are owned. `to_resolve` is never popped, `end_hand` never runs
  (lib.rs:257-260 and 934-937 both gate on `to_resolve.is_empty()`).
  Permanent soft-lock confirmed.
- PORTING_NOTES.md "Preserved quirks" bullet: "DrawDiscard resolver only
  fires if there are takeable cards in discard (cards the player doesn't
  already own)." - present verbatim, and the code has no such filter. The
  doc-vs-code contradiction claim is accurate.

## F3 - Auto-discarded last card of each age pays 3 coins (major)

Verdict: CONFIRMED. Severity: defensible as major; minor also arguable.

- lib.rs:192-195: `let card = self.hands[p].pop().unwrap();
  self.discard.push(card); self.coins[p] += DISCARD_COINS;` inside the
  `max_hand == 1` branch of `end_hand`, for every player without
  PlayFinalCard. Log text (lib.rs:196-199) says only "discarded their last
  card" - the payment is not surfaced.
- RULES.md line 26 says the last card is "automatically discarded" with no
  mention of coins; PORTING_NOTES.md "Preserved quirks" does not list it.
  Not documented as a deviation.
- Rules premise (official: no coins for the unplayed 7th card) is external;
  the code trace is accurate.
- Severity note: the inflation is symmetric (+3 coins/age to every player
  except PlayFinalCard holders, who instead get a full extra action or can
  discard for the same 3 coins), so placings distortion is smaller than the
  finding implies (uniform +~3 VP mostly cancels; residual effects are trade
  liquidity and the Babylon B relative comparison). Still a straight rules
  deviation in a core economy path; major is within charter given
  Correctness ranks first, but minor would also be defensible. No change
  recommended.

## F4 - Same-turn trade of freshly built resources (minor)

Verdict: CONFIRMED. Severity minor: correct.

- `execute_actions`, lib.rs:265-297, iterates `p` in index order and calls
  `execute_build` which immediately mutates `self.cards[player]`
  (lib.rs:326, 351) and coins.
- Later players' `execute_build` -> `resolve_deal` (lib.rs:320, 347) ->
  `can_afford_cost` -> `player_goods_options(left/right)` (lib.rs:470-474,
  579-591) reads the already-mutated `self.cards`, so p+1 can buy from a
  resource card p built this same turn. Asymmetric as described.
- Rules premise (FAQ: only pre-existing resources tradable) is external;
  code trace accurate. Not documented in PORTING_NOTES.md/RULES.md.

## F5 - MimicGuild only copies Bonus-effect guilds (minor)

Verdict: CONFIRMED. Severity minor: correct.

- `mimic_guild_vp`, lib.rs:727-754: after the `kind != CardKind::Guild`
  filter, only `if let CardEffect::Bonus { .. }` is evaluated; any other
  guild effect is skipped.
- Scientists Guild, card.rs:913-921: `CardEffect::Science { fields:
  all_fields() }` - a Guild-kind card with a non-Bonus effect, so it is a
  real skipped case (its marginal science VP via the neighbor copy is never
  considered).
- Rules premise (Olympia B stage 3 may copy any neighbor guild) is external;
  code trace accurate.

## F6 - Wonder-sacrifice card enters the shared discard pile (minor)

Verdict: CONFIRMED. Severity minor: correct.

- lib.rs:324-325: `let hand_card = self.hands[player].remove(card_idx);
  self.discard.push(hand_card);` in the wonder branch of `execute_build`.
- The card is thereafter retrievable by `take_from_discard` and counted in
  `discard_count` (lib.rs:960).
- Stronger than the finding states: the crate's own RULES.md (line 20-21)
  says "wonder N - Discard card N face-down under your wonder", so the code
  contradicts in-repo documentation, not just the external rule. Not listed
  in PORTING_NOTES.md quirks.

## F7 - Both sides of the same wonder can coexist (minor)

Verdict: CONFIRMED. Severity minor: correct.

- `start_game`, lib.rs:115-117: `let mut all_cities = cities();
  all_cities.shuffle(&mut rng); let assigned_cities =
  all_cities[..players].to_vec();`.
- `cities()` (card.rs:1350 onward) contains all 14 entries: Rhodes A/B,
  Alexandria A/B, Ephesus A/B, Babylon A/B, Olympia A/B, Halicarnassus A/B,
  Giza A/B (verified by name listing). Nothing dedupes by board, so e.g.
  Rhodes A and Rhodes B can both be dealt.
- Rules premise (7 physical boards, one side each) is external; code trace
  accurate.

## F8 - Discard pile contents hidden from all players (minor/quality)

Verdict: CONFIRMED. Severity minor: correct.

- `PubState`, lib.rs:66-91: only `discard_count: usize` (lib.rs:74);
  `PlayerState` (lib.rs:93-101) adds only the player's own hand. The
  Halicarnassus resolver player must `take N` by index (command.rs:25-35)
  with no visibility of what N is. Recoverable only from logs.

## F9 - Deal re-validated by index into a recomputed list (nit)

Verdict: ADJUSTED. Recommend severity upgrade nit -> minor (correctness).

- Code as described: `resolve_deal`, lib.rs:418-431, recomputes
  `can_afford_cost` at execution time and does
  `deals.get(idx).cloned().unwrap_or_default()` - out-of-range idx yields an
  empty deal, i.e. neighbors are paid nothing (resources effectively free).
- What is wrong in the original: the claim "verified unreachable today (the
  deal list is append-only between choice and execution)" is false. Between
  `choose_deal` and the player's own slot in `execute_actions`, earlier-
  indexed players' builds mutate neighbors' `cards` and everyone's `coins`
  (F4's mechanism). `can_afford_cost` -> `can_afford_perm`
  (lib/cost/src/lib.rs:165-203) has an early-return at lines 181-184: if any
  single option group entry fully affords the remaining cost, it returns one
  allocation and discards all sibling/deeper branches. A resource card built
  mid-execution by a neighbor adds such an option, so the recomputed deal
  list can be reordered or shrunk, not merely appended. Consequences:
  - stored idx can silently select a different deal than the player chose
    (wrong neighbor paid), or
  - idx can go out of range, hitting `unwrap_or_default()` and building
    with no trade payment at all.
- Both are correctness effects reachable from normal play (player p>0
  chooses `deal 2`, left neighbor at a lower index builds a resource card
  the same turn). Hence minor (correctness), not nit (quality), and the
  recommendation (store the chosen HashMap in the Action) is the right fix.

## F10 - Unguarded player indexing in player_state/command_parser (nit)

Verdict: CONFIRMED. Severity nit: correct.

- lib.rs:984: `hand: self.hands[player].clone()` - no bounds check.
- command.rs:39 `&self.actions[player]`, command.rs:54
  `self.actions[player].is_some() || self.hands[player].is_empty()` - no
  bounds checks.
- Sibling contrast verified: category-5-2/src/lib.rs:413 uses
  `self.hands.get(player).cloned().unwrap_or_default()`;
  sushi-go-2/src/lib.rs:805-807 uses an explicit `player < self.hands.len()`
  guard. Only reachable via framework-supplied out-of-range player.

## F11 - Finished-game scoring block copy-pasted six times (nit)

Verdict: CONFIRMED. Severity nit: correct.

- `command()`, lib.rs:1005-1137: six Ok arms (Build lib.rs:1011, Free 1033,
  Wonder 1055, Discard 1077, Deal 1099, Take 1121) each contain the
  identical `if self.is_finished() { scores / gen_placings / placings_log }`
  + `CommandResponse` epilogue (~15 lines x 6, ~90 lines total).

## F12 - Military-conflict log uses raw player index (nit)

Verdict: CONFIRMED. Severity nit: correct.

- lib.rs:770-776: `N::Player(p)` followed by `N::text(format!(" defeated
  player {} in military conflict (+{} victory, +1 defeat)", right, tokens))`
  - the defeated player is a raw 0-based index in plain text (also off by
  one relative to 1-based user-facing numbering), unlike every other player
  reference which uses `N::Player`.

## F13 - start_hand() is dead-weight (nit)

Verdict: CONFIRMED. Severity nit: correct.

- lib.rs:177-180: `fn start_hand(&mut self) -> Vec<Log> { self.actions =
  vec![None; self.players]; vec![] }`.
- All call sites (end_hand lib.rs:208 and 214) are reached only after
  `execute_actions` already did `self.actions = vec![None; self.players]`
  (lib.rs:295), including the resolver path (take_from_discard -> end_hand,
  lib.rs:934-937, where execute_actions ran earlier in the same hand).
  `start_round` (lib.rs:163) also resets. Redundant reset, empty logs.

## F14 - Test coverage gaps (minor)

Verdict: ADJUSTED (one listed gap is wrong; the rest confirmed).
Severity minor: correct.

Test inventory (lib.rs:1206-1565 plus tests/contract.rs, which only asserts
the generic gamer contract):

- WRONG in the finding: MimicGuild IS tested - `test_card_mimic_guild`
  (lib.rs:1503-1511) gives MICK "Olympia B Wonder Stage 3"
  (CardEffect::MimicGuild, card.rs:1235) with Builders/Workers Guild on
  neighbors and asserts `player_vp(MICK) == 2`. This also exercises
  `mimic_guild_vp` -> `bonus_count` guild counting.
- Confirmed absent (grepped the test module):
  - military conflict resolution / token values (no test touches
    `military_conflicts`, `victory_tokens`, or `attack_strength`)
  - pass direction per age (`pass_hands` never asserted; `test_free_build`
    crosses into age 2 but checks nothing about hand rotation)
  - direct `Bonus` scoring through `player_vp` for a player's own guild /
    commercial bonus cards (Haven appears only in affordability tests)
  - Halicarnassus B DrawDiscard vp (would have caught F1; all take tests
    use Halicarnassus A Stage 2, vp: 0)
  - `deal N` multi-deal selection (no test issues a `deal` command)
  - seed determinism (fixed seed 42 used, but no same-seed/replay assertion)
  - full end-to-end game (test_free_build plays out age 1 by discards only;
    no game reaches finished state)

## F15 - lib.rs is a 1,565-line grab-bag (nit)

Verdict: CONFIRMED. Severity nit: correct.

- `wc -l`: lib.rs is exactly 1565 lines; tests module spans lib.rs:1206-1565
  (360 lines). State machine, trading (`can_afford_cost`, `resolve_deal`,
  `pay_cost`), scoring (`science_vp`, `score_science`, `player_vp`,
  `mimic_guild_vp`), resolver queue, and the `Gamer` impl all live in the
  one file. Optional-split framing is appropriate.

## Summary of severity assessments

- F1 major: keep.
- F2 major: keep.
- F3 major: keep (defensible; symmetric-inflation caveat noted, minor also
  arguable).
- F4 minor: keep.
- F5 minor: keep.
- F6 minor: keep (strengthened: contradicts in-repo RULES.md).
- F7 minor: keep.
- F8 minor: keep.
- F9 nit: UPGRADE to minor (correctness) - unreachability claim is false;
  wrong-deal remap and free-build via `unwrap_or_default` are reachable in
  normal play.
- F10-F13 nit: keep.
- F14 minor: keep, with MimicGuild removed from the gap list.
- F15 nit: keep.
