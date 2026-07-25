# liars-dice-2 review findings

Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/liars-dice-2/`
Reviewed: `src/lib.rs` (419 LOC), `src/command.rs` (67), `src/render.rs` (197), `tests/contract.rs`, `Cargo.toml`, `RULES.md`; Go original `brdgme-go/liars_dice_1/` (`liars_dice.go`, `bid_command.go`, `call_command.go`). Binaries under `src/bin/` skipped per instructions.

### Turn after challenge goes to player after the caller, not the challenge loser (Go cross-reference)
- severity: minor
- category: correctness
- location: game/liars-dice-2/src/lib.rs:208-211
- finding: After a `call`, the next round is started by `next_active_player(current_player)` where `current_player` is still the caller — i.e. the player clockwise after the caller starts, regardless of who lost the die. Official Liar's Dice/Perudo rules have the player who lost the challenge (and a die) start the next round. This exactly matches the Go original (`call_command.go:66-69`) and is explicitly documented in `RULES.md` ("The player who lost a die does not start the next round; the next active player (clockwise from the caller) starts"), so it is a preserved, documented Go quirk, not a fresh port bug. Cross-reference only.
- recommendation: No action unless the project decides to align with official rules; if so, set `current_player = losing_player` (skipping if eliminated) and update `RULES.md`.

### Index panics reachable from inconsistent deserialized state
- severity: minor
- category: correctness
- location: game/liars-dice-2/src/lib.rs:73-77 (`active_players`), 79-83 (`eliminated_player_list`), 100-107 (`roll_dice`), 192-195 (`call` die removal)
- finding: Several methods index `self.player_dice[p]` for `p in 0..self.players` without checking `player_dice.len() == players`, and `call` indexes `self.player_dice[losing_player]` / `self.player_dice[bid_player-related]` trusting `bid_player < players`. All fields are `pub` and the `Game` is serde-deserialized from stored state on each request, so a corrupted/crafted stored game (e.g. `players` larger than `player_dice.len()`, or `bid_player` out of range with `bid_quantity != 0`) panics the HTTP request. Not reachable through normal play — `start()` always builds consistent state — so this is a defense-in-depth gap, likely systemic across game crates.
- recommendation: Consider a `validate()` on load, or use `.get(p)` defensively; at minimum be aware panic-on-bad-state is the current contract. Low priority if state integrity is trusted.

### "fourty" typo preserved from Go (cross-reference)
- severity: nit
- category: consistency
- location: game/liars-dice-2/src/render.rs:104
- finding: `number_str` renders 40+ quantities as "fourty ..." (should be "forty"). This faithfully ports the identical typo in `brdgme-go/brdgme/strings.go:135`. Only visible when a bid quantity reaches 40+, which requires ~8 players' worth of dice — impossible in this 2-6 player game (max 30 dice, and a bid that high is instantly called), so practically unreachable in real games.
- recommendation: Fix the spelling in Rust; no need to preserve the Go typo. Cross-reference, not a fresh finding.

### Bid quantity has no upper bound in the parser
- severity: nit
- category: correctness
- location: game/liars-dice-2/src/command.rs:44-48
- finding: `Int { min: Some(MIN_BID_QUANTITY), max: None }` accepts quantities up to i32::MAX. This is legal per the rules (any higher quantity is a valid raise; the bidder just loses when called), and there is no allocation or arithmetic on the value that could panic or overflow (`number_str` falls back to digits outside 0..1000, render.rs:81-83). Harmless, but `bid 2000000000 6` produces a silly log line and suggest/help text shows no sensible cap. A `max` of total dice in play would be friendlier but is not required by the rules.
- recommendation: Optional: cap at `players * START_DICE_COUNT` for UX, or leave as-is (rules-permitted).

### Binary-only deps declared as library deps (systemic cross-reference)
- severity: nit
- category: dependencies
- location: game/liars-dice-2/Cargo.toml:9-16
- finding: `brdgme_cmd`, `brdgme_fuzz`, and `tokio` (full) are `[dependencies]` of the library though only the `src/bin/` targets and fuzz/CLI tooling need them. Known systemic issue across game crates ("binary-only deps declared as library deps") — cross-reference by name only.
- recommendation: Tracked systemically; no crate-specific action.

### Test gaps: hidden-info redaction, wild-1 bid value, full game
- severity: nit
- category: quality
- location: game/liars-dice-2/src/lib.rs:362-419, game/liars-dice-2/tests/contract.rs
- finding: Tests cover start, a full example round with bid-validation errors, elimination listing, `number_str`, and the shared gamer contract. Missing: (a) a test asserting `pub_state`/`player_state` never expose other players' dice (the key hidden-information property of this game); (b) a call-resolution test where the bid value is 1 (only 1s count, no double-counting — the `*d as i32 == bid_value || *d == 1` condition at lib.rs:168 handles this correctly but it is untested); (c) a play-to-completion test asserting final placings/winner.
- recommendation: Add small unit tests for the three cases above.

## Clean aspects (verified)

- **Hidden-information integrity is correct.** `pub_state` (lib.rs:267-278) exposes only dice *counts*, never values; `player_state` (lib.rs:280-291) clones only the requesting player's own dice; the full reveal (`reveal_table`, render.rs:164-184) is only emitted in the public log at call time, which is exactly when the rules reveal dice. Logs are all `Log::public` and contain no hidden data.
- **No panic/unwrap/expect reachable from crafted command input.** Bid quantity/face are range-checked (lib.rs:119-139); `call` die removal is guarded by `is_empty()` (lib.rs:193); the `unreachable!()` arms in `number_str`/`ones_str` (render.rs:110, 134, 151) are genuinely unreachable given the enclosing range guards; out-of-range `player` indices hit `None` from `command_parser` and error out ("not your turn") rather than panicking.
- **Challenge resolution matches the rules:** counts `face == bid_value || face == 1` across all players' dice (lib.rs:165-172); bidder loses when count < quantity, caller loses otherwise; wild 1s; bid value 1 counts only 1s (no double count). Die loss and elimination (empty dice vec) correct; game ends when fewer than 2 players have dice (lib.rs:252-258).
- **RNG handling improved over Go:** seeded `GameRng` stored in state instead of Go's `time.Now()` re-seed per roll; `call` correctly returns `can_undo: false` (prevents undo/re-roll manipulation) while `bid` is undoable, matching Go semantics.
- **Faithful port with two Go bugs fixed:** Go's `PlayerState` had an inverted bounds condition (`player > 0 && len(g.PlayerDice) < player`, liars_dice.go:80) that always returned empty dice; Rust's `player < self.player_dice.len()` (lib.rs:281) is correct. Go's unguarded `g.PlayerDice[losingPlayer][1:]` would panic on an empty slice; Rust guards it (lib.rs:193-195). Rust also resets `bid_value`/`bid_player` in `start_round` (lib.rs:93-98) where Go left stale values — no visible difference since render keys off `bid_quantity == 0`.
- **Log/render content is consistent:** reveal table is rendered before the die is removed (lib.rs:192-195), so the reveal shows the dice as they were at call time — matches Go ordering.
