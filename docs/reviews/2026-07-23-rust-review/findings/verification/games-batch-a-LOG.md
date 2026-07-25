# Verification LOG: games-batch-a (2026-07-24)

Independent verification of `findings/games-batch-a.md` (unit 3, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

20 findings total in games-batch-a.md, numbered F1-F20 in document order.
Two serial Workers (model fable), split by crate so each reads a coherent
source set:

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | game/roll-through-the-ages-2 | F1 roll()/keep_skulls phase re-match (major), F2 RULES.md dev uniqueness (minor), F3 RULES.md pestilence "every player" (minor), F4 RULES.md skulls never rerolled (minor), F5 food cap 15 undocumented (nit), F6 command() 11x epilogue duplication (minor), F7 build_parser ship max(wood,cloth) (nit), F8 Quarrying cap bypass (nit), F9 roll() n<0 bounds check (nit), F10 `let mut logs = logs;` rebind (nit) | raw/games-batch-a-rtta.md |
| W2 | game/starship-catan-1 | F11 cannon cost checks boosters (major), F12 can_lose_module uses \|\| (major), F13 TradeAndBuild buy skips astro check (major), F14 unbounded buy/sell i32 overflow (major), F15 sensor peek never rendered (major), F16 "Current turn:" shows viewer (minor), F17 dead code next_turn/gain/description/join_dice/start_card (minor), F18 misleading direction error (nit), F19 last_sectors unbounded (nit), F20 flight_actions BTreeMap<usize,bool> (nit) | raw/games-batch-a-starship.md |

Game-rule correctness claims are judged against each crate's RULES.md and
in-code quirk comments. Lead spot-checks all REJECTED/ADJUSTED verdicts;
if a Worker confirms everything, Lead re-verifies its 1-2 hardest
confirmations. Curated report: verification/games-batch-a.md.

### W1 dispatched — roll-through-the-ages-2 (F1-F10)

### W1 returned

9 CONFIRMED, 1 ADJUSTED (F9). Dump: raw/games-batch-a-rtta.md.
- VERIFIED: F1 both scenarios re-traced by the Worker — keep_skulls()
  next_phase() at lib.rs:680-687, stale re-match at 742; Leadership
  extra roll consumed, next-player remaining_rolls decrement both hold.
  Go-inherited (roll_command.go:46-55), no quirk comment, no all-skull
  `roll` test. Major stands.
- ADJUSTED: F9 — guard does admit n == 0 and is parser-unreachable, but
  "silently a no-op" is imprecise: the roll still consumes the reroll
  (decrements remaining_rolls / advances ExtraRoll) while rerolling
  nothing. Nit severity unchanged. Lead spot-check pending.

### W2 dispatched — starship-catan-1 (F11-F20)

### W2 returned

All 10 CONFIRMED (F11-F20). Dump: raw/games-batch-a-starship.md.
- VERIFIED: F14 reachability sharpened — Int::positive() is min 1 / no
  max; several trade cards have `maximum: 0` (card.rs:472-481, 540-556)
  which skips the lib.rs:921 amount cap, so the Flight-branch
  `amount * price` overflow at lib.rs:938 is reachable. Debug panic
  confirmed.
- VERIFIED: F12 — siblings can_fight/can_pay_ransom guard
  `!losing_module` plus phase; can_lose_module lacks both. `&&` fix
  still sufficient.
- No REJECTED/ADJUSTED verdicts to spot-check.

## Lead spot-checks

- CONFIRMED: F9 (the one ADJUSTED verdict) — read lib.rs:708-755.
  Guard at 724 admits n == 0; dice positions are i+1 >= 1 so 0 matches
  nothing, kept = all dice, roll_n(0), then the Phase::Roll arm still
  decrements remaining_rolls. Worker's correction ("consumes the reroll
  while rerolling nothing") is exact; original's "silently a no-op" was
  imprecise. Nit stands.
- CONFIRMED: F12 (hardest W2 confirmation, re-verified against
  rubber-stamping) — read command.rs:158-175 and lib.rs:1655-1690.
  Parser pushes lose_parser() on any Pirate when can_lose_module (always
  true for current player); Game::lose checks only can_lose_module +
  module ownership, then end_flight(). Bypass reproduced exactly.

## Curation complete (2026-07-24)

19/20 CONFIRMED, 1 ADJUSTED (F9, wording only), 0 REJECTED,
0 UNVERIFIABLE. Corrected unit tally (unchanged): 0 critical / 6 major /
6 minor / 8 nit. Report: verification/games-batch-a.md. LOG closed.
