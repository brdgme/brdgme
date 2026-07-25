# no-thanks-2 review findings

Crate: `rust/game/no-thanks-2/` (780 LOC). Go original: `brdgme-go/no_thanks_1/`.
Reviewed: `src/lib.rs` (536), `src/command.rs` (48), `src/render.rs` (151), `tests/contract.rs`, `Cargo.toml`, `RULES.md`. `src/bin/` skipped per instructions (boilerplate).

## Headline

Core game logic is correct and matches both official No Thanks! rules and the Go
original: deck 3..=35 with 9 removed (shuffle 33, take 24), 11 chips for the
supported 3-5 player counts, pass/take/forced-take semantics, run scoring at the
lowest card, chips subtracted, lowest score wins, game ends at deck exhaustion.
Hidden-chips rule is correctly implemented (pub_state withholds all chip counts
during play; player_state reveals only own chips) — an improvement over the Go
original, which returned `nil` for both states. No player-input-reachable panic
paths found. Findings below are all minor/nit.

## Findings

### Vacuous test `test_init_player_chips` asserts nothing
- severity: minor
- category: quality
- location: game/no-thanks-2/src/lib.rs:392-399
- finding: The test builds `Game::default()` (so `players == 0`), calls `init_player_chips()`, then loops `for p in 0..g.players` — the loop body never executes, so the `assert_eq!(11, ...)` is never checked. The test would pass even if `STARTING_CHIPS` were changed to any value or `init_player_chips` filled with zeros. Contrast with `test_init_cards` (lib.rs:383-390) which does assert on the resulting vec length.
- recommendation: Set `g.players = 3` (or use `Game::start(3, 1)`) before calling `init_player_chips()`, and additionally assert `g.player_chips.len() == g.players`.

### Unreachable "no chips" branch in `pass()`
- severity: nit
- category: simplicity
- location: game/no-thanks-2/src/lib.rs:106-110
- finding: `can_pass()` (lib.rs:92-96) already requires `player_chips > 0`, and `pass()` returns early when `!can_pass(player)`, so the second check `if self.player_chips[player] <= 0` can never fire — the "You have no chips left, you must take the card" error message is dead. The Go original had the same redundancy (no_thanks.go:235-240), so this is a faithful port of Go dead code rather than a new defect. Note the defensive pattern is also inconsistent: `can_pass` uses `.get(player).copied().unwrap_or(0)` while `pass` indexes `self.player_chips[player]` directly.
- recommendation: Drop the dead branch (keep only the `can_pass` guard), or if the specific message is desired, fold it into a single check. Low priority.

### Run-grouping logic duplicated between lib.rs and render.rs
- severity: nit
- category: quality
- location: game/no-thanks-2/src/lib.rs:156-176 and game/no-thanks-2/src/render.rs:23-42
- finding: `Game::player_hand_grouped` and `group_sorted` are line-for-line the same run-detection algorithm. Render operates on `PubState` (not `Game`) so it can't call the method directly, which presumably motivated the copy; but any fix to one (e.g. scoring edge cases) must be mirrored in the other.
- recommendation: Extract a shared free function (e.g. `pub fn group_runs(sorted: &[i32]) -> Vec<Vec<i32>>` in lib.rs) used by both `player_hand_grouped` and the renderer.

### Player cap 3-5 deviates from official 3-7 rules (Go-preserved)
- severity: minor
- category: correctness
- location: game/no-thanks-2/src/lib.rs:18-20, 337-339
- finding: Official No Thanks! supports 3-7 players with starting chips scaled by count (11 for 3-5p, 9 for 6p, 7 for 7p). The crate hard-codes `MAX_PLAYERS = 5` and a single `STARTING_CHIPS = 11`. The Go original (no_thanks.go:34-36, 154, 220-225) had exactly the same restriction, so this is a documented Go-preserved deviation, not a port regression — cross-reference only. `RULES.md` accurately describes the implemented 3-5p/11-chip variant.
- recommendation: None required for parity. If 6-7p support is ever wanted, parameterise starting chips by player count; otherwise leave as-is.

### Renderer panics on inconsistent deserialized PubState
- severity: nit
- category: correctness
- location: game/no-thanks-2/src/render.rs:77, 91, 115
- finding: `render()` calls `pub_state.current_card.unwrap()` whenever `finished` is false, and indexes `pub_state.hands[p]` for `p in 0..pub_state.players` without checking `hands.len()`. These are safe for states produced by `Game::pub_state()` (fields are mutually consistent), but `PubState` is `Deserialize` and `Renderer for PubState` is public — rendering a crafted/corrupt deserialized state (e.g. `finished: false, current_card: None`, or `hands` shorter than `players`) panics. Same shape as the `player_state` unchecked index `self.player_chips[player]` at lib.rs:275 (assumes caller-validated player index; cross-cutting across crates).
- recommendation: Optional hardening: `if let Some(card) = pub_state.current_card` and `.get(p)` lookups in the renderer. Acceptable to leave if PubState rendering is only ever fed server-generated states.

## Clean aspects (verified, no findings)

- **Deck construction** (lib.rs:70-78): shuffles all 33 cards and takes 24 — equivalent to removing 9 unseen; `init_cards` result verified in-bounds by test.
- **Forced take at 0 chips** (command.rs:14-29): `pass` parser is only offered when `can_pass` (which requires chips > 0), so a broke player can only `take` — correct per official rules.
- **Scoring** (lib.rs:150-202): runs counted at lowest card only, chips subtracted at game end, lowest score wins via negated metric into `gen_placings`; mid-game `points_int` omits chips (matches Go). Tests cover grouping, scoring, placings.
- **Hidden chips** (lib.rs:243-277): pub_state exposes empty `chips`/`final_scores` until finished; each player's own count only via `player_state`. Correct per official rules; Go exposed everything (states were `nil`). Covered by `test_pub_state_chips_hidden_until_finished`.
- **Game end** (lib.rs:121-148, 229-241): last take empties the deck → finished; final centre chips go to the taker before reset; placings log appended on the finishing take (lib.rs:312-318).
- **Panic audit**: `peek_top_card`'s `expect` (lib.rs:84-86) is guarded by `can_pass`/`can_take`/`is_finished`/`finished` checks on every call site; all `player_hands[player]`/`player_chips[player]` indexing in `pass`/`take` is gated by `currently_moving == player`. No crafted-command input reaches a panic — the parser only yields `Pass`/`Take` tokens.
- **Port fidelity improvements over Go**: `player_hand_sorted` clones instead of Go's in-place mutation of game state (no_thanks.go:291-294); deterministic seeded RNG replaces Go's `time.Now().UnixNano()` seeding; `remaining_after` uses `saturating_sub` instead of Go's `len-1`.
- **Contract test** present (tests/contract.rs) using the shared `assert_gamer_contract`.
- `Cargo.toml`: `tokio` (full) and `brdgme_fuzz` declared as library dependencies for binary-only use — known systemic issue (binary-only deps declared as library deps), cross-reference only. HTTP bin binding and other bin boilerplate not reviewed per instructions.
