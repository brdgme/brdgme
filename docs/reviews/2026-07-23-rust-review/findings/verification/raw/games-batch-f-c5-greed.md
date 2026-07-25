# W3 verification: games-batch-f sections category-5-2 (F24-F31) and greed-2 (F32-F37)

### F24 Player count capped at 8 vs Go/official 2-10 and crate's own RULES.md — CONFIRMED
- evidence: lib.rs:21 `const MAX_PLAYERS: usize = 8;`, lib.rs:482 `vec![2, 3, 4, 5, 6, 7, 8]`; Go category_5_1/game.go:58 `return []int{2, 3, 4, 5, 6, 7, 8, 9, 10}`; RULES.md:3 "A 2-10 player card game". Deck math checks out: 10*10 hands + 4 row starters = 104 = DECK_SIZE. Test at lib.rs:549-555 pins 2..=8 (`Game::start(9, 1).is_err()`).
- severity: agree (minor, correctness) - enforced limit contradicts shipped docs and the Go original; not a crash.
- recommendation-check: valid - either branch (raise to 10 or fix RULES.md) is safe; deck supports 10 exactly, and the test update is correctly called out.

### F25 draw_cards recurses without bound — CONFIRMED
- evidence: lib.rs:270-280. Trace for n > deck+discard: first call drains deck, moves discard into deck (discard now empty), recurses with remaining > 0. Next call again hits the else branch, drains whatever deck holds, sets `self.deck = shuffle(std::mem::take(&mut self.discard), ...)` = empty (discard already empty), recurses with remaining unchanged and both piles empty - zero progress per frame, infinite recursion until stack overflow. n == deck+discard terminates fine (final recursion hits `deck.len() >= n`). Go game.go:213-224 has the identical recursion. Unreachable in normal play (max draw at 8 players is 4+80=84 of 104 conserved cards) but `draw_cards` is `pub`.
- severity: agree (minor, correctness) - latent panic-class hazard in a pub fn, not reachable from commands.
- recommendation-check: valid - a `deck.len() + discard.len() >= n` guard (error or truncate) or a loop with invariant check fixes it without behavior change on valid inputs.

### F26 expect calls in resolve/choose paths — CONFIRMED
- evidence: lib.rs:178 `.expect("row is never empty")`, lib.rs:235 `.expect("auto-play should only play valid cards")`, lib.rs:309 `.expect("choosing player has a played card")`. All three guard invariants maintained by the game logic (rows are re-seeded with one card on take/choose; auto-play plays a card actually in hand; choose_player is only set for a player with a pending play at lib.rs:188). Not reachable from crafted commands; only corrupt deserialized state (Game derives Deserialize, lib.rs:81) could trip them.
- severity: agree (nit, correctness) - matches the batch's convention for panic-on-invariant findings.
- recommendation-check: valid - "acceptable as-is" is the right call.

### F27 "N points until end of game" renders negative after game ends — CONFIRMED
- evidence: render.rs:115-120: `let max_points = pub_state.player_points.iter().copied().max().unwrap_or(0);` then `format!("{} points", END_SCORE - max_points)` unconditionally; `pub_state.finished` exists (lib.rs:110) but is not consulted. A finished game with max 70 renders "**-4 points** until the end of the game."
- severity: agree (nit, quality) - cosmetic display quirk on finished games only.
- recommendation-check: valid - clamping at 0 or skipping when `pub_state.finished` are both trivially safe (the field is already in PubState).

### F28 points() returns raw lower-is-better bullhead totals — ADJUSTED
- evidence: lib.rs:477-479 returns `player_points` as-is; placings correctly negate at lib.rs:337-342 (`vec![-self.player_points[p]]` into gen_placings). Framework contract checked: the Gamer trait default (lib/game/src/game.rs:113) returns `vec![]` with no documented direction; ELO/ratings use `place` from game_players, not points (web/src/db.rs:1536-1548 selects `place`, never points), so ratings are NOT ranked backwards. points() feeds only display (web/src/components/game.rs:247 "Points: {player.points}") - where raw bullheads are arguably correct - and the bot prompt (bot/src/main.rs:570 `score: bot_ctx.game_data.points...`), where a bot could plausibly misread lower-is-better bullheads labeled "score".
- severity: agree (nit, consistency) - the flagged framework risk (stats/ratings backwards) does not materialize; residual concern is only the bot-prompt "score" label.
- recommendation-check: valid as written ("verify the framework's points() contract") - verification done here: no negation needed for ratings; negating would make the displayed "Points" wrong, so the status quo is correct. Any consolidated fix should NOT negate points().

### F29 Card(pub u8) permits invalid cards — CONFIRMED
- evidence: lib.rs:28-29 `pub struct Card(pub u8)` with derive(Serialize, Deserialize) and no range check; heads() (lib.rs:32-45) happily scores Card(0) as 3 bulls (0 % 11 == 0 - actually 0 % 11 == 0 gives 5) and Card(200) as some value. Player commands cannot produce one (command parser is Enum::exact over the hand per the crate's command module; play() checks hand membership at lib.rs:287-290).
- severity: agree (nit, quality) - state-integrity hardening only.
- recommendation-check: valid - private field + constructor or serde validation; correctly scoped as optional.

### F30 test comment typo — CONFIRMED
- evidence: lib.rs:591-592 comment: "11 is a multiple of 1 only; 5 wins." Should be "multiple of 11 only".
- severity: agree (nit, quality).
- recommendation-check: valid.

### F31 hands[0] proxy for all hand sizes — CONFIRMED
- evidence: lib.rs:228 `match self.hands[0].len()` selects end_round (0) / auto-play-all (1) for every player. Uniform hand sizes hold by construction: start_round deals HAND_SIZE to all (lib.rs:144-148); resolve only runs after all players have played (play() at lib.rs:293-298 returns early unless every plays[p] is Some), so each resolution removes exactly one card from each hand.
- severity: agree (nit, quality) - implicit but true invariant, identical to Go.
- recommendation-check: valid - a comment is proportionate.

### F32 Game::score ignores player arg, no can_score check — CONFIRMED
- evidence: greed-2/src/lib.rs:294-295 `pub fn score(&mut self, player: usize, dice: &[Die])` then `let _ = player;`; no can_score/turn check, and the success log uses `N::Player(self.current_player)` (lib.rs:311). Contrast player_roll (lib.rs:320-323) and done (lib.rs:346-349), which gate on can_roll/can_done. Unreachable from commands: command_parser (command.rs:19-23) only includes score_parser when `can_score(player)`, and a non-current player gets no parsers at all. done() calls score(player) internally (lib.rs:355) after can_done already guaranteed player == current_player.
- severity: agree (minor, quality) - latent footgun for direct callers, no current bug.
- recommendation-check: valid - adding `player == self.current_player` (or full can_score) inside score is safe for the done() auto-take loop: can_done guarantees player == current_player and the while-let find guarantees available scores exist, and finished() stays false during the loop because scores are banked only after it. The alternate fix (drop the param, private helper) is also sound.

### F33 E/e score-token collision, `score eee` consumes E1 triple — CONFIRMED
- evidence: lib.rs:53-54 `Die::E1 => "E"`, `Die::E2 => "e"`; command.rs:56-63 builds Token from the concatenated names ("EEE" vs "eee"); Token::parse is case-insensitive via UniCase (lib/game/src/command/parser/mod.rs:5, :51); OneOf::parse returns on first Ok (parser/mod.rs:465-477 `Ok(output) => return Ok(output)` in iteration order); SCORES lists the E1 triple (lib.rs:124-126) before the E2 triple (lib.rs:127-130), and available_scores preserves that order (lib.rs:174-180). Pinned by test_score_case_insensitive_e1_e2_collision (lib.rs:729-746), which asserts E2 dice remain after `score eee`. Not a soft-lock: with the E1 triple gone, the next `score eee` can only match the E2 triple.
- severity: agree (minor, correctness) - the typed case is silently overridden in which physical dice are consumed (score value identical at 300 each, but remaining-dice composition differs); Go parity, test-pinned.
- recommendation-check: valid - "none required; if fixed, disambiguate die names not the parser" is the right direction (parser case-insensitivity is a global feature other games rely on).

### F34 scores/dice length invariants unchecked on deserialized state — CONFIRMED
- evidence: Game derives Deserialize (lib.rs:182) with no validation. Panic points on a short `scores` vec: lib.rs:363/:366 `self.scores[player]` in done(), lib.rs:375 `self.scores[p]` in placings(), render.rs:80 `self.scores[p]` for p in 0..players. lib.rs:522 (points()) iterates rather than indexes, so it cannot panic - it just returns the wrong length; minor citation imprecision, substance holds. `current_player >= players` similarly breaks the modular advance assumptions.
- severity: agree (minor, correctness) - trusted-store reachability only; consistent with sibling findings across ported crates.
- recommendation-check: valid - "no crate-specific action; systemic hook if added" is proportionate.

### F35 duplicated placings-log block in Roll and Done arms — CONFIRMED
- evidence: lib.rs:477-485 (Roll arm) and lib.rs:498-506 (Done arm) contain the identical `if self.is_finished() { let scores: Vec<(usize, i32)> = ... logs.push(placings_log(...)) }` block (cited ranges :476-491/:496-511 are a few lines generous but land on the right code).
- severity: agree (nit, simplicity).
- recommendation-check: valid - a helper or post-match append; note Score's arm intentionally lacks the block (scoring can't finish the game; only roll/done advance current_player, which finished() requires).

### F36 theoretical i32 overflow in turn/banked score — CONFIRMED
- evidence: lib.rs:307 `self.turn_score += value;` and lib.rs:366 `self.scores[player] += self.turn_score;` - plain i32 adds. Unbounded only via absurd hot-dice streaks (~430k consecutive 5000-value rerolls to reach i32::MAX); wraps in release, panics in debug. Go used int (64-bit in practice), so the port technically narrows the range, but unreachable either way.
- severity: agree (nit, correctness) - purely theoretical.
- recommendation-check: valid - "none practical; saturating_add if desired" is right.

### F37 Die::E1 rendered Foreground though RULES.md says black — CONFIRMED
- evidence: lib.rs:64 `Die::E1 => color::NamedColor::Foreground`; RULES.md:18 `| \`E\` | E | black |`; Go greed_1/greed.go:62 `DieE1: render.Black`. Code and shipped rules doc disagree; Foreground is a sensible dark-theme adaptation.
- severity: agree (nit, consistency).
- recommendation-check: valid - updating RULES.md's colour column is the minimal correct fix.
