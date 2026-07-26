# T3-B2: greed-2 + farkle-2 + tic-tac-toe-2 + no-thanks-2 + liars-dice-2

- **Batch**: T3-B2 = WP-33 small-crate cleanup (17 findings)
- **Crates**: `rust/game/greed-2`, `rust/game/farkle-2`,
  `rust/game/tic-tac-toe-2`, `rust/game/no-thanks-2`,
  `rust/game/liars-dice-2`
- **Source**: `findings/games-batch-f.md`, superseded where they differ by
  `findings/verification/games-batch-f.md`
- **Numbering**: games-batch-f (`f Fnn`). Raw and verification numbering are
  identical for this batch - no offset hazard.
- **Rows**: 17 (4 minor / 13 nit). No findings rejected in this batch.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by crate then source file so one session sweeps a file at a
time.

## WP-33 - greed-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F32` | `rust/game/greed-2/src/lib.rs` fn `score` | Validate `player == self.current_player` inside `score` (matching `player_roll`/`done`) or drop the parameter and have `done` call a private helper - the guard is safe for `done`'s internal call path | y |
| `f F36` | `rust/game/greed-2/src/lib.rs` fns `score` and `done` (turn/banked score adds) | Use `saturating_add` for the `turn_score` and `scores[player]` i32 accumulations | n |
| `f F37` | `rust/game/greed-2/RULES.md` dice colour table (vs fn `Die::color`) | Update the RULES.md colour entry for the `E` face from "black" to the `Foreground` colour the code actually uses | n |

## WP-33 - farkle-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F39` | `rust/game/farkle-2/src/lib.rs` fn `score` | Add the same `player == self.current_player` guard the sibling mutators use (or a `debug_assert_eq!`) instead of `let _ = player;` | y |
| `f F40` | `rust/game/farkle-2/src/lib.rs` fns `bust` / `done` (or `pub_state`) | Zero `turn_score`/`remaining_dice` when the game finishes so a finished pub_state stops reporting a stale "Score this turn" | y |
| `f F41` | `rust/game/farkle-2/src/lib.rs` test fn `test_finished_and_placings` | Set `g.current_player = (g.first_player + 1) % 3` so the test never builds an out-of-range player index | n |
| `f F38` | `rust/game/farkle-2/src/render.rs` fn `scoring_table` | Derive the rendered table from `scores()`/`SCORES` instead of hardcoding the eight combinations, keeping only display names local and preserving the current row order | y |
| `f F42` | `rust/game/farkle-2/src/render.rs` fns `render_die` / `render_dice` | Use the crate's `Die` / `&[Die]` alias in place of `u8` / `&[u8]` | n |

## WP-33 - tic-tac-toe-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F47` | `rust/game/tic-tac-toe-2/src/lib.rs` fns `winner` / `matching_line` | Remove the dead `Cell::Empty` arm by giving `matching_line` a mark-only return type, or replace the arm with `unreachable!()` | n |
| `f F45` | `rust/game/tic-tac-toe-2/src/render.rs` fn `render_with_labels` | Replace `1 - start_player` with `(start_player + 1) % NUM_PLAYERS` (add the import) so a crafted `start_player >= 2` cannot underflow | y |
| `f F48` | `rust/game/tic-tac-toe-2/src/lib.rs` fn `play` (log) + `src/render.rs` fns `render_board` / `render_with_labels` + `RULES.md` + test fn `exact_render_markup_matches_the_old_board` | Pick one casing (uppercase matches convention) across the play log, the board cells, the "is X / is O" label and RULES.md, and update the exact-render markup assertion in that test to match | y |

Note: `f F46` (crafted `players` drives unbounded alloc/iteration) is Tier 2
(WP-09a/WP-09b) and is deliberately not in this checklist.

## WP-33 - no-thanks-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F49` | `rust/game/no-thanks-2/src/lib.rs` test fn `test_init_player_chips` | Set `g.players = 3` (or use `Game::start(3, 1)`) before calling `init_player_chips`, and assert `g.player_chips.len() == g.players` so the loop body actually runs | y |
| `f F51` | `rust/game/no-thanks-2/src/lib.rs` fn `pass` | Fold the dead `player_chips[player] <= 0` branch's "no chips left" message into the single `can_pass` check (folding beats deleting - the message is currently never shown) and index chips the same defensive way `can_pass` does | y |
| `f F52` | `rust/game/no-thanks-2/src/lib.rs` fn `player_hand_grouped` + `src/render.rs` fn `group_sorted` | Extract one shared free function (e.g. `pub fn group_runs(sorted: &[i32]) -> Vec<Vec<i32>>`) and call it from both | n |

## WP-33 - liars-dice-2

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `f F58` | `rust/game/liars-dice-2/src/lib.rs` tests module + `rust/game/liars-dice-2/tests/contract.rs` | Add unit tests for pub_state/player_state dice redaction, call resolution with bid value 1 (the wild-1 branch), and a play-to-completion final-placings assertion | y |
| `f F56` | `rust/game/liars-dice-2/src/render.rs` fn `number_str` (and test fn `test_number_str`) | Fix the "fourty" spelling to "forty" and update the test - this IS reachable from ordinary input because the bid parser has no quantity cap | y |
| `f F57` | `rust/game/liars-dice-2/src/command.rs` fn `bid_parser` | Set the quantity `Int` parser's `max` to `players * START_DICE_COUNT` so help/suggest shows a sensible cap (never rejects a legal bid) | y |

## Escalate

None. All 17 fixes compress to one line.
