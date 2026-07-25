# Findings — games-batch-a

Crates: `game/roll-through-the-ages-2` (4,936 LOC), `game/starship-catan-1`
(4,046 LOC). Reviewed against the snapshot worktree
(`/home/beefsack/Development/brdgme-review-snapshot`, HEAD `f8763a5b…`).
All line numbers were spot-verified against the snapshot by the Lead.

**Boilerplate binaries (both crates, reviewed once):** the four `src/bin/*`
files (`_cli`, `_repl`, `_fuzz`, `_http`) and `tests/contract.rs` are
byte-for-byte the standard template and are clean. The `_http` binary's
`.expect("Invalid socket address")` on `ADDR` parsing is a process-startup
failure, which the project error-handling rules explicitly allow. No
per-binary findings; the systemic "108 near-identical binaries" observation
belongs to the dependencies unit.

**Cross-references to issues tracked in other units** (concrete instances in
these crates, not re-flagged as findings): none — no non-ASCII parser input
paths, markup `slice()` uses, or warp-handler unwraps originate in this code.

**Cargo.toml (both crates):** standard game dependency block, no extras, no
unused dependencies evident from the code. Nothing to flag beyond the
workspace-wide version-pinning drift already tracked by the dependencies
unit.

## roll-through-the-ages-2

Overall: a careful, heavily annotated Go port with strong test coverage;
documented Go quirks are reproduced deliberately and flagged in place (those
are not findings). One real logic bug in the roll/phase flow, plus several
RULES.md contradictions with the (authoritative) code. No player-reachable
panics found — all parser bounds trace back to `can_*` guards keeping
`Int::bounded(1, max)` with `max >= 1`.

### roll() re-matches self.phase after keep_skulls() may have advanced it — Leadership extra roll skipped, next player can lose a reroll
- severity: major
- category: correctness
- location: game/roll-through-the-ages-2/src/lib.rs:742
- finding: `roll()` calls `keep_skulls()` (lib.rs:741) and *then* matches on
  `self.phase` (lib.rs:742-753), but `keep_skulls()` itself calls
  `next_phase()` when the reroll empties `rolled_dice` (all skulls,
  lib.rs:680-687). Two bad outcomes via legal commands:
  1. Phase=Roll, player has Leadership, rerolls all remaining dice and gets
     all skulls: `keep_skulls` advances to `Phase::ExtraRoll` (Leadership
     guard), then the `Phase::ExtraRoll` arm in `roll()` fires and calls
     `next_phase()` again — the Leadership extra roll is silently skipped.
     The crate's own test `test_game_keep_skulls_all_disaster_leadership`
     (lib.rs:1891) asserts the *`next`-command* path parks at ExtraRoll, so
     identical game states diverge depending on whether the last action was
     `next` or an all-skull `roll`. Contradicts RULES.md ("Reroll 1 die
     after your last regular roll").
  2. Without Leadership, an all-skull reroll of 5+ dice triggers Revolt,
     wiping goods; with no buying power the cascade blows through
     Build/Trade/Buy/Discard into `next_turn()`, and the *next* player's
     `roll_phase()` sets `remaining_rolls = 2` and parks at `Phase::Roll`.
     Back in the previous player's `roll()`, the `Phase::Roll` arm then
     decrements the next player's `remaining_rolls` to 1 — they silently
     lose a reroll.
  This structure is Go-inherited (`brdgme-go/roll_through_the_ages_1/
  roll_command.go:46-55`), but unlike the other ported quirks it is
  undocumented here and produces objectively wrong state transitions.
- recommendation: Snapshot the phase before calling `keep_skulls()`
  (`let phase = self.phase;`) and match on the snapshot, so the
  Roll/ExtraRoll bookkeeping only runs when `keep_skulls` did not already
  advance the phase. Add regression tests for both scenarios (no test
  currently exercises an all-skull `roll`, only the `next` path). If Go
  fidelity is preferred, document it as a quirk like the others and note the
  divergence from the `next`-path test's expectation.

### RULES.md claims developments are globally unique; code allows each player to buy each one
- severity: minor
- category: correctness
- location: game/roll-through-the-ages-2/RULES.md:44
- finding: RULES.md says "any development already owned by a player can't be
  bought again by anyone". Both `buy_development` and
  `available_developments` (lib.rs:625-631, lib.rs:1089) only check the
  *buying player's own* set — each player can independently buy the same
  development (which matches the physical game). Doc contradicts code.
- recommendation: Fix RULES.md:44 to say each development can be bought once
  *per player*.

### RULES.md says pestilence hits "every player"; the roller is exempt
- severity: minor
- category: correctness
- location: game/roll-through-the-ages-2/RULES.md:90
- finding: The 3-skull pestilence branch (lib.rs:406-430) explicitly skips
  `p == cp`; only opponents take the 3 disaster points (confirmed by test
  `resolve_pestilence_hits_other_players_without_medicine`, lib.rs:2164).
  RULES.md says "3 disaster points to every player".
- recommendation: Change RULES.md to "3 disaster points to every *other*
  player".

### RULES.md says skulls "can never be rerolled"; Leadership's extra roll can reroll them
- severity: minor
- category: correctness
- location: game/roll-through-the-ages-2/RULES.md:87
- finding: `roll_extra_phase` (lib.rs:290-295) merges `kept_dice` (which
  holds all locked skulls) back into `rolled_dice`, and the ExtraRoll parser
  admits any position — a Leadership owner can reroll a skull
  (`test_roll_extra_command`, lib.rs:1913-1927, asserts `roll 7` on a kept
  skull succeeds). RULES.md:87 says skulls "can never be rerolled". Behavior
  is Go-faithful; the doc is wrong.
- recommendation: Amend RULES.md:87 to note the Leadership extra-roll
  exception (or drop "never").

### Food cap of 15 is undocumented
- severity: nit
- category: correctness
- location: game/roll-through-the-ages-2/src/lib.rs:350
- finding: `phase_resolve` clamps food to a maximum of 15 before feeding
  cities (lib.rs:350-359). RULES.md documents neither the cap nor that
  Preservation doubling can be clipped by it.
- recommendation: Add a sentence to the RULES.md food/resolve section.

### command() dispatch: 11 identical copies of the finished-scores epilogue
- severity: minor
- category: simplicity
- location: game/roll-through-the-ages-2/src/lib.rs:1539
- finding: Every match arm in `command()` (lib.rs:1521-1755) repeats the
  same ~12-line `if self.finished { ... scores ... placings_log ... }` block
  (~110 lines of duplication). It obscures the actual per-command dispatch
  and is a maintenance hazard — an edit to one arm is easy to miss in the
  other ten.
- recommendation: Restructure as
  `let mut resp = match value { ... }?;` followed by one shared
  `if self.finished { ... }` epilogue, or extract a small
  `finish_response(resp)` helper.

### build_parser ship variant over-admits: uses max(wood, cloth) instead of min, ignores the 5-ship cap
- severity: nit
- category: correctness
- location: game/roll-through-the-ages-2/src/command.rs:188
- finding: `let max = wood.max(cloth);` — but each ship costs 1 wood *and* 1
  cloth, so the true bound is `min(wood, cloth)` (also `5 - ships`). The
  parser therefore admits amounts `build_ship` then rejects with a GameError:
  a parseable command that can never succeed. No panic and no bad state.
  Go-faithful (`BuildTargetShipParser` has the identical `max`), but
  undocumented here unlike the other quirks.
- recommendation: Either fix to `wood.min(cloth).min(5 - b.ships)`
  (diverging from Go, so document), or add a quirk comment matching the
  style of the others.

### Quarrying bonus bypasses the per-type good cap
- severity: nit
- category: correctness
- location: game/roll-through-the-ages-2/src/player_board.rs:129
- finding: `gain_good` respects `good_maximum`, but the Quarrying `+1` is
  applied via `*self.goods.entry(good).or_insert(0) += 1` (player_board.rs:
  129-135), ignoring the cap — stone can reach 8 (cap 7). Go-faithful, and
  the module documents other quirks (e.g. round-robin restart) but not this
  one.
- recommendation: Add a one-line quirk comment (and optionally a RULES.md
  note); no behavior change needed.

### roll() dice-index bounds check uses n < 0 instead of n < 1
- severity: nit
- category: quality
- location: game/roll-through-the-ages-2/src/lib.rs:724
- finding: `if n < 0 || n > l` lets `n == 0` pass the guard; it is then
  silently a no-op (no die has position 0) rather than an error. Unreachable
  via the parser (`Int::bounded(1, max_i)`), so cosmetic.
- recommendation: Change to `n < 1` for a correct error message, or leave as
  a Go-faithful wart.

### Pointless `let mut logs = logs;` rebind in discard()
- severity: nit
- category: quality
- location: game/roll-through-the-ages-2/src/lib.rs:1252
- finding: `let logs = vec![...]; let mut logs = logs;` — pure shadowing to
  gain mutability.
- recommendation: Write `let mut logs = vec![...]` directly.

### Documented Go quirks preserved deliberately (cross-references, not findings)
- `invade()` log reports `amount` disaster points while applying
  `amount * 2` — documented at lib.rs:1278-1284.
- `take()` wrong-count error echoes `actions.len()` instead of the required
  count — documented at lib.rs:1162-1171.
- Swap error message typo "the you only have room for" (lib.rs:1437) —
  verbatim from Go.

### Per-module verdicts
- `lib.rs` (incl. tests): **not clean** — the roll/keep_skulls phase bug
  (major), the 11× duplicated epilogue, minor nits. Tests are extensive.
- `command.rs`: **clean** apart from the ship-bound nit; all parser gates
  verified against `can_*` guards.
- `render.rs`: **clean** — no panic-reachable indexing; sensible node
  construction.
- `player_board.rs`: **clean** apart from the undocumented Quarrying cap
  bypass; scoring math verified against RULES.md.
- `development.rs`, `good.rs`, `monument.rs`, `dice.rs`, `take.rs`:
  **clean** — all data tables cross-checked against RULES.md.

## starship-catan-1

Overall: well-structured and follows the project anatomy; transaction/fit
machinery, hidden-info redaction, and parser gating are solid, and VP math
matches RULES.md. However, three rules/economy defects are reachable by
legal play, plus a debug-build integer-overflow panic via huge `buy`
amounts, and the render layer hides the Sensor peek from the peeking player.

### Cannon cost scales off boosters, not cannons
- severity: major
- category: correctness
- location: game/starship-catan-1/src/lib.rs:311
- finding: `PlayerBoard::cannon_transaction` adds the +1 science surcharge
  when `self.res(Resource::Booster) >= 3` — it checks the booster count
  inside the *cannon* cost function. The parallel `booster_transaction`
  (lib.rs:301) correctly checks `Resource::Booster`, so this is a copy-paste
  bug. RULES.md states the rule in parallel for both (the surcharge should
  depend on cannons owned). Effect: a player with 3+ boosters and 0 cannons
  overpays for cannons; a player with 3+ cannons and <3 boosters underpays.
- recommendation: Change the condition in `cannon_transaction` to
  `self.res(Resource::Cannon) >= 3`.

### can_lose_module uses ||, enabling voluntary module sacrifice to skip any pirate
- severity: major
- category: correctness
- location: game/starship-catan-1/src/lib.rs:1267
- finding: `can_lose_module` is `self.current_player == player ||
  self.losing_module`. For the current player this is always true, so while
  landed on *any* pirate (before fighting, even a harmless ransom-$3 one)
  the parser offers `lose` (command.rs:165) and `Game::lose` (lib.rs:1666)
  accepts it: the player destroys one of their own module levels and the
  flight ends, bypassing the fight/ransom decision entirely. RULES.md
  documents `lose` only "after losing a module-destroying fight: pick the
  module lost".
- recommendation: Change the guard to
  `self.current_player == player && self.losing_module`.

### Trade-and-build buys never check astro affordability — players can go negative
- severity: major
- category: correctness
- location: game/starship-catan-1/src/lib.rs:996
- finding: The `Phase::Flight` branch of `can_trade` verifies
  `amount * price > astro` (lib.rs:937-947), but the `Phase::TradeAndBuild`
  buy branch (lib.rs:996-1011) only checks `can_fit` and that a buy price
  exists. `Game::trade` then debits astro unconditionally
  (lib.rs:1064-1065), so `buy 2 carbon` at a trading post with $0 leaves the
  player at negative astro — reachable via a fully legal command sequence
  (found a trading post, then `buy N <good>` with insufficient funds).
- recommendation: Add the same `amount * price > astro` check to the
  TradeAndBuild buy branch of `can_trade`, returning a "you only have $N"
  error.

### Unbounded buy/sell amounts overflow i32 (debug panic reachable from player input)
- severity: major
- category: correctness
- location: game/starship-catan-1/src/command.rs:121
- finding: `buy`/`sell` amounts parse via `Int::positive()` (command.rs:121,
  136), admitting any i32 up to `i32::MAX`. `can_trade` computes
  `amount * price` (lib.rs:938), `Game::trade` computes `amount * price`
  (lib.rs:1064), and `fit_transaction` computes `cur + v` (lib.rs:373) with
  plain i32 arithmetic. `buy 2147483647 carbon` at a $2+ flight trade planet
  overflows `amount * price` and panics under overflow checks (debug/CI
  builds); in release the wrapped values happen to be rejected by downstream
  fit/afford checks, but per project policy a player-reachable panic is a
  defect.
- recommendation: Validate the amount before arithmetic — reject amounts
  larger than what can fit/be afforded using `checked_mul`/`checked_add`, or
  cap the parser with `Int::bounded(1, N)`.

### Sensor peek is never rendered to the peeking player
- severity: major
- category: quality
- location: game/starship-catan-1/src/render.rs:108
- finding: `PlayerState::render` passes `Some(&self.peeking)` into `render`
  (render.rs:66), but the parameter is named `_peeking` and unused. No log
  shows the peeked cards either (`sector` logs only "is using the sensor
  module to peek at N cards", lib.rs:1377-1388; `put` logs only "put a card
  on the top of the pile"). A human player using the Sensor module must
  issue `put <#> top|bottom` completely blind — the module is unusable as
  intended. (The data does exist in the `PlayerState` JSON, so API
  clients/bots can see it; the human renderer cannot.)
- recommendation: Render the peeked cards (numbered, matching `put <#>`
  indices) in the `PlayerState` renderer when peeking is non-empty.

### "Current turn:" row displays the viewer, not the current player
- severity: minor
- category: correctness
- location: game/starship-catan-1/src/render.rs:125
- finding: The turn header renders `N::Player(viewer)`, so every player sees
  their own name next to "Current turn:" regardless of whose turn it
  actually is. `pub_state.current_player` is already bound as `current`
  (render.rs:112) and used elsewhere in the same function.
- recommendation: Use `N::Player(current)` in that row.

### Dead code: next_turn, Transaction::gain, Module::description/join_dice, start_card field
- severity: minor
- category: quality
- location: game/starship-catan-1/src/lib.rs:756
- finding: Grep confirms none of these are called/read anywhere in the
  crate: `Game::next_turn` (lib.rs:756, fully shadowed by `done()` inlining
  the same logic at lib.rs:1495-1496), `Transaction::gain` (lib.rs:61),
  `Module::description` and its helper `join_dice` (card.rs:124-149,
  card.rs:174; superseded by `Module::summary` used in render.rs), and the
  `start_card` field (card.rs:257; only ever written as `false`, never
  read).
- recommendation: Delete them, or wire `next_turn` into `done()` if the
  indirection is wanted.

### Misleading direction-mismatch error message in can_trade
- severity: nit
- category: quality
- location: game/starship-catan-1/src/lib.rs:917
- finding: When the card's direction forbids the attempted trade, the
  message interpolates the *attempted* direction: trying to buy at a
  sell-only card yields "you can only buy with this trade card" — the
  opposite of the truth. `direction` (the card's allowed direction,
  lib.rs:885) is in scope.
- recommendation: Interpolate `direction.string()` instead of
  `trade_dir.string()`.

### last_sectors grows unbounded and is rendered in full
- severity: nit
- category: quality
- location: game/starship-catan-1/src/lib.rs:798
- finding: Every flight prepends to `last_sectors` with no cap
  (lib.rs:798-800), and the renderer prints the entire history
  (render.rs:129-139). In a long game this produces an ever-growing state
  field and render line for what is recent-history flavor.
- recommendation: Truncate to a small fixed length (e.g. most recent few) on
  insert.

### flight_actions: BTreeMap<usize, bool> only ever stores true
- severity: nit
- category: simplicity
- location: game/starship-catan-1/src/lib.rs:505
- finding: The map's values are always `true` (only insert site
  lib.rs:822-824); `remaining_actions` and `flight_actions_used` both just
  count `true` values. A `BTreeSet<usize>` expresses the actual data model.
- recommendation: Switch to a set — noting this changes the serialized
  shape, so coordinate with any saved-state compatibility concerns, or leave
  as-is.

### Per-module verdicts
- `lib.rs`: **not clean** — three major rules/economy bugs (cannon cost,
  `lose` guard, missing astro check), one reachable overflow, plus
  dead/duplicated code.
- `render.rs`: **not clean** — sensor peek never rendered (major),
  current-turn display bug (minor).
- `command.rs`: **clean** — gates mirror the `can_*` guards; bounds sensible
  (the unbounded `Int::positive()` amounts are flagged against lib.rs where
  the arithmetic overflows).
- `card.rs`: **clean** logic/data; carries the dead
  `description`/`join_dice`/`start_card` items flagged above.
