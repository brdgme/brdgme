# Unit 04b review - red7-1, zombie-dice-2, battleship-2, for-sale-2, category-5-2

Reviewer: Lead 04b. Read-only; no tests/lints run; no source modified.

## Scope

- Commits reviewed (3 substantive): `071ace6e` (WP-29, red7-1 only, 2 files
  +49/-16), `f16cb02c` (WP-31, zombie-dice-2 + battleship-2, 5 files +81/-40),
  `807ab4e9` (WP-32, for-sale-2 + category-5-2, 6 files +66/-44).
- Commits found to be out of this sub-unit after extraction: `62b293df` touches
  **no `rust/` files at all** (3 files, all under `docs/`) so there is no code
  delta to review; `abffb7aa` (WP-33) touches only `farkle-2`, `greed-2`,
  `liars-dice-2`, `no-thanks-2`, `tic-tac-toe-2` - none of this sub-unit's crates.
- Acceptance criteria recovered from git history: `WP-29-red7-cleanup.md` spec
  (exists) and checklist `T3-B1-zombie-battleship-forsale-category5.md` (WP-31 =
  8 rows, WP-32 = 12 rows). **WP-31, WP-32 and WP-33 have no spec file anywhere in
  history** - the T3-B1 rows are the only acceptance criteria for WP-31/WP-32, so
  each was measured against its stated *purpose*, not its literal instruction.
- Crates: `red7-1`, `zombie-dice-2`, `battleship-2`, `for-sale-2`, `category-5-2`.
- Excluded (sub-unit 04c): `63f4aa91` WP-81 dead-stats deletion, `650e924e` WP-83 parity.
- Findings continue from F-65; this report starts at **F-66**.

(Findings appended below as confirmed.)

## Findings

### F-66 (Medium) - `category-5-2` `choose()` panics on a deserialized state its own `validate` accepts

`rust/game/category-5-2/src/lib.rs:315`

```rust
let played = self.plays[player].expect("choosing player has a played card");
```

`can_choose(player)` is `self.resolving && self.choose_player == player` - it never
looks at `plays[player]`. `validate` (lines 378-436) checks board rows non-empty,
all five parallel-vector lengths, `choose_player` range and every card's value range,
but never the invariant this `expect` depends on: **when `resolving` is true,
`plays[choose_player]` must be `Some`.** A persisted/replayed state with
`resolving: true` and `plays[choose_player]: null` passes `validate`, reaches
`command` -> `choose` and panics.

Why it matters: this is precisely the D-36 deserialized-state trust boundary
WP-09a/WP-09b were meant to close, in a crate that *did* gain a `validate`
override. The override was written to the shape of the parallel-vector sweep
(lengths + indices) and missed the one cross-field invariant the crate's only
remaining `expect` on a command path relies on.

Remediation: add to `validate`

```rust
if self.resolving && self.plays[self.choose_player].is_none() {
    return Err(GameError::internal(
        "category-5-2: resolving with no played card for choose_player",
    ));
}
```

and/or fold the check into `can_choose` so the `expect` becomes unreachable by
construction rather than by documentation.

### F-67 (Medium) - `category-5-2` equal-hand-size invariant is asserted in a comment and then raw-indexed

`rust/game/category-5-2/src/lib.rs:228-241`

```rust
// All hands have equal size by construction (dealt simultaneously each round).
match self.hands[0].len() {
    0 => logs.extend(self.end_round()),
    1 => {
        for p in 0..self.players {
            let card = self.hands[p][0];
            let play_logs = self
                .play(p, card)
                .expect("auto-play should only play valid cards");
```

Two problems:

1. `self.hands[p][0]` is a raw index guarded only by `hands[0].len() == 1`. Nothing
   in `validate` enforces that all hands have the same length, so a state with
   `hands[0].len() == 1` and `hands[1] == []` panics here.
2. `draw_cards` (lines 271-286) is explicitly designed to return **fewer than `n`**
   cards when `deck.len() + discard.len() < n` (the early-return at 272-276). That is
   the one code path in the crate that can legitimately produce unequal hands, and it
   is the path the comment claims cannot exist. It is unreachable with an intact
   104-card deck, but `validate` performs no card-conservation check, so a state with
   a short deck plus discard reaches `start_round` -> short hands -> this panic.

Why it matters: `.expect("auto-play should only play valid cards")` on a command path
plus raw `[0]` indexing is the exact pattern WP-09 was chartered to remove; the
comment satisfies the "document the invariant" instruction without the invariant
being enforced anywhere.

Remediation: enforce equal hand lengths in `validate` (`self.hands.iter().all(|h|
h.len() == self.hands[0].len())`), and replace the auto-play loop's indexing +
`expect` with `if let Some(&card) = self.hands[p].first()` and error propagation.

### F-68 (Medium) - `zombie-dice-2` `take_dice` can panic on `drain(..n)` after its own recovery path fails

`rust/game/zombie-dice-2/src/lib.rs:239-255`

```rust
if self.cup.len() < n {
    logs.push(Log::public(vec![N::text(
        "Not enough dice remaining, returning kept dice to the cup",
    )]));
    let returned: Vec<Dice> = self.kept.iter().map(|dr| dr.dice).collect();
    self.cup.extend(returned);
    self.kept = vec![];
    self.shake_cup();
}
let taken: Vec<Dice> = self.cup.drain(..n).collect();
```

The refill branch is a *best effort*: it returns kept dice and then drains `n`
unconditionally. If `cup.len() + kept.len() < n` the `drain(..n)` range is out of
bounds and panics. With an intact 13-die set this cannot happen
(`cup + kept + current_roll == 13`, `n <= 3`), but `validate` (lines 453-475) checks
only `players >= 2`, `scores.len()`, `current_turn` and `roll_off_players` ranges -
it never checks that the dice are conserved. A persisted state with an empty `cup`,
empty `kept` and empty `current_roll` passes `validate` and panics on the next
`roll` command.

Why it matters: same class as F-66 - a `validate` override that covers the
parallel-vector sweep but not the invariant the crate's remaining panic-capable
operation depends on. Note also that the comment on `take_dice` ("Faithful port of
Go `TakeDice`") documents provenance rather than the precondition.

Remediation: make the drain saturating - `let take = n.min(self.cup.len()); let
taken: Vec<Dice> = self.cup.drain(..take).collect();` - and/or add a dice-conservation
check to `validate`.

### F-69 (Medium) - `for-sale-2` `next_bidder` hangs (infinite loop) on a state `validate` accepts

`rust/game/for-sale-2/src/lib.rs:301-327`

```rust
let remaining = (0..self.players)
    .filter(|p| !self.finished_bidding[*p])
    .count();
if remaining == 1 { ... return logs; }
loop {
    self.bidding_player = (self.bidding_player + 1) % self.players;
    if !self.finished_bidding[self.bidding_player] {
        break;
    }
}
```

`remaining == 0` falls through to the `loop`, which then spins forever because no
index satisfies the break condition. In normal play `remaining` passes through 1 and
the auction resolves, so 0 is unreachable - but a persisted state with
`phase: "buying"` and every `finished_bidding` entry `true` passes `validate`
(lines 400-424 check lengths and `bidding_player < players` only), `can_bid` returns
true, and `bid()` -> `next_bidder()` hangs.

Why it matters: a hang is strictly worse than a panic in this architecture - a panic
surfaces as a `SystemError` response, whereas this pins a game worker at 100% CPU
with no recovery. `for-sale-2` was one of the crates WP-09a hardened, and the
hardening added `.get(..).unwrap_or(..)` to `player_state` (lines 462-465) while
leaving this unbounded loop on the command path untouched.

Remediation: handle `remaining == 0` explicitly (return an internal error, or treat
it as "auction already resolved"), or bound the loop to `self.players` iterations
and error out if no bidder is found.

### F-70 (Low) - `for-sale-2` and `battleship-2` `validate` do not bound the player count, unlike sibling crates

- `rust/game/for-sale-2/src/lib.rs:400-424` - no `MIN_PLAYERS..=MAX_PLAYERS` check on
  `self.players`.
- `rust/game/battleship-2/src/lib.rs:425-447` - no `self.players == NUM_PLAYERS` check.

Compare `rust/game/red7-1/src/lib.rs:563-565`, which does exactly this check first.
Consequences:

- `for-sale-2`: a `players: 1` or `players: 7` state is accepted (all parallel
  vectors are self-consistently sized, and `bidding_player >= self.players`
  incidentally rejects only `players: 0`). `pub_state` then divides by
  `self.players` (lines 445-446) - safe only because 0 is excluded by accident,
  not by design.
- `battleship-2`: `other_player()` (line 188) hardcodes `% NUM_PLAYERS`, so any
  `players != 2` state silently mis-targets `boards`/`left_to_place` while
  `status()`/`placings()`/`points()` iterate `0..self.players`.

Why it matters: `Gamer::validate` is the trust boundary for deserialized state
(D-36). Its per-crate implementations were written inconsistently within the same
work package - the same hardening pass produced a range check in `red7-1` and
omitted it in two crates where the code downstream assumes a fixed player count.

Remediation: add the range/equality check as the first clause of each `validate`.

### F-71 (Low) - F-18's "unmigrated epilogue" crate list is incomplete: `battleship-2` has the same shape

`rust/game/battleship-2/src/lib.rs:531-537` carries the same copy-pasted
`if self.is_finished() { ... placings_log(...) }` epilogue as `for-sale-2`
(lines 490-496, 508-514, 526-532) and `category-5-2` (lines 504-510, 522-528),
with no `was_finished` transition gate. Contrast `zombie-dice-2`
(`rust/game/zombie-dice-2/src/lib.rs:548,562-564`), which is fully migrated:
`let was_finished = self.is_finished();` before dispatch and
`if !was_finished && self.is_finished() { self.finish_epilogue(&mut logs); }` after.

Confirmed per crate for F-18: **for-sale-2 - unmigrated; category-5-2 - unmigrated;
battleship-2 - unmigrated (new, not on F-18's list); zombie-dice-2 - migrated;
red7-1 - unmigrated but pre-ruled out of scope (untouched by WP-08).**

No double-fire is currently reachable in any of the three: each crate's
`command_parser` returns `None` once the game is finished, so a second command is
rejected before the epilogue. This is therefore a maintainability finding, not a
live bug - but it means F-18's scope statement understates the remaining work by
one crate.

Remediation: extend WP-08's `was_finished` + `finish_epilogue` migration to
`battleship-2` alongside `for-sale-2` and `category-5-2`.

### F-01 (High) - re-confirmed, `for-sale-2` permanent wedge

`rust/game/for-sale-2/src/lib.rs:138-150`:

```rust
fn start_buying_round(&mut self) -> Vec<Log> {
    let n = self.players;
    if self.building_deck.len() < n {
        return vec![];
    }
```

Confirmed as filed. Additional detail for the fix: the wedge is worse than
"Active with no legal move". After the early return, `open_cards` is still empty
and `phase` is left at `Buying`, so `status()` reports
`Active { whose_turn: [bidding_player] }`, `can_bid`/`can_pass` both return true,
and `pass()` -> `take_first_open_card()` (line 295) calls
`self.open_cards.remove(0)` on an empty `Vec` - a **panic**, not just a wedge.
`start_selling_round` (lines 152-158) has the identical early return with the same
downstream consequence via the `Selling` path. Both should transition to
`Phase::Finished` (or return an internal error) rather than returning an empty log
list.

**`category-5-2` does not have the F-01 shape**: its round-start path
(`start_round`, lines 134-152) has no bail-out early return; `draw_cards`
degrades by returning short instead. That degradation is covered by F-67.

### F-72 (Medium) - `f F24` never implemented: `category-5-2` `MAX_PLAYERS` is still 8, not 10

Acceptance criterion (T3-B1 checklist, WP-32 row `f F24`):

> `rust/game/category-5-2/src/lib.rs` const `MAX_PLAYERS` + fn `player_counts`
> (and test `test_player_counts`) - Raise `MAX_PLAYERS` to 10 to match
> Go/official/`RULES.md` (deck math is exact at 10) and update `test_player_counts`.

End state:

- `rust/game/category-5-2/src/lib.rs:21` - `const MAX_PLAYERS: usize = 8;`
- `rust/game/category-5-2/src/lib.rs:548-550` - `fn player_counts() -> Vec<usize> { vec![2, 3, 4, 5, 6, 7, 8] }`

The row was closed with the crate unchanged on both counts. WP-32 was declared
complete and the checklist recorded "No findings rejected in this batch", so
nothing tracks this. Deck math confirms the finding was right: 104 cards,
`4 + players * HAND_SIZE` per round = `4 + 100 = 104` at 10 players, exactly
the deck.

Why it matters: this is a user-visible capability regression against the Go
original and against the crate's own `RULES.md` - 9- and 10-player games cannot
be created at all, and `Gamer::start` rejects them.

Remediation: `MAX_PLAYERS = 10`; change `player_counts()` to
`(MIN_PLAYERS..=MAX_PLAYERS).collect()` (the hardcoded literal vec is what let
the two drift apart in the first place - compare `red7-1/src/lib.rs:538-540`,
which derives it); update `test_player_counts`.

### F-72a (High) - the same commit that skipped `f F24` edited `RULES.md` to match the code, erasing the evidence the finding cited

This is the serious half of F-72 and is filed separately because it is a different
class of defect.

`f F24`'s premise was that the code disagreed with "Go/official/`RULES.md`", all
three of which said 10. Commit `807ab4e9` - the WP-32 commit that was supposed to
raise `MAX_PLAYERS` - instead changed `rust/game/category-5-2/RULES.md:3`:

```diff
-A 2-10 player card game (also known as 6 nimmt!). ...
+A 2-8 player card game (also known as 6 nimmt!). ...
```

and left `MAX_PLAYERS: usize = 8` untouched. `test_player_counts`
(lib.rs:616-622) still asserts `vec![2,3,4,5,6,7,8]` and that `Game::start(9, 1)`
errors, so the test agrees with the code and nothing fails. No commit anywhere in
`--all` history ever sets `MAX_PLAYERS` to 10 in this crate.

Why it matters:

1. The remediation moved in the **opposite direction** to the finding. A row asking
   for a capability increase was closed by downgrading the documented capability.
2. `RULES.md` is user-facing and is served through `Gamer::rules()`
   (lib.rs:556-558), so this is a published claim, not an internal note.
3. It destroyed the discrepancy that made the finding auditable. Anyone re-running
   the `f F24` check today finds code, tests and `RULES.md` in perfect agreement and
   concludes the row was fixed. Only the diff reveals otherwise.
4. Per the checklist header, WP-32 recorded "No findings rejected in this batch" -
   so there is no decision record justifying the reversal, and nothing tracks it.
   The checklist's own standing instruction was
   *"if it does not match the description, skip the row and report it - do not
   improvise"*, and this is improvisation.

Remediation: either implement `f F24` as written (`MAX_PLAYERS = 10`, restore
`RULES.md` to "2-10", update `test_player_counts`) or file an explicit decision
record rejecting it - but the `RULES.md` edit must not stand as the closure. Deck
math supports the finding: `4 + 10 * HAND_SIZE == 104 == DECK_SIZE` exactly.

### F-78 - WITHDRAWN (raised, then disproved; recorded so it is not re-filed)

I initially filed `f F27` as only partially fixed, on the theory that
`is_finished()` is a conjunction (all hands empty AND `highest >= END_SCORE`), so a
state with `max_points >= END_SCORE` and non-empty hands would render a negative
"N points until the end of the game". **This is not reachable.** `player_points`
only ever increases in `end_round` (lib.rs:245-269), and at that moment all hands
are empty by definition; `end_round` then checks `is_finished()` and deals a new
round only when it is false. So whenever any hand is non-empty,
`max_points < END_SCORE` necessarily holds, and `END_SCORE - max_points` is
strictly positive in the only branch that computes it. `f F27` is correctly closed
via its "skip the footer when finished" option. Both operands are `i32`, so there
was never an underflow risk either. Moved to Verified good.

Original (incorrect) analysis retained below for audit:

`rust/game/category-5-2/src/render.rs:115-122`:

```rust
if !pub_state.finished {
    let max_points = pub_state.player_points.iter().copied().max().unwrap_or(0);
    ...
    N::Bold(vec![N::text(format!("{} points", END_SCORE - max_points))]),
    N::text(" until the end of the game."),
```

`f F27` offered two remedies - clamp at 0, **or** skip the footer when
`pub_state.finished` - and the second was taken. But `finished` is not the negation
of "someone has reached `END_SCORE`". `is_finished()` (lib.rs:333-341) is a
conjunction:

```rust
for p in 0..self.players {
    if !self.hands[p].is_empty() { return false; }
}
let highest = self.player_points.iter().copied().max().unwrap_or(0);
highest >= END_SCORE
```

A player crosses 66 bullheads at `end_round`, and `end_round` immediately deals a
new round when `!is_finished()`... but scoring happens *before* that check, and
during any round in which the leader is already at or past 66 while hands are
still non-empty, `finished` is `false` and the footer renders
`END_SCORE - max_points` as **zero or negative** - e.g. "-4 points until the end of
the game." Both operands are `i32`, so this is a nonsense display rather than a
panic, which is why it is Low.

The clamp option would have covered this; the skip option does not. Remediation:
apply both - keep the `!finished` guard and clamp with
`(END_SCORE - max_points).max(0)`, or gate on `max_points < END_SCORE` instead of
on `finished`.

### F-73 (Medium) - `f F25` fixed the stack overflow but not the "errors instead of" half; the shortfall now silently corrupts hand sizes

Acceptance criterion (WP-32 row `f F25`):

> `fn draw_cards` - Guard `deck.len() + discard.len() >= n` before recursing (or
> convert to a loop) so an over-large `n` **errors** instead of overflowing the stack.

End state, `rust/game/category-5-2/src/lib.rs:271-286`:

```rust
pub fn draw_cards(&mut self, n: usize) -> Vec<Card> {
    if self.deck.len() + self.discard.len() < n {
        let mut cards: Vec<Card> = self.deck.drain(..).collect();
        cards.append(&mut self.discard);
        return cards;          // <-- silently returns FEWER than n
    }
```

The guard was added in the stated position and the recursion can no longer run
away, so the row is satisfied literally. But the shortfall branch returns a short
`Vec` rather than an error - `draw_cards` does not even return `Result` - and
every caller assumes it got exactly `n`:

- `start_round` (lines 144-148) assigns the short vec straight to `self.hands[p]`,
  producing **unequal hand sizes**;
- `start_round` (lines 141-143) assigns `self.board[i] = self.draw_cards(1)`, which
  can produce an **empty board row** - the very invariant `validate` (line 380) and
  the `.expect("row is never empty")` at line 178 depend on.

So the chosen remediation converts a stack overflow into two silent invariant
violations that are then relied upon by an `expect` and by raw indexing (see F-67).
The row's stated purpose - "an over-large `n` errors" - is unmet.

Remediation: make `draw_cards` return `Result<Vec<Card>, GameError>` and return
`GameError::internal` on the shortfall, propagating through `start_round`; or, at
minimum, have `start_round` detect the short draw and finish the game rather than
dealing an inconsistent round.

### F-74 (Low) - `f F31` was closed with a comment asserting an invariant the crate does not enforce

Acceptance criterion (WP-32 row `f F31`):

> `fn resolve_plays` - Add a short comment (or use an `all`-style check) stating the
> uniform-hand-size invariant behind the `hands[0].len()` proxy.

End state, `rust/game/category-5-2/src/lib.rs:228`:

```rust
// All hands have equal size by construction (dealt simultaneously each round).
```

The row offered two options and the weaker one was taken. Because `draw_cards`
can return short (F-73), the comment is not merely undocumented-but-true - it is
**false** on a reachable code path, and it is the justification for the raw
`self.hands[p][0]` index and the `.expect("auto-play should only play valid cards")`
immediately below it (F-67). Taking the `all`-style check option, or enforcing the
invariant in `validate`, would have closed F-67 and F-73's downstream half as well.

Recorded separately from F-67 because the finding here is about the *choice of
remediation*, not the code defect.

### F-75 (Low) - `zombie-dice-2` `f F5` loop conversion duplicated `start_turn`'s body instead of reusing it

`rust/game/zombie-dice-2/src/lib.rs:257-265` vs `294-299`.

The `f F5` fix broke the `roll -> next_player -> start_turn -> roll` recursion by
splitting `roll_inner` out and converting `next_player` to a loop - correct, and the
stack growth is genuinely gone. But the loop body inlines a verbatim copy of
`start_turn`'s six-line turn reset:

```rust
self.cup = all_dice();
self.shake_cup();
self.kept = vec![];
self.current_roll = vec![];
self.round_brains = 0;
self.round_shotguns = 0;
```

`start_turn` survives (it is still called from `start`, line 449) and now differs from
the copy only in its trailing `self.roll()` vs the loop's `self.roll_inner()`. Any
future turn-reset field will have to be added in two places.

Remediation: extract `fn reset_turn(&mut self)` and have `start_turn` be
`{ self.reset_turn(); self.roll() }` and the loop call `self.reset_turn()` then
`self.roll_inner()`.

### F-76 (Medium) - `e F34` closed with a doc comment; the `red7-1` panic is still reachable from a state `validate` accepts

`rust/game/red7-1/src/lib.rs:240-266`. WP-29's remediation for `e F34` was a
PRECONDITION comment (commit `071ace6e`, "Document `leader_with_suit` non-empty
precondition"):

> PRECONDITION: at least one player must not be eliminated. With every player
> eliminated, `player_map` is empty, `card::leader` returns index 0 for the empty
> slice, and the final `player_map[l_index]` would panic. All four call sites
> satisfy this: ...

The four-call-site argument is correct **for states produced by play**, but the
crate's trust boundary is `Gamer::validate`, and `validate`
(`rust/game/red7-1/src/lib.rs:562-582`) checks `num_players` range, the four
parallel-vector lengths and `current_player` - it never checks that at least one
`eliminated` entry is `false`. A persisted state with
`eliminated: [true, true], finished: false` therefore:

1. passes `validate`;
2. passes `command_parser` (line 59: only `finished` and `current_player` are
   checked), so the `discard` parser is offered;
3. reaches `discard` -> `self.leader_with_suit(card.suit)` (line 339) ->
   `player_map[l_index]` on an empty `player_map` (line 265) -> **panic**.

`leader()` and `leader_with_suit()` are additionally `pub`, so any in-process
consumer (bots, `tools/render_plain`, `tools/repl`) can trigger it directly.

Why it matters: this is a "documented the invariant instead of enforcing it"
closure, and the crate gained a `validate` override in the same programme that
could have enforced it in one line. Documentation is not a trust boundary.

Remediation: add to `validate`

```rust
if !self.finished && self.eliminated.iter().all(|&e| e) {
    return Err(GameError::internal("red7-1: all players eliminated"));
}
```

and/or change `leader_with_suit` to return `Option<(usize, Vec<Card>)>` /
`Result` so the empty case is handled at the four call sites rather than asserted
away.

### F-77 (Nit) - `red7-1` `end_points` doc comment is factually stale after WP-09 added `validate`

`rust/game/red7-1/src/lib.rs:20-27`:

> Only `MIN_PLAYERS..=MAX_PLAYERS` are meaningful (`Gamer::start` rejects anything
> else), but `Game` deserializes `num_players` unvalidated, so the arithmetic
> saturates at 0 rather than underflowing (e F35).

`num_players` is no longer deserialized unvalidated: `validate` at line 563-565
rejects anything outside `MIN_PLAYERS..=MAX_PLAYERS`. The saturating arithmetic is
still correct and worth keeping as defence in depth, but the stated justification
is wrong, and a future reader may conclude (as this comment invites) that no
`num_players` validation exists. Reword to "defence in depth; `validate` also
bounds `num_players`".

### F-35 occurrences (record only, per brief - no fix demanded)

`Status::Finished { stats: vec![] }` in all five crates:
`red7-1/src/lib.rs:422`, `zombie-dice-2/src/lib.rs:481`,
`battleship-2/src/lib.rs:453`, `for-sale-2/src/lib.rs:430`,
`category-5-2/src/lib.rs:442`. Matches sweep 2. No crate in this sub-unit
populates `stats`.

## Verified good

### red7-1 (WP-29, commit `071ace6e`)

- `e F35` fixed properly: `end_points` is `50usize.saturating_sub(players.saturating_mul(5))`
  with a dedicated test (`end_points_saturates_for_impossible_player_counts`,
  covering 10, 11, `usize::MAX`). Values still correct for 2/3/4 (40/35/30).
- `e F33` fixed: dead `PubCard`/`PubSuit` aliases replaced with `pub use card::*`;
  the now-redundant `use crate::card::*` in `mod tests` removed in the same hunk.
- `e F32` (RULES.md turn order / rule-meeting scoring) matches the code: `end_round`
  (lines 178-219) scores only `leader_palette` (the rule-fulfilling cards) and
  `retain`s the rest, exactly as the rewritten RULES.md section now states; the
  "discard ends your turn" wording matches `discard` calling `end_turn`
  (line 365) while `play` does not.
- Task 4 (`e F31` DATA_DOCS.md) was skipped with an explicit, checkable reason in
  the commit message (WP-83 had already landed the full-palette tie-break, making
  the spec's replacement text wrong). This is the right way to decline a spec task
  and is the only correctly-documented skip I found in this sub-unit.
- **Hidden-state audit: no leak.** `Game.hands` and `Game.deck` are the only hidden
  fields. `PubState` exposes `hand_sizes` and `deck_len` only (lines 440, 438);
  `discard_pile` is public by the rules (its top card *is* the active rule).
  `player_state` returns only the viewer's own `hand` (line 452).
- **`Log::public` audit: no leak.** `draw()` (lines 113-146) is the only place card
  identities could escape and it is correctly split: the public log states the
  *count* only ("drew N cards from the deck"), and a separate `Log::private(...,
  vec![player])` carries the card faces. Every other public log
  (`end_round`/`end_game`/`eliminate`/`play`/`discard`) discloses palette, discard
  pile or elimination state, all public by the rules.
- `player_state`/`render` raw indexing is safe: `check_player`
  (`rust/lib/cmd/src/requester/gamer.rs:24-36`) bounds `player` against
  `game.player_count()`, which for `red7-1` is `self.num_players`, and `validate`
  ties every parallel vector's length to `num_players`. **Note the standing brief's
  characterisation of `check_player` as bounding against "a crate constant" is not
  what the code does** - it bounds against `player_count()`, so the exposure exists
  only in crates whose `player_count()` is not the parallel vectors' length.
- `render_player_table`'s `(viewer + i) % pl` (render.rs:70) cannot divide by zero
  because `validate` rejects `num_players < MIN_PLAYERS`.

### zombie-dice-2 (WP-31, commit `f16cb02c`)

- `f F3` fixed on both halves: `roll_off_players` added to `PubState`
  (lib.rs:207-208), populated in `pub_state` (line 523), documented in
  `DATA_DOCS.md`, surfaced in `impl Renderer for PubState`
  (`render.rs:131-139`), and asserted in the field-capture test (line 1049 of the
  diff context).
- `f F5` fixed correctly at the root: the mutually-recursive
  `roll -> next_player -> start_turn -> roll` bust chain is now a single `loop` in
  `next_player` with `roll_inner` returning `(logs, busted)`. Consecutive busts
  `continue` instead of recursing, so stack depth is constant. (One duplication
  nit - F-75.)
- `f F6` fixed the *right* way, and specifically the way the checklist row warned
  about: `if self.roll_off_players != leaders` (line 278) is a full set comparison,
  not the empty-to-non-empty transition guard the row flagged as inadequate for
  mid-rolloff membership changes.
- `f F11`/`f F12` (battleship rows) - see below.
- **No hidden state exists in this crate**, so no leak is possible:
  `PlayerState` is a one-field wrapper around `PubState` (lines 211-215) and
  `player_state` ignores its `player` argument (line 527). Sweep 3's "field capture
  only, no omission assertion" for this crate is therefore correct and not a gap.
- **`Log::public` audit: no leak.** Every log (take_dice refill notice, roll result,
  bust, health-remaining, keep, tie-breaker) discloses only dice and scores, all of
  which are already in `PubState` (`current_roll`, `kept`, `round_brains`,
  `round_shotguns`, `scores`). There are no `Log::private` sites and none are needed.
- WP-08 epilogue migration is complete and correct here: `was_finished` captured
  before dispatch (line 548) and `finish_epilogue` called only on the
  false->true transition (lines 562-564). This is the reference implementation the
  other three crates in this sub-unit still lack.

### battleship-2 (WP-31, commit `f16cb02c`)

- `f F8` fixed exactly as specified: `shoot` bounds-checks `y`/`x` against
  `BOARD_SIZE` at the top and returns `GameError::invalid_input` (lib.rs:315-319),
  ahead of the `self.boards[op][y][x]` index at line 322. Verified the guard
  precedes every indexing site in the function.
- `f F10` fixed: `ship_cell.to_ship().expect("cell is a ship")` replaced with a
  `let Some(ship) = ... else { return Err(GameError::internal(...)) }`
  (lines 336-340). Correct choice - the arm really is unreachable given the
  preceding `Cell::Hit | Cell::Miss` and `Cell::Empty` arms, and it now errors
  rather than panics.
- `f F11` fixed on both halves: `Direction::all() -> &'static [Direction]`
  (lib.rs:118) matching `Ship::all`, with `.to_vec()` added at the
  `Enum::partial` call site in `command.rs:31`.
- `f F12` fixed: both `player_hits_remaining` and `player_ship_hits_remaining`
  return `usize`, with `as i32` casts confined to the two `gen_placings`/scores
  call sites (lib.rs:397, 534).
- **Redaction is correct.** `redact_board` (lines 192-202) blanks only
  `is_ship()` cells to `Cell::Empty`, preserving `Cell::Hit`/`Cell::Miss`. This is
  the right granularity: hit cells are already known to the shooter, and because
  a hit overwrites the ship variant, no ship identity survives redaction.
  `pub_state` un-redacts both boards only when `finished` (lines 472-477).
  `PlayerState.board` is the viewer's own board via `.get(player).copied()`
  (line 493).
- **`Log::public` audit: no leak.** The three logs are "finished placing their
  ships" (no positions), "shot at L and missed", "shot at L and hit a ship", and
  "shot at L and sunk a {ship}". The sunk message names the ship type, which is
  correct Battleship and in any case derivable from the hit pattern. Crucially the
  *hit* message does **not** name the ship - so a hit does not leak which ship was
  struck before it sinks. `player_ship_hits_remaining` is used only to decide
  between the two messages.
- `points()` returning `player_hits_remaining` is not a leak: hit cells are visible
  in the redacted board, so `17 - hits` is already public.
- `can_place` uses `.get(player)` defensively (lines 247-251) and the subsequent
  raw `self.left_to_place[player]` / `self.boards[player]` indexes are dominated by
  it plus `validate`'s length equalities.

### for-sale-2 (WP-32, commit `807ab4e9`)

- `f F18` fixed exactly as the row demanded, including the trap it called out:
  `phase: Option<Phase>` under `#[serde(default)]` (lib.rs:43-44), set explicitly in
  `start_round` (line 116), with `current_phase()` falling back to the
  deck-size inference when `None` (lines 110-112). The rejected
  `#[serde(default)] phase: Phase` form (which would default live Selling games to
  `Buying`) was correctly avoided.
- `f F20` fixed: `start_selling_round`'s autoplay guard is
  `self.hands.iter().all(|h| h.len() == 1)` (line 164), not `hands.first()`.
- `f F23` fixed on both halves: `clear_bids`, `take_first_open_card`,
  `next_bidder`, `highest_bid`, `deck_value`, `start_buying_round`,
  `start_selling_round`, `player_points`, `placings`, `points_int` are all private
  (no `pub`) in the `impl Game` block; `player_state` indexes defensively with
  `.get(player).copied().unwrap_or(0)` / `.cloned().unwrap_or_default()`
  (lines 462-465).
- `f F22` satisfied via the "align the sentinel" option: `render.rs`'s duplicate
  `highest_bid` (render.rs:40-54) now uses `if best >= 0` (was `> 0`), matching
  `Game::highest_bid`'s `-1` no-bid sentinel (lib.rs:331), and both skip players
  with `finished_bidding[p]`. The duplication itself remains (two independent
  functions, one returning `Option`, one a raw tuple) - the row permitted either
  remedy, so this is a pass, not a finding.
- `f F16` fixed: `RULES.md:8` reads "20 cheques: two 0s, then 3..=20", matching
  `cheque_deck()` (lib.rs:93-95), and `RULES.md:31` now states "Ties are broken by
  remaining chips", matching `placings()`'s secondary metric `chips[p]`
  (lib.rs:345-350).
- `f F17` fixed: the Finished-branch log table renders
  `render::bold_num(self.player_points(p))` (line 126), i.e. cheques + chips, which
  matches the `placings_log` scores appended by `command` (lines 492-495 etc.).
- **Redaction is correct and non-trivial.** `PubState` omits `hands`, `cheques`,
  `chips` and both decks entirely, exposing only `open_cards`, `bidding_player`,
  `bids`, `finished_bidding` and two derived round counters. The subtle part is
  right: `bids` is doubled as "current bid" in Buying and "the building card you
  played face-down" in Selling, so `pub_state` **zeroes the whole vector during
  Selling** (lines 449-453) and `PlayerState.bid` gives the viewer only their own
  value (line 465). Without that branch the entire simultaneous-reveal mechanic
  would leak. Backed by `test_pub_state_redacts_hands_and_cheques` (per sweep 3).
- **`Log::public` audit: no leak.** Bids and passes are public by the rules;
  `pass` names the building taken (public - it comes off `open_cards`); the
  Selling-phase `play` returns **no log at all** until the last player has played,
  at which point the reveal loop logs every (building, cheque) pair at once
  (lines 274-288). That is the correct simultaneous-reveal shape.

### category-5-2 (WP-32, commit `807ab4e9`)

- `f F27` fixed correctly via the "skip the footer when finished" option
  (`render.rs:115` gates the whole block on `!pub_state.finished`). Verified
  unreachable-negative: see the withdrawn F-78 above for the proof.
- `f F30` fixed (test comment typo, "11 is a multiple of 11 only").
- `f F28` fixed as specified (label-only): the doc comment on `points()`
  (lib.rs:543) documents raw lower-is-better bullhead totals and states that ELO
  uses placings. `points()` was correctly **not** negated, while `placings()`
  negates internally for `gen_placings` (line 345) - the row's warning was heeded.
- **Redaction is correct, and covers more than the sweep suggests.** `PubState`
  omits `hands`, `deck`, `discard` **and `plays`** (lines 100-113). Omitting
  `plays` is the load-bearing one: the whole game is simultaneous face-down card
  selection, and exposing `plays` would leak every player's chosen card before
  resolution. `player_state` returns only the viewer's own hand via
  `.get(player).cloned().unwrap_or_default()` (line 479).
- **`Log::public` audit: no leak.** Every public log fires *during or after*
  resolution (`resolve_plays` lines 193-220, `choose` lines 316-325,
  `end_round` lines 246-261), so each card named has already been committed and
  revealed. `start_round`'s log states the deal size only, never the cards
  (lines 149-151). There are no `Log::private` sites, and none are needed - hands
  reach players through `player_state`.
- `validate` (lines 378-436) is the most thorough in this sub-unit: board rows
  non-empty, five parallel-vector lengths, `choose_player` range, and a
  `1..=DECK_SIZE` range check applied to every card in board, hands,
  `player_cards`, `plays`, deck and discard. Two invariants are still missing
  (F-66, F-67).
- The `.expect("row is never empty")` at line 178 is genuinely guarded by
  `validate`'s empty-row check - unusual and correct.

## Coverage gaps

- **No log-layer test in any of the five crates.** Confirmed by inspection: every
  `#[test]` in these crates asserts on `Game` fields, `PubState` fields or command
  results; none inspects the `Vec<Log>` a command returns, and none asserts that a
  given `Log` is `private` rather than `public`. The redaction tests that do exist
  (`test_pub_state_redacts_hands_and_cheques`, `test_pub_state_redacts_ships`,
  `pub_state_does_not_leak_hidden_info`) all check `PubState` shape only. The one
  place in this sub-unit where the public/private split is load-bearing -
  `red7-1`'s `draw()` count-only public log plus per-player private card log
  (lib.rs:113-146) - has **no test at all**. A regression that changed
  `Log::private(content, vec![player])` to `Log::public(content)` there would leak
  every drawn card to every player and no test in the repo would fail.
  Recommended minimum: one test per crate with hidden state asserting
  `logs.iter().all(|l| l.public == false || !mentions_hidden_data(l))`, or at least
  a `red7-1` test pinning that the draw log pair is (public count, private faces).
- **`category-5-2` has real hidden state (`hands`, `plays`, `deck`, `discard`) and
  no test asserting `PubState` omits it** (sweep 3 row confirmed by reading
  `test_pub_state_captures_rendered_fields`, lib.rs:869 - it compares public
  fields only). Given that `plays` is the field whose exposure would break the
  game's core mechanic, this is the highest-value missing test in the sub-unit.
- **No test covers a short/exhausted `category-5-2` deck.** `test_game_draw_cards`
  (lib.rs:584-594) exercises reshuffle-from-discard but never the
  `deck.len() + discard.len() < n` shortfall branch that F-73 is about, so the
  short-hand and empty-board-row outcomes are entirely untested.
- **No test covers `for-sale-2`'s `building_deck.len() < players` early return**
  (F-01) or `next_bidder`'s `remaining == 0` fall-through (F-69). Both are
  deserialized-state-only paths and both would be trivial to pin with a
  hand-constructed `Game`.
- **No `validate` test in any of the five crates.** All five have a `validate`
  override (sweep 1) and none has a test that feeds it a malformed state - contrast
  `tic-tac-toe-2` (`validate_rejects_inconsistent_state`), `lost-cities-2`
  (`validate_works`), `modern-art-2`, `age-of-war-2` and `love-letter-2`, which do.
  Since `validate` is the sole trust boundary for deserialized state (D-36) and
  four of this sub-unit's findings (F-66, F-67, F-68, F-76) are gaps *in* these
  `validate` bodies, the absence of tests is what let those gaps through.
- **Routing check:** WP-29 Task 4 (`e F31`, DATA_DOCS.md) was skipped and the reason
  recorded in the commit message, but nothing re-files or re-verifies it - the claim
  "DATA_DOCS.md line 36 now matches the live code" is asserted in a commit message
  with no checklist row, test or follow-up WP tracking it. This is a mild instance
  of the "routed to WP-NN" leak pattern: a self-closed deferral with no receiver.
  I spot-checked the claim and it holds, so this is process risk rather than a
  live defect. No other routings appear in WP-29/WP-31/WP-32.
- **`for-sale-2` `pass()` rounding - not filed, flagged for 04c only.**
  `let half_bid = self.bids[player] / 2;` (lib.rs:246) truncates, so on an odd bid
  the passing player pays the rounded-**down** half and keeps the rounded-up half.
  `RULES.md:17` says "rounded down", so code and crate docs agree. Published For
  Sale rules return half the bid rounded down *to the player* (i.e. the player pays
  the rounded-up half), which is the opposite - a 1-coin discrepancy per
  odd-numbered pass. `for-sale-2` **does** carry parked port-parity items (WP-11
  subset), so this most likely sits inside the park and I have deliberately not
  filed it. 04c owns the parity list and should confirm rather than assume.
- **Parked-item context for this sub-unit** (recovered from history, for 04c):
  `for-sale-2` and `zombie-dice-2` have parked port-parity items under WP-11;
  `red7-1` has one parked item (WP-30 / D-29, empty-winning-set tie-break, `e F30`);
  `category-5-2` and `battleship-2` have **nothing parked** - every finding for
  those two was supposed to ship in WP-32 and WP-31 respectively, which is why
  F-72/F-72a matter: `f F24` has no park to fall back into.
- **Out of my scope, flagged for 04c:** commit `abffb7aa` (WP-33) touches only
  `farkle-2`, `greed-2`, `liars-dice-2`, `no-thanks-2`, `tic-tac-toe-2` - none of
  the five crates in this sub-unit - and commit `62b293df` touches no `rust/` files
  at all (3 files, all under `docs/`). Neither was reviewable here; WP-33's five
  crates appear to belong to no reviewing sub-unit in the Unit 04 breakdown and may
  be uncovered by this session.
