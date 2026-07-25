# greed-2 review (2026-07-23)

Scope: full read of `src/lib.rs`, `src/command.rs`, `src/render.rs`, `tests/contract.rs`,
`Cargo.toml`, `RULES.md`, `DATA_DOCS.md`; compared against Go original
`brdgme-go/greed_1/` (greed.go, scores.go, command.go, score_command.go,
roll_command.go, done_command.go). Binaries under `src/bin/` are the standard
boilerplate and were not reviewed.

## Parity / correctness summary (no findings)

- Scoring table (`SCORES`, lib.rs:77-140) is an exact match to Go `Scores()`
  (scores.go:14-32) in both values and priority order: six-of-a-kind 5000 (any
  face), four D 1000, straight 1000, three $ 600, three G 500, three R 400,
  three E/e 300, single D 100, single G 50. Matches RULES.md.
- `dice_in_dice` / `dice_equals` (lib.rs:146-172) correctly implement multiset
  containment/equality; remaining-dice order is deterministic. Unit-tested.
- Bust handling: Go used recursion (`Roll` -> `NextPlayer` -> `StartTurn`);
  Rust `start_turn` (lib.rs:266-292) uses a loop with the same semantics and
  correctly stops mid-cascade when `finished()` becomes true. Improvement.
- Hot-dice reroll correct: `player_roll` (lib.rs:320-344) rerolls all 6 when
  `remaining_dice` is empty, else rerolls the remainder.
- `done` auto-take (lib.rs:350-356) takes available combos in `scores()`
  priority order, identical to Go `Done` taking `available[0]`. Verified the
  priority order is also score-optimal for every reachable 6-dice multiset
  (no case where a smaller combo taken first would bank more).
- End-game trigger (`finished`, lib.rs:240-242) matches Go `IsFinished`
  exactly: round closes when play returns to `first_player` AND any banked
  score >= 5000. `placings` via shared `gen_placings`; tie semantics
  ([1,1,3]) pinned by test (lib.rs:672-684).
- No minimum-to-open rule in either version; RULES.md agrees.
- No panic/unwrap/index path reachable from crafted command input: the command
  parser only offers currently-valid score tokens, `Die` deserialization is
  enum-bounded (serde only accepts the 6 named variants), and all
  `scores[player]` indexing is gated by `player == current_player`.
- `score` returns `can_undo: true` (no RNG consumed); `roll`/`done` return
  false (RNG consumed). Matches Go.

## Findings

### `Game::score` ignores its `player` argument and skips turn validation
- severity: minor
- category: quality
- location: game/greed-2/src/lib.rs:294-295
- finding: `pub fn score(&mut self, player: usize, dice: &[Die])` discards
  `player` (`let _ = player;`) and performs no `can_score` check, unlike
  `player_roll` and `done` which validate the caller. The log it emits
  attributes the score to `current_player` regardless of the `player` passed.
  Today this is unreachable from crafted input (the command parser is only
  built when `can_score(player)` holds), and the Go original `Score` had the
  same shape, but it is a latent footgun: any future direct caller (as
  `done()` already is at lib.rs:355) can mutate another player's turn.
- recommendation: either validate `player == self.current_player` (and
  non-finished) inside `score` for symmetry with the other mutators, or drop
  the unused parameter and have `done()` call a private helper.

### E/e score-token collision makes `score eee` consume the E1 triple first
- severity: minor
- category: correctness
- location: game/greed-2/src/command.rs:56-64 (also lib.rs:52-55, 124-130)
- finding: `Die::E1.name()` is `"E"` and `Die::E2.name()` is `"e"`, so the
  three-of-a-kind parsers get tokens `"EEE"` and `"eee"`. `Token::parse` is
  case-insensitive (`UniCase`, rust/lib/game/src/command/parser/mod.rs:51)
  and `OneOf` is first-Ok-wins with the E1 triple listed first in `SCORES`,
  so when a player holds both triples, `score eee` consumes the E1 dice.
  Not a soft-lock (the leftover E2 triple can then be scored), and this is
  Go parity — cross-reference, pinned by
  `test_score_case_insensitive_e1_e2_collision` (lib.rs:729-746). Noted as a
  preserved Go quirk, not a fresh regression.
- recommendation: none required for the port; if ever fixed, the die names
  (not the parser) are the place to disambiguate.

### Duplicated placings-log block in the Roll and Done command arms
- severity: nit
- category: simplicity
- location: game/greed-2/src/lib.rs:476-491 and 496-511
- finding: the `if self.is_finished() { ... placings_log(...) }` block is
  copy-pasted verbatim in the `Command::Roll` and `Command::Done` arms of
  `command()`.
- recommendation: extract a small `finish_logs(&self) -> Option<Log>` helper
  (or append after the match) to remove the duplication.

### Scores/dice length invariants unchecked on deserialized state
- severity: minor
- category: correctness
- location: game/greed-2/src/lib.rs:366, 375, 522; game/greed-2/src/render.rs:79
- finding: `Game` is `Deserialize` with no validation, so a stored/crafted
  state with `scores.len() != players` (or `current_player >= players`)
  panics on index in `done` (lib.rs:366), `placings` (lib.rs:375), `points`
  (lib.rs:522) and `PubState::render` (render.rs:79). Game state comes from
  the trusted store rather than direct player input, so reachability is low;
  common shape across the ported crates.
- recommendation: no crate-specific action needed; if a systemic
  state-validation hook is ever added, assert `scores.len() == players` and
  `current_player < players` here.

### Theoretical i32 overflow in turn/banked score arithmetic
- severity: nit
- category: correctness
- location: game/greed-2/src/lib.rs:307, 366
- finding: `turn_score += value` and `scores[player] += turn_score` are plain
  i32 adds. A turn can accumulate arbitrarily via hot-dice rerolls (each
  six-of-a-kind banks 5000 and rerolls all 6), so overflow is mathematically
  possible (~430k consecutive scoring rerolls); it is not reachable through
  any realistic play or crafted input, and Go had the same int arithmetic.
- recommendation: none practical; `saturating_add`/`checked_add` would be the
  defensive fix if ever desired.

### Die::E1 rendered as `Foreground` though RULES.md says black
- severity: nit
- category: consistency
- location: game/greed-2/src/lib.rs:64; game/greed-2/RULES.md:18
- finding: Go used `render.Black` for the `E` face; the port maps it to
  `color::NamedColor::Foreground` while RULES.md still documents the colour as
  "black". Almost certainly a deliberate adaptation (true black is invisible
  on dark terminals), but the rules doc and the code now disagree.
- recommendation: update RULES.md's colour column (or add a note) if
  `Foreground` is intentional.

### Binary-only dependencies declared as library dependencies
- severity: nit
- category: dependencies
- location: game/greed-2/Cargo.toml:9-10, 16
- finding: `brdgme_cmd`, `brdgme_fuzz` and `tokio` are only used by the
  `src/bin/` targets, not the library. Cross-reference of the known systemic
  "binary-only deps declared as library deps" issue; this crate is a standard
  consumer.
- recommendation: track under the systemic issue; no crate-specific action.

## Notes

- `tests/contract.rs` runs the shared `assert_gamer_contract`; the inline
  unit tests in lib.rs cover scoring, bust/flow, finished/placings ties, and
  the E/e collision. Coverage is good for a 983-LOC crate.
- The migration shim on `Game::rng` (lib.rs:191-194,
  `#[serde(default = "GameRng::from_entropy")]`) is documented with a removal
  note; standard pattern.
