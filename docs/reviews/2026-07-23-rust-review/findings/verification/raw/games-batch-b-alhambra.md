# Verification: games-batch-b, section game/alhambra-1 (F16-F28)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5.
All paths relative to snapshot root. Read-only verification; no builds or tests
run. Files read in full: game/alhambra-1/src/{lib.rs,card.rs,render.rs,command.rs},
RULES.md.

## F16 - take() mints duplicate cards (critical)

Verdict: CONFIRMED. Severity: critical is correct.

Evidence:
- Pre-check, game/alhambra-1/src/lib.rs:557-561:
  `for c in cards { if !self.cards.contains(c) { return Err(...) } }`
  Each requested card is tested against the untouched market vec independently;
  no multiplicity accounting.
- Removal loop, lib.rs:570-575:
  ```
  for c in cards {
      if let Some(pos) = self.cards.iter().position(|mc| mc == c) {
          self.cards.remove(pos);
      }
      self.boards[player].cards.push(*c);
  }
  ```
  The push is unconditional - it executes even when `position` returned None.
- Reachability trace for `take b1 b1` with exactly one B1 in market:
  - Parser: the take arm uses `Many::some_spaced(CardParser)`
    (command.rs:149-160); CardParser accepts any letter+digits token, so
    duplicate cards parse fine (unlike spend, which uses `Enum::exact` over the
    hand).
  - lib.rs:557 pre-check: both iterations see B1 in `self.cards` -> passes.
  - lib.rs:562-568 value check: total 2 <= 5 -> passes.
  - Loop iteration 1: B1 found, removed from market, pushed to hand.
  - Loop iteration 2: `position` -> None, removal skipped, B1 pushed to hand
    anyway.
  - Net: market loses one B1, player gains two B1s. Money minted from nothing.
- Contrast: `spend()` uses the clone-and-verify pattern (lib.rs:616-626,
  `hand.iter().position(...) ... None => return Err`), which is exactly what
  take() lacks. Test `test_spend_command_multiple_same_card` (lib.rs:1389) locks
  the correct behavior for spend only; no take-multiplicity test exists.

Severity: reachable economy-corrupting exploit from a single ordinary command;
Correctness at the top of the charter. Critical stands.

## F17 - place indices diverge from rendered indices after a placement (major)

Verdict: CONFIRMED. Severity: major is correct.

Evidence:
- Sentinel left behind: lib.rs:694 (Place/FinalPlace arm of `place()`):
  `self.boards[player].place[n] = Tile::empty();` - the vec is never compacted.
- Raw indexing: lib.rs:664-669 (`_` arm covering both Place and FinalPlace):
  `if n >= self.boards[player].place.len() { ... } self.boards[player].place[n].clone()`
  - indexes the raw vec including Empty sentinels.
- Renderer numbers only non-empty tiles: render.rs:353-367 `render_tile_set`
  does `let non_empty = not_empty(tiles);` then
  `indices.push(format!(" {}  ", i + 1));` over the filtered list.
- Trace: place = [A, B]. `place 1 <coord>` places A and sets place = [Empty, B].
  Render now shows "1: B". Player types `place 1 <coord2>`: raw index 0 = Empty
  tile.
  - `can_place` (lib.rs:528-533) passes: `not_empty(place)` still contains B.
  - Bounds check passes (0 < 2).
  - `grid_is_valid` on the test grid passes: the walk-from-fountain loop skips
    Empty neighbors (card.rs:392-394) and the connectivity check exempts
    Empty entries (card.rs:413-417 `t.tile_type != TileType::Empty && ...`);
    the gap check treats the cell as empty and it is outside-reachable, so the
    Empty tile inserts successfully whenever the coord touches the outside.
  - Result: `grid.insert(coord, Empty)` (lib.rs:680); the coord is now occupied
    (`contains_key` check at lib.rs:654 rejects future placement there);
    `grid_bounds` (card.rs:340-364) iterates keys so bounds expand; log says
    "placed Empty tile" (lib.rs:684, `{:?}`).
  - `remove` at that coord (lib.rs:747,758) pushes the Empty tile into
    `reserve`; `swap` (lib.rs:715,725) does the same - phantom Empty in reserve,
    and since `render_reserve` also filters non-empty while the Action-phase
    place arm indexes `reserve[n]` raw (lib.rs:659-662), the divergence
    propagates to reserve indices too. Doc's claims all check out.
- FinalPlace: same `_` match arms (lib.rs:664, 693), so identical flaw.

Severity: user following the rendered UI corrupts their own board permanently
(blocked coord, phantom tiles, index drift). Major stands.

## F18 - grid_longest_ext_wall terminates wall walk prematurely (major)

Verdict: CONFIRMED, with one precision adjustment. Severity: major is correct
(arguably strengthened - see nondeterminism note).

Evidence - the unconditional break, card.rs:499-516:
```
for rot_num in 0..3i32 {
    let next_wall = VectDir {
        vect: pivot.add(cur.dir.vect().rot_all((rot_num + 2) * rot_dir)),
        dir: cur.dir.rot((rot_num - 1) * rot_dir),
    };
    if grid_tile_at(g, next_wall.vect).tile_type == TileType::Empty {
        continue;
    }
    if !visited.contains_key(&next_wall)
        && grid_is_wall(g, next_wall)
        && !grid_is_internal_wall(g, next_wall)
    {
        wall += 1; ... found = true; cur = next_wall;
    }
    break;
}
```
The `break` at card.rs:516 sits outside the inner `if`, so the first candidate
whose tile is merely non-empty ends the candidate scan whether or not it was a
continuing wall. Empty-tile candidates fall through via `continue`; non-empty
non-wall candidates do not.

Candidate geometry reconstructed (cur = {(0,0), Up}, rot_dir=1, pivot=(0,-1),
rot_all indices verified against DIRS_ALL at card.rs:193-202):
- rot_num=0: vect (1,-1), dir Left  -> turn at corner (diagonal tile)
- rot_num=1: vect (1,0),  dir Up    -> straight
- rot_num=2: vect (0,0),  dir Right -> turn back
Order matches the doc's turn / straight / turn-back claim.

Doc's concrete example re-traced (T0=(0,0) Up wall, T1=(1,0) Up wall,
T2=(1,-1) present, no Left wall; true longest external wall = 2):
- Walk starting at (0,0,Up), rot_dir=1: rot_num=0 candidate is (1,-1,Left).
  T2 is non-empty, so no `continue`; T2 has no Left wall, so the inner if is
  false; `break` fires. The straight candidate (1,0,Up) is never examined.
  found=false -> walk ends with wall=1. rot_dir=-1 finds nothing to the left.
- Later start at (1,0,Up), rot_dir=-1: rot_num=0 candidate is (0,-1,Right) -
  (0,-1) is Empty -> `continue`; rot_num=1 straight candidate is (0,0,Up) - but
  it is already in `visited` from the first walk, so the
  `!visited.contains_key` clause fails and `break` fires. wall=1 again.
  Result: returns 1. Undercount confirmed for this iteration order.

Adjustment (precision, does not rescue the code): `Grid` is
`HashMap<Vect, Tile>` (card.rs:307) and the outer loop is `for (v, t) in
g.iter()` (card.rs:481), so segment start order is HashMap iteration order.
If iteration happens to start at (1,0,Up) instead: rot_dir=-1, rot_num=0
candidate (0,-1,Right) is on an Empty tile -> `continue`; rot_num=1 straight
candidate (0,0,Up) is unvisited and a valid external wall -> counted, wall=2,
and the function returns the correct 2. So the doc's "returns 1" holds for
some HashMap orders, not all: the function nondeterministically returns 1 or 2
on this grid. The blocker junction is one-way (crossable walking left, not
right), so which fragment count wins depends on which segment iterates first.
This makes the bug worse, not better: wall scores are both systematically
undercountable and nondeterministic across process runs (std HashMap
RandomState), i.e. replaying the same seed+commands can score differently.
- Internal-wall predicate checked: grid_is_internal_wall (card.rs:472-475)
  is true only when the adjacent tile has the matching opposite wall; in the
  example (0,-1) is empty, so T0's Up wall is external. Premise sound.
- The two existing tests (lib.rs:1334-1366) use grids whose wall chains have
  no such diagonal non-wall blocker at a corner, so they don't exercise this.

Severity: scoring error (1 pt/segment, three scoring rounds) plus determinism
violation. Major stands.

## F19 - Dirk excluded from final placings (minor)

Verdict: CONFIRMED (code trace); the "Dirk can win" premise is external
official-rules knowledge, not stated in-repo. Severity: minor is correct.

Evidence:
- Dirk accumulates points: `score_type` iterates `0..self.all_players`
  (lib.rs:321) and `score_round` credits `self.boards[p].points += s.points`
  for every scoring player including index 2 (lib.rs:253-257).
- Placings/status exclude him: all six `is_finished()` blocks build scores and
  metrics from `(0..self.human_players)` (e.g. lib.rs:840-847), `status()`
  (lib.rs:975-991) uses `(0..self.human_players)`, and `points()`
  (lib.rs:997-1001) likewise. In a 2-player game the two humans are always
  placed 1st/2nd regardless of Dirk's total.
- RULES.md (read in full, 51 lines) says only "2-player mode adds a bot named
  Dirk" and "Highest total points wins" - the deviation is undocumented, as
  claimed. Wall scoring also excludes Dirk (lib.rs:281 `0..self.human_players`)
  and render shows "N/A" (render.rs:241-247), consistent with a deliberate
  Dirk-doesn't-score-walls design, which makes the placings exclusion look
  intentional-but-undocumented.

## F20 - Reduced money deck for 2-player games (minor)

Verdict: CONFIRMED (code trace); the "official game always uses 108" premise is
external. Severity: minor is correct.

Evidence: card.rs:620-631:
```
let n = if players == 2 { 2 } else { 3 };
for c in Currency::ALL { for v in 1..=9 { for _ in 0..n { deck.push(...) } } }
```
4 currencies x 9 values x n = 72 cards for 2 players, 108 otherwise. RULES.md
does not mention deck size at all; no PORTING_NOTES.md or Go source exists for
alhambra in the snapshot (finding doc's caveat is accurate).

## F21 - Test coverage misses the riskiest logic (minor)

Verdict: CONFIRMED with one small adjustment. Severity: minor is correct.

Evidence - tests actually present (lib.rs:1028-1573): test_parse_card,
test_game_score_type (score tables), 4x grid_is_valid error cases,
test_grid_longest_ext_wall (2 grids, neither containing a diagonal blocker),
test_grid_parse_coord, test_spend_command_multiple_same_card (spend-side
multiplicity only), pub_state_does_not_leak_hidden_info, 5 log-format tests,
command_parser_spend, command_spec_autocomplete, game_starts_and_take_works
(single-card take smoke). Confirmed missing: take multiplicity,
place-index-after-placement, diagonal-blocker wall walk, exact-payment extra
action, overpay ending turn, tie handling in final-place, 2-player/Dirk flows
beyond the join log.

Adjustment: final-place distribution is not entirely untested - log_final_place
(lib.rs:1474-1501) exercises single-currency distribution to the richest of 3
players; what is missing is the tie path and multi-currency cases. Everything
else in the finding is accurate, and the three major findings would indeed have
been caught by the listed tests.

## F22 - is_finished() epilogue copy-pasted into six command arms (nit)

Verdict: CONFIRMED. Severity: nit is correct.

Evidence: identical ~11-line block (`if self.is_finished() { let scores ...
gen_placings ... placings_log }`) at lib.rs:839-849, 862-872, 885-895, 908-918,
931-941, 954-964 - once per Take/Spend/Place/Swap/Remove/Done arm.

## F23 - Invariant-guarded panics in runtime paths (nit)

Verdict: CONFIRMED. Severity: nit is correct.

Evidence, all three genuinely invariant-guarded:
- lib.rs:431 (and the same pattern at lib.rs:601): `Currency::ALL.iter()
  .position(|&c| c == currency).unwrap()` - Currency::ALL (card.rs:15-20)
  contains all four enum variants, so position always succeeds for any
  Currency value, including player-supplied ones at lib.rs:601.
- command.rs:142: `Card::parse(&s).unwrap()` - `s` comes from
  `Enum::exact(hand_cards)` where hand_cards are `c.to_string()` of real hand
  cards (command.rs:126-131); Display output ("B10") always round-trips
  through Card::parse (card.rs:62-74, 77-81).
- card.rs:209: `panic!("Can only call rot_all on unit vector")` - the only
  caller is grid_longest_ext_wall (card.rs:501) with `cur.dir.vect()`, always
  one of the four unit vectors.

## F24 - Gap-check loop range asymmetry (nit)

Verdict: CONFIRMED, including the harmlessness argument. Severity: nit is
correct.

Evidence: card.rs:450-451: `for x in min.x..=max.x { for y in min.y..max.y {` -
row max.y is skipped. Harmlessness re-derived: the outside flood walk
(card.rs:420-448) starts at (min.x-1, min.y-1) and is bounded by
[min-1, max+1]. The entire boundary ring is tile-free by definition of
grid_bounds and 4-connected, so every cell in row max.y+1 is marked connected.
Any empty cell at (x, max.y) is the Up-neighbor of connected (x, max.y+1), and
the walk ignores walls (only Empty/non-empty), so it is always reached and
would pass the check even if the row were included. No gap can hide in the
skipped row. The asymmetry is real but provably behavior-neutral, exactly as
the finding states.

## F25 - Debug formatting in user-facing messages (nit)

Verdict: CONFIRMED. Severity: nit is correct.

Evidence: lib.rs:603-606 `"no tile available for {:?}"` (renders "Blue";
Currency::name() exists at card.rs:31-38 and is used at lib.rs:451,461).
lib.rs:636-640 `" spent {} on {:?} tile"` (renders "Tower"). Additional
instances beyond the doc's list, same pattern: lib.rs:684 (`" placed {:?}
tile"`), lib.rs:730-731 (swap), lib.rs:762 (remove).

## F26 - tile_counts duplicated (nit)

Verdict: CONFIRMED. Severity: nit is correct.

Evidence: render.rs:69-77 free fn `tile_counts(grid: &Grid)` and
card.rs:601-609 `PlayerBoard::tile_counts(&self)` have byte-identical bodies
(entry/or_insert over non-Empty grid values). The render copy exists because
render works over PubBoard's raw Grid, but a free function over &Grid in
card.rs would serve both, as the finding recommends.

## F27 - Grid column headers wrap past 26 columns (nit)

Verdict: CONFIRMED. Severity: nit is correct.

Evidence: render.rs:163-164:
`let col_letter = ((x - x_start) as u8 + b'a') as char;` - offsets > 25 yield
'{', '|', etc. Rendered width is (max.x - min.x + 3) columns; reaching 27
requires a ~25-tile-wide board, theoretically possible (54 tiles in the bag)
but practically unreachable. Note coord parsing (card.rs:544-556) only accepts
a-z anyway, so such columns would also be unaddressable - cosmetic only.

## F28 - Vec-as-queue and HashMap-as-set in flood walks (nit)

Verdict: CONFIRMED. Severity: nit is correct.

Evidence: both walks in grid_is_valid use `walk_stack.first() ...
walk_stack.remove(0)` (card.rs:383-384 and 425-426) and
`HashMap<Vect, bool>` for in_walk_stack/connected (card.rs:380-381, 422-423),
with the bool payload never read (only contains_key). VecDeque/HashSet would be
idiomatic; grids are <= 55 tiles so no practical cost.

## Summary of severity assessments

No severity changes recommended. F16 critical justified (reachable
money-duplication exploit). F17/F18 major justified (permanent board
corruption; systematic + nondeterministic scoring error). F18 gains a
nondeterminism dimension the original finding missed (HashMap iteration order
decides whether the undercount manifests), which reinforces rather than
weakens it. F19-F21 minor, F22-F28 nit all appropriate. F19/F20 code traces
confirmed; their rules premises (Dirk can win officially; official deck is
always 108) rest on external official-rules knowledge - RULES.md is silent on
both, so "undocumented deviation" is accurate either way.
