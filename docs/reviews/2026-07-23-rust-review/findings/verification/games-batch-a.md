# Verification: games-batch-a (unit 3)

Independent verification of `findings/games-batch-a.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Crates: `game/roll-through-the-ages-2`, `game/starship-catan-1`.
Raw verdict dumps: `raw/games-batch-a-rtta.md`,
`raw/games-batch-a-starship.md`. Process log: `games-batch-a-LOG.md`.

## Per-finding verdicts

### roll-through-the-ages-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | roll() re-matches phase after keep_skulls() advanced it | major | CONFIRMED | keep_skulls() calls next_phase() on all-skull rerolls (lib.rs:680-687); stale match at 742 re-traced for both scenarios (Leadership extra roll consumed; next player's remaining_rolls decremented). Go-inherited, undocumented, no all-skull `roll` test |
| F2 | RULES.md:44 claims developments globally unique | minor | CONFIRMED | buy_development / available_developments check only the buyer's own set (lib.rs:625-631, 1089) |
| F3 | RULES.md:90 pestilence "every player"; roller exempt | minor | CONFIRMED | `p == cp` skip at lib.rs:408-411; test at 2164 asserts roller untouched |
| F4 | RULES.md:87 skulls "never rerolled"; Leadership can | minor | CONFIRMED | kept_dice merged back at lib.rs:293-295; test at 1913-1927 rerolls a kept skull |
| F5 | Food cap of 15 undocumented | nit | CONFIRMED | Clamp at lib.rs:350-359; absent from RULES.md |
| F6 | command(): 11 identical finished-scores epilogues | minor | CONFIRMED | 11 copies of the ~12-line block across lib.rs:1521-1755 |
| F7 | build_parser ship bound uses max(wood, cloth) | nit | CONFIRMED | command.rs:188; Go parser has the identical bound; undocumented quirk |
| F8 | Quarrying bonus bypasses per-type good cap | nit | CONFIRMED | Raw `+= 1` at player_board.rs:129-135 skips gain_good's cap; stone can hit 8 |
| F9 | roll() bounds check `n < 0` admits n == 0 | nit | ADJUSTED | Guard and parser-unreachability confirmed, but "silently a no-op" is imprecise: the call still consumes the reroll (remaining_rolls decrement / ExtraRoll advance) while rerolling nothing. Nit stands |
| F10 | Pointless `let mut logs = logs;` rebind | nit | CONFIRMED | lib.rs:1252 |

### starship-catan-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F11 | Cannon cost surcharge checks boosters, not cannons | major | CONFIRMED | cannon_transaction tests `Resource::Booster` (lib.rs:311); RULES.md:172-173 states per-item surcharge; booster_transaction parallel confirms copy-paste |
| F12 | can_lose_module `\|\|` allows voluntary sacrifice on any pirate | major | CONFIRMED | lib.rs:1266-1268; parser offers `lose` on any Pirate (command.rs:165-167); Game::lose ends flight with no fight-lost check (lib.rs:1666-1690); lead re-traced |
| F13 | TradeAndBuild buys never check astro affordability | major | CONFIRMED | Buy branch (lib.rs:996-1011) lacks the Flight branch's astro check; trade debits unconditionally (lib.rs:1064-1065) |
| F14 | Unbounded buy/sell amounts overflow i32 (debug panic) | major | CONFIRMED | Int::positive() has no max; overflow at lib.rs:938 reachable (several cards have `maximum: 0`, skipping the lib.rs:921 cap) and at lib.rs:373 |
| F15 | Sensor peek never rendered to peeking player | major | CONFIRMED | `_peeking` unused (render.rs:108); no log/render exposes peeked card identities |
| F16 | "Current turn:" row shows viewer, not current player | minor | CONFIRMED | N::Player(viewer) at render.rs:125; `current` bound at 112 |
| F17 | Dead code: next_turn, Transaction::gain, description/join_dice, start_card | minor | CONFIRMED | All four confirmed dead by crate-wide grep |
| F18 | Direction-mismatch error interpolates attempted direction | nit | CONFIRMED | trade_dir interpolated at lib.rs:917; card's `direction` in scope |
| F19 | last_sectors unbounded and rendered in full | nit | CONFIRMED | Uncapped prepend at lib.rs:798-800; full history rendered (render.rs:129-139) |
| F20 | flight_actions BTreeMap<usize, bool> only stores true | nit | CONFIRMED | Sole insert site lib.rs:822-824 inserts `true`; consumers count trues |

## Summary

- Findings verified: 20
- CONFIRMED: 19, ADJUSTED: 1 (F9), REJECTED: 0, UNVERIFIABLE: 0
- Corrected tallies for the unit (unchanged from original): 0 critical /
  6 major / 6 minor / 8 nit
- Lead spot-checked the F9 adjustment (reproduced: `roll 0` alone keeps all
  dice, rolls zero, and still burns the reroll via the Phase::Roll arm) and
  re-verified F12 directly (parser gate, Game::lose body, end_flight call
  all as claimed).

## Notable corrections

None changed a severity; one factual refinement:

- F9: the original's "silently a no-op" understates the effect — an
  n == 0 entry rolls nothing, but the command still consumes the reroll
  (or the Leadership extra roll). Still parser-unreachable, so nit is
  the right severity.

Two evidence strengthenings recorded in the raw dumps:

- F12: `can_lose_module` also lacks the phase/pirate-card guards its
  siblings `can_fight`/`can_pay_ransom` carry; the recommended `&&` fix
  is still sufficient because `losing_module` is only set in the
  fight-loss path.
- F14: the Flight-branch overflow is reachable specifically because
  several trade cards have `maximum: 0` (card.rs:472-481, 540-556),
  which skips the lib.rs:921 amount cap — a sharper reachability
  argument than the original's.

Overall assessment: the original games-batch-a review is highly accurate —
all locations, traces, and severities checked out, including the subtle
F1 phase-re-match cascade and the cross-referenced Go sources; the single
adjustment is a wording refinement, not a verdict change.
