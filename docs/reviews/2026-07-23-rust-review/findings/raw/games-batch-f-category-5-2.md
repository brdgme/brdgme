# Review: game/category-5-2 (port of brdgme-go/category_5_1)

Reviewed: src/lib.rs (823 lines incl. tests), src/command.rs, src/render.rs,
tests/contract.rs, Cargo.toml, RULES.md/DATA_DOCS.md skimmed; compared against
brdgme-go/category_5_1 (game.go, card.go, command.go, play_command.go,
choose_command.go, render.go, game_test.go). Binaries under src/bin/ skipped
(systemic boilerplate, tracked elsewhere).

Overall: the port is faithful and the core logic (deck 1..=104, bullhead values,
lowest-difference row placement, 6th-card pickup, too-low choose, ascending
simultaneous resolution, 10-card hands, auto-play of last card, 66+ end
condition, lowest-score placings with standard competition ties) is correct and
well tested. Hidden-information integrity is clean: `PubState` exposes only
board, taken-pile counts, and points — chosen-but-unrevealed `plays` and other
players' hands never leak. Findings below are all low severity.

### Player count capped at 8, diverging from Go (2-10), official 6 nimmt! (2-10), and the crate's own RULES.md
- severity: minor
- category: correctness
- location: game/category-5-2/src/lib.rs:21 (MAX_PLAYERS), lib.rs:481-483 (player_counts), RULES.md:3
- finding: The Rust crate caps the game at 8 players (`MIN_PLAYERS..=MAX_PLAYERS` = 2..=8, `player_counts()` returns 2..=8). The Go original allowed 2-10 (`game.go:35`, `PlayerCounts` returns 2..=10 at game.go:57-59), official 6 nimmt! supports 2-10, and the crate's own RULES.md line 3 still says "A 2-10 player card game", so the in-crate docs contradict the enforced limit. The deck math would still work for 10 players (10*10+4 = 104 exactly, zero cards left in deck after deal — draw_cards would reshuffle discards on later rounds, fine). If 8 is a deliberate platform-wide cap, the RULES.md text should be corrected; otherwise the limit is an unintentional rules divergence from the Go port source.
- recommendation: Either raise MAX_PLAYERS to 10 and add 9,10 to player_counts() to match Go/official rules, or (if 8 is deliberate) fix RULES.md to say 2-8 and note the deviation. Test at lib.rs:549-555 would need updating either way.

### draw_cards recurses without bound — stack overflow if asked for more cards than deck+discard hold
- severity: minor
- category: correctness
- location: game/category-5-2/src/lib.rs:270-280
- finding: `draw_cards` drains the deck, reshuffles the discard into the deck, then recursively calls itself for the remainder. If `n` ever exceeds `deck.len() + discard.len()`, the second call recurses forever (empty deck, empty discard) until stack overflow — a panic-class failure that would kill the HTTP request. Not reachable through normal play (104 cards are conserved; max draw per round is 4 + 10*players = 84 at 8 players) and the Go original has the identical unbounded recursion (game.go:213-224), so this is a preserved latent hazard rather than a regression. Still, it is a `pub fn` with no guard.
- recommendation: Add a debug_assert or return a graceful state when `deck.len() + discard.len() < n` before recursing (e.g. assert total >= n), or convert the recursion to a loop with an explicit invariant check.

### Panic-on-invariant `expect` calls in resolve/choose paths
- severity: nit
- category: correctness
- location: game/category-5-2/src/lib.rs:178 (`row is never empty`), lib.rs:309 (`choosing player has a played card`), lib.rs:235 (`auto-play should only play valid cards`)
- finding: Three `expect` panics guard internal invariants that hold through the public API but are not enforced by the type system: (1) `board[i].last().expect(...)` in resolve_plays assumes no row is ever empty (true: start_round seeds rows and resolution always replaces a taken row with the played card); (2) `plays[player].expect(...)` in choose assumes `resolving && choose_player == player` implies `plays[player].is_some()` — the link is only maintained by resolve_plays control flow; (3) the auto-play `expect` assumes play() of a hand card cannot fail. None are reachable from crafted player commands (can_play/can_choose gate them, and hands/row contents come from internal dealing). The Go original panics in exactly the same spots (game.go:136 `row[len(row)-1]`, game.go:179-183 `panic(err)`), so these are faithful ports of Go hazards. Given each game runs as an HTTP service, a corrupted/migrated game state (serde) could turn these into request-killing panics.
- recommendation: Acceptable as-is given the invariants; if hardening is desired, return `GameError::internal`-style errors instead of panicking, or encode non-empty rows / pending-play in the types.

### "N points until the end of the game" renders a negative number after the game ends
- severity: nit
- category: quality
- location: game/category-5-2/src/render.rs:115-120
- finding: The footer computes `END_SCORE - max_points` and renders it unconditionally, including for finished games where max_points >= 66, producing e.g. "**-4 points** until the end of the game." The Go original has the identical behaviour (render.go:83-86), so this is a preserved Go quirk, not a new bug.
- recommendation: Clamp at 0 or skip the footer when `pub_state.finished` is true.

### points() returns raw bullhead totals where lower is better
- severity: nit
- category: consistency
- location: game/category-5-2/src/lib.rs:477-479
- finding: `points()` exposes player_points (bullheads, lower-is-better) as-is. Placings correctly negate (lib.rs:337-342) and the command path passes negated scores to placings_log (lib.rs:440-443, 458-461), but any other framework consumer of `points()` that assumes higher-is-better (stats, ELO-style ranking) would rank this game backwards. The Go original does the same (game.go:69-75), so it is a preserved quirk; flagging only so the Lead can check how web/stats consumes points() across low-score games.
- recommendation: Verify the framework's points() contract; if higher-is-better is assumed anywhere, return negated values here (and audit other lowest-wins games).

### Binary-only deps declared as library dependencies (cross-reference)
- severity: nit
- category: dependencies
- location: game/category-5-2/Cargo.toml:10 (brdgme_fuzz), Cargo.toml:16 (tokio "full")
- finding: `brdgme_fuzz` and `tokio` are used only by src/bin/ targets (category_5_2_fuzz.rs, category_5_2_http.rs) yet are declared as library dependencies. This is the known systemic "binary-only deps declared as library deps" issue present across all 27 game crates — cross-referenced by name only, no new finding.
- recommendation: Track under the systemic issue (move to a bins-only mechanism once Cargo supports it, or per-bin manifest split).

### Card(pub u8) permits invalid cards via the public tuple field and serde
- severity: nit
- category: quality
- location: game/category-5-2/src/lib.rs:28-29
- finding: `Card` is a public tuple struct, so `Card(0)` or `Card(200)` can be constructed anywhere, and `#[derive(Deserialize)]` accepts them from crafted state. `heads()` then silently mis-scores them (e.g. `Card(0)`: 0 % 11 == 0 -> 5 heads; `Card(105..)` multiples of 5/10/11 also score). Not reachable via player commands (play only accepts cards parsed from the player's own hand via Enum::exact), so this is hardening only.
- recommendation: Make the field private with a `new(u8) -> Option<Card>`/validated constructor, or add a `#[serde(deserialize_with)]` range check (1..=104), if state-integrity hardening is in scope.

### Test comment typo: "11 is a multiple of 1 only"
- severity: nit
- category: quality
- location: game/category-5-2/src/lib.rs:592
- finding: The trailing comment in test_card_heads reads "11 is a multiple of 1 only; 5 wins." — should be "multiple of 11 only". Cosmetic.
- recommendation: Fix the comment.

### hands[0] used as proxy for all players' hand sizes in resolve_plays
- severity: nit
- category: quality
- location: game/category-5-2/src/lib.rs:228
- finding: After resolution, `match self.hands[0].len()` decides end_round (0) vs auto-play (1) for everyone, assuming all hands are always the same size. True by construction (simultaneous play), and identical to Go (game.go:173), but the invariant is implicit; a comment or a check over all hands would make it robust against future changes (e.g. variant rules with unequal hands).
- recommendation: Optional — add a short comment stating the uniform-hand-size invariant.

## Clean areas (verified, no findings)
- Deck composition (1..=104), bullhead values (55->7, %11->5, %10->3, %5->2, else 1): correct per official rules and Go; good test coverage (lib.rs:577-593).
- Row placement rule (highest row-end below the played card), 6th-card pickup (row replaced by played card), too-low-card choose flow, ascending simultaneous resolution, auto-play of final card: all correct and match Go; directly tested (lib.rs:619-730).
- Round/game end: points added at round end, new round started only when not finished, game ends only at round end with max >= 66 (lib.rs:327-335) — matches official rules and Go.
- Placings: lowest bullheads wins, standard competition ties via gen_placings on negated points; tested (lib.rs:768-782).
- Hidden information: PubState (lib.rs:99-113, 394-407) contains no plays and no hands; PlayerState exposes only the viewer's own hand (lib.rs:409-415); chosen cards do not leak before reveal.
- Command gating: play/choose validated against can_play/can_choose, row range validated (1..=4), card must be in hand; out-of-range player index is safely rejected via `plays.get(player)` in can_play (lib.rs:126-128). Parse errors become GameError, no unwrap on player input.
- Player-facing parser uses Enum::exact on the player's actual hand and Int::bounded for rows (command.rs:30-59) — invalid numbers never reach game logic.
- Tests: good coverage of rules branches (placement, full-row pickup, choose, validation errors, end-round scoring, finished gating, pub/player state shape) plus the shared contract test in tests/contract.rs. Considerably stronger than the Go original's three tests.
- render.rs uses safe `.get(...).copied().unwrap_or(0)` for score-table lookups (render.rs:78-95); row rendering bounded at ROW_MAX which matches the 5-card row invariant.
