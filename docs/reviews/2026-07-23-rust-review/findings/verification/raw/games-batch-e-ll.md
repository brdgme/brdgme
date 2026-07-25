# Verification: games-batch-e (love-letter-2 F1-F9, systemic F45-F46)

Verified against snapshot /home/beefsack/Development/brdgme-review-snapshot/rust (f8763a5)
and Go original brdgme-go/love_letter_1. All line numbers re-read from disk, not
trusted from the finding.

## F1 - command() match arms duplicate the wrap-up 8 times (major, simplicity)

Verdict: CONFIRMED. Severity major appropriate.

- `Gamer::command` match spans lib.rs:713-859. Counted exactly 8 `Ok(ParseOutput
  {...})` arms: Princess (714-731), Countess (732-749), King (750-767), Prince
  (768-785), Handmaid (786-803), Baron (804-821), Priest (822-839), Guard
  (840-857). Total ~146 lines; the "~140 lines" claim holds.
- Every arm is byte-identical apart from the `play_*` call:
  ```rust
  let mut logs = self.play_princess(player)?;
  if self.is_finished() {
      let scores: Vec<(usize, i32)> = (0..self.players)
          .map(|p| (p, self.player_points[p] as i32))
          .collect();
      logs.push(placings_log(&self.placings(), Some(&scores)));
  }
  Ok(CommandResponse { logs, can_undo: false, remaining_input: remaining.to_string() })
  ```
- Go's Command (command.go) has the same 8-way dispatch but each Go `*Command`
  helper is 6 lines with no finish/scoring logic - the Rust port amplified the
  duplication. Any change to the finish wrap-up must be made 8 times.
- Severity: major under "significant maintainability problem" is right.

## F2 - end_score uses unreachable!() in a runtime path (minor, correctness)

Verdict: CONFIRMED. Severity minor appropriate.

- lib.rs:24-31: `fn end_score(players: usize)` ends `_ => unreachable!()`.
- Call sites verified: `check_finished()` lib.rs:273
  (`... >= end_score(self.players)`); `pub_state()` lib.rs:685
  (`end_score: end_score(self.players)`); `status()` lib.rs:663 calls
  `check_finished()`.
- `Game` derives `Default` (lib.rs:33) so `players = 0`; `Game` also derives
  `Deserialize` with no validation, so a state blob with `players` outside
  2..=4 panics these paths. Player count is only validated in `start()` at
  lib.rs:645 (`if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)`), matching
  the finding's claim.
- CODING.md: exists at `docs/CODING.md` (repo root docs/, not rust/ root).
  Lines 46-49: "**No panicking code in runtime paths.** `.unwrap()`,
  `.expect()`, `panic!()`, and `unreachable!()` are forbidden in server request
  handlers...". Note the rule's literal scope is server handlers / DB / Leptos
  code; game crates are executed inside the game-service request handler
  (`lib/cmd/src/requester/gamer.rs`), so applying it here is a reasonable
  extension, not a literal quote.
- Not reachable through normal play; minor is right.

## F3 - end_round indexes hands[p][0] without emptiness check (minor, correctness)

Verdict: CONFIRMED. Severity minor appropriate.

- lib.rs:184: `let c = self.hands[p][0];` inside the `for p in 0..self.players`
  loop, guarded only by `if self.eliminated[p] { continue; }` (lib.rs:181).
- Invariant holds today: `eliminate()` (lib.rs:165-167) drains the hand of any
  eliminated player, and every non-eliminated player holds >=1 card at round
  end (draw-before-play cycle). Go end-of-round code has the identical
  `g.Hands[p][0]` shape.
- Latent unchecked index in a request-handler path; minor is right.

## F4 - assert_target / play methods index with unvalidated target (minor, correctness)

Verdict: CONFIRMED. Severity minor appropriate.

- lib.rs:305 `if self.eliminated[target]` and lib.rs:311 the preserved-quirk
  duplicate `if self.eliminated[target]` - no `target < self.players` check
  anywhere in `assert_target` (lib.rs:288-318).
- Downstream unchecked indexes verified: play_king lib.rs:405/407
  (`render::card(self.hands[player][0])` / `self.hands[target][0]`) and the
  swap at 421; play_prince lib.rs:444 (`let target_card =
  self.hands[target][0];`); play_baron lib.rs:500-501.
- `target` originates from the `Player {}` parser over the caller-supplied
  `players: &[String]` slice (command.rs king/prince/baron/priest/guard
  parsers); nothing clamps it to `self.players`. A names slice longer than the
  game's player count makes `eliminated[target]` panic first (Vec of len
  `players`).
- Go parity confirmed: `AssertTarget` (game.go) has no bounds check either.
- Reachability requires a caller-side mismatch, so minor (defence in depth) is
  right rather than critical.

## F5 - commands accepted after the game is finished (minor, correctness)

Verdict: CONFIRMED. Severity minor appropriate.

- command.rs:23-25: `command_parser` gate is only
  `if self.current_player != player { return None; }`; no
  `check_finished()` guard. `command()` (lib.rs:704) has none either.
- Reachability traced: in `end_round` (lib.rs:217-230), when
  `check_finished()` is true the round is NOT restarted and `current_player`
  is left pointing at the player who just played; in most finishes (e.g. Guard
  elimination ending the round) that player still holds 1 card, so
  `command_parser` returns `Some` and further `play_*` calls execute -
  including re-running `end_round` and awarding additional points
  (lib.rs:205 `self.player_points[highest_player] += 1`).
- Go parity confirmed: `CanPlay` (game.go:303-305) is
  `g.CurrentPlayer == player` only; `CommandParser` (command.go) checks
  nothing else.
- Minor is right.

## F6 - redundant no-op hand assignments in play_baron (nit, quality)

Verdict: CONFIRMED. Severity nit appropriate.

- lib.rs:529 `self.hands[player] = vec![player_card];` and lib.rs:532
  `self.hands[target] = vec![target_card];`.
- No-op reasoning verified: `discard_card(player, Card::Baron)` at lib.rs:479
  removes the Baron, leaving exactly `[player_card]` (player drew to 2 cards);
  target holds exactly `[target_card]`. `player_card`/`target_card` are read
  post-discard at lib.rs:500-501, so the assignments rewrite the vectors with
  their current contents.
- Go parity confirmed: char_baron.go `g.Hands[player] = []int{playerCard}` /
  `g.Hands[target] = []int{targetCard}` - verbatim port.
- PORTING_NOTES.md lists 3 preserved quirks (AssertTarget double check, Priest
  doc typo, EndRound tiebreak) - the Baron dead assignments are indeed not
  documented.

## F7 - Guard self-target fallback bypasses the no-Guard-guess check (nit, consistency)

Verdict: CONFIRMED. Severity nit appropriate.

- lib.rs:595-605: `if target == player { ... return Ok(logs); }` runs before
  lib.rs:607-611 `if card == Card::Guard { return Err(...) }`, so a forced
  self-target with a Guard guess succeeds as a plain discard.
- Go parity confirmed: char_guard.go `PlayGuard` has the identical ordering
  (`if target == player { ... return }` before `if card == Guard`).
- Not in PORTING_NOTES.md (verified: only 3 quirks documented). Nit is right.

## F8 - discard_card records discards for cards not held (nit, correctness)

Verdict: CONFIRMED. Severity nit appropriate.

- lib.rs:144-154: removal is conditional
  (`if let Some(pos) = self.hands[player].iter().position(...)`), but
  `self.discards[player].push(card);` at lib.rs:148 is unconditional.
- Go parity confirmed: game.go:134-141 `DiscardCard` -
  `IntRemove(card, g.Hands[player], 1)` (removes up to 1 if present) then
  unconditional `append(g.Discards[player], card)`.
- The `play_*` methods are `pub`, so the implicit only-call-with-held-cards
  contract is real. Nit is right.

## F9 - test module named `test` instead of `tests` (nit, consistency)

Verdict: ADJUSTED. Severity nit appropriate; convention claim corrected.

- lib.rs:916 `mod test {` - confirmed.
- The claim "the rest of the workspace use `mod tests`" is wrong for the game
  crates: 17 of the game crates' lib.rs use `mod test {` (battleship-2,
  greed-2, category-5-2, lost-cities-1, cathedral-2, farkle-2, modern-art-2,
  no-thanks-2, age-of-war-2, liars-dice-2, ...) vs 10 using `mod tests`.
  `rust/lib/` however is uniform: 18 hits for `mod tests`, 0 for `mod test`.
- So love-letter-2 follows the *majority* game-crate style; the inconsistency
  is a workspace-wide split, better captured as a systemic nit than a
  love-letter-2 finding. Substance (Rust convention is `tests`; lib/ agrees)
  stands.

## F45 - binary-only deps declared as library [dependencies] (minor, dependencies)

Verdict: ADJUSTED. Severity minor defensible; the proposed mechanism is wrong
and impact is overstated.

- Facts confirmed (game/love-letter-2/Cargo.toml): lines 9-16 declare
  `brdgme_cmd = { path = "../../lib/cmd" }`, `brdgme_fuzz = { path =
  "../../tools/fuzz" }`, `tokio = { version = "1.52.3", features = ["full"] }`
  under `[dependencies]`; lines 18-19 already have a `[dev-dependencies]`
  `brdgme_cmd ... features = ["test-support"]`. The four `src/bin/` files are
  the only users of cmd/fuzz/tokio (`love_letter_2_cli/fuzz/http/repl.rs`).
  `tokio "full"` for one `#[tokio::main]` is indeed over-broad.
- Boilerplate claim spot-checked: age-of-war-2 and lost-cities-2 Cargo.toml
  have the identical lines (brdgme_cmd:9, brdgme_fuzz:10, tokio full:16,
  dev-dep brdgme_cmd test-support:19). "Identical boilerplate" holds.
- INCORRECT claim: "Binaries in the same package can use
  [dev-dependencies]". Cargo dev-dependencies apply only to tests, examples,
  and benches - `src/bin/` targets build against `[dependencies]`. Moving
  these deps to dev-dependencies would break every game binary. The correct
  fixes are: optional deps + `required-features` on the `[[bin]]` targets, or
  splitting the binaries into a separate crate.
- Impact overstated: a workspace grep shows no in-repo crate depends on any
  game crate as a library (only the workspace-member listing in
  rust/Cargo.toml references them). "Every consumer of a game library
  transitively builds..." is currently vacuous; the cost is the game crate's
  own build graph (which builds the bins anyway). tokio-feature trimming is
  the only concretely realizable win today.
- Severity: minor still defensible as dependency hygiene, but on the
  minor/nit boundary given zero current lib consumers.

## F46 - HTTP binary defaults to privileged port 80 (nit, quality)

Verdict: CONFIRMED. Severity nit appropriate.

- game/love-letter-2/src/bin/love_letter_2_http.rs:8-9:
  `env::var("ADDR").unwrap_or("0.0.0.0:80".to_string())` - line 9 holds the
  literal, as cited.
- Identical in age-of-war-2 (`age_of_war_2_http.rs:9`) and lost-cities-2
  (`lost_cities_2_http.rs:9`).
- Binding :80 unprivileged fails outside containers; in the k8s deployment the
  container context makes it workable, so local-dev-only annoyance. Nit is
  right.

## Summary

| ID | Verdict | Severity |
|----|---------|----------|
| F1 | CONFIRMED | major (keep) |
| F2 | CONFIRMED | minor (keep) |
| F3 | CONFIRMED | minor (keep) |
| F4 | CONFIRMED | minor (keep) |
| F5 | CONFIRMED | minor (keep) |
| F6 | CONFIRMED | nit (keep) |
| F7 | CONFIRMED | nit (keep) |
| F8 | CONFIRMED | nit (keep) |
| F9 | ADJUSTED - workspace-convention claim wrong (17 game crates use `mod test`) | nit (keep) |
| F45 | ADJUSTED - dev-dependencies fix is invalid for bins; no in-repo lib consumers | minor (borderline nit) |
| F46 | CONFIRMED | nit (keep) |

Notes:
- CODING.md is at `docs/CODING.md` (not rust/ root or repo root as the charter
  suggested); its no-panic rule (lines 46-49) literally scopes to server
  handlers / DB / Leptos code, so citing it against game crates is an
  extension by analogy, though a fair one (game code runs inside the
  game-service request handler).
