# W3 triage extraction: games-batch-d + games-batch-e

## games-batch-d: 45 surviving findings = 1c / 6M / 16m / 22n (F13 REJECTED, excluded; F4 minor->major, F15 major->minor, F26 minor->nit, F36 major->minor per verification)
## games-batch-e: 46 surviving findings = 1c / 5M / 18m / 22n (all survive; F37 major->minor per verification)

Both tallies match the expected totals; no discrepancies.

games-batch-d F1 | major | unimplemented!() arms in command dispatch panic if remaining parsers wired in | game/lords-of-vegas-1/src/lib.rs | M | lords-of-vegas-gaps
games-batch-d F2 | major | HashMap/HashSet iteration order makes seeded boss-tie rerolls nondeterministic | game/lords-of-vegas-1/src/board.rs | M | lords-of-vegas-gaps
games-batch-d F3 | minor | resolve_boss_ties silently rerolls dice, never populates its log output | game/lords-of-vegas-1/src/board.rs | M | lords-of-vegas-gaps
games-batch-d F4 | major | Renderer usize underflow; CASINO_TILES branch reachable in ordinary 5-6p play (no supply limits) | game/lords-of-vegas-1/src/render.rs, game/lords-of-vegas-1/src/lib.rs | M | lords-of-vegas-gaps
games-batch-d F5 | minor | Loc::parse_str (Deserialize path) accepts out-of-range lots; lot 0 underflows neighbours() | game/lords-of-vegas-1/src/board.rs | M | state-trust-panics
games-batch-d F6 | minor | lazy_static for TILES instead of std LazyLock/OnceLock | game/lords-of-vegas-1/src/tile.rs, game/lords-of-vegas-1/Cargo.toml | M | deps-modernize
games-batch-d F7 | nit | unreachable!() in starting-cash fold lacks invariant comment at the use site | game/lords-of-vegas-1/src/lib.rs | M | lords-of-vegas-gaps
games-batch-d F8 | nit | serde_json is a runtime dep but only used in tests | game/lords-of-vegas-1/Cargo.toml | M | deps-modernize
games-batch-d F9 | nit | Redundant FromIterator import (edition 2024 prelude) | game/lords-of-vegas-1/src/board.rs | M | lords-of-vegas-gaps
games-batch-d F10 | nit | Hardcoded literal 3 instead of BLOCK_WIDTH in renderer | game/lords-of-vegas-1/src/render.rs | M | lords-of-vegas-gaps
games-batch-d F11 | nit | Casino colour names in RULES.md don't match rendered colours | game/lords-of-vegas-1/RULES.md, game/lords-of-vegas-1/src/casino.rs | M | lords-of-vegas-gaps
games-batch-d F12 | nit | Player counts 2-6 deviate from official 2-4 (possibly deliberate) | game/lords-of-vegas-1/src/lib.rs | D | player-count-caps: confirm 2-6 intended, document in RULES.md
games-batch-d F14 | major | No bonus token awarded for 6/7-card sales; contradicts own UI and DATA_DOCS | game/jaipur-2/src/lib.rs | M | jaipur-rules
games-batch-d F15 | minor | Next-round starting player is not the round loser (official-rules premise uncorroborated) | game/jaipur-2/src/lib.rs | D | jaipur-rules: confirm "loser starts" rulebook quote, restore major if confirmed
games-batch-d F16 | minor | Camel token counted as bonus token for end-of-round tie-break | game/jaipur-2/src/lib.rs | D | jaipur-rules: tie-break component adjudication
games-batch-d F17 | minor | RULES.md is a one-line stub; rules() serves an empty page | game/jaipur-2/RULES.md | M | jaipur-rules
games-batch-d F18 | minor | Mixed-type `sell dia gold` silently coerced to N of first good | game/jaipur-2/src/command.rs | M | jaipur-fixes
games-batch-d F19 | nit | Dead `parsers.is_empty()` branch in command_parser | game/jaipur-2/src/command.rs | M | jaipur-fixes
games-batch-d F20 | nit | Silent unwrap_or(Good::Diamond) fallback masks parser regressions | game/jaipur-2/src/command.rs | M | jaipur-fixes
games-batch-d F21 | nit | Placings-log block duplicated between Take and Sell arms | game/jaipur-2/src/lib.rs | M | epilogue-dup
games-batch-d F22 | nit | "N rounds remaining" overstates remaining rounds in best-of-3 | game/jaipur-2/src/render.rs | M | jaipur-fixes
games-batch-d F23 | nit | Camel count hidden in renderer but exact in PubState — inconsistent info policy | game/jaipur-2/src/render.rs, game/jaipur-2/src/lib.rs | D | jaipur-rules: pick one camel-visibility policy
games-batch-d F24 | minor | Round 2 passes hands right; official Sushi Go passes left every round (documented deviation) | game/sushi-go-2/src/lib.rs | D | sushi-go-rules: port parity vs official rules
games-batch-d F25 | minor | All-tied pudding case awards nothing in 2p; dummy dilutes the +6 | game/sushi-go-2/src/lib.rs | D | sushi-go-rules: 2p split + dummy participation
games-batch-d F26 | nit | Pudding score tiebreak correct per official rules but undocumented in RULES.md | game/sushi-go-2/RULES.md | M | sushi-go-rules
games-batch-d F27 | minor | test_hand_passing_left is vacuous and self-contradicting | game/sushi-go-2/src/lib.rs | M | sushi-go-fixes
games-batch-d F28 | minor | draw_count dead (2,9) entry plus silent unwrap_or(9) fallback | game/sushi-go-2/src/lib.rs | M | sushi-go-fixes
games-batch-d F29 | nit | Pudding hint "least -6" shown in 2p games where penalty doesn't apply | game/sushi-go-2/src/lib.rs | M | sushi-go-fixes
games-batch-d F30 | nit | Maki second-place `<= 3` guard is a no-op rule-lookalike suppressing a log | game/sushi-go-2/src/lib.rs | M | sushi-go-fixes
games-batch-d F31 | nit | render_name logic duplicated between lib.rs and render.rs, fragile underflow expression | game/sushi-go-2/src/lib.rs, game/sushi-go-2/src/render.rs | M | sushi-go-fixes
games-batch-d F32 | nit | Dummy-slot guard reads playing[DUMMY] before the players==2 check | game/sushi-go-2/src/lib.rs | M | sushi-go-fixes
games-batch-d F33 | nit | Finished-game placings-log block duplicated across command() arms | game/sushi-go-2/src/lib.rs | M | epilogue-dup
games-batch-d F34 | critical | Infinite busy-loop with unbounded log growth when all hands empty after settle (round 4, legal play) | game/modern-art-2/src/lib.rs | M | modern-art-rules
games-batch-d F35 | major | Round 4 can start on an empty-handed player, soft-locking the game | game/modern-art-2/src/lib.rs | M | modern-art-rules
games-batch-d F36 | minor | Payout pays cumulative value for all purchases incl. non-top-3 artists (documented, Go-inherited) | game/modern-art-2/src/lib.rs, game/modern-art-2/RULES.md | D | modern-art-rules: port parity vs official payout rule
games-batch-d F37 | major | Artists with zero cards played are ranked and awarded $20/$10 (undocumented, Go-inherited) | game/modern-art-2/src/lib.rs | D | modern-art-rules: port parity vs official ranking rule
games-batch-d F38 | minor | unreachable!() and unchecked indexing in round_cards on deserialized state | game/modern-art-2/src/lib.rs | M | state-trust-panics
games-batch-d F39 | minor | RULES.md wrongly says auction winner takes the next turn | game/modern-art-2/RULES.md | M | modern-art-rules
games-batch-d F40 | minor | RULES.md Double-auction section omits Once Around as a valid added card | game/modern-art-2/RULES.md | M | modern-art-rules
games-batch-d F41 | minor | Game end mid-auction leaves stale State::Auction; final render shows bogus auction + $0 bid | game/modern-art-2/src/lib.rs | M | modern-art-fixes
games-batch-d F42 | nit | "Current bid: $0 by auctioneer" rendered before any bid exists | game/modern-art-2/src/render.rs, game/modern-art-2/src/lib.rs | M | modern-art-fixes
games-batch-d F43 | nit | Sealed/once-around bid ties broken in favor of the auctioneer | game/modern-art-2/src/lib.rs | D | modern-art-rules: tie-break edition adjudication
games-batch-d F44 | nit | can_add allocates a throwaway Vec via unwrap_or(&vec![]) | game/modern-art-2/src/lib.rs | M | modern-art-fixes
games-batch-d F45 | nit | Guarded bid.unwrap() in whose_turn_players; is_none_or expresses it panic-free | game/modern-art-2/src/lib.rs | M | modern-art-fixes
games-batch-d F46 | nit | Redundant `use std::default::Default` import | game/modern-art-2/src/lib.rs | M | modern-art-fixes
games-batch-e F1 | major | command() duplicates the same ~20-line finish/response wrap-up 8 times | game/love-letter-2/src/lib.rs | M | epilogue-dup
games-batch-e F2 | minor | end_score unreachable!() panics on default/corrupt Game via pub_state/status | game/love-letter-2/src/lib.rs | M | state-trust-panics
games-batch-e F3 | minor | end_round indexes hands[p][0] without checking emptiness | game/love-letter-2/src/lib.rs | M | state-trust-panics
games-batch-e F4 | minor | assert_target/play_* index state with unvalidated target player index | game/love-letter-2/src/lib.rs | M | state-trust-panics
games-batch-e F5 | minor | Commands still accepted and executed after the game is finished | game/love-letter-2/src/command.rs, game/love-letter-2/src/lib.rs | M | love-letter-fixes
games-batch-e F6 | nit | Redundant no-op hand assignments in play_baron (undocumented Go quirk) | game/love-letter-2/src/lib.rs | M | love-letter-fixes
games-batch-e F7 | nit | Guard self-target fallback skips Guard-guess validation (undocumented Go quirk) | game/love-letter-2/src/lib.rs, game/love-letter-2/PORTING_NOTES.md | M | love-letter-fixes
games-batch-e F8 | nit | discard_card records discards for cards the player does not hold | game/love-letter-2/src/lib.rs | M | love-letter-fixes
games-batch-e F9 | nit | mod test vs mod tests — workspace-wide inconsistency, not a crate defect (17 vs 10 split) | game/love-letter-2/src/lib.rs | D | workspace-conventions: pick one test-module naming convention
games-batch-e F10 | minor | Six unwrap/expect sites on game-service runtime paths (invariant-guarded) | game/age-of-war-2/src/lib.rs, game/age-of-war-2/src/command.rs, game/age-of-war-2/src/render.rs | M | state-trust-panics
games-batch-e F11 | minor | completed_lines HashSet serializes nondeterministically in persisted state | game/age-of-war-2/src/lib.rs | M | aow-fixes
games-batch-e F12 | nit | Not-your-turn returned as unstructured invalid_input instead of GameError::NotYourTurn | game/age-of-war-2/src/lib.rs | M | aow-fixes
games-batch-e F13 | nit | Placings-log tail triplicated across Attack/Line/Roll arms | game/age-of-war-2/src/lib.rs | M | epilogue-dup
games-batch-e F14 | nit | Finished games keep emitting duplicate placings logs on each accepted roll | game/age-of-war-2/src/lib.rs | M | epilogue-dup
games-batch-e F15 | nit | clan_conquered logic (incl. stale-player quirk) duplicated in renderer | game/age-of-war-2/src/lib.rs, game/age-of-war-2/src/render.rs | M | aow-fixes
games-batch-e F16 | nit | Player-facing help text "discard one dice" should be "die" | game/age-of-war-2/src/command.rs | M | aow-fixes
games-batch-e F17 | major | Finished stats hardcoded to players 0/1; 3-player games omit player 2's stats | game/lost-cities-2/src/lib.rs | M | lc2-fixes
games-batch-e F18 | major | player_state() unchecked hands[player] index panics on crafted PlayerRender request | game/lost-cities-2/src/lib.rs | M | lost-cities-shared
games-batch-e F19 | minor | Draw logs silently dropped when the draw empties the deck | game/lost-cities-2/src/lib.rs | M | lost-cities-shared
games-batch-e F20 | minor | PlayerState.hand documented as sorted but serialized in acquisition order | game/lost-cities-2/src/lib.rs, game/lost-cities-2/DATA_DOCS.md | M | lost-cities-shared
games-batch-e F21 | minor | Stats.investments never incremented; Stats.expeditions write-only | game/lost-cities-2/src/lib.rs | D | lost-cities-shared: keep-and-surface vs drop the stats fields
games-batch-e F22 | minor | unreachable!() arms panic for player counts outside 2..=3 on deserialized state | game/lost-cities-2/src/lib.rs, game/lost-cities-2/src/render.rs | M | state-trust-panics
games-batch-e F23 | nit | Perspective index uses % MAX_PLAYERS instead of % self.players | game/lost-cities-2/src/render.rs | M | lc2-fixes
games-batch-e F24 | nit | Game-over log regressed vs lost-cities-1 (no winner/margin announcement) | game/lost-cities-2/src/lib.rs | M | lc2-fixes
games-batch-e F25 | nit | Discard piles expose only the top card; official piles are fully inspectable | game/lost-cities-2/src/lib.rs | D | lost-cities-shared: expose full piles vs document the simplification
games-batch-e F26 | nit | Potential usize underflow in draw-count computation | game/lost-cities-2/src/lib.rs | M | lost-cities-shared
games-batch-e F27 | minor | Deployed GameVersion blurb still advertises a strictly two-player game | k8s/base/game/lost-cities-2/game-version.yaml | M | lc2-fixes
games-batch-e F28 | nit | Stale build-release and .rls.toml template cruft (also in acquire-1, lords-of-vegas-1) | game/lost-cities-2/build-release, game/lost-cities-2/.rls.toml | M | stale-template-files
games-batch-e F29 | critical | CardParser byte-index slicing panics on non-ASCII input, reachable from Play endpoint | game/red7-1/src/command.rs | M | red7-charbyte
games-batch-e F30 | major | Player with zero rule-fulfilling cards treated as winning; official rules say cannot win | game/red7-1/src/card.rs, game/red7-1/src/lib.rs | D | red7-rules: define empty-winning-set behaviour (leader Option, elimination, discard reject)
games-batch-e F31 | minor | DATA_DOCS.md tie-break description contradicts both code and official rules | game/red7-1/DATA_DOCS.md | M | red7-rules
games-batch-e F32 | minor | RULES.md omits play-then-discard combo and misdescribes scoring | game/red7-1/RULES.md | M | red7-rules
games-batch-e F33 | nit | Aliased PubCard/PubSuit re-export unused and non-conventional | game/red7-1/src/lib.rs | M | red7-fixes
games-batch-e F34 | nit | leader_with_suit indexes player_map[l_index], panics if all players eliminated | game/red7-1/src/lib.rs | M | red7-fixes
games-batch-e F35 | nit | end_points arithmetic underflows for player counts above 10 (pub fn, no guard) | game/red7-1/src/lib.rs | M | red7-fixes
games-batch-e F36 | major | player_state() unchecked hands[player] index panics on crafted PlayerRender request | game/lost-cities-1/src/lib.rs | M | lost-cities-shared
games-batch-e F37 | minor | draw_hand_full drops the draw's public+private logs when the draw empties the deck | game/lost-cities-1/src/lib.rs | M | lost-cities-shared
games-batch-e F38 | minor | PlayerState.hand documented as sorted but never sorted at all | game/lost-cities-1/src/lib.rs, game/lost-cities-1/DATA_DOCS.md | M | lost-cities-shared
games-batch-e F39 | minor | Stats.investments never written; Stats.expeditions write-only | game/lost-cities-1/src/lib.rs | D | lost-cities-shared: same keep-or-drop stats decision as lc2 F21
games-batch-e F40 | minor | stats.expeditions increment counts rounds-with-a-play, not expeditions | game/lost-cities-1/src/lib.rs | D | lost-cities-shared: depends on the stats keep-or-drop decision
games-batch-e F41 | nit | HAND_SIZE - hand.len() debug underflow (release clamped by num > dl check) | game/lost-cities-1/src/lib.rs | M | lost-cities-shared
games-batch-e F42 | nit | Hardcoded literal 2 instead of PLAYERS const (more sites than originally cited) | game/lost-cities-1/src/lib.rs | M | lc1-fixes
games-batch-e F43 | nit | score() uses is_none()-guarded unwrap() | game/lost-cities-1/src/lib.rs | M | lc1-fixes
games-batch-e F44 | nit | render.rs builds throwaway empty Vecs for map lookups | game/lost-cities-1/src/render.rs | M | lc1-fixes
games-batch-e F45 | minor | Binary-only deps (brdgme_cmd, brdgme_fuzz, tokio full) declared as lib deps in ~27 game crates | game/love-letter-2/Cargo.toml | D | boilerplate-bins: dev-dependencies fix is INVALID; choose optional deps + required-features vs separate bin crate
games-batch-e F46 | nit | HTTP binary defaults to privileged port 80, fails unprivileged without ADDR | game/love-letter-2/src/bin/love_letter_2_http.rs | M | boilerplate-bins

## Grouping notes

- Cross-crate pattern: **state-trust-panics** — panics reachable via deserialized/corrupt state or crafted requests rather than normal play: d-F5 (lov Loc::parse_str), d-F38 (modern-art round_cards), e-F2/F3/F4 (love-letter), e-F10 (age-of-war unwrap cluster), e-F22 (lc2 unreachable arms). The two request-reachable player_state panics (e-F18/F36) are the same one-line defect in both lost-cities crates and could alternatively be fixed once at the requester layer (lib/cmd/src/requester/gamer.rs) — a single upstream bounds check would cover both crates plus future ones.
- Cross-crate pattern: **epilogue-dup** — the finished-game placings-log/CommandResponse wrap-up is copy-pasted across command() arms in jaipur-2 (d-F21), sushi-go-2 (d-F33), love-letter-2 (e-F1, 8 copies, rated major), and age-of-war-2 (e-F13, plus the e-F14 duplicate-log amplification that a transition-gated helper would also fix). A shared helper or at least one identical refactor pattern applied per crate is the natural package.
- **modern-art-rules** is the batch-d hot spot: d-F34 (critical hang) and d-F35 (soft-lock) share one missing invariant (empty-hand handling at round boundaries) and merit a combined fix with round-4 regression tests; d-F36/F37/F43 are Go-port-inherited rules deviations awaiting the cross-unit port-parity-vs-official-rules adjudication. F37 compounds F36's payouts (phantom artist values inflate cumulative payouts). The mechanical F34/F35 fix does not need to wait on the D items.
- **lost-cities-shared**: lc1 is deprecated-but-deployed; most defects are shared verbatim (player_state panic, dropped draw logs, sorted-hand doc lie, dead stats, underflow). Fixing both crates in one package avoids drift; the stats fields (e-F21/F39/F40) need one keep-or-drop decision applied to both.
- **red7-charbyte** (e-F29) is a self-contained one-file critical, independent of the red7-rules design work (e-F30 leader semantics, with e-F31/F32 doc rewrites downstream of that decision).
- **boilerplate-bins** spans ~27 game crates: e-F45's original `[dev-dependencies]` recommendation is INVALID per verification (dev-deps do not apply to src/bin targets; the move would break every game binary) — the fix needs an architecture choice (optional deps + required-features, or a separate bin crate); the tokio feature trim is the immediately realizable win. e-F46 (port 80 default) would ride the same boilerplate-touching change.
- Other verification-flagged recommendation corrections: d-F13 (jaipur camel count) was REJECTED outright — its recommended fix (Camel => 11) would have introduced a real bug (14 camels); excluded from the rows above. d-F26's premise was refuted (official Sushi Go does tiebreak on puddings) — residual work is doc-only. e-F9's premise was inverted (mod test is the game-crate majority) — now a workspace-convention decision, not a crate fix.
- Determinism sub-theme: d-F2 (lov RNG-order divergence, major) and e-F11 (aow HashSet serialization order) are both hash-iteration-order defects; different consequences (replay divergence vs byte-level state diffs) but the same mechanism — worth one sweep for HashMap/HashSet in persisted/RNG-adjacent paths.
- Doc-vs-code fix packages (pure doc edits, safe to batch): d-F11, d-F17, d-F26, d-F39, d-F40, e-F31, e-F32, plus the e-F27 k8s blurb.
