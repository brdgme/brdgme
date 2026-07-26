# WP-83: parity fixes released from the rules park

**Findings:** `a F1`, `b F7`, `e F30`. **Decision:** D-35. **Status:** READY.

**Game rules parity is otherwise PARKED** (`BLOCKED-ON-USER-RULES-REVIEW`:
WP-11, WP-12, WP-16, WP-20, WP-26, WP-30) pending Michael's own review of the
`RULES.md` files. **These three findings were individually RELEASED from that
park by D-35 and are FIX NOW. Do not re-park them.** Also settled, and **not in
scope here**: `b F4` (seven-wonders same-turn trade) was **re-parked** under a
binding user correction; `d F37` (modern-art zero-card artists) was **REJECTED -
not a bug**.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising. No line numbers are
> cited on purpose - the tree is under concurrent edit.**

*Slightly over the ~120-line Tier 2 cap (Lead-accepted): three independent fixes
in three crates in one file, so ~50 lines per fix - well under the cap on a
per-fix basis. Each section is self-contained; land as three commits if easier.*

## 1. `a F1` - roll-through-the-ages: stale phase re-match in `roll()`

**Bug:** `Game::roll` re-reads `self.phase` *after* `keep_skulls()` may have
already advanced it, so the post-roll bookkeeping is applied to the wrong phase -
and, in the worst case, to the **next player**.

**Confirmed against LIVE code - CORRECT as written.** Read `Game::roll`,
`keep_skulls`, `next_phase`, `roll_phase`, `roll_extra_phase` in
`rust/game/roll-through-the-ages-2/src/lib.rs`. (Crate is
`roll-through-the-ages-**2**`, not `-1`.)

- `roll()` ends `logs.extend(self.keep_skulls());` then
  `match self.phase { Phase::Roll => { self.remaining_rolls -= 1; ... } ... }`.
- `keep_skulls()` calls `next_phase()` when every remaining rolled die is a
  skull (except the Leadership/`ExtraRoll` case).
- Without Leadership that cascades `Roll -> ExtraRoll -> ... -> next_turn ->
  start_turn -> preserve_phase -> roll_phase`, which sets `phase = Roll` and
  `remaining_rolls = 2` **for the new current player**; the stale `match` then
  decrements that player to 1.
- With Leadership the cascade stops at `ExtraRoll`; the stale `match` hits the
  `ExtraRoll` arm and calls `next_phase()` again, consuming the extra roll.

**Fix:** capture the phase before `keep_skulls()` and skip the block if it
already advanced - the transition has then happened, and `roll_phase()` resets
`remaining_rolls` on its own.

```rust
let phase_before = self.phase;
logs.extend(self.keep_skulls());
if self.phase == phase_before {
    match phase_before { /* existing arms, unchanged */ }
}
```

`Phase` already derives `Clone, Copy, PartialEq, Eq`, so the capture is free.

**Test:** using the crate's existing `Roll`-phase test constructor, set up a
2-player game at `phase = Roll`, `current_player = 0`, `remaining_rolls = 2`,
and drive a roll that comes back all skulls. Assert `current_player == 1` **and
`remaining_rolls == 2`** (today: 1). Second case: give player 0
`DevelopmentId::Leadership` and repeat; assert `phase == Phase::ExtraRoll`
(today it has already advanced past it).

## 2. `b F7` - seven-wonders: both sides of one wonder board in play

**Bug:** `card::cities()` returns all **14** A/B entries flat; `start_game`
shuffles and takes the first `players`, so "Rhodes A" and "Rhodes B" - two faces
of one physical board - can be dealt in the same game.

**Confirmed against LIVE code - CORRECT as written.** Read `cities()` in
`rust/game/seven-wonders-1/src/card.rs` and `Game::start_game` in
`rust/game/seven-wonders-1/src/lib.rs`. Pairing is carried **only in
`City.name: String`**, as `"<Board> A"` / `"<Board> B"` (Rhodes, Alexandria,
Ephesus, Babylon, Olympia, Halicarnassus, Giza). There is no `Side` enum and no
board id. `start_game` does
`all_cities.shuffle(&mut rng); let assigned_cities = all_cities[..players].to_vec();`
with `MAX_PLAYERS = 4`.

**Fix:** in `start_game`, group the 14 into 7 boards by name prefix, shuffle the
*boards*, take `players`, then pick one side per board.

```rust
let mut by_board: BTreeMap<String, Vec<City>> = BTreeMap::new();
for c in cities() {
    let board = c.name.strip_suffix(" A").or_else(|| c.name.strip_suffix(" B"))
        .unwrap_or(&c.name).to_string();
    by_board.entry(board).or_default().push(c);
}
let mut boards: Vec<Vec<City>> = by_board.into_values().collect();
boards.shuffle(&mut rng);
let assigned_cities: Vec<City> = boards[..players].iter()
    .map(|sides| sides[rng.random_range(0..sides.len())].clone()).collect();
```

**`BTreeMap`, not `HashMap`** - grouping order must be deterministic or the
seeded RNG stops reproducing games. Confirm the RNG method name (`random_range`
vs `gen_range`) against the crate's `rand` version. Nothing downstream indexes
`Game.cities` back into `cities()`; the change is local to setup.

**Test:** for `players` in `2..=4` across seeds `0..200`, assert the assigned
city names with a trailing `" A"`/`" B"` stripped are all distinct. Fails today.

## 3. `e F30` - red7: empty winning set wins on seat order

**Bug:** `card::leader` ranks only by rule-filtered winning set. All-empty sets
tie at `len 0` and the strict `>` leaves the lowest seat index leading, so a
player fulfilling nothing "wins" the round.

**Release evidence - record this so nobody re-parks it as a rules question.**
`rust/game/red7-1/DATA_DOCS.md` already documents the tie-break this fix
implements:

> Ties within a rule are broken by the highest card in the winning set, then by
> the highest card overall in the palette.

The second clause is **not implemented at all**. That is why D-35's release
condition held: this is a code/doc mismatch inside red7-1's own data docs, not an
open rules question.

**Confirmed against LIVE code - CORRECT as written.** Read
`leader(palettes: &[Vec<Card>])` in `rust/game/red7-1/src/card.rs` and
`Game::leader_with_suit` in `rust/game/red7-1/src/lib.rs`. `leader_with_suit`
passes `rule_fn(&self.palettes[p])` - the **winning set**, not the palette - so
`leader` never sees the full palette. Its `.max().unwrap_or((0, 0))` is exactly
the all-empty case.

**Fix:** give `leader` both and compare the full documented key
`(winning_set.len(), max rank_key of winning set, max rank_key of full palette)`.
Take pairs, e.g. `leader(entries: &[(Vec<Card>, Vec<Card>)]) -> (usize, Vec<Card>)`
(winning set, full palette); keep returning the **winning set**; have
`leader_with_suit` push `(rule_fn(&self.palettes[p]), self.palettes[p].clone())`.
Keep the strict `>` and seat-order final fallback - card ranks are unique, so a
full three-part tie is only reachable when every palette is literally empty.
Update both call sites: `Game::leader_with_suit` and the `test_leader` unit test.

**Test:** two players, current rule Violet (`most_cards_below_4`); both palettes
hold only cards of rank >= 4 (both winning sets empty), with player 1 holding the
single highest card. Assert `g.leader().0 == 1`; today it returns 0. Add a second
case with two non-empty, equal-length winning sets to prove the first-level
tie-break is unchanged.

## 4. Scope guard

- Three **surgical** fixes. **Do not touch any `RULES.md`** in these crates, and
  do not widen into the parked rules questions they carry.
- **Do not fix other findings noticed in passing** - report them to the Lead.
- No public API change beyond `card::leader`'s signature in red7-1.

## 5. Verification

AGENTS.md forbids workspace-wide builds on dev machines, so use `-p`. Package
names confirmed from the manifests - `roll-through-the-ages-2`,
`seven-wonders-1`, `red7-1`. For each: `cargo clippy -p <crate> --all-targets`
and `cargo test -p <crate>`. Each fix ships with its test above; existing tests
must still pass unchanged.
