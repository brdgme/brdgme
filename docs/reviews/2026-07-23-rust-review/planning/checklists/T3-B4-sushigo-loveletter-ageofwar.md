# T3-B4: sushi-go-2 + love-letter-2 + age-of-war-2

- **Batch**: T3-B4 = WP-24 (sushi-go-2, 7 findings) + WP-27 (love-letter-2 +
  age-of-war-2, 8 findings)
- **Crates**: `rust/game/sushi-go-2`, `rust/game/love-letter-2`,
  `rust/game/age-of-war-2`
- **Sources**: `findings/games-batch-d.md` and `findings/games-batch-e.md`,
  each superseded where they differ by `findings/verification/games-batch-d.md`
  and `findings/verification/games-batch-e.md`
- **Numbering**: games-batch-d (`d Fnn`) for WP-24, games-batch-e (`e Fnn`) for
  WP-27. Raw and verification numbering are identical in both batches - no
  offset hazard.
- **Rows**: 15 (4 minor / 11 nit). No findings rejected in either package
  (review-wide rejections were `d F13` and `ws F30`, neither in scope). No rows
  are decision-blocked - `decisions-needed.md` mentions none of these three
  crates, and both packages are READY.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by crate then source file so one session sweeps a file at a
time.

## WP-24 - sushi-go-2 (`src/lib.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `d F27` | `rust/game/sushi-go-2/src/lib.rs` test fn `test_hand_passing_left` | Delete the vacuous test (real coverage lives in test fn `test_passing_direction`) or replace its body with real hand-passing assertions | y |
| `d F28` | `rust/game/sushi-go-2/src/lib.rs` fns `player_draw_counts` / `draw_count` | Replace the table lookup + `.unwrap_or(9)` with an exhaustive `match players` so no caller can silently fall back, keeping 2/3 => 9, 4 => 8, 5 => 7 | y |
| `d F29` | `rust/game/sushi-go-2/src/lib.rs` fn `Card::explanation` (Pudding arm) | Note that the fewest-pudding penalty does not apply in 2p (e.g. "end: most 6, least -6 (no penalty in 2p)") | n |
| `d F30` | `rust/game/sushi-go-2/src/lib.rs` fn `score` (maki second-place award) | Drop the `second_players.len() <= 3` condition - integer division already yields 0 points for a 4-way second-place tie, so only the suppressed log changes | n |
| `d F32` | `rust/game/sushi-go-2/src/lib.rs` fn `play` (two-card "save one for the dummy" guard) | Reorder the guard so `self.players == 2` is tested before `self.playing[DUMMY].is_none()`, matching fn `can_dummy` | n |

## WP-24 - sushi-go-2 (`src/lib.rs` + `src/render.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `d F31` | `rust/game/sushi-go-2/src/lib.rs` fn `Game::render_name` + `rust/game/sushi-go-2/src/render.rs` free fn `render_name` | Keep one implementation (free fn in render.rs, called by the method) and write the dummy check as `player >= players` instead of `player > players - 1` | n |

## WP-24 - sushi-go-2 (`RULES.md`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `d F26` | `rust/game/sushi-go-2/RULES.md` `## Scoring` section (behaviour source: `src/lib.rs` fn `placings`) | Doc-only: document that score ties are broken by most pudding cards - verification refuted the finding's premise, the official Gamewright rulebook does break ties this way, so **do not** change `placings` or `test_placings_pudding_tiebreaker` | n |

## WP-27 - love-letter-2 (`src/command.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F5` | `rust/game/love-letter-2/src/command.rs` fn `command_parser` | Return `None` when `self.check_finished()` so a finished game offers no commands and post-game plays cannot re-run `end_round` | y |

## WP-27 - love-letter-2 (`src/lib.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F8` | `rust/game/love-letter-2/src/lib.rs` fn `discard_card` | Only push to `discards[player]` when the card was actually removed from the hand, so the public discard record cannot be corrupted by a `play_*` call for an unheld card | y |
| `e F6` | `rust/game/love-letter-2/src/lib.rs` fn `play_baron` | Delete the two dead `self.hands[...] = vec![...]` assignments (`discard_card` already left exactly `[player_card]`) | n |
| `e F7` | `rust/game/love-letter-2/src/lib.rs` fn `play_guard` + `PORTING_NOTES.md` | No behaviour change: add a PORTING_NOTES entry recording that the `target == player` early return deliberately precedes the Guard-guess validation, as in Go | n |

## WP-27 - age-of-war-2 (`src/lib.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F11` | `rust/game/age-of-war-2/src/lib.rs` the `Game` struct field `completed_lines` | Change `HashSet<usize>` to `BTreeSet<usize>` (same API surface is used) so the persisted blob serializes deterministically; the existing sort in the pub-view builder can stay or go | n |
| `e F12` | `rust/game/age-of-war-2/src/lib.rs` fn `command` (the `Gamer` impl) | Return `GameError::NotYourTurn` instead of `GameError::invalid_input("not your turn")` when `command_parser` yields `None` | y |

## WP-27 - age-of-war-2 (`src/lib.rs` + `src/render.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F15` | `rust/game/age-of-war-2/src/lib.rs` fn `Game::clan_conquered` + `rust/game/age-of-war-2/src/render.rs` free fn `clan_conquered` | Extract one shared helper over `&[bool]` + `&[Option<usize>]` and call it from both, preserving the stale-player-on-`false` quirk verbatim | y |

## WP-27 - age-of-war-2 (`src/command.rs`)

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `e F16` | `rust/game/age-of-war-2/src/command.rs` roll command description | Change "discard one dice and roll the rest" to "discard one die and reroll the rest", updating any suggest/help snapshot test carrying the spec text | n |

## Decision-blocked rows

None. Neither WP-24 nor WP-27 sits behind an open decision, and no sushi-go /
love-letter / age-of-war item appears in `planning/decisions-needed.md`. The
parked rules-review packages (WP-11, WP-12, WP-16, WP-20, WP-26, WP-30) own no
findings in this batch.

## Not in this checklist (owned elsewhere)

- `d F33` (sushi-go-2 `command()` duplicates the finished-game placings-log
  block across the Play and Dummy arms) - owned by
  `specs/WP-08-finish-placings-epilogue-dedup.md`.
- `e F13` and `e F14` (age-of-war-2 triplicated placings-log tail; duplicate
  placings logs after finish) - owned by
  `specs/WP-08-finish-placings-epilogue-dedup.md`. `e F12` above touches the
  same `command` fn, so land WP-08's age-of-war edit and `e F12` in one sitting
  to avoid conflicts.
- `e F10` (age-of-war-2 unwrap/expect cluster) and the love-letter-2
  `assert_target` / `end_round` / `end_score` items - Tier 2, owned by
  `specs/WP-09a-deserialized-state-boundary.md` /
  `specs/WP-09b-game-crate-state-trust-sweep.md`.
- `d F24` (round-2 passing direction) and `d F25` (all-tied pudding award) are
  outside WP-24's scope list and are not to be touched here.

## Escalate

None. All 15 fixes compress to one line.
