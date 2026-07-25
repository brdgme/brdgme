# Verification LOG: games-batch-b (2026-07-24)

Independent verification of `findings/games-batch-b.md` (unit 4, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

35 findings total in games-batch-b.md, numbered F1-F35 in document order.
Three serial Workers (model fable per user override), split by crate so
each reads a coherent source set:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | game/seven-wonders-1 | F1 Halicarnassus B DrawDiscard vp never scored (major), F2 DrawDiscard resolver soft-lock (major), F3 auto-discarded 7th card pays 3 coins (major), F4 same-turn trade of fresh resources (minor), F5 MimicGuild only copies Bonus guilds (minor), F6 wonder sacrifice card enters discard (minor), F7 both sides of same wonder dealable (minor), F8 discard pile hidden from players (minor/quality), F9 deal re-validated by index into recomputed list (nit), F10 unguarded player indexing (nit), F11 finished-game epilogue x6 (nit), F12 military log raw player index (nit), F13 start_hand dead-weight (nit), F14 test coverage gaps (minor), F15 lib.rs 1,565-line grab-bag (nit) | raw/games-batch-b-7w.md |
| W2 | game/alhambra-1 | F16 take() mints duplicate cards (critical), F17 place indices diverge after placement (major), F18 grid_longest_ext_wall premature break (major), F19 Dirk excluded from placings (minor), F20 reduced 2p money deck (minor), F21 test coverage misses riskiest logic (minor), F22 epilogue x6 (nit), F23 invariant-guarded panics (nit), F24 gap-check range asymmetry (nit), F25 Debug formatting in messages (nit), F26 tile_counts duplicated (nit), F27 column headers wrap past 26 (nit), F28 Vec-as-queue/HashMap-as-set (nit) | raw/games-batch-b-alhambra.md |
| W3 | game/splendor-2 | F29 prestige tie broken by most cards (minor), F30 take() never validates gems (minor), F31 local cost.rs vs lib/cost assessment (minor/dependencies), F32 reserve parser offers row-3 (nit), F33 epilogue x5 (nit), F34 "remaning" typo (nit), F35 .expect() in visit_phase (nit) | raw/games-batch-b-splendor.md |

Game-rule correctness claims are judged against each crate's RULES.md /
PORTING_NOTES.md and in-code comments; official-rules claims without an
in-repo source are checked for internal consistency and flagged if they
rest solely on the original reviewer's rules knowledge. Lead spot-checks
all REJECTED/ADJUSTED verdicts; if a Worker confirms everything, Lead
re-verifies its 1-2 hardest confirmations. Curated report:
verification/games-batch-b.md.

### W1 dispatched — seven-wonders-1 (F1-F15)

### W1 returned

13 CONFIRMED, 2 ADJUSTED (F9, F14). Dump: raw/games-batch-b-7w.md.
- F9 ADJUSTED with severity upgrade nit -> minor: the original's
  "verified unreachable" claim is refuted — mid-execution builds by
  earlier players change can_afford_perm output (lib/cost early return),
  so the recomputed deal list can shrink/reorder between choice and
  execution; wrong-deal remap or out-of-range -> free build via
  unwrap_or_default is reachable. Lead spot-check pending.
- F14 ADJUSTED (severity unchanged): MimicGuild IS tested
  (test_card_mimic_guild lib.rs:1503); all other listed gaps confirmed
  absent. Lead spot-check pending.
- F3 confirmed with caveat: coin inflation is symmetric across players,
  so minor is arguable; major kept.

### Lead spot-checks (W1)

- F9 ADJUSTED upheld — read lib.rs:265-297/324-326/418-431/456-517 and
  lib/cost/src/lib.rs:165-203. execute_actions mutates cards in index
  order; resolve_deal recomputes can_afford_cost at execution; the deal
  list derives from can_afford_perm allocations over neighbor goods, and
  the early return at lib/cost:182-184 fires at every recursion level,
  so a neighbor card built mid-execution can reorder or shrink the list.
  "Append-only, verified unreachable" is refuted; nit -> minor upgrade
  accepted (wrong-neighbor payment or free build via unwrap_or_default).
- F14 ADJUSTED upheld — test_card_mimic_guild exists at lib.rs:1503-1511
  (Olympia B Wonder Stage 3 + Builders/Workers Guild, asserts vp == 2);
  MimicGuild removed from the gap list, other gaps stand, minor kept.

### W2 dispatched — alhambra-1 (F16-F28)

### W2 returned

11 CONFIRMED, 2 ADJUSTED (F18, F21). Dump: raw/games-batch-b-alhambra.md.
- F16 critical upheld: unconditional push at lib.rs:574, take arm uses
  CardParser (arbitrary tokens) so the dup is reachable; spend uses
  clone-and-verify by contrast.
- F18 ADJUSTED (major kept): break is unconditional as claimed, but on
  the doc's exact grid the result is HashMap-iteration-order dependent
  (start T0 -> 1, start T1 -> 2), so wall scoring is undercountable AND
  nondeterministic — strengthens the finding. Lead spot-check pending.
- F21 ADJUSTED (minor kept): log_final_place does smoke-test
  single-currency final-place distribution; the tie path is the gap.
  Lead spot-check pending.
- No severity changes.

### Lead spot-checks (W2)

- F18 ADJUSTED upheld — read card.rs:477-531. The break at 516 is
  unconditional after the first non-Empty candidate; `pub type Grid =
  HashMap<Vect, Tile>` (card.rs:307) so the outer `g.iter()` order is
  nondeterministic, and the visited-insert at 491 blocks the recovery
  walk when the blocked start tile iterates first. Undercount real,
  result additionally order-dependent. Major stands.
- F21 ADJUSTED upheld — log_final_place (lib.rs:1474-1499) does exercise
  final_place_phase distribution for a single currency; the missing
  coverage is the tie path. Minor stands.

### W3 dispatched — splendor-2 (F29-F35)

### W3 returned

All 7 CONFIRMED, no adjustments. Dump: raw/games-batch-b-splendor.md.
Since W3 confirmed everything, Lead directly re-verified its two
hardest confirmations to guard against rubber-stamping:
- F29: read lib/game/src/game.rs:154-172 (gen_placings sorts keys
  ascending, assigns places via `.rev()` over sorted keys, so a larger
  metric vector wins ties) and lib.rs:193-206 (`vec![prestige,
  cards.len()]`) — confirms more-cards-wins-tie inversion.
- F31: read lib/cost/src/lib.rs full `pub fn` list (new, from_keys, add,
  inv, sub, pos_neg, can_afford, take, drop, is_zero, trim, sum, keys,
  to_keys, can_afford_perm) — no get/set exists, confirming the
  dependencies claim.

## Curation complete (2026-07-24)

31/35 CONFIRMED, 4 ADJUSTED (F9 severity upgrade nit->minor; F14, F18,
F21 factual refinements only), 0 REJECTED, 0 UNVERIFIABLE. Corrected
unit tally: 1 critical / 5 major / 13 minor / 16 nit (was 1/5/12/17;
F9 moved nit -> minor). Report: verification/games-batch-b.md. LOG
closed.
