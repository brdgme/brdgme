# W4 verification: games-batch-f F38-F48 (farkle-2, tic-tac-toe-2)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust

### F38 Scoring table duplicated between lib.rs and render.rs — CONFIRMED
- evidence: render.rs:24-46 `scoring_table()` hardcodes 8 (name, points) entries; lib.rs:47-82 `SCORES` holds the same 8 combinations/values. No code links them; a change to SCORES silently drifts the rendered help table (Go has no rendered scoring table at all, so this duplication is Rust-side only).
- severity: agree, minor/consistency is right - drift risk, no current bug.
- recommendation-check: valid. Deriving from `scores()` works; display names ("Single 1", "Three 2s") are mechanically derivable from the dice multiset. One caveat: SCORES order (Single 5 first, Three 1s last) differs from the current display order (Single 1 first), so a naive derivation changes row order - cosmetic, not a bug.

### F39 score() is pub but ignores its player argument — CONFIRMED
- evidence: lib.rs:245-246 `pub fn score(&mut self, player: usize, dice: &[Die]) ... { let _ = player; ...}` with no turn check, while `player_roll` (lib.rs:272) checks `can_roll` and `done` (lib.rs:298) checks `can_done`. Gating happens in command.rs:19 (`if self.can_score(player)` before offering the parser). Go parity confirmed: Go `Score` (score_command.go) also never reads `player`, and Go `Done` does validate via `CanDone` - same asymmetry.
- severity: agree, nit/quality.
- recommendation-check: valid. Adding a `can_score(player)` guard is safe: command() only reaches score() when the parser was offered, i.e. can_score already held; tests call score() only with `g.current_player`. `debug_assert_eq!` alternative also safe.

### F40 Finished game leaks stale turn_score/remaining_dice into pub_state/render — CONFIRMED
- evidence: `bust()` (lib.rs:203-212) advances current_player without zeroing turn_score/remaining_dice; the reset lives in `start_turn()` (lib.rs:225-227), which is skipped when `finished()` (lib.rs:290-292, :313-315). `pub_state()` (lib.rs:365-380) copies turn_score/remaining_dice unconditionally and the renderer (render.rs:52-64) always shows the "Score this turn" table. Go identical: `NextPlayer` (farkle.go) skips `StartTurn` when finished and `PubRender` always prints `g.TurnScore` - preserved quirk as claimed. One nuance: on a finishing done, turn_score was banked (not lost), so the display is stale rather than wrong-in-both-senses; on a finishing bust it shows points that were actually lost.
- severity: agree, nit/correctness (cosmetic; placings/scores unaffected).
- recommendation-check: valid. Either zeroing turn_score when finished or skipping the table when `self.finished` is safe; the PubState already carries `finished` so the renderer option needs no state change.

### F41 Test sets out-of-range current_player — CONFIRMED
- evidence: lib.rs:614 `g.current_player = g.first_player + 1;` in `test_finished_and_placings` with a 3-player game (lib.rs:611). If seed 1 yields `first_player == 2`, current_player becomes 3. Today the only uses are the `finished()` equality compare (lib.rs:615-616) and `placings()` which iterates 0..players, so nothing panics; the intent (a non-first player) also still holds since 3 != first_player. Latent hazard only, exactly as characterized.
- severity: agree, nit/quality.
- recommendation-check: valid. `(g.first_player + 1) % 3` preserves the "not first player" intent for all seeds.

### F42 render.rs uses u8 instead of the Die alias — CONFIRMED
- evidence: render.rs:6 `pub fn render_die(d: u8)` and render.rs:13 `pub fn render_dice(dice: &[u8], ...)`; lib.rs:25 declares `pub type Die = u8` and lib.rs uses `Die`/`Vec<Die>` throughout (e.g. :91, :140, :245).
- severity: agree, nit/consistency.
- recommendation-check: valid; `Die` is `pub` and render.rs already imports from `crate` (render.rs:4).

### F43 Simplified Farkle variant, Go-faithful cross-reference — CONFIRMED (external basis for published-rules claims)
- evidence: lib.rs:22 `WIN_SCORE: i32 = 5000`; lib.rs:47-82 SCORES contains only single 1/5 and the six three-of-a-kinds - matches Go `Scores()` (scores.go) value-for-value and Go's 5000 target (farkle.go `IsFinished`). RULES.md:32-45 documents exactly these combinations with "Only these exact combinations score". The claim that published Farkle also scores straights/three pairs/4+-of-a-kind to 10000 rests on external rulebook knowledge - marked external basis per charter, and the finding itself frames it as a cross-reference, not a divergence.
- severity: agree, nit/correctness as informational cross-reference.
- recommendation-check: valid ("none required").

### F44 PubState renderer indexes scores[p] without a length check — CONFIRMED
- evidence: render.rs:71-78 `for p in 0..self.players { ... self.scores[p] ... }` - a PubState with `players > scores.len()` panics. Server-generated states always satisfy the invariant (pub_state() clones the full vec, lib.rs:367-370). Same pattern flagged crate-wide.
- severity: agree, nit/correctness (unreachable via server path).
- recommendation-check: valid; zipping `(0..players)` with `scores` is a correct hardening if ever needed.

### F45 `1 - start_player` underflows on crafted state — CONFIRMED
- evidence: render.rs:34 `let o_player = 1 - start_player;` - with `start_player >= 2` from a forged/desynced state this is a usize underflow: panic under debug/overflow-checks, wrap to a huge `N::Player` index in release. Overflow-checks claim verified: workspace Cargo.toml defines `[profile.dev]`, `android-dev`, `server-dev`, `wasm-dev`, `wasm-release` and no profile sets `overflow-checks` (grep over all Cargo.toml files: no hits), so release inherits the default `overflow-checks = false`. Internal inconsistency verified: lib.rs:141 `Cell::O => (self.start_player + 1) % NUM_PLAYERS` computes the same mapping safely. Only reachable with a forged state (server states have start_player in 0..2, lib.rs:177).
- severity: agree, minor/correctness - fragile + internally inconsistent, but forged-state-only, so not major.
- recommendation-check: valid. `(start_player + 1) % NUM_PLAYERS` in render.rs needs only adding `NUM_PLAYERS` to the existing `use crate::{...}` (render.rs:5) and produces identical results for all legal inputs (start_player 0 -> 1, 1 -> 0).

### F46 Crafted `players` count drives unbounded allocation/iteration — CONFIRMED
- evidence: lib.rs:154-160 `placings()` builds `metrics: Vec<Vec<i32>>` sized by `self.players`; lib.rs:268-273 `points()` likewise collects `0..self.players`. `players` is a plain pub usize field (lib.rs:52) with no deserialization validation. Requester claim verified: lib/cmd/src/requester/gamer.rs:28, :37, :41, :45 all do `serde_json::from_str(game)` straight into `G` with no post-deserialization checks, and `renders()` (gamer.rs:70) iterates `(0..game.player_count())` building a full PlayerRender per index - a forged `players` of e.g. u64::MAX makes Status/PubRender/PlayerRender requests allocate wildly or loop effectively forever. Systemic across all games, as noted.
- severity: agree, minor/correctness - real DoS shape but requires a forged state reaching the requester; consistent with treating trust-boundary hardening as systemic rather than per-crate major.
- recommendation-check: valid. A requester-layer validation is the right systemic fix; the per-game `players == NUM_PLAYERS` check is trivially correct for tic-tac-toe since NUM_PLAYERS is a const (lib.rs:19).

### F47 Dead, misleading Cell::Empty arm in winner() — CONFIRMED
- evidence: lib.rs:146-148 `matching_line` returns `(line[0] != Cell::Empty && ...).then_some(line[0])` - it can only ever yield X or O. The mapping at lib.rs:139-143 nevertheless has `Cell::Empty => self.start_player`, which is unreachable, and if it ever became reachable it would falsely credit the start player with a win. Exactly as described.
- severity: agree, nit/quality (dead code in the canonical example crate).
- recommendation-check: valid. Either a mark-only enum or `unreachable!()` removes the misleading mapping; `unreachable!()` is the minimal change and cannot fire given matching_line's guard.

### F48 Mark casing inconsistent between logs and board render — ADJUSTED
- evidence: the inconsistency is real but the split is misdescribed. Log: lib.rs:107-111 mark is uppercase `"X"`/`"O"`, rendered bold at lib.rs:116. Board cells: render.rs:16-17 lowercase `N::text("x")`/`N::text("o")`. BUT the "is X / is O" label is uppercase, not lowercase as the finding states: render.rs:36 `N::text(" is X, ")` and render.rs:38 `N::text(" is O")`, confirmed by the exact-render test lib.rs:598 `"{{player 0}} is X, {{player 1}} is O"`. RULES.md:10-16 documents lowercase `x`/`o`. So the actual split is: log uppercase, label uppercase, board lowercase, RULES.md lowercase - the finding's claim that "the board and label use lowercase" is half wrong.
- severity: agree, nit/consistency.
- recommendation-check: valid - "pick one casing and use it everywhere" still applies; note that changing the board glyphs also requires updating the exact-render test (lib.rs:589-600) and RULES.md, whichever direction is chosen.
