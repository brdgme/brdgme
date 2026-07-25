# games-batch-f findings (2026-07-24)

Unit 8 of 13. Crates reviewed in the snapshot worktree
(`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`): zombie-dice-2, battleship-2,
for-sale-2, category-5-2, greed-2, farkle-2, tic-tac-toe-2, no-thanks-2,
liars-dice-2 (~8.0k LOC). All except tic-tac-toe-2 are ports of `brdgme-go`
originals, which were read and compared; tic-tac-toe-2 has no Go source and was
judged against standard tic-tac-toe rules. Raw worker dumps:
`findings/raw/games-batch-f-<crate>.md`.

**Totals: 58 findings — 0 critical, 2 major, 22 minor, 34 nit.**

Unit-wide notes (stated once, not repeated per crate):

- Every crate in this batch declares binary-only deps (`brdgme_cmd`,
  `brdgme_fuzz`, `tokio`) as library `[dependencies]` — instance of the known
  systemic issue, tracked in `findings/dependencies.md`. Not re-flagged below.
- Every crate's `Game`/`PubState` is fully `pub` + `Deserialize` with no
  validation, so crafted stored state (not player command input) can panic the
  HTTP request in various index/drain paths. Per-crate instances are flagged
  below only where notable; the systemic fix belongs in the requester/serde
  layer (see also tic-tac-toe-2's unbounded-`players` finding).
- Go-preserved rules deviations are cross-references, not regressions; several
  below join the project-wide "port parity vs official rules" decision list
  (modern-art-2 payout, splendor-2 tie-break, etc.).
- Overall this batch is in good shape: no critical findings, and no
  panic/unwrap path reachable from crafted *command* input in any of the nine
  crates. The two majors are both hidden-information leaks in `pub_state`.

## zombie-dice-2

Faithful, well-tested port of `zombie_dice_1`; dice tables, bust/keep/win and
rolloff logic verified against Go and the official SJG rules.

### Cup draw order leaked to all players in PubState
- severity: major
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:194 (`pub cup: Vec<Dice>`, doc says "in draw order"), populated at game/zombie-dice-2/src/lib.rs:443
- finding: `pub_state()` exposes the full shuffled cup **in draw order** to every player. In the physical game dice are drawn from the cup "without looking": the composition of remaining dice is deducible public info, but the *order* in which colours come out is hidden. Because the cup is only shuffled at turn start / refill, any API/bot client can read the exact colours of the next dice to be drawn (e.g. "the next two draws are red") and make roll/keep decisions with perfect foreknowledge — a real hidden-information leak that materially changes the game. The Go original returned `nil` from `PubState()`, so nothing leaked; the leak is new in the Rust port. `DATA_DOCS.md` even claims "Zombie Dice has no hidden information per player", which is inaccurate.
- recommendation: don't serialize cup order in `PubState`. Either expose only per-colour counts (matching `render_cup`, render.rs:51-73, which already renders only counts), or sort/canonicalize the cup vector in `pub_state()` so order carries no information, and fix the DATA_DOCS.md claim.

### Cup refill returns shotgun dice to the cup (Go quirk, deviates from official rules)
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:242-250 (`take_dice` refill branch)
- finding: When the cup runs short, ALL kept dice (brains **and** shotguns) are returned to the cup. The official SJG rulebook says only brain dice go back; shotgun dice stay out for the rest of the turn. Returning shotguns lets the same physical die shotgun the player again within one turn. Faithful port of Go `TakeDice`; RULES.md:31 documents the ported behaviour. Cross-reference for the port-parity decision.
- recommendation: decide deliberately: keep the Go behaviour (note the official-rules deviation in RULES.md) or return only `Face::Brain` kept dice to the cup.

### Tiebreak rolloff state not exposed in PubState / render
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:185-207 (`PubState` fields), game/zombie-dice-2/src/lib.rs:438-455 (`pub_state()`), game/zombie-dice-2/src/render.rs:92-145
- finding: `Game::roll_off_players` (lib.rs:173) is never copied into `PubState` and `render()` shows nothing about an active rolloff; the only signal is the transient "tie breaker round!" log. A client rendering mid-rolloff cannot tell a rolloff is active or which players participate (non-participants' turns are silently skipped). Go had no structured pub state, so the Rust `PubState` is new and simply incomplete.
- recommendation: add `roll_off_players` to `PubState`, populate it in `pub_state()`, surface it in `render()` and DATA_DOCS.md.

### Panic paths on inconsistent deserialized state (`drain(..n)`, `scores` indexing, `% players`)
- severity: minor
- category: correctness
- location: game/zombie-dice-2/src/lib.rs:251 (`self.cup.drain(..n)`), :368 (`self.scores[self.current_turn]`), :384-390 (`leaders()` indexes `scores[p]`), :267 (`% self.players` panics on `players == 0`), game/zombie-dice-2/src/render.rs:132-139
- finding: These trust invariants `start()` establishes (`scores.len() == players`, cup/kept/current_roll partition of the 13 dice, `players > 0`). Unreachable from player *commands*, but `Game` derives `Deserialize` with all-pub fields and no validation, so malformed stored state panics the HTTP request. Go panics identically (`g.Cup[:n]`). Common shape across the ported crates.
- recommendation: if state hardening is done systemically, clamp `n` to `cup.len()` after refill and validate lengths/turn on load; otherwise acceptable given the invariant.

### Unbounded recursion chain on repeated busts (theoretical)
- severity: nit
- category: quality
- location: game/zombie-dice-2/src/lib.rs:347 (`roll()` bust → `next_player()`), :288-291 (`next_player()` recursion), :262 (`start_turn()` → `roll()`)
- finding: `roll → next_player → start_turn → roll` recurses once per consecutive busted turn; an arbitrarily long run of consecutive busts grows the stack without bound (probability ~zero, RNG server-side, not adversarially steerable). Same shape as Go.
- recommendation: none required; convert to a loop if touched for other reasons.

### Rolloff tie announcement re-logged on every wrap while still tied
- severity: nit
- category: quality
- location: game/zombie-dice-2/src/lib.rs:276-286
- finding: each wrap to player 0 with the score still tied ≥13 re-emits the identical "tie breaker round!" log and reassigns `roll_off_players`, spamming the log during a long rolloff. Faithful to Go; cosmetic.
- recommendation: only announce when `roll_off_players` transitions from empty to non-empty.

### Duplicated finish-handling block in both `command()` arms
- severity: nit
- category: simplicity
- location: game/zombie-dice-2/src/lib.rs:483-499 (Roll arm) and :500-520 (Keep arm)
- finding: the two arms are byte-for-byte identical except for `player_roll` vs `keep` — ~15 duplicated lines (scores vec + `placings_log`) that must stay in sync.
- recommendation: match only to select the action, then run the shared finish/response code once.

## battleship-2

Clean, faithful port of `battleship_1`; placement/shooting/sunk/phase/win logic
and pub-state redaction all correct; 30+ unit tests plus the contract test.
Divergences from Go are documented improvements (unambiguous-prefix ship
parsing, deterministic direction parsing) apart from the dropped bounds check
below.

### shoot() drops Go's bounds validation; direct indexing can panic on out-of-range Loc
- severity: minor
- category: correctness
- location: game/battleship-2/src/lib.rs:308-348 (indexing at :317, :328, :332)
- finding: Go's `Shoot` validated `IsValidLocation(y, x)` and errored on off-board shots. The port omits the check and indexes `self.boards[op][y][x]` directly. `Loc { y, x }` has public fields and no validity invariant, so any non-parser caller can panic the process. Unreachable in the HTTP flow (`loc` only comes from `Enum::exact(all_locations())`, command.rs:24), so defense-in-depth, not an exploitable input path. (`place_ship` does validate bounds, lib.rs:277-282.)
- recommendation: restore Go parity with a bounds check at the top of `shoot` returning `GameError::invalid_input`, or make `Loc` construction validated with private fields.

### Indexing trusts `players`/Vec lengths; inconsistent with defensive `.get()` elsewhere
- severity: minor
- category: correctness
- location: game/battleship-2/src/lib.rs:378-380 (`is_finished` via :354), :423-425 (`status`), :387-389 (`placings`), :270/:283/:290 (`place_ship`)
- finding: several methods index `boards[p]`/`left_to_place[p]` for `p in 0..self.players`, panicking if a deserialized `Game` ever has lengths out of sync, while `can_place` (:249), `player_state` (:460-461) and `place_parser` (command.rs:51) use `.get()` defensively — the crate is internally inconsistent about the invariant. Not reachable from crafted command input.
- recommendation: pick one strategy: validate state once on load, or use `.get()` consistently as `can_place` already does.

### expect("cell is a ship") in shoot sunk-detection branch
- severity: nit
- category: quality
- location: game/battleship-2/src/lib.rs:331
- finding: provably unreachable (the `Hit`/`Miss`/`Empty` arms above leave only ship variants), but an `expect` on the hot path turns any future `Cell` variant addition into a request-killing panic.
- recommendation: bind the ship via the match pattern or `match ship_cell.to_ship()` with an erroring `None` arm.

### Ship::all() and Direction::all() have inconsistent return types
- severity: nit
- category: consistency
- location: game/battleship-2/src/lib.rs:64 vs :118
- finding: `Ship::all() -> &'static [Ship]` but `Direction::all() -> Vec<Direction>`; the allocation is gratuitous and the asymmetry untidy.
- recommendation: make both return `&'static [T]`.

### Hit-count helpers return i32 for non-negative counts
- severity: nit
- category: quality
- location: game/battleship-2/src/lib.rs:350, :362
- finding: `player_hits_remaining`/`player_ship_hits_remaining` return `i32` for values that can never be negative (kept to feed `gen_placings`, matching Go).
- recommendation: optional — return `usize`/`u32` and cast at the placings call site.

## for-sale-2

Very faithful port of `for_sale_1`; core auction/selling logic, turn handling
and scoring correct for the ported ruleset; no panic reachable from command
input. Known consumer of the lib/game `doc_int` min:None help-rendering bug
(command.rs:43-46 `Int { min: None, ... }`) — cross-reference only, fixed at
the lib level.

### Hidden-info leak: selling-phase plays exposed via PubState.bids
- severity: major
- category: correctness
- location: game/for-sale-2/src/lib.rs:258 (`play` stores into `bids`), surfaced at game/for-sale-2/src/lib.rs:411-412 (`pub_state` clones `bids`/`finished_bidding` verbatim)
- finding: the selling phase is simultaneous secret selection ("Each player secretly selects one building to play", RULES.md:23), but `play()` records the played building in `self.bids[player]` and `pub_state()` exposes the full `bids` vector to everyone. Any bot/API client reading `pub_state` during the selling phase sees exactly which building each already-played opponent chose before picking its own. The HTML renderer (render.rs:101-109) only shows the viewer's own play, so the leak is invisible in the UI but present in the JSON contract. Cross-reference: Go has the identical leak (`ToPubState` includes `Bids`) — a faithfully preserved Go flaw, but a genuine hidden-information violation either way.
- recommendation: redact `bids` while `phase == Selling` in `pub_state()` (zero all entries — pub_state is shared — and let each `player_state` re-add its own), or store selling-phase plays in a separate private field instead of reusing `bids`.

### Passing pays floor(bid/2); official rules round the payment up (Go quirk)
- severity: minor
- category: correctness
- location: game/for-sale-2/src/lib.rs:233
- finding: `let half_bid = self.bids[player] / 2;` floors, so passing on a bid of 5 costs 2; official For Sale rules round the payment **up** (pays 3). Go does the same integer division and RULES.md:17 honestly documents the implemented behaviour. Cross-reference for the port-parity decision.
- recommendation: if rules fidelity is desired: `.div_ceil(2)` plus a RULES.md update; otherwise leave and keep the cross-reference.

### Deck/chip setup deviates from official For Sale (Go quirk)
- severity: minor
- category: correctness
- location: game/for-sale-2/src/lib.rs:19 (STARTING_CHIPS=15 flat), :85-91 (20 buildings 1..=20; 20 cheques {0,0,3..=20}), :375-381 (3p removes 2 cards per deck only)
- finding: official: 30 property cards (1-30), 30 cheques (two each of 0 and 2..=15), 3p removes 6 per deck, 4p removes 2 per deck, chips scale with player count. The port (like Go) uses 20 buildings, 20 cheques with no 2s, removes 2 per deck only for 3p, and gives a flat 15 chips. RULES.md documents the ported variant. Cross-reference only.
- recommendation: none required for parity; keep documented as a deliberate Go-compatible variant.

### RULES.md cheque deck description is factually wrong
- severity: minor
- category: quality
- location: game/for-sale-2/RULES.md:7-8
- finding: "30 cheques: two 0s, then 2..=20" matches neither the code (20 cheques, values {0, 0, 3..=20} — no 2s, lib.rs:89-91) nor official rules. Also RULES.md:31 "Ties share a place" contradicts the chips tie-break in `placings()` (lib.rs:332-337).
- recommendation: fix to "20 cheques: two 0s, then 3..=20" and amend the tie sentence to mention the chips tie-break.

### End-of-game "scores" log shows cheque totals only, not final scores
- severity: minor
- category: correctness
- location: game/for-sale-2/src/lib.rs:118 (only `deck_value(&self.cheques[p])` rendered), :122-126
- finding: the Finished-branch log labels the table "The scores are" but shows only each player's cheque sum, omitting leftover chips that are part of the final score (`player_points`, lib.rs:328-330). This table is always emitted on game end, then `command()` appends a correct `placings_log` — players see two contradictory "score" summaries. Go is identical (cross-reference).
- recommendation: render `player_points(p)` (or cheques + chips as separate columns) in the finished table.

### Phase inferred from deck sizes via SELL_THRESHOLD magic constant
- severity: minor
- category: quality
- location: game/for-sale-2/src/lib.rs:20 (`SELL_THRESHOLD: usize = 18`), :94-104 (`current_phase`)
- finding: whether `open_cards` holds buildings or cheques is inferred by `cheque_deck.len() >= 18` — a magic number that works only because the first selling draw always drops the deck below 18. The `Phase` enum exists (lib.rs:22-29) but is never stored. Correct today, silently breaks if deck sizes/player counts change. `status()` (lib.rs:386-401) separately re-implements the finished predicate from raw deck emptiness — two sources of truth.
- recommendation: store `phase: Phase` in `Game` (serde-defaulted for migration), transition it explicitly in `start_round`, and have `status()` delegate to `current_phase()`.

### Panic-on-empty-deck paths reachable only from corrupt/deserialised state
- severity: nit
- category: correctness
- location: game/for-sale-2/src/lib.rs:133, :144 (`split_off(len - n)`), :266, :282 (`open_cards.remove(0)`), :153 (`hands[p][0]` during autoplay)
- finding: all unreachable through legal play (deck sizes are multiples of player count by construction), but a corrupted/migrated state blob would panic the HTTP request rather than error.
- recommendation: optional hardening: `hands[p].first()` in the autoplay and graceful no-op/error on short decks.

### Selling autoplay keys off only player 0's hand size
- severity: nit
- category: quality
- location: game/for-sale-2/src/lib.rs:151
- finding: `self.hands.first().is_some_and(|h| h.len() == 1)` assumes all hands have equal size — true by construction, identical to Go; the invariant is implicit.
- recommendation: none needed; if touched, use `self.hands.iter().all(|h| h.len() == 1)`.

### Tie ranking diverges from Go GenPlacings (dense → standard competition)
- severity: nit
- category: consistency
- location: game/for-sale-2/src/lib.rs:332-337 and test at :792-807
- finding: Go `GenPlacings` used dense ranking ([1,1,2]); Rust `gen_placings` (lib/game/src/game.rs:154-179) uses standard competition ([1,1,3]), codified by this crate's test. Lib-level port-wide divergence, cross-referenced here for the lib/game unit; self-consistent within the crate.
- recommendation: track at the lib/game level; no per-crate action.

### render::highest_bid duplicates game logic with a different sentinel
- severity: nit
- category: simplicity
- location: game/for-sale-2/src/render.rs:40-50 vs game/for-sale-2/src/lib.rs:316-326
- finding: the renderer re-implements `highest_bid` over `PubState` with `best > 0` as the "no bid" test (game code uses -1); duplicated logic can drift.
- recommendation: accept the duplication (small, presentational) or expose a shared `Option`-returning helper.

### Helper methods unnecessarily pub; player_state indexes unchecked
- severity: nit
- category: consistency
- location: game/for-sale-2/src/lib.rs:131-345 (all helpers `pub`), :417-425 (`player_state` indexes `hands[player]` etc.)
- finding: crate-internal plumbing (`clear_bids`, `take_first_open_card`, `next_bidder`, `highest_bid`, `deck_value`, `start_*_round`) is exposed `pub`; `player_state()` panics on out-of-range player index (same pattern across game crates).
- recommendation: trim visibility to `pub(crate)`/private where the bins don't need it.

## category-5-2

Faithful port of `category_5_1`; core 6 nimmt! logic (deck, bullheads, row
placement, 6th-card pickup, too-low choose, ascending resolution, 66+ end,
lowest-wins placings) correct and well tested; hidden-info integrity clean.

### Player count capped at 8, contradicting Go/official rules (2-10) and the crate's own RULES.md
- severity: minor
- category: correctness
- location: game/category-5-2/src/lib.rs:21 (`MAX_PLAYERS = 8`), :481-483 (`player_counts`), game/category-5-2/RULES.md:3
- finding: the crate enforces 2..=8 players while Go allowed 2..=10, official 6 nimmt! supports 2-10, and RULES.md line 3 still says "A 2-10 player card game" — the docs contradict the enforced limit. Deck math works for 10 (10*10+4 = 104 exactly).
- recommendation: either raise MAX_PLAYERS to 10 to match Go/official, or fix RULES.md to say 2-8 and note the deviation; update the test at lib.rs:549-555 either way.

### draw_cards recurses without bound — stack overflow if asked for more cards than deck+discard hold
- severity: minor
- category: correctness
- location: game/category-5-2/src/lib.rs:270-280
- finding: if `n` ever exceeds `deck.len() + discard.len()`, the second recursive call recurses forever (empty deck, empty discard) until stack overflow — a panic-class failure killing the HTTP request. Not reachable in normal play (104 cards conserved; max draw 84 at 8 players) and Go has the identical recursion, so a preserved latent hazard in a `pub fn` with no guard.
- recommendation: guard `deck.len() + discard.len() >= n` before recursing, or convert to a loop with an explicit invariant check.

### Panic-on-invariant `expect` calls in resolve/choose paths
- severity: nit
- category: correctness
- location: game/category-5-2/src/lib.rs:178 (`row is never empty`), :309 (`choosing player has a played card`), :235 (`auto-play should only play valid cards`)
- finding: three `expect` panics guard invariants that hold through the public API but are not type-enforced; Go panics in the same spots. Unreachable from crafted commands; a corrupt deserialized state could turn them into request-killing panics.
- recommendation: acceptable as-is; if hardening, return errors or encode the invariants in the types.

### "N points until the end of the game" renders a negative number after the game ends
- severity: nit
- category: quality
- location: game/category-5-2/src/render.rs:115-120
- finding: footer computes `END_SCORE - max_points` unconditionally, producing e.g. "**-4 points** until the end of the game." for finished games. Go quirk, preserved.
- recommendation: clamp at 0 or skip the footer when `finished`.

### points() returns raw bullhead totals where lower is better
- severity: nit
- category: consistency
- location: game/category-5-2/src/lib.rs:477-479
- finding: `points()` exposes lower-is-better bullhead totals as-is (placings correctly negate at :337-342); any framework consumer of `points()` assuming higher-is-better (stats/ratings) would rank this game backwards. Go does the same — flagged so the web/stats contract for low-score games gets checked.
- recommendation: verify the framework's `points()` contract; if higher-is-better is assumed, return negated values (and audit other lowest-wins games).

### Card(pub u8) permits invalid cards via the public tuple field and serde
- severity: nit
- category: quality
- location: game/category-5-2/src/lib.rs:28-29
- finding: `Card(0)`/`Card(200)` can be constructed anywhere or deserialized from crafted state, and `heads()` silently mis-scores them. Not reachable via player commands (play only accepts cards from the player's own hand via `Enum::exact`).
- recommendation: private field + validated constructor, or a serde range check (1..=104), if state-integrity hardening is in scope.

### Test comment typo: "11 is a multiple of 1 only"
- severity: nit
- category: quality
- location: game/category-5-2/src/lib.rs:592
- finding: comment should read "multiple of 11 only". Cosmetic.
- recommendation: fix the comment.

### hands[0] used as proxy for all players' hand sizes in resolve_plays
- severity: nit
- category: quality
- location: game/category-5-2/src/lib.rs:228
- finding: `match self.hands[0].len()` decides end_round vs auto-play for everyone, assuming uniform hand sizes (true by construction; identical to Go). The invariant is implicit.
- recommendation: optional — a short comment stating the uniform-hand-size invariant.

## greed-2

Good shape: scoring table, bust cascade (improved from Go recursion to a loop),
hot-dice reroll, `done` auto-take priority, end-game trigger and tie placings
are exact Go parity and match RULES.md; no panic path reachable from crafted
command input.

### Game::score ignores its `player` argument and skips turn validation
- severity: minor
- category: quality
- location: game/greed-2/src/lib.rs:294-295
- finding: `pub fn score(&mut self, player: usize, ...)` discards `player` (`let _ = player;`) and performs no `can_score` check, unlike `player_roll`/`done`; the log attributes the score to `current_player` regardless. Unreachable from crafted input today (parser only built when `can_score(player)` holds), Go had the same shape, but a latent footgun for any future direct caller.
- recommendation: validate `player == self.current_player` inside `score` for symmetry with the other mutators, or drop the parameter and have `done()` call a private helper.

### E/e score-token collision makes `score eee` consume the E1 triple first (Go quirk)
- severity: minor
- category: correctness
- location: game/greed-2/src/command.rs:56-64 (also lib.rs:52-55, :124-130)
- finding: `Die::E1.name()` is `"E"` and `Die::E2.name()` is `"e"`; `Token::parse` is case-insensitive and `OneOf` is first-Ok-wins with the E1 triple first in `SCORES`, so holding both triples, `score eee` consumes the E1 dice. Not a soft-lock (the E2 triple scores next) and pinned by `test_score_case_insensitive_e1_e2_collision` — Go parity, cross-reference only.
- recommendation: none required; if ever fixed, disambiguate the die names, not the parser.

### Scores/dice length invariants unchecked on deserialized state
- severity: minor
- category: correctness
- location: game/greed-2/src/lib.rs:366 (`done`), :375 (`placings`), :522 (`points`), game/greed-2/src/render.rs:79
- finding: `Game` is `Deserialize` with no validation; a stored state with `scores.len() != players` or `current_player >= players` panics on index. Low reachability (state from trusted store); common shape across ported crates.
- recommendation: no crate-specific action; assert lengths if a systemic state-validation hook is added.

### Duplicated placings-log block in the Roll and Done command arms
- severity: nit
- category: simplicity
- location: game/greed-2/src/lib.rs:476-491 and :496-511
- finding: the `if self.is_finished() { ... placings_log(...) }` block is copy-pasted verbatim in both arms.
- recommendation: extract a `finish_logs()` helper or append after the match.

### Theoretical i32 overflow in turn/banked score arithmetic
- severity: nit
- category: correctness
- location: game/greed-2/src/lib.rs:307, :366
- finding: plain i32 adds; a turn can accumulate arbitrarily via hot-dice rerolls (~430k consecutive scoring rerolls to overflow). Not reachable in any realistic play; Go had the same arithmetic.
- recommendation: none practical; `saturating_add` if ever desired.

### Die::E1 rendered as `Foreground` though RULES.md says black
- severity: nit
- category: consistency
- location: game/greed-2/src/lib.rs:64, game/greed-2/RULES.md:18
- finding: Go used `render.Black` for the `E` face; the port maps it to `Foreground` while RULES.md still documents "black". Almost certainly deliberate (true black invisible on dark terminals), but doc and code disagree.
- recommendation: update RULES.md's colour column if `Foreground` is intentional.

## farkle-2

Careful, faithful port of `farkle_1`; scoring, multiset kept-dice validation,
bust, hot dice, banking, end trigger all match Go and RULES.md; library code
explicitly hardened against corrupted die values (0/7) with tests. Seeded
`GameRng` replaces Go's per-roll time seeds.

### Scoring table duplicated between lib.rs and render.rs
- severity: minor
- category: consistency
- location: game/farkle-2/src/render.rs:24-46 (vs game/farkle-2/src/lib.rs:47-82)
- finding: the help/scoring table rendered to players hardcodes the eight combinations and values in `scoring_table()`, duplicating the authoritative `SCORES` static. A future change to `SCORES` silently drifts the rendered table out of sync with actual scoring (and RULES.md).
- recommendation: derive the rendered table from `scores()`/`SCORES`, keeping only display names local.

### score() is pub but ignores its `player` argument
- severity: nit
- category: quality
- location: game/farkle-2/src/lib.rs:245-246
- finding: same shape as greed-2: `let _ = player;`, no turn validation; correctness relies on `command()` gating via `can_score` while siblings `player_roll`/`done` do validate. Go parity, but inconsistent within the crate.
- recommendation: add the same guard as the siblings, or a `debug_assert_eq!(player, self.current_player)`.

### Finished game leaks stale `turn_score`/`remaining_dice` into pub_state/render
- severity: nit
- category: correctness
- location: game/farkle-2/src/lib.rs:203-212 (`bust`), :297-317 (`done`), :365-380 (`pub_state`)
- finding: on a finishing bust/done, `turn_score` keeps its last value and `remaining_dice` the last roll, so a finished game's pub_state/render reports a non-zero "Score this turn". Cosmetic; placings/scores correct; Go identical (preserved quirk).
- recommendation: zero `turn_score` when finished, or skip the table in the renderer when `self.finished`.

### Test sets out-of-range `current_player`
- severity: nit
- category: quality
- location: game/farkle-2/src/lib.rs:614
- finding: `test_finished_and_placings` does `g.current_player = g.first_player + 1` with 3 players; if the seeded `first_player` is 2 this constructs a state the real game can never reach, harmless today but panic-prone for future assertions.
- recommendation: use `(g.first_player + 1) % 3`.

### render.rs uses `u8` instead of the `Die` alias
- severity: nit
- category: consistency
- location: game/farkle-2/src/render.rs:6, :13
- finding: `render_die(d: u8)`/`render_dice(dice: &[u8], ...)` bypass the crate's own `pub type Die = u8` used everywhere else.
- recommendation: use `Die`/`&[Die]`.

### Simplified Farkle variant (no straight/three-pairs/4+-of-a-kind, 5000 target) — Go-faithful cross-reference
- severity: nit
- category: correctness
- location: game/farkle-2/src/lib.rs:22, :47-82; game/farkle-2/RULES.md:32-49
- finding: published Farkle rules also score a 1-6 straight, three pairs and 4+-of-a-kind, usually require an opening minimum and play to 10000. This crate implements only single 1s/5s and three-of-a-kind to 5000, exactly matching Go and accurately documented in RULES.md ("Only these exact combinations score"). Cross-reference for the port-parity decision, not a divergence.
- recommendation: none required; revisit only if the project wants full Farkle rules.

### PubState renderer indexes `scores[p]` without a length check
- severity: nit
- category: correctness
- location: game/farkle-2/src/render.rs:71-80
- finding: a crafted/desynced `PubState` with `players > scores.len()` panics the renderer; unreachable for server-generated states. Same pattern as other crates.
- recommendation: none required; `zip` the range with scores if the renderer is ever exposed to deserialized input.

## tic-tac-toe-2

No Go source; judged against standard tic-tac-toe rules. Core logic correct
(move validation, all 8 win lines, draw, placings) with genuinely strong tests.
As the canonical minimal example other authors copy, its nits matter more than
most.

### `1 - start_player` underflows on crafted state
- severity: minor
- category: correctness
- location: game/tic-tac-toe-2/src/render.rs:34
- finding: `let o_player = 1 - start_player;` underflows the `usize` when a deserialized state has `start_player >= 2` (panic in debug, silent wrap to `N::Player(usize::MAX - k)` in release — the workspace sets no release `overflow-checks`). lib.rs:141 computes the same value safely as `(self.start_player + 1) % NUM_PLAYERS`, so the crate is internally inconsistent as well as fragile. Only reachable with a forged state.
- recommendation: use `(start_player + 1) % NUM_PLAYERS` here too.

### Crafted `players` count drives unbounded allocation/iteration
- severity: minor
- category: correctness
- location: game/tic-tac-toe-2/src/lib.rs:154-160 (`placings`), :268-273 (`points`)
- finding: `placings()`/`points()` iterate `0..self.players`, and `lib/cmd` `renders()` iterates `0..game.player_count()`; `players` is a plain `usize` deserialized from request JSON with no validation, so a forged state with a huge `players` causes massive allocation or a near-infinite render loop. Systemic in that every game trusts deserialized state; noted here because the requester deserializes `Game` verbatim (`lib/cmd/src/requester/gamer.rs:28,37,41,45`).
- recommendation: systemic fix in the requester/validation layer; at minimum games could validate `players == NUM_PLAYERS` on entry.

### Dead, misleading `Cell::Empty` arm in `winner()`
- severity: nit
- category: quality
- location: game/tic-tac-toe-2/src/lib.rs:139-143
- finding: `matching_line` never returns `Cell::Empty`, so the arm mapping it to `start_player` is unreachable and would be a bug if reachable — exactly the kind of dead arm other authors copy from this canonical crate.
- recommendation: have `matching_line` return a mark-only type, or replace the arm with `unreachable!()`.

### Mark casing inconsistent between logs and board render
- severity: nit
- category: consistency
- location: game/tic-tac-toe-2/src/lib.rs:107-118 and game/tic-tac-toe-2/src/render.rs:16-17
- finding: the play log renders uppercase bold `X`/`O`; the board and "is X / is O" label use lowercase `x`/`o`; RULES.md documents lowercase. Cosmetic, but this is the crate others copy.
- recommendation: pick one casing (uppercase matches convention) and use it everywhere.

## no-thanks-2

Core logic fully correct vs official rules and Go (deck 3-35 minus 9, 11 chips
for 3-5p, pass/take/forced-take, run scoring at lowest card, hidden chips —
an improvement over Go, which exposed nothing). No player-input-reachable
panics.

### Vacuous test `test_init_player_chips` asserts nothing
- severity: minor
- category: quality
- location: game/no-thanks-2/src/lib.rs:392-399
- finding: the test builds `Game::default()` (so `players == 0`), calls `init_player_chips()`, then loops `for p in 0..g.players` — the loop body never executes, so the `assert_eq!(11, ...)` is never checked. The test passes even if `STARTING_CHIPS` changes or `init_player_chips` fills zeros.
- recommendation: set `g.players = 3` (or use `Game::start(3, 1)`) before calling, and assert `g.player_chips.len() == g.players`.

### Player cap 3-5 deviates from official 3-7 rules (Go quirk)
- severity: minor
- category: correctness
- location: game/no-thanks-2/src/lib.rs:18-20, :337-339
- finding: official No Thanks! supports 3-7 players with scaled starting chips (11 for 3-5p, 9 for 6p, 7 for 7p); the crate hard-codes `MAX_PLAYERS = 5` and a single `STARTING_CHIPS = 11`. Go had the same restriction; RULES.md accurately describes the implemented variant. Cross-reference for the port-parity decision.
- recommendation: none required for parity; parameterise starting chips by player count if 6-7p support is ever wanted.

### Unreachable "no chips" branch in `pass()`
- severity: nit
- category: simplicity
- location: game/no-thanks-2/src/lib.rs:106-110
- finding: `can_pass()` already requires chips > 0 and `pass()` returns early on `!can_pass(player)`, so the second `player_chips[player] <= 0` check ("You have no chips left...") is dead. Go had the same redundancy. Note also the inconsistency: `can_pass` uses `.get(player).copied().unwrap_or(0)` while `pass` indexes directly.
- recommendation: drop the dead branch, or fold the specific message into a single check.

### Run-grouping logic duplicated between lib.rs and render.rs
- severity: nit
- category: quality
- location: game/no-thanks-2/src/lib.rs:156-176 and game/no-thanks-2/src/render.rs:23-42
- finding: `Game::player_hand_grouped` and `group_sorted` are line-for-line the same run-detection algorithm (the renderer operates on `PubState`, motivating the copy); any fix to one must be mirrored in the other.
- recommendation: extract a shared free function, e.g. `pub fn group_runs(sorted: &[i32]) -> Vec<Vec<i32>>`.

### Renderer panics on inconsistent deserialized PubState
- severity: nit
- category: correctness
- location: game/no-thanks-2/src/render.rs:77, :91, :115
- finding: `render()` unwraps `pub_state.current_card` whenever `finished` is false and indexes `hands[p]` for `p in 0..players` without length checks — safe for server-generated states, panics on crafted ones (same cross-cutting shape as `player_state`'s unchecked `player_chips[player]` at lib.rs:275).
- recommendation: optional hardening (`if let Some(card)`, `.get(p)`); acceptable if rendering is only fed server-generated states.

## liars-dice-2

Very good shape: hidden-information integrity correct (pub_state exposes only
dice counts, reveal only at call time), no panic reachable from crafted command
input, and the port fixes two real Go bugs (inverted `PlayerState` bounds check
that always returned empty dice; unguarded `[1:]` die-removal slice).

### Turn after challenge goes to player after the caller, not the challenge loser (Go quirk)
- severity: minor
- category: correctness
- location: game/liars-dice-2/src/lib.rs:208-211
- finding: after a `call`, the next round is started by `next_active_player(current_player)` where `current_player` is still the caller — the player clockwise after the caller starts regardless of who lost the die. Official Liar's Dice/Perudo rules have the challenge loser start the next round. Exactly matches Go and is explicitly documented in RULES.md, so a preserved documented quirk — cross-reference for the port-parity decision.
- recommendation: no action unless aligning with official rules; then set `current_player` to the losing player (skipping eliminations) and update RULES.md.

### Index panics reachable from inconsistent deserialized state
- severity: minor
- category: correctness
- location: game/liars-dice-2/src/lib.rs:73-77 (`active_players`), :79-83 (`eliminated_player_list`), :100-107 (`roll_dice`), :192-195 (`call` die removal)
- finding: several methods index `player_dice[p]` for `p in 0..self.players` assuming `player_dice.len() == players` and valid `bid_player`; a corrupted stored game panics the HTTP request. Not reachable in normal play (`start()` builds consistent state); likely systemic across game crates.
- recommendation: validate on load or use `.get(p)` defensively; low priority if state integrity is trusted.

### "fourty" typo preserved from Go
- severity: nit
- category: consistency
- location: game/liars-dice-2/src/render.rs:104
- finding: `number_str` renders 40+ quantities as "fourty ..." (should be "forty"), faithfully porting the identical typo in `brdgme-go/brdgme/strings.go:135`. Practically unreachable (needs a bid ≥40 with max 30 dice in play).
- recommendation: fix the spelling in Rust; no need to preserve the Go typo.

### Bid quantity has no upper bound in the parser
- severity: nit
- category: correctness
- location: game/liars-dice-2/src/command.rs:44-48
- finding: `Int { min: Some(MIN_BID_QUANTITY), max: None }` accepts quantities up to i32::MAX — legal per the rules and harmless (no arithmetic/allocation on the value; `number_str` falls back to digits), but `bid 2000000000 6` produces a silly log line and help/suggest shows no sensible cap.
- recommendation: optional: cap at `players * START_DICE_COUNT` for UX, or leave as-is.

### Test gaps: hidden-info redaction, wild-1 bid value, full game
- severity: nit
- category: quality
- location: game/liars-dice-2/src/lib.rs:362-419, game/liars-dice-2/tests/contract.rs
- finding: missing tests for (a) `pub_state`/`player_state` never exposing other players' dice (the game's key hidden-information property), (b) call resolution with bid value 1 (the `*d as i32 == bid_value || *d == 1` condition at lib.rs:168 handles it correctly but untested), (c) play-to-completion asserting final placings.
- recommendation: add small unit tests for the three cases.
