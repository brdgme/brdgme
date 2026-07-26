# T3-B3: splendor-2 + lib/cost + texas-holdem-2

- **Batch**: T3-B3 = WP-17 (splendor-2 + `lib/cost` consolidation, 8 findings)
  + WP-18 (texas-holdem-2 cleanup, 4 findings)
- **Crates**: `rust/game/splendor-2`, `rust/lib/cost`,
  `rust/game/texas-holdem-2`
- **Sources**: `findings/games-batch-b.md`, `findings/games-batch-c.md`,
  `findings/lib-support.md`, `findings/dependencies.md` - each superseded
  where it differs by its file under `findings/verification/`.
- **Numbering conventions** (differ per prefix - read this before resolving
  any ID back to a findings file):
  - `b` (games-batch-b) and `c` (games-batch-c): raw and verification
    numbering are identical positionally. No offset.
  - `ls` (lib-support): **VERIFICATION numbering**. The raw file has 46
    findings, verification has 45 - raw F10 (ANSI/plain renderer escaping) is
    absent from verification, so **every raw `ls` number >= 10 is +1 against
    verification**. Resolving `ls F38`/`ls F39` positionally against the RAW
    file reads the WRONG finding. Both live under the raw file's `## lib/cost`
    section.
  - `dp` (dependencies): **no verification file exists** for the dependencies
    unit - `dp F27` was lead-verified only. Treat its wording as unconfirmed
    by the verification pass.
- **Rows**: 12 total - 9 implementable now (main tables) + 3 blocked on D-25.
  No findings in this batch were rejected by verification.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.
- `c F2` (MAX_PLAYERS 8 vs Go 9) is deliberately NOT here - it sits in WP-20,
  which is parked. `b F33` / `c F6` (epilogue duplication) belong to WP-08.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by crate then source file so one session sweeps a file at a
time.

## WP-17 - splendor-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `b F30` | `rust/game/splendor-2/src/lib.rs` fn `take` | Reject any requested token not in `GEMS` at the top of `take` so the action layer enforces the gem invariant regardless of caller | y |
| `b F34` | `rust/game/splendor-2/src/lib.rs` fn `take` (the "not enough tokens" error string) | Fix the "remaning" typo to "remaining" | n |
| `b F35` | `rust/game/splendor-2/src/lib.rs` fn `visit_phase` | Replace the auto-visit `.expect("invariant: auto-visit must always succeed")` with `unwrap_or_else` returning the error as logs | n |
| `b F32` | `rust/game/splendor-2/src/command.rs` fn `reserve_parser` (built from fn `loc_parser`) | Filter row-3 (own-reserve) locations out of the reserve parser, and correct the stale test comment in `lib.rs` that claims row 3 is already excluded | y |

## WP-17 - lib/cost

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `ls F38` | `rust/lib/cost/src/lib.rs` fn `Cost::new` | Move `new()` out of the `K: Clone`-bounded impl block into one bounded only by `Hash + Eq` (`Default` needs no `Clone`) | n |

## WP-18 - texas-holdem-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `c F1` | `rust/game/texas-holdem-2/src/command.rs` fn `raise_parser` | Use `self.min_raise()` instead of `self.largest_raise` for the `Int` parser's `min`, and rewrite the doc comment (the real `LargestRaise` quirk lives in `Game::can_raise`, not here) | y |
| `c F3` | `rust/game/texas-holdem-2/src/lib.rs` fn `bet_up_to` | Remove the `.expect("BetUpTo always bets an affordable amount")` by clamping through an infallible internal helper, or keep it with an explicit Go-mirroring exception comment | n |
| `c F4` | `rust/game/texas-holdem-2/src/lib.rs` fn `next_player_in_set` + `rust/game/texas-holdem-2/src/card.rs` fn `pop_n` | No correctness change - confirm all three Go-mirroring panics still carry their documenting comments, and convert to `debug_assert!` only if the no-panic rule is being enforced strictly | n |
| `c F5` | `rust/game/texas-holdem-2/src/poker.rs` struct `HandResult` + enum `Category` | Drop the `Option` from `HandResult.category`, defaulting it to `Category::None` via `#[derive(Default)]` + `#[default]` on the variant, and delete both `unwrap_or(Category::None)` call sites | n |

## BLOCKED ON D-25 - do not implement until answered

D-25 ("lib/cost consolidation") is **still unanswered as of 2026-07-25**.
Option **A** (recommended): port splendor-2 onto `brdgme_cost`, adding
`get`/`set` to `lib/cost` first and keeping splendor's gold-joker
`can_afford` free function as a crate-local extension. Option **B**: fold
`lib/cost` into seven-wonders-1 (its only current consumer) and delete the
lib. The three rows below are the *same* consolidation question viewed from
three findings units - they must be implemented together as one change, and
neither option may be started until D-25 is decided. Whichever option is
chosen, add a serde round-trip test of a serialized splendor `Game` to lock
persisted-state compatibility (both types are
`pub struct Cost(pub HashMap<Resource, i32>)` newtypes and serialize
identically, but the test pins it).

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `b F31` | `rust/game/splendor-2/src/cost.rs` (whole module) + `rust/lib/cost/src/lib.rs` | Under option A: add generic `get`/`set` to `lib/cost`, then replace `cost.rs` with `pub type Cost = brdgme_cost::Cost<Resource>;` plus the retained gold-joker `can_afford` free function | y |
| `ls F39` | `rust/game/splendor-2/src/cost.rs` (whole module) + `rust/game/splendor-2/Cargo.toml` deps | Same change as `b F31` - add the `brdgme_cost` dependency and delete the ~155 duplicated lines; `from_resources`/`add`/`inv`/`sub`/`sum`/`can_afford` are verified semantically equivalent to the lib's | y |
| `dp F27` | `rust/lib/cost/Cargo.toml` + `rust/game/splendor-2/src/cost.rs` | Same change as `b F31`/`ls F39` - resolve the half-shared state in one direction (A: second consumer; B: delete the lib); lead-verified only, no verification-pass entry | y |

## Escalate

None. All 12 fixes compress to one line.
