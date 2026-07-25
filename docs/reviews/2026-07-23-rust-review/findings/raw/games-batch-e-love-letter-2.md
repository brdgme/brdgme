# Raw findings: game/love-letter-2 (games batch E)

Reviewer scope: `rust/game/love-letter-2/` (lib.rs, command.rs, render.rs, card.rs,
tests/contract.rs, Cargo.toml, 4 src/bin binaries), cross-referenced against
`brdgme-go/love_letter_1/`. PORTING_NOTES.md read; documented preserved quirks
(AssertTarget double-Eliminated check, Priest doc typo, EndRound winner-selection,
draw-from-removed[0] semantics, empty Status::Active.eliminated) are NOT flagged.

`rules()` returns `include_str!("../RULES.md")` (src/lib.rs:879) — convention satisfied.
DATA_DOCS.md / BASIC_STRATEGY.md / ADVANCED_STRATEGY.md all present and included
(src/lib.rs:882-892).

Go parity was verified line-by-line for game.go and char_{guard,king,prince,baron,
countess,princess}.go: log wording, ordering of asserts, self-target fallback,
tiebreak logic, burn-card counts (2p=4, else 1), end scores (7/5/4) all match.
No correctness divergence from the Go original was found in game rules logic.

## Findings

### `end_score` uses `unreachable!()` in a runtime path
- severity: minor
- category: correctness
- location: game/love-letter-2/src/lib.rs:29
- finding: `end_score()` ends in `_ => unreachable!()`. It is called from
  `check_finished()` (lib.rs:273), `pub_state()` (lib.rs:685) and indirectly from
  `status()` — all runtime server paths. `Game::default()` has `players = 0`, so any
  code path that calls `pub_state()`/`status()` on a default or corruptly-hydrated
  `Game` (e.g. malformed DB state) panics and kills the request. Not reachable from
  crafted player input through `Game::start` (player count validated at lib.rs:645),
  but violates the CODING.md "no panicking code in runtime paths" rule.
- recommendation: Return a safe fallback for out-of-range counts (e.g. `usize::MAX`
  so the game never reports finished, or `match ... _ => 4`), or make the invariant
  unrepresentable. At minimum downgrade to `debug_assert!` + fallback.

### `end_round` indexes `hands[p][0]` without checking emptiness
- severity: minor
- category: correctness
- location: game/love-letter-2/src/lib.rs:184
- finding: `let c = self.hands[p][0];` panics if any non-eliminated player has an
  empty hand at round end. The invariant "non-eliminated players always hold exactly
  one card" holds for all current play paths (verified against Go), so this is a
  latent panic rather than a reachable bug, but it is an unchecked indexing path in
  a runtime request handler.
- recommendation: Use `.first()` and skip/continue on `None`, or document the
  invariant with a comment if the indexing is kept for Go parity.

### `assert_target` and play methods index state with unvalidated `target`
- severity: minor
- category: correctness
- location: game/love-letter-2/src/lib.rs:305
- finding: `assert_target()` indexes `self.eliminated[target]` (lines 305, 311)
  with no `target < self.players` check, and the play methods then index
  `self.hands[target][0]` (play_king lib.rs:405-407, play_prince lib.rs:444,
  play_baron lib.rs:500-501). `target` originates from the command parser's
  `Player {}` parser, whose indices come from the `players: &[String]` slice passed
  into `command()` — nothing clamps it to the game's actual player count. If a
  caller ever passes a names slice longer than `self.players`, a crafted command
  naming the extra "player" panics the game service request. Go has the identical
  defect (`g.Eliminated[target]` unguarded), so this is parity, but Rust-side the
  fix is one cheap bounds check.
- recommendation: Add `if target >= self.players { return Err(GameError::invalid_input("invalid target player")) }`
  at the top of `assert_target()`.

### Commands are still accepted after the game is finished
- severity: minor
- category: correctness
- location: game/love-letter-2/src/command.rs:23
- finding: `command_parser()` only checks `self.current_player != player`; there is
  no `is_finished()`/`check_finished()` guard. After the game ends, the current
  player still holds a card from the final round, so the parser offers commands and
  `command()` will happily execute plays (discards, eliminations, `end_round`
  re-runs awarding more points) on a finished game. Go has the same shape (CanPlay
  is only `CurrentPlayer == player`), and the web layer presumably blocks commands
  on finished games, but the crate itself doesn't enforce it — defence in depth is
  otherwise applied (`assert_can_play` per PORTING_NOTES).
- recommendation: Return `None` from `command_parser()` (or an error from
  `command()`) when `self.check_finished()`.

### `command()` match arms duplicate the same ~20-line wrap-up 8 times
- severity: major
- category: simplicity
- location: game/love-letter-2/src/lib.rs:698
- finding: Each of the 8 `Ok(ParseOutput { ... })` arms in `Gamer::command`
  (lib.rs:713-857) repeats the identical block: call play_*, then
  `if self.is_finished()` build `scores` and push `placings_log`, then construct
  `CommandResponse { logs, can_undo: false, remaining_input: remaining.to_string() }`.
  ~140 lines of copy-paste where only the play_* call differs; any change to the
  finish/scoring behaviour must be edited in 8 places (an easy place for one arm to
  drift, exactly the class of bug the ported Go `AssertTarget` copy-paste exhibits).
- recommendation: Collapse to a single `Ok(ParseOutput { value, remaining, .. })`
  arm, `match value { Command::Princess => self.play_princess(player)?, ... }` to
  get `logs`, then run the shared finish/response wrap-up once.

### Redundant no-op hand assignments in `play_baron`
- severity: nit
- category: quality
- location: game/love-letter-2/src/lib.rs:529
- finding: `self.hands[player] = vec![player_card];` (and the mirror at line 532)
  are no-ops: `discard_card(player, Card::Baron)` at line 479 already removed the
  Baron, leaving exactly `[player_card]` (and `player_card` was read from
  `hands[player][0]` after the discard). The assignments are a verbatim port of the
  same dead assignments in Go `PlayBaron`, but unlike the other preserved quirks
  this one is not documented in PORTING_NOTES.
- recommendation: Either drop the two assignments, or keep them and add a
  PORTING_NOTES entry noting the deliberate verbatim port.

### Guard self-target fallback silently ignores an invalid Guard guess
- severity: nit
- category: consistency
- location: game/love-letter-2/src/lib.rs:595
- finding: When all other players are protected/eliminated and the player targets
  themselves, the `target == player` early-return (lines 595-605) runs before the
  `card == Card::Guard` validation (line 607), so `guard mick guard` succeeds as a
  plain discard instead of returning "you can't use Guard against other Guards".
  This ordering is a verbatim port of Go `PlayGuard` (same check order in
  char_guard.go), but unlike the other preserved quirks it is not documented in
  PORTING_NOTES.
- recommendation: No behaviour change; add a one-line PORTING_NOTES entry (or code
  comment) recording the preserved ordering.

### `discard_card` records discards for cards the player does not hold
- severity: nit
- category: correctness
- location: game/love-letter-2/src/lib.rs:144
- finding: `discard_card()` removes the card "if present" but unconditionally
  pushes it to `discards[player]`, so a `play_*` call for a card not in hand still
  corrupts the public discard record. Reachability is guarded by the command parser
  (only offers cards actually in hand), so this is not input-reachable today; it is
  a fragile implicit contract for the `pub play_*` API. Matches Go `DiscardCard`.
- recommendation: Return an error (or no-op) when the card is absent, or make the
  play_* methods private to the crate and document the parser-gating contract.

### Test module named `test` instead of conventional `tests`
- severity: nit
- category: consistency
- location: game/love-letter-2/src/lib.rs:916
- finding: `mod test` — Rust convention (and the rest of the workspace, e.g. the
  integration test in tests/contract.rs) uses `mod tests`.
- recommendation: Rename to `mod tests`.

## Boilerplate binaries (systemic — captured once for all ~27 game crates)

The 4 binaries in this crate are thin one-liner wrappers with no per-crate logic:
`love_letter_2_cli.rs` (13 lines), `love_letter_2_repl.rs` (7 lines),
`love_letter_2_fuzz.rs` (5 lines), `love_letter_2_http.rs` (13 lines). Verified
against `game/acquire-1/Cargo.toml` that the Cargo.toml pattern below is identical
across crates; assume the binary bodies are too (other reviewers check deviations).

### Binary-only dependencies declared as library `[dependencies]`
- severity: minor
- category: dependencies
- location: game/love-letter-2/Cargo.toml:9
- finding: `brdgme_cmd`, `brdgme_fuzz`, and `tokio = { features = ["full"] }`
  (Cargo.toml:9-16) are only used by the `src/bin/` binaries, yet are declared as
  library dependencies — so every consumer of the game library (e.g. an aggregator
  web binary linking all ~27 game crates) transitively builds the fuzz harness, the
  cmd/requester stack, and all of tokio's "full" feature set. Binaries in the same
  package can use `[dev-dependencies]`, so there is no need for these to be lib
  deps. `tokio "full"` is additionally over-broad for a single
  `#[tokio::main]` + one async call.
- recommendation: Move `brdgme_cmd`, `brdgme_fuzz`, `tokio` to `[dev-dependencies]`
  (keep the existing `brdgme_cmd` dev-dep with `test-support` merged), and reduce
  tokio features to `["rt-multi-thread", "macros"]`.

### HTTP binary defaults to privileged port 80
- severity: nit
- category: quality
- location: game/love-letter-2/src/bin/love_letter_2_http.rs:9
- finding: `env::var("ADDR").unwrap_or("0.0.0.0:80".to_string())` — binding port 80
  requires root / CAP_NET_BIND_SERVICE, so the binary fails out of the box when run
  unprivileged without `ADDR` set (e.g. local dev). The `.expect("Invalid socket
  address")` at line 11 is startup-config validation, acceptable per CODING.md's
  acceptable-panics carve-out.
- recommendation: Default to an unprivileged port (e.g. `0.0.0.0:8080`) or require
  `ADDR` explicitly.

## Modules noted clean

- **card.rs**: clean. Deck composition (5 Guard, 2 each Priest/Baron/Handmaid/
  Prince, 1 King/Countess/Princess = 16) matches the classic edition and Go;
  enum discriminants match Go int constants; colors match Go render constants.
  No Chancellor in this edition — correct.
- **render.rs**: clean. All public-state accesses use defensive `.get()`,
  table layout ported per PORTING_NOTES (Center alignment fix documented).
  `help_table` counts derived from `initial_deck()` so they cannot drift.
- **Serde views**: no information leaks found. `PubState` structurally omits deck,
  removed cards, and hands (verified by the `pub_state_does_not_leak_hidden_info`
  test, lib.rs:1158); `PlayerState.hand` is only the requesting player's own hand
  (lib.rs:694 uses `.get(player)` safely). Private logs (drawn card, King swap,
  Baron comparison, Priest peek) are correctly scoped to the entitled players;
  Baron reveals both cards to both players per the rules; Guard-correct and
  end-round hand reveals match Go.
- **Win/elimination logic**: Princess discard elimination, Handmaid protection
  reset at `start_turn`, `next_player` skip-eliminated loop, `end_round` trigger
  on `num_remaining <= 1` and deck-out, and points/placings all verified against
  Go with no divergence. `next_player` cannot infinite-loop because `end_round`
  (which resets elimination or finishes) always fires when <=1 player remains.
- **tests/contract.rs**: minimal `assert_gamer_contract::<Game>()` delegation,
  correct shape.
