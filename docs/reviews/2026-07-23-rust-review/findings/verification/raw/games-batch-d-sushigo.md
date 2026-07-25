# W3 verification: sushi-go-2 (F24-F33)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust/game/sushi-go-2 (commit f8763a5)
Go original: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go/sushi_go_1

## F24 pass direction alternates instead of always-left

- verdict: CONFIRMED
- evidence:
  - lib.rs:361-372: `} else if self.round % 2 == 1 { ... self.hands.rotate_left(1); } else { ... self.hands.rotate_right(1); }` - left in odd rounds, right in even rounds. Also lib.rs:293 `let pass_dir = if self.round == 2 { "right" } else { "left" };`.
  - Go parity: game.go:180-195 identical logic (`g.Round%2 == 1` -> pass left, else right) - inherited from Go port. Confirmed.
  - RULES.md:9: "3 rounds. Round 1 passes left, round 2 passes right, round 3 passes left." - documented, self-consistent. Confirmed.
- severity: upheld (minor/correctness). Deviates from official rules but consistently implemented and documented.
- evidence basis: official Gamewright Sushi Go rules from model knowledge - hands are always passed to the left (clockwise) every round; no alternating direction.

## F25 round-3 pudding: all-tied awards nothing; dummy participates

- verdict: CONFIRMED
- evidence:
  - lib.rs:492-497: `if first == last { output.push(... "no points awarded") }` - when everyone (including dummy) has equal puddings, the +6 is not awarded. In 2p there is no fewest penalty (lib.rs:513 `if self.players != 2`), so an all-tied table should still split +6 per official intent; code gives 0.
  - Dummy inclusion: lib.rs:460 `let mut pudding = vec![0i32; self.all_players];` and loop 472-487 iterates all slots 0..all_players (3 in 2p). The dummy can enter `first_players` and receive/dilute points at lib.rs:499 `let first_points = 6 / first_players.len() as i32;` and 510-512. Confirmed.
  - Go parity: game.go:288 `for p := 0; p < g.AllPlayers; p++` and game.go:306 `if first == last` - identical. Confirmed.
- severity: upheld (minor/correctness). Edge case; the dummy variant itself is non-official so exact official mapping is fuzzy, but 2p all-tied-gets-0 contradicts the "split points if tied for most" rule.
- evidence basis: official rules from model knowledge - ties for most puddings split the 6 (rounded down); in 2p no player loses points for fewest.

## F26 placings pudding tiebreaker "not in official rules"

- verdict: REJECTED (core claim wrong; residual doc nit stands)
- evidence:
  - Code claim accurate: lib.rs:273-278 `placings()` builds `vec![self.player_points[p], self.pudding_cards(p)]` per player and calls `gen_placings`. Test lib.rs:1461-1471 `test_placings_pudding_tiebreaker` locks it. Go parity: game.go:439-446 with comment "ties broken by number of pudding cards". RULES.md says only "Most points wins" (line 3) - silent on tiebreaker. All verified.
  - But the central claim "official rules have no tiebreaker" is false: the official Gamewright Sushi Go rulebook specifies that on a points tie, the player with the most pudding cards wins. The implementation matches the official rule; it is not a correctness defect.
- severity: corrected - not minor/correctness; at most nit/docs (RULES.md omits the official tiebreaker). Code behavior is correct.
- evidence basis: model knowledge of the Gamewright Sushi Go rulebook end-of-game section ("In case of a tie, the player with the most pudding cards wins"). Stated with high confidence; not checked against a physical rulebook.

## F27 test_hand_passing_left asserts nothing

- verdict: CONFIRMED
- evidence: lib.rs:1398-1416 - body sets hands/playing, then `g.command(MICK, "play 1", &n).unwrap_err();` (1410), comment "Actually all already have playing set, so we need to call end_hand directly" (1411), `g.end_hand();` (1412), then comments 1413-1415 "we can't easily check passing here. Instead test passing with 2-card hands." No assert of any game state - zero assertions (unwrap_err is the only check, and it verifies the error path, not passing). Real coverage: lib.rs:1418-1438 `test_passing_direction` asserts post-rotation hands.
- severity: upheld (minor/quality). Dead test that documents its own uselessness.

## F28 dead (2,9) draw-count entry and silent fallback

- verdict: CONFIRMED (with one precision note)
- evidence:
  - lib.rs:140-142: `&[(2, 9), (3, 9), (4, 8), (5, 7)]`. lib.rs:144-150: `draw_count` with `.unwrap_or(9)` silent fallback.
  - Sole production call site lib.rs:292 `draw_count(self.all_players)`; in 2p games `all_players` is 3 (lib.rs:744), so the (2,9) entry is unreachable in production. Note: the unit test lib.rs:1301 does call `draw_count(2)`, so "only called with self.all_players" is true of non-test code only.
  - Hand sizes verified: 3p=9, 4p=8, 5p=7; 2p deals as 3 players x 9. Go original deck.go:57-62 has the same map including `2: 9, // Usually 10, but we implement the variant.`
- severity: upheld (minor/quality).
- evidence basis: official rules from model knowledge - standard deal is 2p=10, 3p=9, 4p=8, 5p=7; the crate implements a dummy variant instead, matching the Go source's own comment.

## F29 pudding explanation claims "least -6" even in 2p

- verdict: CONFIRMED
- evidence: lib.rs:123 `Card::Pudding => "end: most 6, least -6",` - static explanation shown via render.rs hand_table (render.rs:125-133) to all players regardless of player count. lib.rs:513 `if self.players != 2 {` guards the -6 award, so in 2p the explanation overstates the penalty.
- severity: upheld (nit/correctness) - misleading UI text only.

## F30 second-place maki guard suppresses only the log line

- verdict: CONFIRMED
- evidence: lib.rs:440 `if first_players.len() == 1 && second > 0 && second_players.len() <= 3 {` - both the log (442-451) and the point award (452-454) are inside the guard. Excluded case requires `second_players.len() >= 4`, only reachable as a 4-way second-place tie in 5p (sole first + 4 seconds). There `second_points` would be `3 / 4 == 0` (i32 division, lib.rs:441), so skipping the award changes no scores; only the would-be "awarding 0 points" log line is suppressed. Score outcome identical - the finding's careful framing holds. Go parity: game.go:259 identical guard including award inside it.
- severity: upheld (nit/correctness) - cosmetic log omission, no score impact.

## F31 render_name duplicated; underflow when players == 0

- verdict: ADJUSTED
- evidence:
  - Duplication: lib.rs:251-260 (method `render_name(&self, player)` using `self.players`, `brdgme_color::NamedColor::Grey`) vs render.rs:39-48 (free fn `render_name(player, players)` using imported `NamedColor::Grey`). Same logic and output, but NOT byte-for-byte: different signatures (method vs free function), different receiver of the player count, different path qualification. "Duplicated logic" is accurate; "byte-for-byte" is not.
  - Underflow: both compute `player > self.players - 1` (lib.rs:252) / `player > players - 1` (render.rs:40) on usize - underflows (debug panic / release wrap) iff players == 0. Only reachable via `Game::default()`/`PubState::default()` or corrupted state; `start` enforces 2-5 (lib.rs:735).
- severity: upheld (nit/consistency). Corrected claim: logic duplicated (not textually identical); underflow expression confirmed but unreachable in valid games.

## F32 chopsticks guard reads playing[DUMMY] before players == 2 check

- verdict: CONFIRMED
- evidence: lib.rs:675-678:
  ```
  if player == self.controller
      && self.playing[DUMMY].is_none()
      && self.players == 2
      && self.hands[player].len() == 2
  ```
  `self.playing[DUMMY]` (index 2) is evaluated before the `self.players == 2` term. In 3-5p games this reads real player 2's playing slot (harmless - the later `players == 2` term makes the guard false, and controller stays 0, so no behavioral bug). Would index-panic only if `playing.len() < 3`, i.e. all_players < 3, which cannot occur for started games (all_players is 3..=5, lib.rs:744) - only a default/corrupt struct. Condition ordering issue as stated.
- severity: upheld (nit/quality).

## F33 duplicated finished-check block in Play and Dummy command arms

- verdict: CONFIRMED
- evidence: lib.rs:839-856 (Command::Play arm) and lib.rs:857-874 (Command::Dummy arm) - apart from the `self.play(...)` vs `self.dummy(...)` call, the ~12 lines (is_finished check, scores vec build, `placings_log` push, `CommandResponse { logs, can_undo: false, remaining_input }`) are repeated verbatim in both arms. Finding's 838-874 range is accurate.
- severity: upheld (nit/simplicity).

## Cross-cutting

- Every behavioral quirk verified (F24, F25, F26-code, F28, F30) is a faithful port of brdgme-go/sushi_go_1 - the Rust crate reproduces the Go logic nearly line-for-line, including its rule deviations and its "variant" 2p deal.
- RULES.md accurately documents the implemented (variant) behavior for passing (F24) and 2p pudding penalty (F27's sibling F29 text aside), but omits the placings tiebreaker (F26 residual).
- The only verdict overturned is F26: the pudding tiebreaker is in the official Gamewright rules, so the implementation is correct and the finding reduces to a RULES.md documentation nit.
