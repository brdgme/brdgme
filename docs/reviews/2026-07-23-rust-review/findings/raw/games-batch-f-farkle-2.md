# farkle-2 review findings

Scope: `rust/game/farkle-2/` (lib.rs, command.rs, render.rs, tests/, Cargo.toml, RULES.md)
compared against Go original `brdgme-go/farkle_1/`. The 4 `src/bin/` binaries are
boilerplate and were not reviewed in depth.

Overall: this is a careful, faithful port. No critical or major findings. Game logic
(scoring table, kept-dice multiset validation, bust, hot dice, banking, end trigger,
placings) matches the Go original and the crate's own RULES.md. Library code contains
no `unwrap`/`expect`/`panic`/unchecked-index paths reachable from player input;
`dice_in_dice` is explicitly hardened against corrupted die values (0/7) with tests
(lib.rs:91-116, tests at lib.rs:534-542). Improvements over Go: seeded `GameRng`
instead of per-roll `time.Now()` seeds, stricter player-count validation (Go only
checked `< 2`), and an iterative (rather than recursive) bust cascade.

### Scoring table duplicated between lib.rs and render.rs
- severity: minor
- category: consistency
- location: game/farkle-2/src/render.rs:24-46 (vs game/farkle-2/src/lib.rs:47-82)
- finding: The help/scoring table rendered to players hardcodes the eight combinations
  and point values in `scoring_table()`, duplicating the authoritative `SCORES` static
  in lib.rs. If a combination or value is ever changed in `SCORES`, the rendered table
  silently drifts out of sync with actual scoring (and with RULES.md).
- recommendation: Derive the rendered table from `scores()` (e.g. render the dice with
  `render_dice` plus value), or at minimum generate point values from `SCORES` so only
  the display names are local.

### `score()` is pub but ignores its `player` argument
- severity: nit
- category: quality
- location: game/farkle-2/src/lib.rs:245-246
- finding: `pub fn score(&mut self, player: usize, dice: &[Die])` discards `player`
  (`let _ = player;`) and performs no turn validation; correctness relies entirely on
  the `command()` path gating via `can_score`. Any other in-crate caller (bot harness,
  future code) can score out of turn. `player_roll`/`done` do validate via
  `can_roll`/`can_done`, so `score` is the odd one out. Go `Score` has the same shape,
  so this is a faithful port, but the Rust version already deviates by validating in
  the sibling methods.
- recommendation: Add the same guard style as the siblings
  (`if !self.can_score(player) { return Err(...) }`), or at least a
  `debug_assert_eq!(player, self.current_player)`.

### Finished game leaks stale `turn_score`/`remaining_dice` into pub_state/render
- severity: nit
- category: correctness
- location: game/farkle-2/src/lib.rs:203-212 (`bust`), 297-317 (`done`), 365-380 (`pub_state`)
- finding: When the game ends via a bust or a final `done`, `turn_score` is left at
  its last value (for a finishing bust, the points that were just "lost") and
  `remaining_dice` keeps the last roll. `pub_state` therefore reports a non-zero
  "Score this turn" and leftover dice for a finished game, and the final render shows
  them. Cosmetic only — placings/scores are correct. The Go original has identical
  behaviour (TurnScore never reset on finish), so this is a preserved quirk, not a
  divergence.
- recommendation: Optionally zero `turn_score` when `finished()` becomes true, or have
  the renderer skip the "Remaining dice"/"Score this turn" table when `self.finished`.

### Test sets out-of-range `current_player`
- severity: nit
- category: quality
- location: game/farkle-2/src/lib.rs:614
- finding: `test_finished_and_placings` does `g.current_player = g.first_player + 1`
  with 3 players; if the seeded `first_player` is 2, `current_player` becomes 3, an
  invalid player index. No method called afterwards indexes by `current_player`, so it
  is harmless today, but the test constructs a state the real game can never reach and
  would panic if any future assertion touched `scores[current_player]`.
- recommendation: Use `(g.first_player + 1) % 3`.

### render.rs uses `u8` instead of the `Die` alias
- severity: nit
- category: consistency
- location: game/farkle-2/src/render.rs:6,13
- finding: `render_die(d: u8)` and `render_dice(dice: &[u8], ...)` bypass the crate's
  own `pub type Die = u8` alias used everywhere else in the crate.
- recommendation: Use `Die` / `&[Die]` for consistency.

### Binary-only deps declared as library deps (known systemic issue — cross-reference)
- severity: nit
- category: dependencies
- location: game/farkle-2/Cargo.toml:8-16
- finding: `brdgme_cmd`, `brdgme_fuzz`, and `tokio` (full features) are in
  `[dependencies]` although the library itself only needs `brdgme_color`,
  `brdgme_game`, `brdgme_markup`, `rand`, `serde`; the first three are only used by the
  `src/bin/` targets. This is the known repo-wide "binary-only deps declared as library
  deps" issue; farkle-2 is an ordinary consumer, nothing crate-specific.
- recommendation: Track under the systemic issue; no crate-specific action.

### Simplified Farkle variant (no straight/three-pairs/4+-of-a-kind, no opening minimum, 5000 target) — Go-faithful cross-reference
- severity: nit
- category: correctness
- location: game/farkle-2/src/lib.rs:22,47-82; game/farkle-2/RULES.md:32-49
- finding: Standard/published Farkle rules also score a 1-6 straight, three pairs, and
  four/five/six of a kind, usually require an opening minimum (e.g. 300/500), and play
  to 10000 with a final round. This crate implements only single 1s/5s and
  three-of-a-kind, no opening minimum, and a 5000 target with the round closing when
  play returns to the first player. This exactly matches the Go `farkle_1` original
  (`Scores()` in scores.go, `IsFinished` in farkle.go:137-147) and is accurately
  documented in RULES.md ("Only these exact combinations score", "First to 5000"), so
  it is a preserved Go quirk, not a port divergence. Note 4-of-a-kind is still
  reachable as triple + single (e.g. four 1s = 1000 + 100), which is consistent.
- recommendation: None required; only revisit if the project ever wants full Farkle
  rules (would be a rules change in both RULES.md and `SCORES`).

### PubState renderer indexes `scores[p]` without a length check
- severity: nit
- category: correctness
- location: game/farkle-2/src/render.rs:71-80
- finding: `PubState` derives `Deserialize`, so a crafted/desynced state with
  `players > scores.len()` would panic the renderer at `self.scores[p]`. In practice
  `PubState` is produced by `Game::pub_state()` where lengths always match, and
  rendering happens server-side from live game state, so this is not reachable from
  normal play; noted for completeness (same pattern likely exists in other crates).
- recommendation: None required; optionally `zip` scores with the player range if the
  renderer is ever exposed to deserialized input.

## Checks that came back clean
- Scoring values match RULES.md and Go exactly (50/100/200/300/400/500/600/1000).
- Kept-dice validation is a true multiset check (`dice_in_dice`) and kept dice must
  exactly equal a scoring combination (`dice_equals` against the table) — no way to
  score non-contributing dice or dice you don't have; error precedence ("doesn't
  score" before "don't have those dice") matches Go.
- Bust loses turn points and cascades turns correctly; hot dice (all 6 scored → reroll
  6) implemented at lib.rs:276-280; `done` banks without auto-scoring leftovers
  (explicitly tested, lib.rs:659-674).
- End condition (`current_player == first_player && any score >= 5000`) is a faithful
  port of Go `IsFinished`; ties share placings via `gen_placings` (tested,
  lib.rs:618-621).
- No arithmetic-safety concerns: i32 scores, bounded in practice; parser dice bounded
  to 1..=6 before the `as Die` cast (command.rs:46-51).
- Command gating (`can_score`/`can_roll`/`can_done`) is correct in all reachable
  states; a player always has at least one available command while active.
- Undo flags match Go (score undoable, roll/done not — roll/done consume RNG).
- `tests/contract.rs` runs the shared `assert_gamer_contract`; good unit-test coverage
  including corrupted-die hardening.
