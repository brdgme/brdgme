# Unit 04c review - Unit 04 cleanup + parity commits, plus WP-33

Reviewer: Lead 04c. Read-only; no tests/lints run; no source modified.

## Scope

- **(a) Cleanup + parity commits**: `63f4aa91` (WP-81 dead stats-machinery
  deletion), `650e924e` (WP-83 parity fixes: roll-through-the-ages-2,
  seven-wonders-1, red7-1).
- **(b) `abffb7aa` (WP-33)**: `farkle-2`, `greed-2`, `liars-dice-2`,
  `no-thanks-2`, `tic-tac-toe-2` - coverage hole closed here.
  `62b293df` confirmed to touch no `rust/` files (docs only) - skipped.
- **(c)** Game-crate coverage table, below.
- Findings numbered from **F-78**.

Acceptance criteria recovered: `specs/WP-81-stats-deletions.md`,
`specs/WP-83-parity-fixes-released.md`, `checklists/T3-B2-small-game-crates.md`
(WP-33 = 17 rows across the five crates; **WP-33 has no spec file anywhere in
history** - the T3-B2 rows are the only criteria).

(Findings appended below as confirmed.)

## Findings

### F-78 (Low) - WP-33's overflow fix (`f F36`) landed in `greed-2` only, and even there skipped the identical add one line above

`rust/game/farkle-2/src/lib.rs:260`, `rust/game/farkle-2/src/lib.rs:310`,
`rust/game/greed-2/src/lib.rs:365`

`f F36` asked for `saturating_add` on "the `turn_score` and `scores[player]` i32
accumulations". In `greed-2` both assignments were converted
(`:309`, `:368`). But:

- `greed-2:365` still builds the log text with a raw
  `(self.scores[player] + self.turn_score).to_string()` - the *same* addition,
  three lines above its own `saturating_add`. On overflow this panics in debug
  and wraps in release, and in release the log then disagrees with the state
  (which is correctly clamped).
- `farkle-2` is a near line-for-line twin of `greed-2` (same `Game` fields,
  same `score`/`done`/`bust`/`start_turn` shape) and received **neither** half:
  `farkle-2:260` is `self.turn_score += value` and `farkle-2:313` is
  `self.scores[player] += self.turn_score`, plus the same raw add in the log at
  `:310`. The checklist scoped `f F36` to `greed-2` alone, and nothing swept
  the sibling.

Why it matters: neither crate's `validate` (`farkle-2:481-502`,
`greed-2:418-439`) bounds `scores[..]` or `turn_score` at all - they only check
`scores.len()`, `current_player` and `first_player`. So a persisted/crafted
state with `scores[p] == i32::MAX` reaching `done()` is exactly the D-36
deserialized-state path, and it hits an unguarded `+`.

Remediation: use `saturating_add` for the log expression in both crates, and
apply the `f F36` fix to `farkle-2:260` and `farkle-2:313`. Better: compute
`let banked = self.scores[player].saturating_add(self.turn_score);` once and use
it for both the log and the assignment.

### F-79 (Low) - `farkle-2` `scoring_table` (`f F38`) fix is partial and its new test re-hardcodes what the row asked to remove

`rust/game/farkle-2/src/render.rs:26-46`, test at `:100-128`

`f F38`: "Derive the rendered table from `scores()`/`SCORES` instead of
hardcoding the eight combinations, keeping only display names local".
The implementation still hardcodes all eight dice combinations *and* the eight
display names, and derives only the point values:

```rust
let pts = scores().iter().find(|s| s.dice.as_slice() == *dice).map(|s| s.value).unwrap_or(0);
```

Three problems:
1. The combination list is still hardcoded, so a combination added to `SCORES`
   silently never appears in the rendered table - the drift the row was about.
2. `unwrap_or(0)` silently renders `0` points if a combination is changed or
   removed from `SCORES`, converting drift into wrong output rather than a
   failure.
3. The new test `test_scoring_table_matches_legacy_output` rebuilds the **old
   hardcoded `(&str, i32)` list** and asserts equality with it. So the test pins
   the table to the pre-fix literals rather than to `SCORES`; it does not detect
   case 1 at all, and for case 2 it fails with a confusing "legacy" mismatch.

Net effect: one hardcoded list became three (render dice list, render names,
test legacy list) plus a silent-zero fallback.

Remediation: iterate `scores()` and map each `Score` to a display name via a
small `fn combination_name(dice: &[Die]) -> String` (or a `&[(&[Die], &str)]`
name table looked up *from* `scores()`, erroring/`unreachable!` on a missing
name so a new combination cannot be silently dropped). Assert in the test that
`scoring_table().len() == scores().len() + 1`.

### F-80 (Low) - `liars-dice-2`'s new bid cap rejects bids the game's own rules accept, contradicting its acceptance criterion

`rust/game/liars-dice-2/src/command.rs:44-47`, test at `:314-337`

`f F57`: "Set the quantity `Int` parser's `max` to `players * START_DICE_COUNT`
**so help/suggest shows a sensible cap (never rejects a legal bid)**."

The cap was made enforcing, and the commit's own new test asserts it:

```rust
let over_cap = format!("bid {} 6", cap + 1);
parser.parse(&over_cap, &[]).expect_err("a bid above the cap must be rejected");
```

`Game::bid` (`lib.rs:109-159`) accepts any `quantity >= 1` that strictly
increases the bid; bidding above the number of dice in play is a legal (if
losing) bluff under the rules and under `bid()`. The parser and `bid()` now
disagree. The reachable consequence: at `bid_quantity == players *
START_DICE_COUNT` with `bid_value == 6`, every quantity `bid()` would accept is
above the cap, so `command_parser` offers a bid parser that can produce no
accepted bid, and the player gets an `Int`-range parse error instead of a game
rule message. (They can still `call`, so this is not a wedge.)

Secondary note: `f F56`'s stated justification - "this IS reachable from
ordinary input because the bid parser has no quantity cap" - is falsified by
`f F57` landing in the same commit; the `number_str` >999 fallback is now
unreachable from ordinary input (max cap is 6 * 5 = 30). The spelling fix is
still correct, but the two rows were not reconciled.

Remediation: either drop the enforcing behaviour (if `Int` cannot advertise a
soft max, leave `max: None` and document the cap in the `Doc` description) or
accept the cap deliberately and add the corresponding rule check to
`Game::bid` so the two agree, and delete the "never rejects a legal bid"
criterion. Do not leave the parser stricter than the game.

### F-81 (Low) - `no-thanks-2`'s chip redaction is exactly reconstructible from the public log

`rust/game/no-thanks-2/src/lib.rs:117-121` (`pass` log),
`:130-150` (`take` log), `:282-286` (`pub_state`)

`PubState::chips` is documented "Only populated when the game is finished; empty
during play", and `test_pub_state_chips_hidden_until_finished` (`:557`) asserts
it. But every chip movement is emitted as a `Log::public` naming the player:

- `pass` -> "{player} passed on the {card}" - each occurrence is exactly `-1` chip.
- `take` -> "{player} took the {card} and {n} chips" - exactly `+n`.

`STARTING_CHIPS` is a public constant, so
`chips[p] = 11 - passes(p) + sum(taken(p))` is exact from the log stream alone.
This is systemic pattern 3 (no crate tests the log layer): the redaction test
passes while the redaction achieves nothing.

Caveat stated honestly: in physical No Thanks! a player who counts chips placed
on cards can derive the same numbers, so the correct resolution may be to drop
the pretence rather than change the logs. Flagging for an owner ruling, not
asserting a rules violation.

Remediation (pick one): (a) declare chips public - populate `PubState::chips`
during play and delete the "hidden until finished" claim from the field doc,
`DATA_DOCS.md` and the test; or (b) if secrecy is intended, the pass/take logs
must not be per-player public, which changes the game's feel and needs an owner
decision. (a) is the honest, simple option.

### F-82 (Low) - `tic-tac-toe-2` `validate` bounds `players` and `start_player` but not `current_player`, which `status()` publishes

`rust/game/tic-tac-toe-2/src/lib.rs:190-204`, `:258-270`

WP-33's `f F45` correctly removed the `1 - start_player` underflow in
`render_with_labels`, and `validate` bounds `start_player < NUM_PLAYERS`. But
`current_player` is never bounded, and `status()` returns
`whose_turn: vec![self.current_player]` verbatim. A deserialized state with
`current_player: 9` passes `validate`, then hands an out-of-range seat index to
the web layer's turn handling. This is systemic pattern 2b (the override exists
and covers the sweep's shape, but not the neighbouring field on the same render
path).

Remediation: add
`if self.current_player >= NUM_PLAYERS { return Err(GameError::internal(...)) }`
to `validate`.

### F-83 (Medium) - WP-83's `a F1` fix does not fix the primary case the finding describes, and its new test was written to the code instead of to the acceptance criterion

`rust/game/roll-through-the-ages-2/src/lib.rs:741-756`, test at `:3266-3277`

The fix is the spec's literal snippet:

```rust
let phase_before = self.phase;
logs.extend(self.keep_skulls());
if self.phase == phase_before {
    match phase_before { /* Roll => remaining_rolls -= 1, ... */ }
}
```

`self.phase == phase_before` cannot distinguish "`keep_skulls` did not advance
the phase" from "`keep_skulls` advanced the phase all the way around and landed
back on `Phase::Roll`". The second case is exactly the worst case the finding
cited. Trace it in the current code:

- `keep_skulls` (`:680-687`): all rolled dice are skulls -> `rolled_dice` empty
  -> `next_phase()`.
- `next_phase` (`:257-269`): `Roll -> roll_extra_phase` -> (no Leadership)
  `next_phase` -> `Collect -> Resolve -> Build -> Trade -> Buy -> Discard ->
  next_turn`.
- `next_turn` (`:570-579`) advances `current_player`, then `start_turn`
  (`:250-254`) -> `preserve_phase` (`:272-278`) -> `next_phase` ->
  `roll_phase` (`:281-287`), which sets `self.phase = Phase::Roll` and
  `self.remaining_rolls = 2` **for the new player**.
- Back in `roll`, `self.phase == phase_before` is now `Roll == Roll` -> **true**,
  so the stale `match` runs and decrements the *new* player's
  `remaining_rolls` from 2 to 1.

That cascade requires every intermediate phase to auto-skip, which happens when
the rolling player has nothing to do - an all-skull reroll leaves no workers, so
`build_phase` skips; with no goods `trade_phase` skips; with no coins
`buy_phase` skips; `discard_phase` skips under the goods limit. That is a
routine early-game situation, not a contrived one.

The commit's new test conceals this. The spec prescribed: "Assert
`current_player == 1` **and `remaining_rolls == 2`** (today: 1)." The test that
landed asserts the opposite of the first half:

```rust
assert_eq!(MICK, g.current_player);   // MICK == 0, i.e. player did NOT change
assert_eq!(Phase::Buy, g.phase);
assert_eq!(2, g.remaining_rolls);
```

`MICK` is seat 0 (`:1676`) and `new_blank` starts at `current_player: MICK`
(`:1689-1699`), so the test chose a scenario in which the cascade stops at
`Phase::Buy` - the one shape where the `==` guard happens to work - and
recorded that as the expected behaviour. This is the third instance in this
review of a fix whose test was adjusted to agree with the code rather than with
the criterion the finding set (cf. Unit 04b's two cases).

Why it matters: a player silently starts their turn with 1 roll instead of 2,
with no log and no error. `remaining_rolls` is `i32` (`:68`) and `can_roll`
(`:151`) requires `remaining_rolls > 0`, so there is no panic or underflow -
just a wrong, valid-looking state. `roll-through-the-ages-2` also has **no
`validate` override** (F-06 list), so nothing downstream would notice.

Remediation: stop inferring "did it advance" from the phase value. Make
`keep_skulls` report it, e.g.
`fn keep_skulls(&mut self) -> (Vec<Log>, bool)` returning whether it called
`next_phase`, and gate the block on `!advanced`. A one-line stopgap that closes
the traced case is to also compare the player -
`if self.phase == phase_before && self.current_player == player_before` - since
the cascade always passes through `next_turn`; but the explicit boolean is the
correct and readable fix. Then restore the spec's test: assert
`current_player == 1` **and** `remaining_rolls == 2` for the full-cascade case,
in addition to keeping the existing `Phase::Buy` case.

### F-84 (Low) - WP-83's `b F7` test covers 3-4 players; `seven-wonders-1` supports 3-7, and 7 is the boundary case

`rust/game/seven-wonders-1/src/lib.rs:1748-1774` (test), `:22-23`, `:122-143`

The spec asserted "`start_game` does ... with `MAX_PLAYERS = 4`". That is wrong:
`MIN_PLAYERS = 3`, **`MAX_PLAYERS = 7`** (`:22-23`). The implementer correctly
deviated from the spec's `2..=4` loop (2 is not a legal count) but kept the
upper bound at 4, so the test exercises `3..=4` and leaves 5, 6 and 7 uncovered.

7 players is the interesting case: `cities()` yields 14 entries, the grouping
yields exactly 7 boards, and `boards[..players]` then consumes every single
group. That slice index is a raw index into a length derived from card data -
if `cities()` ever gained or lost an entry, or a name stopped ending in `" A"`/
`" B"` in a way that merged two groups, a 7-player game would panic at setup.
The fix is correct today (and matters more than the spec thought - with 14
cities shuffled flat and 7 dealt, a duplicate board was near-certain at 7
players), but nothing tests the boundary.

Remediation: change the test loop to `for players in MIN_PLAYERS..=MAX_PLAYERS`,
and replace `boards[..players]` with a checked form that returns
`GameError::internal` (or `PlayerCount`) if `boards.len() < players`, so a card
-data change fails loudly instead of panicking.

## Verified good

### `63f4aa91` (WP-81) - dead stats-machinery deletion

Checked line-by-line against `specs/WP-81-stats-deletions.md`. The
`lost-cities-1`/`-2` half is 04a's; the `acquire-1` half is reviewed here.

- `rust/game/acquire-1/src/stats.rs` deleted outright, not left as a husk, as
  §2 required. `mod stats;`, `use crate::stats::Stats;`, the `Player.stats`
  field and `stats: Stats::default()` in `impl Default for Player` all removed.
- All eleven `self.players[..].stats.*` mutation sites removed from
  `handle_found_command`, `handle_buy_command`, `handle_merge_command`,
  `pay_bonuses`, `sell` and `handle_trade_command`, and **only** those
  statements - every surrounding gameplay effect (`money -=`, `money +=`,
  `take_shares`, `return_shares`, `extend_corp`, phase transitions,
  `let mut can_undo = true;`) is intact. No binding used only by a deleted
  stats line was left dangling and no binding still needed was removed
  (`price`, `corp`, `major_per`, `minor_per`, `money`, `n`, `receive`, `into`
  all still consumed by surviving code).
- `status()`'s `stats: vec![]` untouched, as §2 required (and as F-35/WP-20
  parks).
- No `RULES.md` touched, matching §4's scope guard; option A (wiring stats up)
  was not substituted.
- No manifest change, matching §2's collateral note.
- §5's verification greps re-run and all return zero:
  `to_brdgme_stats` across `rust/` = 0; `mod stats|stats::Stats|\.stats\.` in
  `rust/game/acquire-1/` = 0; `investments|expeditions +=` across both
  lost-cities crates = 0.
- Persisted-state safety confirmed, not assumed:
  `rust/game/acquire-1/src/lib.rs:1199` is
  `#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]` with **no**
  `#[serde(deny_unknown_fields)]`, so old rows carrying a `stats` object
  deserialize fine with the field gone. §2's claim holds.

### `liars-dice-2` post-call round starter matches its own RULES.md

Checked because the code's behaviour (`lib.rs:208-211`: next round starts with
`next_active_player(caller)`) differs from standard Liar's Dice, where the
player who lost the die starts. `rust/game/liars-dice-2/RULES.md:37` states it
explicitly: "The player who lost a die does not start the next round; the next
active player (clockwise from the caller) starts." Code and documented rules
agree, and WP-33 did not touch this file, so this is pre-existing documented
intent, not a doc edited to match code. Any disagreement with the published
rules is a parked rules question, not a WP-33 defect.

### `650e924e` (WP-83) - `red7-1` `e F30` empty-winning-set tie-break

`rust/game/red7-1/src/card.rs:297-330`, `rust/game/red7-1/src/lib.rs:237-250`

Implements the full three-part key the spec prescribed - `(winning_set.len(),
max rank_key of winning set, max rank_key of full palette)` - as a lexicographic
tuple compare, keeps the strict `>` and hence the seat-order final fallback, and
still returns the *winning set* rather than the palette (so
`leader_with_suit`'s contract is unchanged). `l_key`'s third component correctly
reads `entries[leader_idx].1`, i.e. the current leader's own full palette, not a
stale one. `leader_with_suit` pushes `(rule_fn(&palettes[p]), palettes[p])` as
specified. Both prescribed tests are present, plus the existing `test_leader`
was migrated to the new signature without weakening its assertions. The
documented DATA_DOCS tie-break is now actually implemented.

### `650e924e` (WP-83) - `seven-wonders-1` `b F7` duplicate wonder board

`rust/game/seven-wonders-1/src/lib.rs:122-143`

Groups the 14 A/B `City` entries into boards by stripped name prefix, shuffles
the *boards*, takes `players`, then picks one side per board - exactly the
prescribed shape. `BTreeMap` (not `HashMap`) is used, so grouping order is
deterministic and seeded games stay reproducible, which was the spec's explicit
warning. `random_range(0..sides.len())` cannot receive an empty range because
groups are only created by pushing. The prescribed test is present and does
assert the real property (board names distinct across many seeds), not a
weaker proxy.

### `abffb7aa` (WP-33) rows verified correct

- **`greed-2 f F32` / `farkle-2 f F39`** - `score` now returns
  `invalid_input("can't play at the moment")` when `player !=
  self.current_player`, in both crates, with a test each. The guard is safe on
  `greed-2`'s internal call path: `done` requires `can_done(player)` which
  requires `player == self.current_player` (`greed-2/src/lib.rs:234-236`), so
  the auto-scoring `while` loop at `:353-358` cannot trip its own new guard.
  That loop also terminates - each `score` call removes a non-empty multiset
  from `remaining_dice`.
- **`greed-2 f F37`** - `Die::E1.color()` really is
  `NamedColor::Foreground` (`greed-2/src/lib.rs:64`), so the RULES.md table now
  matches the code. Nit only: "foreground" is opaque to a player reading the
  rules; "the default text colour" would read better. Recorded as a
  doc-follows-code change that the checklist row explicitly directed, not a
  finding.
- **`farkle-2 f F40`** - `pub_state` zeroes `turn_score` and empties
  `remaining_dice` when finished, computing `finished()` once instead of twice.
  Test present.
- **`farkle-2 f F41`** - `g.current_player = (g.first_player + 1) % 3` - the
  out-of-range index in the test is gone.
- **`farkle-2 f F42`** - `render_die`/`render_dice` now take `Die` / `&[Die]`.
  Cosmetic (it is `pub type Die = u8`) but that is all the row asked for.
- **`tic-tac-toe-2 f F47`** - `Cell::Empty => unreachable!(...)` is genuinely
  unreachable: `matching_line` (`:146-148`) starts with
  `line[0] != Cell::Empty &&`, so it can never return `Some(Cell::Empty)`.
  This is the *better* of the two options the row offered, and unlike F-65's
  pattern it replaces a silently-wrong default (`Cell::Empty =>
  self.start_player`, i.e. "the X player wins an empty line") rather than
  papering over a live fallback.
- **`tic-tac-toe-2 f F45`** - `(start_player + 1) % NUM_PLAYERS` replaces
  `1 - start_player`; underflow gone, test present.
- **`tic-tac-toe-2 f F48`** - casing is now consistent: board cells `X`/`O`
  (`render.rs:65-66`), the "is X, is O" label, RULES.md, and the `play` log
  (`lib.rs:107-111`, which was already uppercase - that is why the diff shows
  no `play` change). The exact-render markup assertion was updated in step.
- **`no-thanks-2 f F49`** - `test_init_player_chips` now sets `players: 3` and
  asserts `player_chips.len() == players`, so the loop body actually runs.
- **`no-thanks-2 f F51`** - the "no chips left" message is now reachable and
  covered by `test_pass_no_chips_left_message`. The row's "fold into the single
  `can_pass` check" was implemented as a nested re-derivation inside the
  `!can_pass` arm rather than a literal fold; the message is emitted and chips
  are indexed the same defensive `.get(..).copied().unwrap_or(0)` way, so the
  row's intent is met.
- **`no-thanks-2 f F52`** - one `pub fn group_runs` in `lib.rs:191-210`, called
  from both `player_hand_grouped` and `render::group_sorted`. Genuine dedup.
- **`liars-dice-2 f F56`** - "fourty" -> "forty" with the test updated (see
  F-80 for the interaction with `f F57`).
- **`liars-dice-2 f F58`** - all three prescribed tests exist:
  redaction, the wild-1 call branch (arithmetic checked by hand: one matching
  die vs a bid of 2, so the bidder loses - assertions correct), and a
  play-to-completion `placings`/`status` assertion.

### `62b293df`

Confirmed: 3 files, all under `docs/reviews/2026-07-23-rust-review/planning/`,
zero `rust/` changes. No code review needed.

### Nits recorded, not raised as findings

- `liars-dice-2` declares `MIN_BID_QUANTITY`, `MIN_BID_VALUE` and
  `MAX_BID_VALUE` (`lib.rs:20-22`) but `Game::bid` (`:119-139`) hardcodes `1`
  and `(1..=6)` instead of using them; only `command.rs` consumes the
  constants. Pre-existing, outside every WP-33 row.
- `no-thanks-2` uses `self.player_chips.get(player).copied().unwrap_or(0)`
  in `can_pass`/`pass` but raw `self.player_chips[player]` in
  `final_player_score`/`player_state`. Both are safe given `validate`, but the
  inconsistency is the shape systemic pattern 2 warns about.

### Crate-level checks (WP-33 crates)

- **F-06 / `validate` overrides**: all five crates override `Gamer::validate`
  (`farkle-2:481`, `greed-2:418`, `liars-dice-2:252`, `no-thanks-2:236`,
  `tic-tac-toe-2:190`). All five reject `players == 0` implicitly, because each
  bounds an index field with `>= self.players`, and `0 >= 0` holds.
- **Raw indexing on the render path**: `farkle-2` `placings()`/`points()` and
  `greed-2` `placings()` index `scores[p]` over `0..players` - covered by the
  `scores.len() != players` check. `liars-dice-2` `pub_state`/`placings`/
  `active_players`/`next_active_player` index `player_dice[p]` over
  `0..players` - covered by the `player_dice.len() != players` check.
  `no-thanks-2` `player_state` indexes `player_chips[player]` and
  `player_hand_*` index `player_hands[player]` - both lengths checked. The one
  gap found is F-82 (`tic-tac-toe-2` `current_player`).
- **`Log::public` audit**: hand-audited every site in all five crates.
  `farkle-2`, `greed-2` and `tic-tac-toe-2` have no per-player hidden state at
  all, so their public logs cannot leak. `liars-dice-2`'s only reveal is
  `render::reveal_table` inside `call()` (`lib.rs:192-207`), which is the
  legitimate simultaneous reveal, emitted *before* `start_round()` re-rolls, so
  it shows the dice that were actually called - correct. `no-thanks-2` is the
  one leak: F-81.
- **F-18 (unmigrated epilogue) - `farkle-2` confirmed.** `farkle-2` has **no**
  `finish_epilogue` helper and carries the epilogue inline **twice**, copy-pasted
  verbatim, in the `Roll` and `Done` arms of `command`
  (`lib.rs:425-433` and `:446-454`), each gated only on `if self.is_finished()`
  with no `!was_finished`. `greed-2` by contrast does have
  `fn finish_epilogue` (`greed-2/src/lib.rs:381-389`) - consistent with Unit
  01c's finding, not re-reviewed. Mitigating detail worth recording for the
  remediation plan: in `farkle-2` a double epilogue is not actually reachable,
  because `player_roll`/`done` are gated on `can_roll`/`can_done`, both of
  which require `!self.finished()`. The defect is the duplication and the
  missing migration, not a live double-log.
- **Redaction tests (systemic pattern 4)**: of the five WP-33 crates only
  `liars-dice-2` and `no-thanks-2` have a redaction-asserting test, and F-81
  shows `no-thanks-2`'s is defeated by the log layer. `farkle-2`, `greed-2` and
  `tic-tac-toe-2` genuinely have no per-player hidden state, so their absence is
  correct rather than a gap.
- **F-35 (parked, record only)**: `Status::Finished { stats: vec![] }` occurs
  at `farkle-2:355`, `greed-2:443`, `greed-2` (second site per `00-sweeps.md`),
  `liars-dice-2:278`, `no-thanks-2:257`, `tic-tac-toe-2:260` (plus two test-only
  sites at `:515`/`:530`). No fixes demanded; recorded per the WP-20 park.
- **`roll-through-the-ages-2` `roll()` dice-number guard**: `lib.rs:724` is
  `if n < 0 || n > l`, which accepts `n == 0` while its own message says
  "between 1 and {l}". Checked rather than assumed: the parser is
  `Int::bounded(1, max_i)` (`command.rs:388`) and `fn roll` is private, so `0`
  is unreachable. Nit only - the guard should read `n < 1` to match its
  message.

### Carry-forward answered: `for-sale-2`'s half-bid rounding IS inside the WP-11 park

The brief asked for confirmation, not assumption. Recovered from
`868094a6:docs/reviews/2026-07-23-rust-review/planning/work-packages.md`:

> ### WP-11 batch-f port-parity adjudication - BLOCKED-ON-USER-RULES-REVIEW
> (D-30 + D-35 PARKED 2026-07-25 - do not pick up)
> - Scope (8): f F2, f F14, f F15, f F21, f F33, f F43, f F50, f F54
> - Paths: game/{zombie-dice-2,for-sale-2,greed-2,farkle-2,no-thanks-2,liars-dice-2}/src
>   + RULES.md files, lib/game/src/game.rs (F21 gen_placings)

and `planning/raw/w4-gamesf-webserver.md:19`:

> games-batch-f F14 | minor | Passing pays floor(bid/2); official rules round up
> (Go quirk) | game/for-sale-2/src/lib.rs, game/for-sale-2/RULES.md | D |
> port-parity

Corroborated by `planning/findings/verification/games-batch-f-LOG.md:20` and
`planning/findings/raw/consolidation-units-7-9.md:200`. **Confirmed: parked as
`f F14` under WP-11, deliberately not fixed.** Not a remediation gap. Note for
the unified report: WP-11 also parks parity items in `greed-2`, `farkle-2`,
`no-thanks-2`, `liars-dice-2` and `zombie-dice-2` - four of the five WP-33
crates - so no parity observation in those crates should be raised without
first checking `f F2/F15/F21/F33/F43/F50/F54`.

## Game-crate coverage

28 directories under `rust/game/`. Column values were derived by locating each
report's `## Verified good` section by line range and grepping only that range:
`V` = named in that report's `## Verified good`, `m` = mentioned elsewhere in
the report only, `-` = absent.

| Crate | 01c | 02 | 03a | 03b | 04a | 04b | 04c | 11 | Primary owner |
|---|---|---|---|---|---|---|---|---|---|
| acquire-1 | V | - | V | m | - | - | V (WP-81) | - | **03a** |
| age-of-war-2 | V | - | - | - | V | m | - | - | **04a** |
| alhambra-1 | V | V | - | m | - | - | - | - | **02** |
| battleship-2 | - | - | - | - | m | V | - | - | **04b** |
| category-5-2 | m | - | - | - | m | V | - | - | **04b** |
| cathedral-2 | - | - | m | V | - | - | - | - | **03b** |
| farkle-2 | m | - | - | - | - | m | **V** | - | **04c** |
| for-sale-2 | m | V | - | - | m | V | - | - | **04b** |
| greed-2 | V | - | - | - | - | m | **V** | - | **04c** |
| hanamikoji-1 | - | - | - | - | - | - | - | (Unit 11) | **11 - not yet done** |
| jaipur-2 | V | - | m | V | - | - | - | - | **03b** |
| liars-dice-2 | - | - | - | - | - | m | **V** | - | **04c** |
| lords-of-vegas-1 | - | - | m | V | - | - | - | - | **03b** (WIP crate) |
| lost-cities-1 | m | - | - | - | V | - | V (WP-81) | - | **04a** |
| lost-cities-2 | - | - | - | - | V | m | V (WP-81) | - | **04a** |
| love-letter-2 | V | - | - | - | V | m | - | - | **04a** |
| modern-art-2 | m | V | m | - | - | m | - | - | **02** |
| no-thanks-2 | - | - | - | - | - | m | **V** | - | **04c** |
| red7-1 | m | - | - | - | m | V | V (WP-83) | - | **04b** |
| roll-through-the-ages-2 | V | - | - | - | - | - | **V (WP-83 only)** | - | **04c - partial only** |
| seven-wonders-1 | V | V | m | - | - | - | V (WP-83) | - | **02** |
| splendor-2 | V | - | V | m | - | - | - | - | **03a** |
| starship-catan-1 | V | V | m | - | - | - | - | - | **02** |
| sushi-go-2 | V | - | - | - | V | - | - | - | **04a** |
| sushizock-2 | m | - | m | V | - | - | - | - | **03b** |
| texas-holdem-2 | V | - | V | m | - | - | - | - | **03a** |
| tic-tac-toe-2 | m | - | - | - | - | m | **V** | - | **04c** |
| zombie-dice-2 | V | V | - | - | m | V | - | - | **04b** |

Every one of the 28 crates now has an owning sub-unit. Two qualifications, both
in the next section.

## Coverage gaps

1. **`roll-through-the-ages-2` has never had a crate-level review.** Its only
   coverage is (a) 01c's epilogue-shape check and (b) this unit's review of the
   single function WP-83 touched (`roll`). It is a 3,290-line crate - the
   largest game crate in the tree - and nothing in the whole review has read
   its command surface, its `pub_state`/`player_state`, its scoring, or its
   `Gamer` impl. It also has **no `validate` override** (F-06 list) and **no
   redaction test** (pattern 4), and F-83 shows the one function that *was*
   reviewed contained an unfixed state-corruption bug. Recommend a dedicated
   pass. It was never assigned because `00-breakdown.md` reached it only via
   WP-83's parity row.
2. **`hanamikoji-1` is unreviewed** - correctly, it is Unit 11's, which has not
   started. Its known carry-forward (single unguarded epilogue at `:833`, no
   `finish_epilogue`) stands.
3. **`lords-of-vegas-1`** is covered by 03b but is a WIP crate by owner ruling;
   no completeness claim is made for it.
4. Not a gap but worth stating: `01c`'s `V` marks are **epilogue-shape only**.
   For `roll-through-the-ages-2`, `greed-2` and `seven-wonders-1` that is the
   only 01c coverage, so a `V` in the 01c column must not be read as a
   crate-level review.
5. Within this unit, `red7-1` and `seven-wonders-1` were reviewed only for their
   WP-83 delta (their crate-level owners are 04b and 02 respectively), and
   `acquire-1`/`lost-cities-*` only for their WP-81 delta.


