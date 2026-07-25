# Raw findings: lost-cities-2 (games batch E)

Reviewer: Worker subagent, review-only audit of `/home/beefsack/Development/brdgme-review-snapshot/rust/game/lost-cities-2/`.
No Go original exists in `brdgme-go/` (confirmed absent); all rules findings below were judged against the official Lost Cities rules, not a Go port. No `PORTING_NOTES.md` exists in either lost-cities crate.
`rules()` verified conformant: `include_str!("../RULES.md")` at `game/lost-cities-2/src/lib.rs:652-654`.

Rules verified CORRECT against official rules (2-player mode), for the record:
- 3 investment (wager) cards multiply expedition score by (investments + 1); expedition cost -20; strictly ascending number play enforced (`play()`, lib.rs:353-402); investment after a number card rejected; per-expedition shared discard piles; round ends immediately when the deck empties (`draw_hand_full`, lib.rs:441-445); 8-card hand (2p); scoring `(sum - 20) * multiplier`; +20 bonus for 8+ cards counting wagers (`score()`, lib.rs:697-730). All match official rules.
- The 3-player mode (7-card hand, cost 15, bonus +15 at 7+ cards) is NOT part of official Lost Cities (officially 2-player only) — it is a documented house variant in RULES.md:23 and RULES.md:61-64, so treated as deliberate, not a finding.

Modules found clean:
- `src/command.rs` — parser restricts play/discard cards to actual hand contents (`player_card_parser`, command.rs:65-70); `take` target validated at the game layer; no panic paths.
- Serde views — `PubState` exposes only deck count (not order), top card per discard pile, scores, expeditions; `PlayerState.hand` is the requester's own hand only. No opponent-hand or deck-order leak.
- `src/bin/*` and `Cargo.toml` — byte-identical to the cross-crate boilerplate modulo crate name (verified by diff against splendor-2 and age-of-war-2; only import ordering noise). No deviations; systemic boilerplate issues intentionally not re-flagged.
- `tests/contract.rs` uses `assert_gamer_contract::<Game>()` per convention.

---

### Status::Finished stats hardcoded to players 0 and 1 — breaks 3-player games
- severity: major
- category: correctness
- location: game/lost-cities-2/src/lib.rs:534
- finding: `status()` builds `stats: vec![self.player_stats(0), self.player_stats(1)]`. lost-cities-2 supports 3 players (`MIN_PLAYERS..=MAX_PLAYERS` = 2..=3, lib.rs:26-27), so a finished 3-player game returns stats for only the first two players; player 2's stats are silently missing. The surrounding code was generalized (`placings()` at lib.rs:491-497 iterates `0..self.players`) but this line was carried over verbatim from lost-cities-1, where the hardcode is correct because -1 is 2-player-only (lost-cities-1/src/lib.rs:531).
- recommendation: `stats: (0..self.players).map(|p| self.player_stats(p)).collect()`.

### `player_state()` panics on out-of-range player index (request-reachable)
- severity: major
- category: correctness
- location: game/lost-cities-2/src/lib.rs:570
- finding: `player_state()` does `hand: self.hands[player].clone()` — unchecked indexing. `handle_player_render` in `rust/lib/cmd/src/requester/gamer.rs:174` passes the `player` field of a `Request::PlayerRender` straight into `game.player_state(player)` with no bounds check, so a crafted request to the game HTTP service with `player >= hands.len()` panics the handler (violates docs/CODING.md "no panicking code in runtime paths"). Every other accessor in this crate uses `.get()`/`.get_mut()` with `GameError` (e.g. `take()` lib.rs:269-272, `remove_player_card()` lib.rs:297-312); `player_state` is the exception. The `Play` command path itself is safe (`command_parser` returns `None` for any player != current_player, lib.rs:22 in command.rs). Same defect exists in lost-cities-1 (lib.rs:566).
- recommendation: `self.hands.get(player).cloned().unwrap_or_default()` (PlayerState is not fallible), or add a bounds check in `handle_player_render` returning a UserError.

### Draw logs silently dropped when the draw empties the deck
- severity: minor
- category: correctness
- location: game/lost-cities-2/src/lib.rs:441-445
- finding: `draw_hand_full` accumulates the public "P drew a card, N remaining" and private "You drew …" logs into the local `logs`, but when the draw empties the deck it returns `self.end_round()` directly, discarding `logs`. The final draw of every round is never logged — players see scoring/new-round logs with no record of the last card drawn. Same behavior in lost-cities-1 (lib.rs:434-438), so it's inherited, but it looks accidental: the fix is to extend `logs` with `end_round()`'s result, mirroring how `end_round` itself merges `start_round` logs (lib.rs:188-191).
- recommendation: `if self.deck.is_empty() { let mut l = logs; l.extend(self.end_round()?); Ok(l) } else { Ok(logs) }`.

### `PlayerState.hand` documented as sorted but serialized unsorted
- severity: minor
- category: consistency
- location: game/lost-cities-2/src/lib.rs:102-103 (doc comment), game/lost-cities-2/DATA_DOCS.md:19, game/lost-cities-2/src/lib.rs:570 (implementation)
- finding: The doc comment on `PlayerState.hand` and DATA_DOCS.md both state "Cards currently in this player's hand, sorted by expedition then value", but `player_state()` returns `self.hands[player].clone()` in acquisition order — hands are never globally sorted (only the per-draw batch is sorted before being appended, lib.rs:418, and `render_hand` sorts a copy for display, render.rs:304-305). Consumers of the JSON API (bots, operator, LLM tooling) reading DATA_DOCS will assume sorted input that isn't delivered.
- recommendation: either sort the hand in `player_state()` (`hand.sort()` — `Card: Ord`) to match the docs, or fix the docs.

### `Stats.investments` never incremented; `Stats.expeditions` incremented but never surfaced
- severity: minor
- category: quality
- location: game/lost-cities-2/src/lib.rs:51-52, game/lost-cities-2/src/lib.rs:383, game/lost-cities-2/src/lib.rs:455-489
- finding: `Stats.investments` is declared and serialized but no code path ever increments it — dead state carried in every saved game. `Stats.expeditions` is incremented in `play()` (lib.rs:383) but `player_stats()` only exposes Plays/Discards/Draws/Takes, so it is write-only. Same in lost-cities-1. Either the stats display was planned and never finished, or the fields should go.
- recommendation: increment `investments` in `play()` for `Value::Investment` and surface both fields in `player_stats()`, or remove the dead fields.

### `unreachable!()` arms panic on any player count outside 2..=3
- severity: minor
- category: correctness
- location: game/lost-cities-2/src/lib.rs:673-695 (`expedition_cost`, `hand_size`, `expedition_bonus_size`), game/lost-cities-2/src/render.rs:187 (`render_tableau`)
- finding: These helpers `unreachable!()` on players ∉ {2,3}. `self.players` is validated at `Game::start` (lib.rs:505), but `Game` is `Deserialize` with no validation on load — a corrupted/malformed state blob (the Play/Status requests carry the full game JSON) with `players: 4` or with `hands.len() < players` turns these into panics, as do the `self.hands[player]`-style indexes in the request path (see player_state finding). Additionally `score()` is `pub fn score(players: usize, …)` (lib.rs:697) and reaches `expedition_cost`'s `unreachable!()` for any caller passing another count. Low reachability in practice (states are written by the web backend), but they are panicking runtime paths.
- recommendation: return `GameError::internal` from a checked path, or clamp/default; at minimum, note the invariant on `Game.players`.

### Perspective index uses `% MAX_PLAYERS` instead of `% self.players`
- severity: nit
- category: correctness
- location: game/lost-cities-2/src/render.rs:130
- finding: `let p = player.unwrap_or(0) % MAX_PLAYERS;` — the modulus is the crate-wide maximum (3), not the actual player count of this game. In a 2-player game an index of 2 would survive the modulo and mis-render (opponent computed as `next_player(2, 2)` = 0, own tableau missing via `.get()`). Today unreachable because `player_state` panics earlier for out-of-range players, so it is latent; the lost-cities-1 equivalent (`cmp::min(player.unwrap_or(0), 1)`) clamped to the real count. Semantically wrong and misleading.
- recommendation: `% self.players` (and import `MAX_PLAYERS` no longer needed in render.rs).

### Game-over log regressed vs lost-cities-1 (no winner announcement)
- severity: nit
- category: quality
- location: game/lost-cities-2/src/lib.rs:198-200 (vs lost-cities-1/src/lib.rs:171-192)
- finding: lost-cities-1's `game_over_log` announced the winner and margin ("P2 won by 34 points") or the tie score; lost-cities-2 reduced it to a bare "The game is over." The `placings_log` appended in the `Draw` command arm (lib.rs:617-623) partially compensates, but the per-game log stream itself lost information -1 had. Looks like accidental drift during the 3-player generalization (winner computation wasn't generalized, so it was dropped rather than ported).
- recommendation: generalize -1's winner log (the `leaders()` helper at lib.rs:120-134 already computes what is needed) or confirm the regression is accepted.

### Discard piles expose only the top card; official rules have face-up, inspectable piles
- severity: nit
- category: correctness
- location: game/lost-cities-2/src/lib.rs:86-87, game/lost-cities-2/src/lib.rs:551-559
- finding: Judged against official rules (no Go port): in physical Lost Cities discard piles are face-up and players may inspect the buried cards (only the top card may be taken). `PubState.discards` exposes only `HashMap<Expedition, Value>` of the top card per pile, so the digital game hides information the physical game makes public (card-counting is part of the game). Deliberate simplification is plausible — many digital implementations do this — but it is a rules-fidelity deviation worth a conscious decision. Same in lost-cities-1.
- recommendation: expose the full ordered discard lists in `PubState` (they are derived from `Game.discards`, already public information), or document the deviation in RULES.md.

### Potential usize underflow in draw-count computation
- severity: nit
- category: correctness
- location: game/lost-cities-2/src/lib.rs:408
- finding: `let mut num = hand_size(self.players) - hand.len();` underflows (debug panic; release wrap then clamped by the `num > dl` guard, which would then drain the entire remaining deck) if a hand ever exceeds the hand size. Unreachable through normal play (take replaces the played card 1:1), but the subtraction pattern is fragile — a saturating form is free.
- recommendation: `hand_size(self.players).saturating_sub(hand.len())`.

### Stale legacy files in crate root
- severity: nit
- category: quality
- location: game/lost-cities-2/build-release, game/lost-cities-2/.rls.toml, game/lost-cities-2/.gitignore
- finding: `build-release` is a pre-Rust-workspace lambda/muslrust packaging script (`clux/muslrust`, fetches `github.com/brdgme/lambda/index.js`) — obsolete under the current `rust/Dockerfile` + docker-bake pipeline. `.rls.toml` configures the long-dead Rust Language Server and is even malformed (`build_lib = truetarget` — missing newline). Same cruft exists in acquire-1 and lords-of-vegas-1 (only those three crates have `build-release`/`​.rls.toml`), so it looks like these files were copied from an old template and never cleaned.
- recommendation: delete `build-release`, `.rls.toml`; trim `.gitignore` to what's still relevant.

### Deployed blurb still advertises a strictly two-player game
- severity: minor
- category: consistency
- location: k8s/base/game/lost-cities-2/game-version.yaml (blurb field), also rust/game/lost-cities-2/RULES.md:3
- finding: The GameVersion blurb for lost-cities-2 says "A tense two-player card game of investment and restraint" (copied verbatim from lost-cities-1's manifest), but -2 advertises `player_counts() = [2, 3]` (lib.rs:644-646) and RULES.md documents the 3-player variant. Players browsing the new-game page are told it's 2-player-only. (RULES.md itself is fine — it says "A 2-3 player card game".)
- recommendation: update the blurb in k8s/base/game/lost-cities-2/game-version.yaml to mention 2-3 players.

---

## lost-cities-1 vs lost-cities-2 relationship assessment

**Conclusion: lost-cities-1 is the legacy, superseded version; lost-cities-2 is its active replacement. The duplication is deliberate and justified by the deprecation lifecycle, not accidental.**

Evidence:
- `k8s/base/game/lost-cities-1/game-version.yaml:10` sets `isDeprecated: true`; lost-cities-2's manifest does not. Both remain in `k8s/base/game/kustomization.yaml:21-22`, the Tiltfile, docker-bake.hcl and `rust/Dockerfile`, matching the documented process in `docs/porting/GAME_PORTING.md` ("When replacing a previously deployed GameVersion, the new version gets its own manifests and the old GameVersion is marked `isDeprecated: true`") — the old service stays deployed so in-flight games can finish.
- `docs/porting/GAME_PORTING.md` ("Versioning" section): lost-cities-1 predates the versioning rule; lost-cities-2 is the later replacement port. `docs/BACKLOG.md` (~line 361) notes lost-cities-1 is a "native Rust -1 edition (no Go predecessor)".
- Note: GAME_PORTING.md also claims -2 was "a later replacement port from the in-repo Go code", but no lost-cities Go implementation exists in `brdgme-go/` (confirmed absent). Either the doc is stale or the Go source was removed after the port. Rules correctness for -2 therefore could not be checked against a Go original and was judged against official rules instead.

**Rules relationship:** -1 is strictly 2-player; -2 is a generalization adding a (non-official, but RULES.md-documented) 3-player variant: hand 8→7, expedition cost 20→15, bonus threshold 8→7, bonus +20→+15 (constants at lost-cities-2/src/lib.rs:30-35). Both use the same 5 expeditions — -2 does NOT implement the 6-expedition/6th-color variant. Where they overlap (all 2-player logic), they are consistent: identical card model (`card.rs` differs only in receiver style), identical scoring formula and test vectors, identical play/discard/take/draw validation, identical phase machine, identical tests modulo the `score(players, …)` signature.

**Accidental-looking drift (all flagged above):**
1. `status()` stats hardcode — harmless in -1, a real 3-player bug in -2 (major finding #1).
2. `game_over_log` — -1 announces winner/margin, -2 emits a bare "The game is over." (nit #8).
3. Cosmetic only: -1 wraps parse errors (`Err(GameError::invalid_input(e.to_string()))`, lost-cities-1/src/lib.rs:625) where -2 returns `Err(e)` directly; `deck.shuffle(&mut rng)` vs `deck.as_mut_slice().shuffle(&mut rng)`. Not findings.

Both crates share the `player_state` unchecked-index panic and the dropped-draw-logs behavior, i.e. the defects were inherited from -1 rather than introduced by -2 (except the stats hardcode, which became a defect only through -2's 3-player support).
