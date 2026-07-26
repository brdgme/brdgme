# T3-B1: zombie-dice-2 + battleship-2 + for-sale-2 + category-5-2

- **Batch**: T3-B1 = WP-31 (7 findings) + WP-32 (12 findings)
- **Crates**: `rust/game/zombie-dice-2`, `rust/game/battleship-2`,
  `rust/game/for-sale-2`, `rust/game/category-5-2`
- **Source**: `findings/games-batch-f.md`, superseded where they differ by
  `findings/verification/games-batch-f.md`
- **Numbering**: games-batch-f (`f Fnn`). Raw and verification numbering are
  identical for this batch - no offset hazard.
- **Rows**: 19 (7 + 12). No findings rejected in this batch.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by crate then source file so one session sweeps a file at a
time.

## WP-31 - zombie-dice-2 + battleship-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F3` | `rust/game/zombie-dice-2/src/lib.rs` struct `PubState` + fn `pub_state` | Add a `roll_off_players` field to `PubState` and populate it from `Game::roll_off_players` in `pub_state` | y |
| `f F5` | `rust/game/zombie-dice-2/src/lib.rs` fns `next_player` / `start_turn` | Convert the `roll -> next_player -> start_turn -> roll` bust chain into a loop so consecutive busts do not grow the stack | n |
| `f F6` | `rust/game/zombie-dice-2/src/lib.rs` fn `next_player` | De-duplicate the "tie breaker round!" log by comparing the newly computed rolloff set against the existing `roll_off_players` and logging only on change - a plain empty-to-non-empty transition guard would MISS legitimate mid-rolloff membership changes | y |
| `f F3` (render half) | `rust/game/zombie-dice-2/src/render.rs` `impl Renderer for PubState` fn `render` | Surface the active rolloff and its participants in the rendered output | n |
| `f F8` | `rust/game/battleship-2/src/lib.rs` fn `shoot` | Bounds-check the target `Loc` against board dimensions at the top of `shoot` and return `GameError::invalid_input`, restoring Go parity | y |
| `f F10` | `rust/game/battleship-2/src/lib.rs` fn `shoot` | Replace `expect("cell is a ship")` in the sunk-detection branch by binding the ship through the match pattern (or an erroring `None` arm) | n |
| `f F11` | `rust/game/battleship-2/src/lib.rs` fn `Direction::all` + `rust/game/battleship-2/src/command.rs` fn `place_parser` | Change `Direction::all` to return `&'static [Direction]` matching `Ship::all`, and add `.to_vec()` at the `Enum::partial(Direction::all())` call site in `place_parser` | n |
| `f F12` | `rust/game/battleship-2/src/lib.rs` fns `player_hits_remaining` / `player_ship_hits_remaining` | Return `usize` instead of `i32` and cast at the `gen_placings` call site | n |

Note: `f F1` and `f F13` are Tier 2 (WP-10) and are deliberately not in this
checklist.

## WP-32 - for-sale-2 + category-5-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F17` | `rust/game/for-sale-2/src/lib.rs` fn `start_round` (Finished-branch log table) | Render `player_points(p)` (or cheques and chips as separate columns) instead of the cheque sum alone so the table matches the appended placings log | y |
| `f F18` | `rust/game/for-sale-2/src/lib.rs` struct `Game` + fn `current_phase` + fn `status` | Store `phase: Option<Phase>` on `Game`, set it explicitly in `start_round`, and have `current_phase` fall back to the existing deck-size inference when it is `None` - do NOT use `#[serde(default)] phase: Phase`, which defaults to `Buying` and corrupts live Selling games | y |
| `f F20` | `rust/game/for-sale-2/src/lib.rs` fn `start_selling_round` | Replace the `hands.first()` autoplay guard with `self.hands.iter().all(\|h\| h.len() == 1)` | n |
| `f F23` | `rust/game/for-sale-2/src/lib.rs` helper fns (`clear_bids`, `take_first_open_card`, `next_bidder`, `highest_bid`, `deck_value`, `start_*_round`) + fn `player_state` | Downgrade crate-internal helpers from `pub` to `pub(crate)`/private where the bins do not need them, and index defensively with `.get()` in `player_state` | n |
| `f F22` | `rust/game/for-sale-2/src/render.rs` fn `highest_bid` | Align the duplicated renderer helper with `Game::highest_bid`'s no-bid sentinel (or drop it in favour of a shared `Option`-returning helper) | n |
| `f F16` | `rust/game/for-sale-2/RULES.md` cheque-deck section + tie-break sentence | Correct the deck line to "20 cheques: two 0s, then 3..=20" and amend the "ties share a place" sentence to describe the chips tie-break used by `placings()` | n |
| `f F24` | `rust/game/category-5-2/src/lib.rs` const `MAX_PLAYERS` + fn `player_counts` (and test `test_player_counts`) | Raise `MAX_PLAYERS` to 10 to match Go/official/`RULES.md` (deck math is exact at 10) and update `test_player_counts` | y |
| `f F25` | `rust/game/category-5-2/src/lib.rs` fn `draw_cards` | Guard `deck.len() + discard.len() >= n` before recursing (or convert to a loop) so an over-large `n` errors instead of overflowing the stack | y |
| `f F28` | `rust/game/category-5-2/src/lib.rs` fn `points` | Label-only: document that `points` returns raw lower-is-better bullhead totals and that ratings use `place` - do NOT negate `points()`, ELO is placings-driven and negation would be a display regression | n |
| `f F31` | `rust/game/category-5-2/src/lib.rs` fn `resolve_plays` | Add a short comment (or use an `all`-style check) stating the uniform-hand-size invariant behind the `hands[0].len()` proxy | n |
| `f F30` | `rust/game/category-5-2/src/lib.rs` test fn `test_card_heads` | Fix the comment typo to "11 is a multiple of 11 only" | n |
| `f F27` | `rust/game/category-5-2/src/render.rs` fn `render` (footer) | Clamp the "N points until the end of the game" value at 0, or skip the footer entirely when `pub_state.finished` | y |

## Escalate

None. All 19 fixes compress to one line.
