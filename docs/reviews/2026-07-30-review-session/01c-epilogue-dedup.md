# Unit 01c - WP-08 / WP-08b epilogue + placings dedup

Continuation of `01-core-libraries.md` (F-01..F-08) and
`01b-color-cmd-gameclient.md` (F-09..F-17). Findings here start at **F-18**.

## Commits in scope

- `f13450a1` (2026-07-28) "refactor(game): WP-08 finish/placings epilogue
  dedup" - 11 files, 1158+/852-
- `c14bc655` (2026-07-28) "refactor(game): WP-08b epilogue dedup riders
  (acquire-1, starship-catan-1)" - 2 files, 108+/170-

## Crates reviewed (13, all of them)

age-of-war-2, alhambra-1, greed-2, jaipur-2, love-letter-2,
roll-through-the-ages-2, seven-wonders-1, splendor-2, sushi-go-2,
texas-holdem-2, zombie-dice-2 (WP-08); acquire-1, starship-catan-1 (WP-08b).
Plus `rust/lib/game/src/game.rs` and `rust/lib/game/src/game_log.rs`, and a
census of the 14 unmigrated game crates that call `placings_log`.

## The spec, recovered

The WP-08 spec was deleted from the tree by the review-compaction commit. It is
recoverable at
`git show 868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/WP-08-finish-placings-epilogue-dedup.md`
and it matters a great deal for judging this work package, because it
**explicitly rejected the shared-abstraction design** that the Unit 01 breakdown
assumed was built. Verbatim, section 0:

> Only the last line - `logs.push(placings_log(&placings, Some(&scores)))` - is
> genuinely common, and `brdgme_game::placings_log` is already that shared
> helper. Everything above it diverges [...] A `lib/game` helper would have to
> take both `scores` and `placings` as arguments - i.e. it would *be*
> `placings_log` - or take closures, which is worse than the duplication.
> **Therefore: an identical per-crate extract.** No file under `rust/lib/` is
> touched by this WP.

That is a sound call and it is what shipped: **neither commit touches
`rust/lib/game`**, and both `gen_placings` and `placings_log` long pre-date the
work package (`placings_log` since "G6a: placings log for score-based games";
`gen_placings` since the original import). Anyone reviewing this commit pair as
"a shared epilogue abstraction was introduced in lib/game" is reviewing
something that was deliberately not built - do not raise it as a miss.

The spec also pre-ruled the exclusions (`red7-1` untouched; `lost-cities-1/-2`
no code change, dual winner announcement stays by design and belongs to WP-28;
`modern-art-2` excluded because WP-25 hoists its own epilogue) and pre-ruled
"lift the scores/placings expressions **verbatim**, do not improve them".

## What the shared primitives guarantee

`rust/lib/game/src/game.rs:158` `gen_placings(metrics: &[Vec<i32>]) -> Vec<usize>`:

- Lexicographic multi-key comparison, first key most significant, via
  `cmp_fallback`.
- **Higher is better**: keys are sorted ascending then walked in reverse, so the
  largest metric vector gets place 1. A "lowest wins" game must negate. No crate
  in this unit is lowest-wins.
- Ties group by *exact vector equality* and a tie of N consumes N places
  (1,1,3 - standard competition ranking).
- A shorter vector loses to a longer one sharing its prefix (`[12,35]` <
  `[12,35,0]`), because `cmp_fallback` ranks empty as `Less`.
- Output is indexed by player, so player-index stability holds.
- Non-obvious but important: `Ordering::Equal` between two *distinct* keys is
  impossible (equal-and-same-length implies the same `HashMap` key), so the
  nondeterministic `HashMap` iteration order cannot leak into the result. The
  output is deterministic.

`rust/lib/game/src/game_log.rs:36` `placings_log`: winners = every index with
placing 1; "X wins!" for 1, "X and Y tie!" for exactly 2, "It's a tie!" for 0 or
3+; optional score tail in the caller's order.

## Findings

### F-18 (Medium) Four crates carry the same duplicated epilogue and were never scoped; 14 crates never got the transition gate

The review's headline finding was "copy-pasted finish/placings epilogue". WP-08
closed it for 13 crates. The copy-paste is still in the tree, at the same
multiplicity, in crates no finding was ever filed against (working-tree
`placings_log(` call sites):

| Crate | epilogue sites | in WP-08 scope? |
|---|---|---|
| `rust/game/for-sale-2/src/lib.rs` | 3 - `:495`, `:513`, `:531` | no, and no finding filed |
| `rust/game/sushizock-2/src/lib.rs` | 3 - `:744`, `:765`, `:786` | no, and no finding filed |
| `rust/game/category-5-2/src/lib.rs` | 2 - `:509`, `:527` | no, and no finding filed |
| `rust/game/farkle-2/src/lib.rs` | 2 - `:432`, `:453` | no, and no finding filed |
| `rust/game/red7-1/src/lib.rs` | 2 - `:498`, `:516` | no - **explicitly excluded**, do not re-raise |

`for-sale-2` and `sushizock-2` have *three* duplicated copies each - more than
`jaipur-2`, `sushi-go-2` or `zombie-dice-2`, which had two and were migrated. So
the original review, not the remediation, is where this went wrong: the spec's
riders table is finding-driven, and no finding was filed for these four. WP-08
executed its list faithfully; the list was incomplete.

The substantive half: all 14 unmigrated crates use the old
`if self.is_finished() { push placings_log }` form with **no `!was_finished`
guard**. That guard is the one genuine behaviour fix in WP-08 (finding e F14 -
age-of-war-2 amplified the log because `can_roll` is a deliberately preserved Go
quirk that ignores finished status, so each post-finish `roll` appended another
placings log). The spec notes the gate is "a no-op in every crate whose
`command_parser` already returns `None` once finished" - which is true of most
crates, but it was never checked for the 14 that were skipped, and
`for-sale-2` is exactly the crate where `01-core-libraries.md` F-05 found the
command path converts a panic into a silent no-op, i.e. a crate whose command
gating is already known to be loose.

Why it matters: duplicate finish logs are user-visible and the finish log is what
the web layer renders for a completed game. Small blast radius, but it is the
defect class the work package existed for, left half-closed with no record that
these four crates were considered.

Remediation:
1. Apply the same `finish_epilogue` + `!was_finished` treatment plus the same two
   tests to `for-sale-2`, `sushizock-2`, `category-5-2`, `farkle-2`. Purely
   mechanical; batch into the Unit 03/04 follow-up rather than opening a new WP.
2. For the 10 single-site crates, either add the guard or record per crate that
   `command_parser` returns `None` once finished. `tic-tac-toe-2:246` is the only
   one passing `None` for scores - that is correct, leave it.
3. Do **not** touch `red7-1`, `lost-cities-1/-2` or `modern-art-2`; the spec
   ruled on all four.

### F-19 (Low) Five crates build their placings metric twice, so `status()` and the finish log are only equal by coincidence

The spec required lifting each crate's expressions **verbatim**, so this is not
an execution defect - but it is the state the tree is now in, and it is worth
recording because it is the duplication that actually matters.

| Crate | duplicated expression |
|---|---|
| `rust/game/jaipur-2/src/lib.rs:699-715` (`status()`) vs `:665-677` (`finish_epilogue`) | `(0..NUM_PLAYERS).map(\|p\| vec![i32::from(self.winners().contains(&p))])` |
| `rust/game/roll-through-the-ages-2/src/lib.rs` | `self.scores().into_iter().map(\|s\| vec![s])` |
| `rust/game/starship-catan-1/src/lib.rs` | `(0..2).map(\|p\| vec![self.player_boards[p].victory_points()])` |
| `rust/game/seven-wonders-1/src/lib.rs` | `(0..self.players).map(\|p\| vec![self.player_vp(p), self.coins[p]])` |
| `rust/game/alhambra-1/src/lib.rs` | `(0..self.human_players).map(\|p\| vec![self.boards[p].points])` |

The other eight crates have a single private `placings()`/`calc_placings()`
that both `status()` and `finish_epilogue` call, which makes drift structurally
impossible. In these five the two expressions are currently identical - I
compared each pair - but the invariant "the placings in the log equal the
placings in `Status::Finished`" is held only by the same code being typed twice.
Those two values are compared by clients and consumed by the rating code.

Remediation: extract the metric into each crate's own
`fn placings(&self) -> Vec<usize>` and call it from both sites. Note that
roll-through-the-ages-2's own new test already asserts the two agree - the
author saw the hazard and pinned it with a test; make it structural instead.

### F-20 (Low) `starship-catan-1`'s epilogue hardcodes `0..2` beside `0..self.players`

`rust/game/starship-catan-1/src/lib.rs`, inside `finish_epilogue` - two adjacent
statements disagree about how many players there are:

```rust
let scores: Vec<(usize, i32)> = (0..self.players)...
let placings = gen_placings(&(0..2).map(...)...);
```

Starship Catan is 2-player-only so the ranges are equal today, and the magic `2`
is inherited rather than introduced (the spec forbade improving it). But WP-08's
whole point was to create one named function that *is* the definition of the
epilogue, which was the moment to make the two agree. If `player_counts()` ever
gains a value, `placings` would be short by `self.players - 2` entries and
`placings_log` would under-report winners.

Remediation: use `0..self.players` in both, or `debug_assert_eq!(self.players, 2)`.

### F-21 (Low) `acquire-1` is the only migrated crate with no regression test, and the reasoning that made that safe is unrecorded

`rust/game/acquire-1/src/lib.rs:247-262`.

WP-08b moved acquire-1's epilogue out of the `Command::Done` arm into the shared
trailing `.map(|(mut logs, can_undo)| ...)`, so it now fires for any of the 9
arms that transitions to finished. The spec set this crate's test column to `n`
on the grounds that "Behaviour is identical today - `end_turn`/`end()` is
reachable only from `Done`", and the test census confirms acquire-1's `#[test]`
set is byte-identical before and after (14 functions, 1561 -> 1562 lines) while
the other 12 crates gained 16 tests between them.

I verified the spec's claim rather than taking it: `handle_end_command`
(`:1167`) only sets `self.last_turn = true` and cannot finish the game;
`handle_buy_command` (`:608-651`) does not auto-end the turn even at
`remaining == 0`; only `handle_done_command` (`:658`) reaches `end_turn`. Every
handler opens with `assert_not_finished()`, so `was_finished` is in fact always
`false` where it is read. **The claim holds - this is not a behaviour change.**

The finding is that nothing in the tree records or enforces it. The safety of
the widening rests on three separate properties of unrelated functions, in a
crate whose finish path also carries the WP-19 fixes. Make one of `Buy` or
`Keep` end the turn - a plausible future change - and acquire-1 silently starts
emitting a placings log from a new arm with no test to notice.

Remediation: add the two tests every sibling crate got (finish via `done`:
exactly one placings log, last, `can_undo` false, scores equal `status()`'s
placings; plus a non-finishing command emitting none). Cheap, and it is the only
untested call site in the package.

## Verified good

### The migration shape

All 13 crates were migrated to the same shape, and it is the right one:

```rust
let was_finished = self.is_finished();          // rtta: self.finished
let (mut logs, can_undo, remaining) = match output { ... };
if !was_finished && self.is_finished() {
    self.finish_epilogue(&mut logs);
}
Ok(CommandResponse { logs, can_undo, remaining_input: remaining.to_string() })
```

- The `!was_finished` transition gate is a genuine fix (finding e F14), not
  cosmetic: it makes the epilogue fire once per false->true transition instead
  of once per command-while-finished.
- Hoisting out of the per-arm bodies means arms that could finish the game but
  carried no epilogue now get one. The spec called this out as intended
  (greed-2's `Score` arm; 12 of starship-catan-1's 17 arms) - a latent-bug fix,
  not a regression.
- Per-arm `can_undo` was lifted verbatim. I checked every arm in every crate
  against the arm it replaced: no flips. texas-holdem-2's `Raise` - the only
  `true` in the package - survives and is asserted by its new test.
- The `Err(e)` arms kept their existing, *non-uniform* shapes rather than being
  normalised (`splendor-2` and `love-letter-2` propagate the parse error
  unwrapped; the rest wrap it in `invalid_input`). Correct: normalising would
  have been an unmandated behaviour change.

### Per-crate equivalence - all 13 verified

- **greed-2** - `placings()` (single key `scores[p]`) untouched; epilogue a
  verbatim lift. `Score` arm gains coverage, per spec. Equivalent.
- **sushi-go-2** - `placings()` untouched, two-key
  `[player_points, pudding_cards]`, both higher-is-better, so `gen_placings`'
  descending order gives the official "most puddings" tiebreak. Score tail still
  prints `player_points` only, as before. Equivalent.
- **roll-through-the-ages-2** - 11 identical epilogue copies collapsed to one;
  metric (`board.score()`, single key) identical; predicate is the `self.finished`
  field, per spec. Equivalent.
- **love-letter-2** - 8 arms collapsed; epilogue calls `self.placings()`
  (`player_points as i32`). Equivalent. `remaining` is now `.to_string()`d per
  arm rather than once at the tail - one extra allocation on the success path,
  immaterial.
- **seven-wonders-1** - two-key `[player_vp(p), coins[p]]`, both
  higher-is-better, identical to the pre-commit expression and to `status()`.
  Equivalent.
- **alhambra-1** - `[boards[p].points]` over `0..human_players`, identical and
  matching `status()`. Equivalent.
- **splendor-2** - `placings()` untouched, so the two-key prestige/tiebreak
  comparison is byte-for-byte the old behaviour. Equivalent.
- **texas-holdem-2** - `placings()` (`player_total_money`) untouched; `Raise`'s
  `can_undo: true` preserved and pinned. Equivalent.
- **age-of-war-2** - epilogue calls `calc_placings()`; score tail
  `scores() as i32`. Equivalent, **and** this is the one crate where the
  transition gate is load-bearing at runtime (the Go `can_roll` quirk keeps
  accepting `roll` after finish). Its new
  `post_finish_roll_appends_zero_placings_logs` test drives a real post-finish
  command and asserts zero placings logs - the spec's acceptance criterion for
  e F14, met exactly.
- **jaipur-2** - verbatim lift, including the pre-existing oddity that the
  placings metric is a *boolean* (`winners().contains(&p)`) while the score tail
  is total token value, and that one function mixes `0..2` and `0..NUM_PLAYERS`.
  The spec explicitly ordered this kept verbatim. No behaviour change.
- **zombie-dice-2** - epilogue calls `self.placings()` (`scores[p]`).
  Equivalent.
- **starship-catan-1** - metric (`victory_points()`) identical; arm coverage
  deliberately widened 5 -> 17 per spec, which is a fix (previously a finish
  reachable through 12 of the arms would have been announced by nothing).
  Equivalent modulo the intended widening. See F-20 for the `0..2` nit.
- **acquire-1** - metric unchanged; the arm-coverage widening is a genuine no-op,
  verified against `handle_end_command`, `handle_buy_command` and
  `assert_not_finished` (see F-21). Equivalent.

### `status()` and the finish log agree in all 13 crates

I compared, per crate, the placings expression used by `status()` against the one
used by `finish_epilogue`. All 13 match - eight via a shared private
`placings()`/`calc_placings()` helper, five by having the same expression written
out twice (F-19). No crate can announce one set of placings in the log and report
a different set in `Status::Finished`.

### Tests: nothing was deleted or weakened

Checked explicitly, since a dedup is a classic place to quietly drop tests. The
`#[test]` function set at `f13450a1~1` vs `c14bc655`, all 13 crates: **no test
function was removed or renamed by either commit.** Twelve of 13 gained tests
(16 new test functions), and they assert the spec's section-5 acceptance
criteria rather than just "it compiles": exactly one placings log, it is last,
`can_undo` unchanged, non-finishing commands emit none, and in several crates
`status()`'s placings equal the crate's own `placings()`. Multi-arm crates test
two different finishing paths, as the spec required. acquire-1 gained none
(F-21).

Every line-count drop (alhambra -29, RTTA -79, starship-catan -63,
seven-wonders -2, love-letter -7) is accounted for by collapsed match arms; each
of those crates has *more* test code after than before.

One later deletion, outside this unit: `sushi-go-2` lost `test_hand_passing_left`
between `c14bc655` and HEAD. That belongs to WP-24 (`66053159`) / Unit 04 -
flagged only so Unit 04 checks it.

### Post-WP-08 drift is benign

Seven of the 13 crates were touched again after `c14bc655`. I checked the commit
subjects: `2c28ae85` (WP-65 workspace hygiene) touches six of them and is
dependency metadata; the rest are their own work packages (WP-27, WP-33, WP-24,
WP-18, WP-31, T3-B3, T3-B4, `ae04843c`). No later commit re-touched a
`finish_epilogue` body, and all 13 still have the migrated shape at HEAD.

## Coverage gaps

- **Undo interaction not traced.** The gate makes the epilogue fire once per
  forward transition, but undo restores a prior state; a game undone back across
  the finish boundary and re-finished will append a second placings log as a new
  entry. Probably correct, but WP-40 / Unit 06 owns the undo path and should
  check it against this gate.
- **`gen_placings` with ragged metric vectors not exercised.** No crate emits
  differing arity per player, so it is latent, but `cmp_fallback`'s "empty is
  `Less`" rule means `[5]` loses to `[5, -100]`, which is surprising and
  undocumented. Not a WP-08 regression; noted for whoever next touches
  `gen_placings`.
- **Score-tail *correctness* not assessed.** I verified each crate's score tail
  is unchanged from before the commit, not that it is right. Several crates print
  a different quantity in the tail than they rank by (jaipur prints token totals
  but ranks by a boolean; sushi-go prints points but ranks by points+puddings).
  All pre-existing; rules parity is parked per SUMMARY.md.
- **`rust/web` consumers not read.** `rust/web/src/game/placing.rs`,
  `db/rating.rs`, `db/games.rs`, `game/import.rs` and `email/render.rs` all
  consume placings. `gen_placings`' output contract (1-based, ties consume
  places) is untouched by these commits so no consumer contract moved, but I did
  not read them. Units 05-08.
- **The four unmigrated duplicated crates (F-18) were not reviewed**, only
  counted. Units 03/04 own `sushizock-2`, `category-5-2`, `for-sale-2` and
  `farkle-2`.
