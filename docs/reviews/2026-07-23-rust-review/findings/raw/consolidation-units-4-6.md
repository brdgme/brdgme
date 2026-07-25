# Consolidation notes: units 4-6 (games-batch-b/c/d)

Sources: findings/games-batch-b.md, games-batch-c.md, games-batch-d.md plus
their verification reports. F-IDs are per the verification reports (each unit
numbers its own F1..Fn).

## Unit 4: games-batch-b (seven-wonders-1, alhambra-1, splendor-2)

### Tallies
- Verification-corrected: 1 critical / 5 major / 13 minor / 16 nit (35 findings).
- Original curated tally: 1 critical / 5 major / 12 minor / 17 nit. Difference:
  F9 upgraded nit -> minor.
- Rejected findings: none.

### Verification
- Verdicts: 31 CONFIRMED, 4 ADJUSTED (F9, F14, F18, F21), 0 REJECTED,
  0 UNVERIFIABLE.
- F9 (7w deal re-validated by index): UPGRADED nit -> minor. Original's
  "verified unreachable" claim is false - mid-execution builds mutate neighbor
  goods and can_afford_perm's early return (lib/cost:181-184) can reorder or
  shrink the recomputed deal list, so the stored index can pay the wrong
  neighbor or go out of range and build free via unwrap_or_default().
- F14 (7w test gaps): MimicGuild IS tested (test_card_mimic_guild); remove
  from gap list; other gaps real. Minor stands.
- F18 (alhambra wall walk): the concrete example is HashMap-iteration-order
  dependent, so wall scoring is undercounted AND nondeterministic across runs -
  a strengthening. Major stands.
- F21 (alhambra test gaps): final-place has a single-currency smoke test; the
  real gap is the tie path. Minor stands.
- No recommendations flagged invalid in this unit.
- Evidence strengthenings: F6 contradicts the crate's own RULES.md; F16
  reachability rests on take using CardParser vs spend's clone-and-verify;
  F25 has three more {:?} sites.

### Headlines
Critical:
- F16 alhambra take() mints duplicate cards - availability pre-check has no
  multiplicity accounting and the push is unconditional on position() miss;
  `take b1 b1` with one B1 in market duplicates money. Money-duplication
  exploit from crafted input. game/alhambra-1/src/lib.rs:570.

Notable majors (all 5):
- F2 7w DrawDiscard resolver permanent soft-lock - resolver fires whenever
  discard non-empty but take_from_discard rejects owned cards and the parser
  offers only `take`; turn pinned forever. PORTING_NOTES claims a filter that
  does not exist. game/seven-wonders-1/src/lib.rs:410.
- F1 7w Halicarnassus B wonder-stage VP never scored - player_vp has no
  DrawDiscard arm; 3 VP lost per game. game/seven-wonders-1/src/lib.rs:706.
- F3 7w auto-discarded 7th card pays 3 coins - end_hand pays DISCARD_COINS
  contrary to official rules, up to 9 coins/player/game inflation.
  game/seven-wonders-1/src/lib.rs:192.
- F17 alhambra place indices diverge after placement - Empty sentinels left
  in the vec; render numbers non-empty tiles, place() indexes raw vec; can
  insert phantom Empty tiles, permanently block coords, pollute reserve.
  game/alhambra-1/src/lib.rs:664.
- F18 alhambra grid_longest_ext_wall premature break - unconditional break
  after first candidate undercounts wall scores nondeterministically.
  game/alhambra-1/src/card.rs:516.

### Unit state
Three mid-size ports (7 Wonders, Alhambra, Splendor). Alhambra carries the
unit's worst defects (state-corruption exploit plus two major scoring/index
bugs); seven-wonders has a soft-lock, a scoring omission, and several
official-rules deviations undocumented in PORTING_NOTES; splendor-2 is clean
apart from Go-parity quirks. Original review was highly accurate - all 35
locations checked out; the only verdict flip was F9's unreachability analysis.

### Theme evidence
- Request-reachable panics / state corruption: F16 (exploit, not panic but
  request-reachable corruption); F9 (free build via unwrap_or_default).
- Go-port parity vs official rules: F29 (tie broken by MOST cards, verified
  Go-parity), F3/F4/F6/F7 (7w deviations, no Go source, undocumented in
  PORTING_NOTES), F19/F20 (alhambra Dirk exclusion, 72-card 2p deck), plus
  batch-b's documented splendor Go-parity deviations (cross-referenced, not
  findings).
- Boilerplate duplication: F11 (epilogue x6 7w), F22 (x6 alhambra), F33 (x5
  splendor) - is_finished/placings block copy-paste is universal.
- Duplicated dependencies / local reimplementation: F31 (splendor local
  cost.rs vs lib/cost; lib/cost lacks get/set, migration mechanical once
  added).
- Nondeterminism from HashMap iteration: F18 (Grid = HashMap makes wall
  scoring run-dependent).
- Test coverage gaps on riskiest logic: F14, F21 (majors in both crates would
  have been caught).
- Hidden info / rendering inconsistency: F8 (discard hidden though open info
  officially).
- Defensive-guard inconsistency across sibling crates: F10 (unguarded player
  indexing where sushi-go-2/category-5-2 guard).

### Discussion candidates
- F29 splendor prestige tie broken by MOST cards: keep Go parity (test locks
  it in) or match official fewest-cards rule - explicit adjudication needed.
- F3/F6/F7 seven-wonders deviations (discard coins, sacrifice-to-discard, both
  wonder sides dealt): no Go source in snapshot; fix vs document-as-quirk is a
  product call. F7's fix also perturbs RNG draw ordering (determinism).
- F19 alhambra Dirk excluded from placings: include Dirk in final comparison
  (platform placings semantics for a non-player) or document as
  simplification.
- F20 alhambra 2p 72-card deck: official uses full 108; deliberate variant or
  bug - needs rulebook confirmation.
- F5 7w MimicGuild cannot copy Science guilds: extending requires a marginal
  science-VP evaluation design.
- F8 7w hidden discard pile: exposing contents changes PubState shape -
  UI/protocol decision.

## Unit 5: games-batch-c (texas-holdem-2, acquire-1, cathedral-2, sushizock-2)

### Tallies
- Verification-corrected: 0 critical / 4 major / 14 minor / 16 nit (34 findings).
- Original curated tally: 0 critical / 4 major / 15 minor / 15 nit. Difference:
  F23 downgraded minor -> nit.
- Rejected findings: none.

### Verification
- Verdicts: 33 CONFIRMED, 1 ADJUSTED (F23), 0 REJECTED, 0 UNVERIFIABLE.
- F23 (cathedral flood-fill traversable cathedral): DOWNGRADED minor -> nit.
  Code facts and Go parity all verify, but the "undocumented" premise is
  refuted - the crate's own RULES.md documents the cathedral as a capturable
  piece identity inside enclosures, matching the implementation. Residual:
  add a walk-site comment referencing RULES.md. (This set the "documented
  in-crate wins" precedent later applied to unit 6's F36.)
- No recommendations flagged invalid.
- Evidence strengthenings: F8 also contradicts RULES.md:153-154 ("result
  (1-6)"); F25's panic concretely reachable because the request harness
  (requester/gamer.rs:130) forwards player index unvalidated; F29 pinned:
  no profile overrides overflow-checks so all default dev/test builds panic;
  F14+F15 compound (mass redraw can drain the bag and trigger premature end).

### Headlines
Critical: none.

Majors (all 4):
- F7 acquire player_counts() excludes 6 players - `(2..6).collect()` vs
  MAX_PLAYERS=6; platform can never offer 6p Acquire, the headline count.
  game/acquire-1/src/lib.rs:313.
- F8 acquire 2p dummy die roll is 1..=5, never 6 - contradicts both the start
  log and RULES.md ("result (1-6)"); systematically weakens the dummy.
  game/acquire-1/src/lib.rs:902.
- F22 cathedral Box::leak per parser construction - loc_parser leaks 100
  strings on EVERY command()/command_spec() call; unbounded traffic-driven
  memory leak in the long-running service; "one-time" comment false.
  game/cathedral-2/src/command.rs:26.
- F29 sushizock steal n = i32::MIN overflow - `len as i32 - n` with
  Int::any() parser input overflows; panics in all default dev/test builds,
  release safe only by wrap luck. Port-introduced (Go ints wrap silently).
  game/sushizock-2/src/lib.rs:460 and :502.

Other notable: F25 cathedral pieces() panics on out-of-range player index,
concretely reachable via the unvalidated harness (minor); F14/F15 acquire
redraw discards temporarily-unplayable tiles and bag exhaustion ends the game
mid-turn, compounding (minor).

### Unit state
Texas-holdem-2 and sushizock-2 are faithful line-by-line ports with only small
divergences; cathedral-2 is clean logic-wise but carries the leak major;
acquire-1 (fresh implementation, no Go source) contributes half the findings
including both rules-facing majors plus a dead stats subsystem. Original
review was highly accurate - one verdict flip, on a documentation premise
rather than code.

### Theme evidence
- Request-reachable panics: F29 (overflow panic from crafted steal), F25
  (pieces() panic via unvalidated player index), F9 (panic! in pay_bonuses,
  merge/end path), F10 (expect() cluster on HashMap keys incl. render path,
  real concern for legacy deserialized states), F26 (to_key overflow, latent).
- Bot-slot/player-count validation: F7 (player_counts vs MAX_PLAYERS
  mismatch), F2 (texas-holdem caps at 8 vs Go's 9, undocumented).
- Go-port parity vs official rules: F1 (raise parser min diverges from Go
  AND the preservation comment is factually wrong), F23 (cathedral flood-fill
  is Go parity and RULES.md-documented), preserved-quirk verification done
  against Go for texas-holdem/cathedral/sushizock.
- Unmaintained/unused dependencies: F16 (unused thiserror in acquire), F27
  (unused rand in cathedral-2, possibly uniform boilerplate).
- Error-swallowing / silent inconsistency: F11 ("Trades" stat reports merges),
  F12 (stats tracked but never surfaced - write-only dead code hiding F11).
- Boilerplate duplication: F6 (placings-log x5 texas-holdem), F34 (take/steal
  near-verbatim duplicates - the i32::MIN bug had to be spotted twice).
- Missing placings-log consistency: F30 (sushizock roll-path finish omits the
  placings log other arms emit).
- lib/game suggest Many-ignores-max cross-ref: F31 (sushizock roll is the
  most user-visible victim; fix tracked in the lib unit).
- Nondeterminism from HashSet iteration: F21 (found-parser corp order).
- Performance from cloning: F20 (full-game deep clone per command_parser
  build for can_end).

### Discussion candidates
- F13 acquire random start player vs initial-tile-draw rule: simplification
  vs rulebook fidelity; RULES.md silent.
- F14 acquire full-hand redraw discards temporarily-unplayable tiles: edition
  choice (later Hasbro vs classic 3M/AH) needs confirming; also interacts
  with F15.
- F15 acquire bag exhaustion ends game mid-turn: edition-dependent behavior;
  needs an explicit decision, no Go port to match.
- F23 cathedral flood-fill: decide whether cathedral-as-non-wall is preserved
  defect #5 (comment it) or should match official enclosure rules.
- F2 texas-holdem 8 vs 9 max players: deliberate cap or port slip.
- F12 acquire stats subsystem: wire into status() or delete - product call.

## Unit 6: games-batch-d (lords-of-vegas-1, jaipur-2, sushi-go-2, modern-art-2)

### Tallies
- Verification-corrected: 1 critical / 6 major / 16 minor / 22 nit (45
  surviving findings).
- Original per-finding fields: 1 critical / 8 major / 16 minor / 21 nit over
  46. Note: the findings file's own summary line ("1 critical, 9 major, 14
  minor, 22 nit") miscounts its own blocks.
- Rejected, excluded from tally: F13 (jaipur "8 camels vs official 11") -
  confirmed rejected by the verification report. start_round conjures the 3
  market camels out of thin air (lib.rs:223), so 8 deck + 3 market = 11 in
  play and the deck ends at 40 exactly as official. The finding's recommended
  fix (Camel => 11) would itself have introduced a bug (14 camels).

### Verification
- Verdicts: 38 CONFIRMED, 7 ADJUSTED (F4, F7, F15, F26, F31, F36, F45),
  1 REJECTED (F13), 0 UNVERIFIABLE.
- F4 (LoV render underflow): UPGRADED minor -> major. CASINO_TILES branch is
  reachable in ordinary 5-6p play (build() has no supply/colour limit), so
  every render panics in debug builds.
- F15 (jaipur round starter): DOWNGRADED major -> minor on evidence basis
  only - code claims all verified but the "loser starts" rule has no in-repo
  source (RULES.md is a stub). Restore major if the rulebook quote is
  confirmed at adjudication.
- F26 (sushi-go pudding tiebreak): DOWNGRADED minor -> nit, premise refuted -
  the official rulebook DOES break score ties by most puddings; the
  implementation is correct. Residual: RULES.md omits the tiebreak.
- F36 (modern-art all-cards payout): DOWNGRADED major -> minor - RULES.md
  explicitly documents cumulative payout "even if the artist didn't place
  this round" (batch-c precedent). Kept minor, flagged for cross-unit
  adjudication since the doc may canonize the Go port's deviation.
- F7, F31, F45: detail-level adjustments, severities stand.
- Invalid recommendations flagged: F13's recommendation ("Change Good::Camel
  => 8 to => 11 ... would create 14 camels - a bug"); F45's quoted expression
  "dropped `> 0` and would not compile" (substance stands).
- Evidence strengthenings: F2 nondeterminism also affects the NUMBER of RNG
  draws via the recursive re-tie pass; F37's counts map is seeded with zeros,
  closing the original's argument gap; F34/F35 share one missing invariant
  and merit a combined fix; F37's phantom values compound F36's payouts.

### Headlines
Critical:
- F34 modern-art infinite busy-loop when all hands empty after a settle -
  skip loop has no all-hands-empty guard and pushes a log per iteration;
  legally reachable in round 4 (0 cards dealt, no 5th-card trigger);
  hang + unbounded log growth. Go-inherited. game/modern-art-2/src/lib.rs:452.

Notable majors (5 of 6):
- F35 modern-art round 4 can start on an empty-handed player - end_round has
  no empty-hand skip; deadlock (no parser output, no pass). Shares one
  missing invariant with F34; combined fix. game/modern-art-2/src/lib.rs:368.
- F37 modern-art zero-card artists ranked and awarded $20/$10 - counts map
  seeded with zeros plus -1 sentinel; reachable (5-0-0-0-0 round); NOT
  documented in RULES.md (unlike F36); inflates all later rounds' values.
  game/modern-art-2/src/lib.rs:318.
- F14 jaipur no bonus token for 6/7-card sales - bonus map keys exactly
  3/4/5; contradicted in-repo by render.rs ("5 or more") and DATA_DOCS.md
  ("3+"). Genuine scoring defect. game/jaipur-2/src/lib.rs:521.
- F1 LoV unimplemented!() in command dispatch - five arms one parser-wiring
  line away from panicking on valid commands. game/lords-of-vegas-1/src/lib.rs:182-186.
- F4 LoV renderer usize underflow - build() enforces no dice/token/casino-tile
  supply; CASINO_TILES underflow reachable in ordinary 5-6p play.
  game/lords-of-vegas-1/src/render.rs:117 (also 80, 85).
(6th major: F2 LoV HashMap/HashSet iteration feeds boss-tie RNG - seeded
replay divergence, affects draw count too. game/lords-of-vegas-1/src/board.rs:248,278,314-344.)

### Unit state
Modern-art-2 is the problem crate: a legally reachable infinite loop, a
deadlock, and a scoring cluster all inherited verbatim from the Go port.
Lords-of-vegas-1 is an explicitly partial port whose gaps (panic macros,
missing supply enforcement, nondeterministic tie resolution) are structural;
jaipur-2 and sushi-go-2 are strong crates with mostly rules-adjudication
findings. Weakest-verified unit of the three: one rejected finding, one
refuted premise, and two majors moved in opposite directions.

### Theme evidence
- Request-reachable panics: F1 (unimplemented!() one line from reachable),
  F4 (render underflow reachable in ordinary play), F29-unit5-style latent
  panics F38 (unreachable!()/unchecked indexing, Deserialize-exposed), F5
  (parse_str/neighbours underflow from corrupt state), F45 (guarded unwrap).
- TOCTOU/determinism: F2 (HashMap/HashSet iteration order consumes seeded RNG
  differently across processes - breaks deterministic replay/audit).
- Go-port parity vs official rules: the entire modern-art cluster
  F34/F35/F36/F37/F43 is inherited verbatim from Go (line-cited); sushi-go
  F24 (pass direction) and F25 (pudding edge) likewise; cross-unit
  adjudication explicitly requested by the original review.
- Error-swallowing / silent behavior: F3 (boss-tie rerolls produce no log and
  disable undo silently), F18 (mixed-type sell silently coerced to first
  good), F20 (silent Diamond fallback).
- Bot-slot/player-count validation: F12 (LoV 2-6 vs official 2-4, external
  basis), F28 (dead (2,9) draw_count entry + silent fallback).
- Unmaintained dependencies: F6 (lazy_static vs std LazyLock/OnceLock), F8
  (serde_json runtime dep used only in tests).
- Boilerplate duplication: F21 (jaipur placings-log x2), F33 (sushi-go x2) -
  same epilogue-duplication pattern as units 4-5.
- Docs-vs-code divergence (recurring in this unit): F17 (RULES.md one-line
  stub), F39/F40 (modern-art RULES.md wrong/incomplete), F11 (casino colours
  vs RULES.md), F22 (rounds-remaining overstatement), F26 residual, F29
  (pudding hint false in 2p).
- Hidden-info inconsistency: F23 (renderer hides camel count PubState
  exposes exactly).
- Test quality: F27 (vacuous self-contradicting test).

### Discussion candidates
- Port parity vs official rules, modern-art cluster (the unit's big one):
  F36 (all-purchases payout - RULES.md documents it, but the doc may canonize
  a Go defect; material economy change), F37 (zero-card artists awarded -
  undocumented, but fixing diverges from Go), F43 (auctioneer-favorable
  tie-break; edition rule unconfirmed). F34/F35 fixes also require deciding
  round-4 end semantics (when does the round/game end if nobody can play).
- F15 jaipur next-round starter: needs the official rulebook quote confirmed
  to restore major; then define behavior on a full tie.
- F16 jaipur camel token in tie-break: official component distinction is
  external-rules based; decide and document.
- F24 sushi-go round-2 right pass: documented self-consistent deviation -
  close as deliberate or fix for fidelity.
- F25 sushi-go 2p all-tied pudding split and dummy participation:
  rules-ambiguous; decide and document.
- F12 LoV player counts 2-6 vs official 2-4: deliberate house extension?
- F14-unit6 has no design question (mechanical clamp), but its regression
  test needs a 6+ sale scenario defined.
