# Findings — games-batch-b (2026-07-23)

Crates reviewed: `game/seven-wonders-1` (3,809 LOC), `game/alhambra-1` (2,966
LOC), `game/splendor-2` (2,719 LOC). Snapshot worktree
`/home/beefsack/Development/brdgme-review-snapshot`, HEAD
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. Review-only; no builds or tests
were run. All three crates' four boilerplate binaries
(`_cli`/`_repl`/`_http`/`_fuzz`) are byte-for-byte the standard pattern — no
deviations to report. `tests/contract.rs` is the standard contract harness in
all three.

## game/seven-wonders-1

### Halicarnassus B wonder-stage VP is never scored
- severity: major
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:706
- finding: `player_vp` only matches `CardEffect::VP`, `CardEffect::Bonus`, and
  `CardEffect::MimicGuild`; everything else falls through `_ => {}`.
  Halicarnassus B stages 1/2/3 carry `CardEffect::DrawDiscard { vp: 2 }`,
  `{ vp: 1 }`, `{ vp: 0 }` (card.rs:1269,1277 — matching the official 2/1/0 VP
  values), so the `vp` payload is silently dropped. A Halicarnassus B player
  loses 3 VP per game.
- recommendation: add `CardEffect::DrawDiscard { vp } => vp += vp,` to the
  `player_vp` match, plus a scoring test covering Halicarnassus B.

### Reachable permanent soft-lock in the DrawDiscard resolver
- severity: major
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:410
- finding: The resolver fires whenever `!self.discard.is_empty()`, but
  `take_from_discard` (lib.rs:908-925) rejects any card the player already
  owns, and while a resolver is pending `command_parser` offers only `take`.
  If every card in the discard pile is already built by the resolving player
  (easy in age 1: own Lumber Yard, second Lumber Yard copy discarded, then
  build the Halicarnassus stage), no command can ever succeed and `status()`
  reports that player's turn forever. `PORTING_NOTES.md` explicitly claims
  "DrawDiscard resolver only fires if there are takeable cards in discard
  (cards the player doesn't already own)" — the code does not implement that
  filter.
- recommendation: at queue time (and re-check at resolve time, since the pile
  changes during `execute_actions`) only push the resolver if
  `self.discard.iter().any(|c| !self.cards[player].iter().any(|o| o.name == c.name))`;
  alternatively/additionally offer a "pass" command in the resolver state.

### Auto-discarded 7th card of each age pays 3 coins
- severity: major
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:192
- finding: `end_hand` does `let card = self.hands[p].pop().unwrap();
  self.discard.push(card); self.coins[p] += DISCARD_COINS;`. Official rules:
  the unplayed last card of an age is discarded with **no coins**. This
  inflates every player's economy by up to 9 coins per game (~3 VP plus extra
  trade liquidity), systematically distorting scoring. Not listed as a
  preserved quirk in `PORTING_NOTES.md`; `RULES.md` is silent.
- recommendation: drop the `self.coins[p] += DISCARD_COINS` line in the
  end-of-age auto-discard path (keep it in `execute_discard` for player-chosen
  discards); update the log text which currently hides the payment.

### Same-turn trade of freshly built resources
- severity: minor
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:265
- finding: `execute_actions` resolves actions sequentially p0..pn, mutating
  `cards`/`coins` as it goes, so player p+1's `can_afford_cost` sees resource
  cards player p built *this same turn*. Official FAQ: resources bought from a
  neighbor must have been available before that card was played. Common
  digital shortcut, but asymmetric (earlier-indexed players can't
  reciprocate).
- recommendation: snapshot each player's tradable goods at hand start (before
  any execution) and use the snapshot in `can_afford_cost` during
  `execute_actions`, or document the deviation in `PORTING_NOTES.md`/`RULES.md`.

### MimicGuild (Olympia B stage 3) can only copy Bonus-effect guilds
- severity: minor
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:735
- finding: `mimic_guild_vp` silently skips guilds like Scientists Guild
  (`CardEffect::Science`), which is a legitimate copy target in the official
  rules (gain a science symbol of choice at scoring).
- recommendation: extend the mimic logic to evaluate `Science`-effect guilds
  (compute marginal `science_vp` with the extra wildcard) or document the
  restriction.

### Wonder-stage sacrifice card enters the shared discard pile
- severity: minor
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:324
- finding: `execute_build`'s wonder path does `self.discard.push(hand_card)`.
  Officially the sacrificed card is placed face-down under the wonder board
  and never enters the discard pile — so it should not be retrievable by
  Halicarnassus nor inflate `discard_count`. Not documented as a preserved
  quirk.
- recommendation: drop the push (or track sacrificed cards separately); if
  kept for Go parity, note it in `PORTING_NOTES.md`.

### Both sides of the same wonder can be dealt in one game
- severity: minor
- category: correctness
- location: game/seven-wonders-1/src/lib.rs:115
- finding: `cities()` (card.rs:1350-1479) lists all 14 A/B entries and
  `start_game` takes the first `players` after shuffling, so e.g. "Rhodes A"
  and "Rhodes B" can coexist. Officially there are 7 boards with one side
  chosen each.
- recommendation: pick `players` distinct boards, then choose a side per board
  (mind determinism ordering of the extra RNG draws), or document the
  deviation.

### Discard pile contents are hidden from all players
- severity: minor
- category: quality
- location: game/seven-wonders-1/src/lib.rs:74
- finding: `PubState` exposes only `discard_count`, so the Halicarnassus
  player must `take N` blind by index. Discards are open information in the
  physical game (recoverable here only by replaying logs).
- recommendation: add `discard: Vec<Card>` (or names) to `PubState` and render
  it; index-based `take N` then becomes meaningful.

### Chosen trade deal re-validated by index into a recomputed list
- severity: nit
- category: quality
- location: game/seven-wonders-1/src/lib.rs:418
- finding: `resolve_deal` recomputes deals at execution time and
  `deals.get(idx).cloned().unwrap_or_default()` would silently build **for
  free** if the stored index were ever out of range. Verified unreachable
  today (the deal list is append-only between choice and execution), but the
  invariant is implicit and fragile.
- recommendation: store the chosen deal (`HashMap<i32, i32>`) inside
  `Action::Build` at `choose_deal` time instead of an index into a recomputed
  list.

### Unguarded player indexing in player_state / command_parser
- severity: nit
- category: consistency
- location: game/seven-wonders-1/src/lib.rs:984
- finding: `player_state` indexes `self.hands[player]` and `command_parser`
  indexes `self.actions[player]` (command.rs:39,54) without bounds checks.
  Sibling crates guard this (`sushi-go-2`, `category-5-2`). Only reachable if
  the framework passes an out-of-range player (upstream-guarded), so low risk.
- recommendation: match the sibling-crate defensive pattern.

### Finished-game scoring block copy-pasted six times
- severity: nit
- category: simplicity
- location: game/seven-wonders-1/src/lib.rs:1011
- finding: Six verbatim copies of the `is_finished()` → scores →
  `gen_placings` → `placings_log` → `CommandResponse` block (~90 lines) in
  `command()`.
- recommendation: extract a `fn respond(&self, logs) -> CommandResponse`
  helper.

### Military-conflict log uses raw player index
- severity: nit
- category: consistency
- location: game/seven-wonders-1/src/lib.rs:773
- finding: `" defeated player {} in military conflict"` interpolates a raw
  index instead of `N::Player(right)` like every other player reference in
  logs.
- recommendation: use `N::Player(right)`.

### start_hand() is dead-weight indirection
- severity: nit
- category: simplicity
- location: game/seven-wonders-1/src/lib.rs:177
- finding: `start_hand()` resets `self.actions` to `None`, already guaranteed
  by `execute_actions`/`start_round` at every call site, and returns an empty
  log vec.
- recommendation: inline or remove.

### Test coverage gaps in risky logic
- severity: minor
- category: quality
- location: game/seven-wonders-1/src/lib.rs:1206
- finding: Affordability/trade, science math, free-build, PlayFinalCard, and
  take-from-discard are tested, but nothing covers: military conflict
  resolution/token values, pass direction per age, `Bonus`/guild scoring
  (Haven, Strategists, Builders), Halicarnassus B VP (would have caught the
  first finding above), MimicGuild, multi-deal selection via `deal N`, seed
  determinism, or a full end-to-end game.
- recommendation: add tests for scoring paths and conflicts; a deterministic
  full-game replay test would catch most state-machine regressions.

### lib.rs is a 1,565-line grab-bag
- severity: nit
- category: simplicity
- location: game/seven-wonders-1/src/lib.rs:1
- finding: State machine, trading, scoring, resolver queue, `Gamer` impl, and
  360 lines of tests in one file. Scoring (`science_vp`, `score_science`,
  `player_vp`, `mimic_guild_vp`) and trading (`can_afford_cost`,
  `resolve_deal`, `pay_cost`) are cohesive enough to lift into
  `scoring.rs`/`trade.rs`.
- recommendation: optional split; no behavioral change.

Clean/verified: `card.rs` card DB (costs, chains, per-player-count tables,
guild set, all 14 cities' wonder stages checked against official 1st-edition
values), `render.rs`, `command.rs` (parser bounds re-validated downstream),
`rules()` uses `include_str!("../RULES.md")`, determinism via serializable
`GameRng`, `lib/cost` integration is used correctly, no panic path reachable
from crafted player input. Go source for seven-wonders is absent from the
snapshot, so Go-parity intent for the coin-payment, same-turn-trade,
sacrifice-to-discard, and both-wonder-sides findings rests on
`PORTING_NOTES.md` (which does not document them) — they are rules deviations
either way.

## game/alhambra-1

### take() mints duplicate cards (money-duplication exploit)
- severity: critical
- category: correctness
- location: game/alhambra-1/src/lib.rs:570
- finding: The availability pre-check (lib.rs:557-561) tests each requested
  card against `self.cards` independently with no multiplicity accounting, and
  the removal loop pushes the card into the player's hand even when it wasn't
  found in the market:
  `if let Some(pos) = self.cards.iter().position(|mc| mc == c) { self.cards.remove(pos); } self.boards[player].cards.push(*c);`.
  With one `B1` in the market, `take b1 b1` passes the ≤5 total check and
  gives the player two `B1`s — free money from crafted player input. (Severity
  elevated to critical by Lead: reachable game-state corruption / exploit.)
- recommendation: remove-first-then-push only on success and error when a
  requested card can't be found — mirror `spend()`'s clone-and-verify pattern
  (lib.rs:616-626) — or count requested duplicates against market multiplicity
  up front.

### place command indices diverge from rendered indices after a placement
- severity: major
- category: correctness
- location: game/alhambra-1/src/lib.rs:664
- finding: Placed tiles remain in `boards[player].place` as `Tile::empty()`
  sentinels, but `render_tile_set` (render.rs:362-368) numbers only non-empty
  tiles while `place()` (lib.rs:694) indexes the raw vec. Buy two tiles
  `[A, B]`, `place 1 a1` → `[Empty, B]`; render now shows "1: B", but
  `place 1 ...` resolves raw index 0 = the Empty tile, which is then inserted
  into the grid (passes `grid_is_valid` since Empty cells are skipped). The
  coord is permanently blocked, "placed Empty tile" is logged, grid bounds
  expand, and `remove`/`swap` can push the phantom Empty into `reserve`,
  shifting reserve indices too. Same flaw in `FinalPlace`.
- recommendation: in `place()`'s Place/FinalPlace arms resolve `n` against the
  non-empty subsequence (e.g. `.iter_mut().filter(|t| t.tile_type != Empty).nth(n)`),
  or compact the vec on placement instead of leaving sentinels.

### grid_longest_ext_wall terminates wall walk prematurely
- severity: major
- category: correctness
- location: game/alhambra-1/src/card.rs:516
- finding: The `break` after the first non-empty candidate fires
  unconditionally, not just when a continuing wall was found:
  `if !visited.contains_key(&next_wall) && grid_is_wall(g, next_wall) && !grid_is_internal_wall(g, next_wall) { wall += 1; ... } break;`.
  Candidates are ordered turn-at-corner / go-straight / turn-back, so a
  diagonal neighbor without the turning wall (extremely common on real boards)
  breaks the walk before the straight candidate is tried. Concrete undercount:
  T0=(0,0) Up wall, T1=(1,0) Up wall, diagonal T2=(1,-1) no Left wall — true
  longest external wall is 2, walk returns 1. Wall scores (1 pt/segment, every
  scoring round) are systematically undercounted. The two existing unit tests
  (lib.rs:1335-1366) don't contain this configuration. (Hand-traced, not
  machine-confirmed — review disallowed running tests.)
- recommendation: only `break` when a continuation was found (move `break`
  inside the `if`) so all three rotation candidates are tried; add a unit test
  with the diagonal-blocker configuration.

### Dirk excluded from final placings
- severity: minor
- category: correctness
- location: game/alhambra-1/src/lib.rs:843
- finding: Final placings/scores (and the 5 copies) and `status()`
  (lib.rs:975-991) are computed over `0..self.human_players` only. Under
  official 2-player rules Dirk can win the game; here the two humans are
  always placed 1st/2nd regardless of Dirk's majority total. Undocumented
  rules deviation (possibly a deliberate platform simplification).
- recommendation: either include Dirk in the final comparison (e.g. a "Dirk
  wins" log/placings entry) or document the deviation in RULES.md.

### Reduced money deck for 2-player games
- severity: minor
- category: correctness
- location: game/alhambra-1/src/card.rs:620
- finding: `build_deck` uses 2 copies of each value per currency (72 cards)
  for 2 players, 3 copies (108) otherwise. The official game uses the full
  108-card money deck regardless of player count; the 2-player variant only
  adds Dirk. Changes money-card odds and scoring-card timing. No Go source
  exists to confirm parity.
- recommendation: verify against the official rulebook; if deliberate,
  document it in RULES.md, otherwise drop the `players == 2` branch.

### Test coverage misses the riskiest logic
- severity: minor
- category: quality
- location: game/alhambra-1/src/lib.rs:1028
- finding: Present tests cover `score_type` tables, grid validity error cases,
  two longest-wall grids, coord parsing, and command/log smoke tests. Missing:
  `take` multiplicity, place-index-after-placement, wall walk with diagonal
  blockers, exact-payment extra action, overpay ending the turn, final-place
  distribution incl. ties, and 2-player/Dirk flows. The three major findings
  above would all have been caught by such tests.
- recommendation: add unit tests for the listed paths.

### is_finished() epilogue copy-pasted into six command arms
- severity: nit
- category: quality
- location: game/alhambra-1/src/lib.rs:838
- finding: The scores/placings block is duplicated identically (~12 lines × 6)
  in `command()`.
- recommendation: collapse the match to extract `(logs, remaining)` per arm,
  then run the finish-check once.

### Invariant-guarded panics in runtime paths
- severity: nit
- category: consistency
- location: game/alhambra-1/src/lib.rs:431
- finding: `Currency::ALL.iter().position(...).unwrap()` (lib.rs:431),
  `Card::parse(&s).unwrap()` inside the spend parser map (command.rs:142), and
  `panic!("Can only call rot_all on unit vector")` (card.rs:209). All
  genuinely invariant-guarded; none reachable from crafted input.
- recommendation: optional — `expect("...")` naming the invariant documents
  intent. Low priority.

### Gap-check loop range asymmetry (x inclusive, y exclusive)
- severity: nit
- category: correctness
- location: game/alhambra-1/src/card.rs:450
- finding: `for y in min.y..max.y` vs `min.x..=max.x`. Traced as provably
  harmless (border cells are always reachable by the outside flood walk via
  the `max.y + 1` ring), but the asymmetry reads like an off-by-one bug.
- recommendation: use `..=` on both ranges for clarity, or comment why the
  border row can't hide a gap.

### Debug formatting in user-facing messages
- severity: nit
- category: consistency
- location: game/alhambra-1/src/lib.rs:603
- finding: `"no tile available for {:?}"` renders "no tile available for
  Blue"; `" spent {} on {:?} tile"` (lib.rs:637-639) logs "spent R3 on Tower
  tile". `Currency::name()` exists and is used elsewhere.
- recommendation: use `currency.name()` / `tile_type.abbr().trim()` for
  display consistency.

### tile_counts duplicated between render.rs and PlayerBoard
- severity: nit
- category: quality
- location: game/alhambra-1/src/render.rs:69
- finding: `tile_counts` is verbatim-duplicated in render.rs and
  `PlayerBoard::tile_counts` (card.rs:601-609).
- recommendation: move the helper to a free function over `&Grid` in card.rs
  and use it from both.

### Grid column headers wrap past 26 columns
- severity: nit
- category: quality
- location: game/alhambra-1/src/render.rs:163
- finding: `((x - x_start) as u8 + b'a') as char` wraps into punctuation for
  grids wider than 26 columns. Practically unreachable (54 tiles max, typical
  boards ≤ ~15 wide).
- recommendation: none required; clamp or widen the alphabet if defensive
  polish is wanted.

### Vec-as-queue and HashMap-as-set in flood walks
- severity: nit
- category: quality
- location: game/alhambra-1/src/card.rs:383
- finding: Both flood walks in `grid_is_valid` use `walk_stack.remove(0)`
  (O(n) per pop) and `HashMap<Vect, bool>` as a set. Grids are tiny, so this
  is purely idiomatic-Rust polish.
- recommendation: `VecDeque::pop_front` and `HashSet<Vect>`.

Clean/verified: binaries (standard boilerplate), command parsers (byte-safe;
`Enum::exact` longest-match handles "b10" vs "B1" correctly; `Int::positive`
bounds all checked), `grid_is_valid` (wall matching, walk-from-fountain,
no-fountain, gap rules), `score_type` majority scoring (round-scoped slices,
tie grouping, truncation match official rules), scoring-card injection
positions (2nd and 4th fifths in draw order), turn/economy rules (exact
payment → extra action, overpay → turn ends, market refill timing, reserve
limits, end-game money distribution), determinism via `GameRng`, hidden-info
handling, `include_str!` doc conventions, contract test. No Go alhambra
source exists in the snapshot and there is no `PORTING_NOTES.md`, so the Dirk
and deck-size findings are judged against official rules only and may be
deliberate undocumented choices.

## game/splendor-2

### Prestige ties broken by most cards instead of fewest
- severity: minor
- category: correctness
- location: game/splendor-2/src/lib.rs:195
- finding: `placings()` feeds `vec![prestige, cards.len()]` into
  `gen_placings`, which sorts metric vectors descending — on equal prestige
  the player with **more** development cards places higher. Official Splendor:
  fewest development cards wins the tie. The unit test
  `test_placings_tie_broken_by_card_count` (lib.rs:1221-1234) locks in the
  inverted direction. Verified **Go-parity**, not a fresh bug:
  `brdgme-go/splendor_1/game.go` `Placings()` uses the identical metric and
  `brdgme-go/brdgme/placings.go` sorts reverse.
- recommendation: either negate the metric (`-(cards.len() as i32)`) and fix
  the test, accepting divergence from Go; or keep Go-parity and document the
  deviation explicitly in the `placings()` doc comment.

### take() action layer never validates that requested tokens are gems
- severity: minor
- category: correctness
- location: game/splendor-2/src/lib.rs:293
- finding: `take(0, &[Resource::Gold, Resource::Gold])` would succeed (bank
  gold = 5 ≥ 4); gold is excluded only by the parser
  (`tokens_parser(false)`, command.rs:172-188). Not reachable through
  `Gamer::command` (the parser is the only producer of `Command::Take`), so
  defense-in-depth only. Also Go-parity.
- recommendation: add `if tokens.iter().any(|t| !GEMS.contains(t)) { return Err(...) }`
  in `take()` so the action layer enforces the invariant regardless of caller.

### Local cost.rs vs lib/cost: consolidation assessment
- severity: minor
- category: dependencies
- location: game/splendor-2/src/cost.rs:1
- finding: The prior conclusion that `lib/cost` is a semantic **superset** of
  this module is **not quite correct**. (a) Semantics: `new`, `add`, `inv`,
  `sub`, `sum`, `Default`, and the `can_afford` method are equivalent, and
  `from_resources(&[Resource])` ≡ lib's `from_keys`. **But lib/cost has no
  `get(r)`/`set(r, v)`**, which splendor uses pervasively (~50 call sites:
  `pay` arithmetic lib.rs:268-285, reserve gold lib.rs:436-442, `start`
  lib.rs:544-548, render.rs:52,216-296, most tests). lib/cost also lacks
  splendor's free `can_afford(a, c)` gold-joker function (cost.rs:79-87) —
  game-specific (it is `splendor_1/amount.go`, not `libcost`), should stay in
  splendor regardless. (b) Serde: **safe** — both are
  `pub struct Cost(pub HashMap<Resource, i32>)` newtypes serializing
  identically (`{"Diamond": 3, ...}`); no persisted-game breakage. (c)
  Invasiveness: 4 source files touch `Cost` (lib.rs ~45 sites incl. tests,
  render.rs ~12, player_board.rs ~5, card.rs only the `cost!` macro, which
  works unchanged); migration is mechanical *if* `get`/`set` are first added
  to lib/cost.
- recommendation: add `get`/`set` to `lib/cost` (trivial generic methods, also
  useful for seven-wonders-1, the existing consumer), then replace
  `splendor-2/src/cost.rs` with `pub type Cost = brdgme_cost::Cost<Resource>;`
  plus the retained gold-joker `can_afford` next to
  `PlayerBoard::can_afford`. Add a serde round-trip test of a serialized
  `Game` to lock compatibility. Low-risk, moderate diff.

### reserve parser offers row-3 (own reserve) locations
- severity: nit
- category: quality
- location: game/splendor-2/src/lib.rs:118
- finding: `reserve_parser` reuses `loc_parser`, which includes the player's
  row-3 reserve locations when the reserve is non-empty — `reserve A4` parses
  and is only rejected at the action layer, and `A4` appears in reserve
  autocomplete suggestions. The test comment at lib.rs:1061-1063 claims the
  opposite ("the loc parser never offers row 3 as a `reserve` target either")
  — factually wrong given command.rs:129.
- recommendation: filter row-3 choices out of the reserve parser, or fix the
  stale comment.

### is_finished() epilogue copy-pasted into five command arms
- severity: nit
- category: consistency
- location: game/splendor-2/src/lib.rs:634
- finding: The scores/placings-log block is duplicated verbatim (~50 lines
  total) in all five `command()` match arms.
- recommendation: extract a helper, or compute it once after action dispatch.

### Typo in user-facing error message
- severity: nit
- category: quality
- location: game/splendor-2/src/lib.rs:326
- finding: "there aren't enough tokens **remaning** to take that". Go-parity
  typo (`take_command.go` has the same misspelling), but fixing a user-visible
  string is protocol-safe.
- recommendation: fix the spelling.

### .expect() in visit_phase auto-visit
- severity: nit
- category: quality
- location: game/splendor-2/src/lib.rs:230
- finding: `.expect("invariant: auto-visit must always succeed")`. Verified
  genuinely unreachable (phase just set, player is current, index from valid
  range, `ended` can't flip mid-chain). Go panics identically. Acceptable,
  but technically a panicking call in a runtime path.
- recommendation: optional — `unwrap_or_else` returning the error as logs
  would fully satisfy the no-panic convention.

Clean/verified: `card.rs` (counts tested; representative cards/nobles
spot-checked against physical Splendor), `player_board.rs`, `render.rs`
(hidden-info handling correct), `command.rs` (apart from the nit above),
binaries (standard boilerplate), `include_str!` doc conventions, determinism,
all indexing bounds-checked, no panic path reachable from crafted player
input, no arithmetic over/underflow reachable.

Documented Go-parity rules deviations (cross-references, not findings —
verified against `brdgme-go/splendor_1/`): no blind reserve from deck tops; no
partial takes when the bank is low (lib.rs:299,336 — officially 1-2 different
tokens may be taken when 3 types aren't available; can't soft-lock);
discard-down may overshoot below 10 (lib.rs:482); `visit()` deliberately
doesn't re-check noble affordability (lib.rs:493-496, documented inline and
locked in by test). These are inherited Go behavior; if any are to be fixed
they should be fixed deliberately with the divergence documented.
