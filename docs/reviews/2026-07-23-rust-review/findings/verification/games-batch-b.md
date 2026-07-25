# Verification: games-batch-b (unit 4)

Independent verification of `findings/games-batch-b.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Crates: `game/seven-wonders-1`, `game/alhambra-1`, `game/splendor-2`.
Raw verdict dumps: `raw/games-batch-b-7w.md`, `raw/games-batch-b-alhambra.md`,
`raw/games-batch-b-splendor.md`. Process log: `games-batch-b-LOG.md`.

## Per-finding verdicts

### seven-wonders-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | Halicarnassus B DrawDiscard vp never scored | major | CONFIRMED | player_vp (lib.rs:706-722) has no DrawDiscard arm; card.rs:1269/1277/1285 carry vp 2/1/0; internally inconsistent regardless of official rules |
| F2 | DrawDiscard resolver permanent soft-lock | major | CONFIRMED | No queue-time filter (lib.rs:410); take_from_discard rejects owned cards (lib.rs:921-923); parser offers only `take` (command.rs:21-37); status pins the turn (lib.rs:1152); PORTING_NOTES quirk claim present and false |
| F3 | Auto-discarded 7th card pays 3 coins | major | CONFIRMED | lib.rs:192-195 pays DISCARD_COINS; log hides the payment; undocumented. Caveat: inflation is symmetric across players, so minor also arguable; major kept |
| F4 | Same-turn trade of freshly built resources | minor | CONFIRMED | execute_actions mutates cards in index order; later players' resolve_deal reads the mutated state |
| F5 | MimicGuild only copies Bonus guilds | minor | CONFIRMED | mimic_guild_vp matches only CardEffect::Bonus; Scientists Guild is CardEffect::Science (card.rs:913-921) |
| F6 | Wonder sacrifice enters shared discard | minor | CONFIRMED | lib.rs:324-325; strengthened: contradicts the crate's own RULES.md ("face-down under your wonder") |
| F7 | Both sides of one wonder can coexist | minor | CONFIRMED | start_game takes first N of all 14 shuffled A/B cities (lib.rs:115-117) |
| F8 | Discard contents hidden from all players | minor | CONFIRMED | PubState exposes discard_count only (lib.rs:74) |
| F9 | Deal re-validated by index into recomputed list | nit | ADJUSTED (nit -> minor) | Code as described, but "verified unreachable" is false: earlier-indexed builds mutate neighbor goods mid-execution and can_afford_perm's early return (lib/cost:181-184) can reorder or shrink the deal list — wrong-neighbor payment or free build via unwrap_or_default is reachable in normal play |
| F10 | Unguarded player indexing | nit | CONFIRMED | lib.rs:984, command.rs:39/54; sibling guards verified in category-5-2 and sushi-go-2 |
| F11 | Finished-game epilogue x6 | nit | CONFIRMED | Six identical blocks at lib.rs:1011/1033/1055/1077/1099/1121 |
| F12 | Military log uses raw player index | nit | CONFIRMED | lib.rs:770-776; also off-by-one vs 1-based user numbering |
| F13 | start_hand() dead-weight | nit | CONFIRMED | Redundant reset; execute_actions already resets at lib.rs:295 |
| F14 | Test coverage gaps | minor | ADJUSTED | MimicGuild IS tested (test_card_mimic_guild, lib.rs:1503-1511); all other listed gaps confirmed absent. Minor stands |
| F15 | lib.rs 1,565-line grab-bag | nit | CONFIRMED | Exactly 1565 lines; tests span 1206-1565 |

### alhambra-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F16 | take() mints duplicate cards | critical | CONFIRMED | Pre-check (lib.rs:557-561) has no multiplicity accounting; push at lib.rs:574 is unconditional on position() miss; take arm uses CardParser so `take b1 b1` is reachable; spend's clone-and-verify contrast confirmed |
| F17 | place indices diverge after placement | major | CONFIRMED | Empty sentinels left at lib.rs:694; raw-vec indexing (lib.rs:664-669) vs filtered render numbering (render.rs:353-367); full corruption path traced incl. reserve pollution; FinalPlace shares the arm |
| F18 | grid_longest_ext_wall premature break | major | ADJUSTED | Break at card.rs:516 is unconditional as claimed, but on the doc's exact grid the result depends on HashMap iteration order (Grid = HashMap<Vect,Tile>, card.rs:307): start T0 -> 1, start T1 -> 2. Wall scoring is undercountable AND nondeterministic — strengthens the finding; major stands |
| F19 | Dirk excluded from final placings | minor | CONFIRMED | All placings/status/points paths loop 0..human_players; Dirk scores via score_type but never places; rules premise external |
| F20 | Reduced 2-player money deck | minor | CONFIRMED | card.rs:621 `n = if players == 2 { 2 } else { 3 }` (72 vs 108); rules premise external |
| F21 | Test coverage misses riskiest logic | minor | ADJUSTED | Inventory matches except log_final_place does smoke-test single-currency final-place distribution; the tie path is the actual gap. Minor stands |
| F22 | Epilogue x6 | nit | CONFIRMED | Six verbatim blocks (lib.rs:839-849 et al) |
| F23 | Invariant-guarded panics | nit | CONFIRMED | All three verified invariant-guarded, incl. the player-supplied currency path |
| F24 | Gap-check range asymmetry | nit | CONFIRMED | Harmlessness proof upheld: boundary ring is empty and connected, so row max.y is always outside-reachable |
| F25 | Debug formatting in messages | nit | CONFIRMED | Both cited sites, plus three more `{:?}` instances (place/swap/remove logs) |
| F26 | tile_counts duplicated | nit | CONFIRMED | Byte-identical bodies render.rs:69-77 vs card.rs:601-609 |
| F27 | Column headers wrap past 26 | nit | CONFIRMED | Also unaddressable via coord parser past 'z', so cosmetic only |
| F28 | Vec-as-queue / HashMap-as-set | nit | CONFIRMED | remove(0) and never-read bool payloads in both walks |

### splendor-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F29 | Prestige tie broken by MOST cards | minor | CONFIRMED | Metric at lib.rs:199-202; gen_placings (lib/game/src/game.rs:154-172) sorts ascending and assigns places via .rev(), so larger metric wins; test locks in inversion; Go-parity verified (game.go, placings.go sort.Reverse) |
| F30 | take() never validates tokens are gems | minor | CONFIRMED | take(0,[Gold,Gold]) passes (bank gold 5 >= 4); gold excluded only by tokens_parser(false) (command.rs:183); parser is the sole Command::Take producer |
| F31 | cost.rs vs lib/cost assessment | minor | CONFIRMED | lib/cost has no get/set (full API listed); gold-joker can_afford has no lib equivalent; serde shapes identical at K=Resource; 4 files touch Cost, cost! macro unaffected. Wording nit: lib's type is generic Cost<K>, not literally Cost(HashMap<Resource,i32>) |
| F32 | reserve parser offers row-3 locations | nit | CONFIRMED | loc_parser adds row-3 (command.rs:95-100); reserve_parser reuses unfiltered (command.rs:129); test comment at lib.rs:1061-1063 factually wrong |
| F33 | Epilogue x5 | nit | CONFIRMED | Identical blocks at lib.rs:635-640, 653-658, 671-676, 689-694, 707-712 |
| F34 | "remaning" typo | nit | CONFIRMED | lib.rs:326; same typo at brdgme-go/splendor_1/take_command.go:77 |
| F35 | .expect() in visit_phase | nit | CONFIRMED | All three failure paths of visit() excluded by the call context (lib.rs:222-235) |

## Summary

- Findings verified: 35
- CONFIRMED: 31, ADJUSTED: 4 (F9, F14, F18, F21), REJECTED: 0,
  UNVERIFIABLE: 0
- Corrected tallies for the unit: 1 critical / 5 major / 13 minor /
  16 nit (original: 1 critical / 5 major / 12 minor / 17 nit; F9
  upgraded nit -> minor)
- Lead spot-checked all four adjustments (F9 via lib/cost:165-203 early
  return + execute_actions mutation order; F14 via test_card_mimic_guild;
  F18 via card.rs:477-531 and Grid = HashMap; F21 via log_final_place)
  and, since W3 confirmed everything, directly re-verified its two
  hardest confirmations (F29 gen_placings sort direction; F31 lib/cost
  API surface).

## Notable corrections

One severity change:

- F9 (seven-wonders-1 deal index): the original's "verified unreachable
  today (the deal list is append-only between choice and execution)" is
  false. Mid-execution builds by earlier-indexed players change neighbor
  goods, and can_afford_perm's early return (lib/cost/src/lib.rs:181-184)
  fires at every recursion level, so the recomputed deal list can be
  reordered or shrunk — the stored index can pay the wrong neighbor or go
  out of range and build with no trade payment via unwrap_or_default().
  Upgraded nit -> minor (correctness).

Three factual refinements, no severity change:

- F14: MimicGuild is actually tested (test_card_mimic_guild,
  lib.rs:1503-1511) and should be removed from the gap list; all other
  listed gaps are real.
- F18: the undercount is real but the doc's concrete example is
  iteration-order dependent (Grid is a HashMap): walking from T0 yields
  1, from T1 yields 2. Wall scoring is therefore also nondeterministic
  across runs — a strengthening, not a weakening.
- F21: final-place distribution has a single-currency smoke test
  (log_final_place); the genuinely missing coverage is the tie path.

Evidence strengthenings recorded in the raw dumps: F6 contradicts the
crate's own RULES.md, not just external rules; F16's reachability rests on
the take arm using CardParser (arbitrary tokens) while spend uses
Enum::exact + clone-and-verify; F25 has three additional `{:?}` sites.

Overall assessment: the original games-batch-b review is highly accurate —
all 35 locations and code traces checked out, including the critical
alhambra take() exploit and both major grid findings; the only verdict-level
correction is F9, where the reviewer's unreachability analysis missed the
interaction with mid-turn state mutation, flipping a "fragile but safe"
nit into a reachable minor correctness bug.
