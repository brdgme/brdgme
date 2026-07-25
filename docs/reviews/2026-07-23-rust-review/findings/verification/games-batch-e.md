# Verification: games-batch-e (unit 7)

Independent verification of `findings/games-batch-e.md` (originally reviewed
by Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Crates: `game/love-letter-2`, `game/age-of-war-2`, `game/lost-cities-2`,
`game/red7-1`, `game/lost-cities-1`, plus the batch's shared boilerplate
section. Raw verdict dumps: `raw/games-batch-e-ll.md`,
`raw/games-batch-e-aow-red7.md`, `raw/games-batch-e-lc2.md`,
`raw/games-batch-e-lc1.md`. Process log: `games-batch-e-LOG.md`.

Go sources: love_letter_1 and age_of_war_1 exist in the snapshot's
brdgme-go and were used for every port-parity claim; lost-cities-1/-2 and
red7-1 have no Go source, so their official-rules claims were checked
against in-crate docs where possible and their evidence basis flagged as
external otherwise.

## Per-finding verdicts

### love-letter-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | command() 8x duplicated finish/response wrap-up | major | CONFIRMED | Eight `Ok(ParseOutput)` arms (lib.rs:713-857) each repeat the identical is_finished/scores/placings_log/CommandResponse block; only the `play_*` call differs |
| F2 | end_score `unreachable!()` in runtime path | minor | CONFIRMED | `_ => unreachable!()` reachable via pub_state()/status() on a default/corrupt Game (players=0); normal play guarded at lib.rs:645. Note: CODING.md's no-panic rule (docs/CODING.md:46-49) literally scopes to server handlers/DB/Leptos — applies to game crates only by analogy via the request path |
| F3 | end_round `hands[p][0]` unchecked | minor | CONFIRMED | Latent unchecked index in a runtime handler; invariant holds on current paths (Go-parity verified) |
| F4 | assert_target/play_* unvalidated target index | minor | CONFIRMED | No `target < self.players` check at lib.rs:305,311; play_king/prince/baron index hands[target][0]; oversized names slice panics; Go identical |
| F5 | Commands accepted after game finished | minor | CONFIRMED | command_parser() checks only current_player, no finished guard; post-game plays execute incl. re-running end_round; Go command.go same shape |
| F6 | play_baron no-op hand assignments | nit | CONFIRMED | Both assignments dead (discard_card already left `[player_card]`); verbatim Go char_baron.go port; NOT in PORTING_NOTES.md |
| F7 | Guard self-target skips Guard-guess validation | nit | CONFIRMED | `target == player` early-return precedes the `card == Card::Guard` check; verbatim Go char_guard.go ordering; NOT in PORTING_NOTES.md |
| F8 | discard_card records unheld cards | nit | CONFIRMED | Unconditional push to discards[player]; matches Go DiscardCard; parser-gated today |
| F9 | `mod test` vs conventional `mod tests` | nit | ADJUSTED (details; nit stands) | Premise wrong: 17 game crates use `mod test` vs 10 `mod tests` — love-letter-2 follows the game-crate majority; only lib/ is uniformly `mod tests` (Lead re-grepped). Recast as workspace-wide inconsistency, not a crate defect |

### age-of-war-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F10 | unwrap/expect cluster in runtime paths | minor | CONFIRMED | All six sites verified (scores position, check_end_of_turn owner, line_action + line_parser currently_attacking, render_castle/render_castles owners); each invariant holds in normal flow; same CODING.md scope note as F2 |
| F11 | `completed_lines: HashSet` nondeterministic serialization | minor | CONFIRMED | Game is the persisted serde blob; pub view sorts (lib.rs:434-435), confirming output-side-only fix |
| F12 | Not-your-turn as unstructured invalid_input | nit | CONFIRMED | GameError::NotYourTurn variant exists and is bypassed |
| F13 | Placings-log tail triplicated | nit | CONFIRMED | Identical block in Attack/Line/Roll arms (lib.rs:473-482, 491-500, 509-519) |
| F14 | Finished games emit duplicate placings logs | nit | CONFIRMED | can_roll quirk preserved; amplification genuinely new vs Go — age_of_war_1/game.go:50-64 Command never appends placings logs (checked directly) |
| F15 | clan_conquered duplicated in renderer | nit | CONFIRMED | Two copies incl. the stale-player quirk (lib.rs:93-113, render.rs:10-30) |
| F16 | "discard one dice" help text | nit | CONFIRMED | command.rs:117; carried from Go |

### lost-cities-2

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F17 | Finished stats hardcoded to players 0/1 | major | CONFIRMED | `stats: vec![self.player_stats(0), self.player_stats(1)]` (lib.rs:534) with player_counts() = [2,3] (lib.rs:644) and generalized placings(); 3-player finished games silently omit player 2 (Lead re-read both lines) |
| F18 | player_state() unchecked index, request-reachable | major | CONFIRMED | `self.hands[player].clone()` (lib.rs:570); handle_player_render (lib/cmd requester/gamer.rs:170-182) passes the envelope's player straight through (Lead re-read). Kept major not critical: index comes from the server-side envelope, not player command text |
| F19 | Draw logs dropped when draw empties deck | minor | CONFIRMED | Local logs discarded on the `return self.end_round()` branch |
| F20 | PlayerState.hand documented sorted, unsorted | minor | CONFIRMED | Doc + DATA_DOCS.md:19 vs acquisition-order hand; only per-draw batch sorted |
| F21 | Stats.investments dead; expeditions write-only | minor | CONFIRMED | investments never incremented; expeditions incremented but never surfaced |
| F22 | `unreachable!()` outside players 2..=3 | minor | CONFIRMED | Game derives Deserialize without validation; state blob rides Play/Status requests; pub score() unguarded |
| F23 | Perspective `% MAX_PLAYERS` not `% self.players` | nit | CONFIRMED | render.rs:130; strengthened — the scores section in the same file clamps correctly (render.rs:51-54), making this the clear outlier |
| F24 | Game-over log regressed vs -1 (no winner line) | nit | CONFIRMED | -1 announced winner+margin; -2 bare "The game is over."; leaders() helper already computes what's needed |
| F25 | Discard piles expose top card only | nit | CONFIRMED (external basis) | Code side verified (top value per pile only); the face-up-inspectable premise is official-rulebook knowledge, no in-crate doc either way |
| F26 | Draw-count usize underflow | nit | CONFIRMED | `hand_size(self.players) - hand.len()`; latent |
| F27 | k8s blurb still says two-player | minor | ADJUSTED (details; minor stands) | Blurb confirmed verbatim vs player_counts()=[2,3] and RULES.md 3p variant; location is game-version.yaml line 9, not line 1 |
| F28 | Stale build-release / .rls.toml | nit | ADJUSTED (details; nit stands) | Files stale as claimed, but `.rls.toml` is NOT malformed — exactly `build_lib = true`, no trailing newline (Lead od-verified); "truetarget" was a cat-concatenation artifact with .gitignore |

### red7-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F29 | CardParser non-ASCII byte-slice panic | critical | CONFIRMED | Guard counts chars (command.rs:23-24), slices cut bytes (:31,34-35); `play r€` panics mid-`€` (Lead re-read the code). Reachability traced Request::Play -> handle_play -> Chain2(Token, AfterSpace(CardParser)), ASCII-safe wrappers deliver the non-ASCII tail; no catch_unwind in lib/; any current player can trigger |
| F30 | Zero-rule-fulfilling player treated as winning | major | CONFIRMED (external basis) | Concrete trace holds: all-empty winning sets tie at len 0 / (0,0) and the strict `>` (card.rs:311) keeps index 0 as leader (Lead re-read leader()); all three consequences (survives done, discard into empty rules, 0-point round win) confirmed in lib.rs; deviation undocumented in RULES.md/DATA_DOCS.md; "cannot win" premise is official-rulebook knowledge |
| F31 | DATA_DOCS.md tie-break description fictional | minor | CONFIRMED | "then highest card overall in the palette" implemented nowhere |
| F32 | RULES.md turn/scoring gaps | minor | CONFIRMED (detail softened) | Play-then-discard combo undocumented (code permits it, can_discard ignores has_played); scoring line wrong vs lib.rs:176-179. "Lists as alternatives" slightly generous — RULES.md is silent, not contradictory |
| F33 | PubCard/PubSuit aliased re-export unused | nit | CONFIRMED | Workspace grep: zero references |
| F34 | leader_with_suit `player_map[l_index]` panic | nit | CONFIRMED | leader() returns (0, vec![]) for empty input; all-eliminated case unreachable today |
| F35 | end_points underflow above 10 players | nit | CONFIRMED | pub fn, callers validated 2..=4 |

### lost-cities-1

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F36 | player_state() unchecked index, request-reachable | major | CONFIRMED | Same defect as F18; hands always exactly 2 entries; crafted PlayerRender with player >= 2 panics |
| F37 | draw_hand_full drops draw logs on deck-empty | major | ADJUSTED (major -> minor) | Defect real — lib.rs:434-438 returns `self.end_round()` and discards the accumulated logs (Lead re-read) — but it is byte-for-byte the defect the same review rated minor in lost-cities-2 (F19); log-only loss, state correct. Aligned both at minor |
| F38 | PlayerState.hand documented sorted, never sorted | minor | CONFIRMED | Stronger than -2: `drawn.sort()` (lib.rs:411) only sorts the private log; the hand gets no sorting at all |
| F39 | Stats.investments never written; expeditions write-only | minor | CONFIRMED | As claimed |
| F40 | expeditions increment condition mismatches name | minor | CONFIRMED | Fires only when the whole tableau is empty — counts rounds-with-a-play, not expeditions |
| F41 | HAND_SIZE - hand.len() underflow | nit | CONFIRMED (detail refined) | Debug panics; release wrap is immediately clamped by the `num > dl` check (lib.rs:403-405) so release drains the deck rather than panicking |
| F42 | Literal 2 vs PLAYERS const | nit | CONFIRMED | All four cited sites verified; undercounts — bare 2s also at lib.rs:511-512, 638, 642 |
| F43 | score() is_none()-guarded unwrap() | nit | CONFIRMED | As claimed |
| F44 | render.rs throwaway empty Vecs | nit | CONFIRMED | render.rs:185,196 |

### Shared / systemic (boilerplate binaries)

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F45 | Binary-only deps as lib `[dependencies]` | minor | ADJUSTED (recommendation invalid; minor stands) | Dep placement facts and boilerplate-identical claim verified (age-of-war-2, lost-cities-2 spot-checked; Lead re-read the Cargo.toml). But `[dev-dependencies]` do NOT apply to `src/bin/` targets — the proposed move would break every game binary. Correct fix: optional deps + `required-features` on `[[bin]]`, or a separate bin crate. Also no in-repo crate consumes a game crate as a library, so the transitive-build impact is currently vacuous; the tokio "full" feature trim is the realizable win |
| F46 | HTTP binary defaults to privileged port 80 | nit | CONFIRMED | `unwrap_or("0.0.0.0:80")`; fails unprivileged without ADDR |

## Summary

- Findings verified: 46
- CONFIRMED: 42, ADJUSTED: 4 (F9, F28, F37, F45), REJECTED: 0,
  UNVERIFIABLE: 0. All 46 findings survive.
- Corrected tallies for the unit: 1 critical / 5 major / 18 minor /
  22 nit. Original per-finding fields: 1 critical / 6 major / 17 minor /
  22 nit (the findings file carries no summary tally line; recounted from
  the blocks per unit-6 guidance).
- Severity changes: F37 major -> minor (log-only loss, aligned with the
  identical lost-cities-2 defect F19 that the same review rated minor).
- Lead spot-checked every ADJUSTED verdict directly against the snapshot
  (F9 mod-test grep counts; F28 od -c on .rls.toml; F37 lib.rs:398-439;
  F45 Cargo.toml + Cargo dev-dependency semantics) and, for the
  all-confirm Workers, re-verified the hardest confirmations: F29
  (command.rs:18-43), F30 (card.rs:297-317), F17 (lib.rs:534,644), F18
  (lib.rs:570 + requester/gamer.rs:170-182).

## Notable corrections

- F45 (shared boilerplate deps): the finding's facts hold but its
  recommended fix is wrong — Cargo `[dev-dependencies]` apply to
  tests/examples/benches, not `src/bin/` targets, so moving
  brdgme_cmd/brdgme_fuzz/tokio there would break all 4 binaries in every
  game crate. Any fix plan must use optional deps + `required-features`
  or a separate bin crate. The "every consumer transitively builds the
  fuzz harness" impact claim is also currently vacuous (no in-repo
  library consumers of game crates).
- F37 (lost-cities-1 draw logs) downgraded major -> minor: identical
  defect to lost-cities-2's F19, which the same review rated minor; the
  review was internally inconsistent, not wrong on the facts.
- F9 (mod test naming) premise inverted: `mod test` is the game-crate
  majority convention (17 vs 10); only lib/ is uniformly `mod tests`.
- F28 (.rls.toml "malformed") — the file is well-formed
  (`build_lib = true`, no trailing newline); the quoted "truetarget" was
  the original reviewer concatenating it with .gitignore. Stale-file
  substance stands.
- CODING.md scope note (affects F2/F10 framing, no verdict change): the
  no-panic rule at docs/CODING.md:46-49 literally scopes to server
  handlers/DB/Leptos; applying it to game crates is extension by analogy,
  defensible because game code runs inside the game-service request
  handler.

Evidence strengthenings recorded in the raw dumps: F14's placings-log
amplification confirmed new vs Go (game.go:50-64 never appends placings
logs); F23's `% MAX_PLAYERS` is the outlier against a correctly-clamping
sibling path in the same file (render.rs:51-54); F29's reachability traced
end-to-end through the parser chain with no catch_unwind; F38 is stronger
in -1 than stated (no sorting at all, even per-batch); F42 undercounts the
bare-2 sites.

Overall assessment: the strongest batch verified so far — zero rejections,
zero severity inflations on code facts, and accurate line numbers
throughout, including the headline critical (F29) and both request-
reachable panics (F18/F36), all reproduced end-to-end. The four
adjustments are a severity alignment, two detail corrections, and one
invalid fix recommendation (F45) that matters for the remediation plan
rather than the finding itself.
