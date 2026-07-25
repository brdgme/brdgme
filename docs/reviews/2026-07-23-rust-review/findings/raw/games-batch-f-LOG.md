# games-batch-f Lead review log

Unit 8 of 13. Crates: zombie-dice-2, battleship-2, for-sale-2, category-5-2,
greed-2, farkle-2, tic-tac-toe-2, no-thanks-2, liars-dice-2 (~8.0k LOC).
Snapshot: /home/beefsack/Development/brdgme-review-snapshot @ f8763a5.

Go source availability (in snapshot brdgme-go/): zombie_dice_1, battleship_1,
for_sale_1, category_5_1, greed_1, farkle_1, no_thanks_1, liars_dice_1 exist;
tic-tac-toe has NO Go source → judge against official rules.

Known systemic issues already tracked (do not re-derive, cross-reference only):
core parser non-ASCII panics; markup slice() byte/char bug; lib/cmd warp unwrap;
suggest Many max; binary-only deps as library deps; http bins bind 0.0.0.0:80;
108 boilerplate binaries; for-sale-2 doc_int min:None help-rendering consumer.

## Dispatch log

- (pending) worker 1: zombie-dice-2
- (pending) worker 2: battleship-2
- (pending) worker 3: for-sale-2
- (pending) worker 4: category-5-2
- (pending) worker 5: greed-2
- (pending) worker 6: farkle-2
- (pending) worker 7: tic-tac-toe-2
- (pending) worker 8: no-thanks-2
- (pending) worker 9: liars-dice-2

## Verify/reject decisions

(none yet)

### 2026-07-24 worker 1 returned: zombie-dice-2
- Raw file: findings/raw/games-batch-f-zombie-dice-2.md (9 findings, incremental appends confirmed)
- Counts: 0 crit / 1 major / 4 minor / 4 nit
- Headlines: PubState leaks shuffled cup draw order (major, hidden-info leak);
  shotgun-dice cup refill is a documented Go quirk (cross-ref); rolloff invisible
  in render; 2 state-deser panic paths (not command-reachable); otherwise clean port.
- Lead decision: PENDING curation (will verify line numbers against snapshot).

### 2026-07-24 worker 2 returned: battleship-2
- Raw file: findings/raw/games-batch-f-battleship-2.md (7 findings)
- Counts: 0 crit / 0 major / 2 minor / 5 nit
- Headlines: clean faithful port; shoot() drops Go IsValidLocation guard
  (lib.rs:308-348, panic-able by non-parser callers); Vec-length-trusting
  indexing inconsistency; expect("cell is a ship") nit.
- Lead decision: PENDING curation.

### 2026-07-24 worker 3 returned: for-sale-2
- Raw file: findings/raw/games-batch-f-for-sale-2.md (11 findings + clean aspects)
- Counts: 0 crit / 1 major / 4 minor / 6 nit
- Headlines: selling-phase hidden-info leak — played buildings in `bids` exposed
  verbatim in pub_state (lib.rs:258,411-412; Go had same leak); pass floor(bid/2)
  vs official round-up + deck/chip deviations = Go quirks (cross-ref); RULES.md
  cheque description wrong; SELL_THRESHOLD magic number vs Phase enum.
- Lead decision: PENDING curation.

### 2026-07-24 worker 4 returned: category-5-2
- Raw file: findings/raw/games-batch-f-category-5-2.md (8 findings + clean areas)
- Counts: 0 crit / 0 major / 2 minor / 6 nit
- Headlines: faithful correct port; player cap 8 vs RULES.md/official 10 (minor);
  draw_cards unbounded recursion (lib.rs:270-280, latent); 3 expect panic paths
  (nits); hidden-info clean.
- Lead decision: PENDING curation.

### 2026-07-24 worker 5 returned: greed-2
- Raw file: findings/raw/games-batch-f-greed-2.md (7 findings)
- Counts: 0 crit / 0 major / 3 minor / 4 nit
- Headlines: good shape, Go parity, no command-reachable panics; Game::score
  ignores player arg (lib.rs:295, latent footgun); E/e token collision =
  documented Go quirk (cross-ref).
- Lead decision: PENDING curation.

### 2026-07-24 worker 6 returned: farkle-2
- Raw file: findings/raw/games-batch-f-farkle-2.md (8 findings + clean checks)
- Counts: 0 crit / 0 major / 1 minor / 7 nit
- Headlines: clean faithful port; render.rs:24-46 hardcoded scoring table
  duplicates lib.rs SCORES static (drift risk, minor); simplified variant is
  documented Go quirk; no command-reachable panics.
- Lead decision: PENDING curation.

### 2026-07-24 worker 7 returned: tic-tac-toe-2
- Raw file: findings/raw/games-batch-f-tic-tac-toe-2.md (5 findings)
- Counts: 0 crit / 0 major / 3 minor / 2 nit
- Headlines: core logic correct (no Go source; judged vs standard rules);
  render.rs:34 `1 - start_player` underflow on crafted state (inconsistent with
  lib.rs:141 modulo); crafted players count → unbounded alloc in placings/points;
  canonical-example nits.
- Lead decision: PENDING curation.

### 2026-07-24 worker 8 returned: no-thanks-2
- Raw file: findings/raw/games-batch-f-no-thanks-2.md (5 findings)
- Counts: 0 crit / 0 major / 2 minor / 3 nit
- Headlines: core logic fully correct vs official rules; chips hidden (better
  than Go); test_init_player_chips vacuous (lib.rs:392-399, 0 players);
  unreachable "no chips" branch in pass() (lib.rs:106-110); 3-5p cap vs official
  3-7 = Go quirk cross-ref.
- Lead decision: PENDING curation.

### 2026-07-24 worker 9 returned: liars-dice-2
- Raw file: findings/raw/games-batch-f-liars-dice-2.md (6 findings + clean aspects)
- Counts: 0 crit / 0 major / 2 minor / 4 nit
- Headlines: hidden-info integrity correct; round-starter after challenge =
  player after caller, not loser (official Perudo) — documented Go quirk
  (lib.rs:208-211); defense-in-depth indexing gap; port fixes two Go bugs.
- Lead decision: PENDING curation.

## Curation phase started 2026-07-24
All 9 raw files present. Reading raw files and verifying line numbers against
snapshot before writing findings/games-batch-f.md.

### 2026-07-24 curation: line-number verification
Spot-verified against snapshot: zombie-dice-2 lib.rs:194/443/242-251, for-sale-2
lib.rs:258/233/411-412, tic-tac-toe-2 render.rs:34 + lib.rs:141, battleship-2
lib.rs:308-332, category-5-2 lib.rs:21/270-280, no-thanks-2 lib.rs:106-110/392-399,
liars-dice-2 lib.rs:208-211, greed-2 lib.rs:294-295. All match worker claims.

### Verify/reject decisions
- ACCEPTED: both majors (zombie-dice-2 cup-order leak; for-sale-2 bids leak) —
  verified fields and pub_state clone sites in snapshot.
- ACCEPTED: all worker minors/nits except per-crate "binary-only deps as library
  deps" blocks — REJECTED as findings (known systemic issue, handover says do not
  re-derive); noted once in the curated file header instead. tic-tac-toe-2's
  binary-deps block (rated minor by worker) dropped on the same grounds.
- for-sale-2 doc_int min:None consumer: noted in crate summary, not a finding
  (per brief: cross-reference only).
- zombie-dice-2 two deser-state panic findings merged into one.

### 2026-07-24 UNIT COMPLETE
- Curated file written: findings/games-batch-f.md
- Final counts verified by grep: 58 findings = 0 critical / 2 major / 22 minor / 34 nit
- Per-crate: zombie-dice-2 7 (0/1/3/3), battleship-2 5 (0/0/2/3), for-sale-2 11
  (0/1/5/5), category-5-2 8 (0/0/2/6), greed-2 6 (0/0/3/3), farkle-2 7 (0/0/1/6),
  tic-tac-toe-2 4 (0/0/2/2), no-thanks-2 5 (0/0/2/3), liars-dice-2 5 (0/0/2/3)
- All 9 raw worker files + this LOG present in findings/raw/.
