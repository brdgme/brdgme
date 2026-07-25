# Raw findings: game/lost-cities-1 (games batch E)

Reviewer: Worker (games-batch-e, lost-cities-1)
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/lost-cities-1/`

Context notes:
- lost-cities-1 is deprecated-but-deployed (superseded by lost-cities-2); real defects still matter because it serves live games, but severity is noted in that light.
- There is NO Go original for lost-cities in `brdgme-go/` (confirmed absent). All rules findings below were judged against the official Lost Cities rules and the crate's own `RULES.md` (which accurately restates them), not against a Go port.
- No `PORTING_NOTES.md` exists in this crate, so there are no documented preserved quirks to cross-reference.
- `rules()` returns `include_str!("../RULES.md")` (src/lib.rs:645-647) — matches project convention. `data_docs()`, `basic_strategy()`, `advanced_strategy()` likewise include their markdown files.

## Findings

### player_state() unchecked hands[player] index — panic via crafted PlayerRender request
- severity: major
- category: correctness
- location: game/lost-cities-1/src/lib.rs:566
- finding: `player_state()` does `hand: self.hands[player].clone()` with no bounds check. `player_state` is invoked from `Request::PlayerRender { player, game }` in `rust/lib/cmd/src/requester/gamer.rs:44-46` → `handle_player_render` (gamer.rs:174) with the `player` field taken straight from the deserialized request JSON. A crafted request with `player >= 2` panics the handler (index out of bounds; hands always has exactly 2 entries). Same defect pattern as found in lost-cities-2. All other crate entry points guard the player index (`assert_player_turn` via `whose_turn`), so this is the one reachable panic from crafted input. A panic kills the game-service request; the http bin serves this over warp.
- recommendation: Use `self.hands.get(player).cloned().unwrap_or_default()` in `player_state()`, or have the requester layer reject out-of-range player indices before calling `player_state`.

### draw_hand_full drops the draw's public+private logs when the draw empties the deck
- severity: major
- category: correctness
- location: game/lost-cities-1/src/lib.rs:434-438
- finding: `draw_hand_full` builds `logs` containing the public "drew a card, N remaining" log and the private "You drew ..." log, but then: `if self.deck.is_empty() { self.end_round() } else { Ok(logs) }`. When the draw takes the last card, `end_round()` is called and the accumulated draw logs are silently dropped — players never see what was drawn on the final draw of a round/game (the private "You drew X" is information the drawing player is entitled to and needs for the next round is moot, but the public draw log and the private card reveal are lost from the log stream; on the last round the game ends with no record of the final draw). Same defect pattern as found in lost-cities-2. Judged against official rules: the draw itself is legal (round ends after the last card is drawn), so state is correct — only the logs are lost.
- recommendation: Return `logs.extend(end_round_logs)` — e.g. `let mut l = self.end_round()?; logs.append(&mut l); Ok(logs)` — so the draw logs precede the round-end logs.

### PlayerState.hand documented as "sorted" but is never sorted
- severity: minor
- category: consistency
- location: game/lost-cities-1/src/lib.rs:92 (rustdoc), game/lost-cities-1/DATA_DOCS.md:18, vs game/lost-cities-1/src/lib.rs:562-568
- finding: The rustdoc on `PlayerState.hand` says "Cards currently in this player's hand, sorted by expedition then value", and DATA_DOCS.md:18 repeats "Cards are sorted by expedition then value." Nothing sorts the hand: `player_state()` returns `self.hands[player].clone()` in raw order (initial deal order plus appended draws; `draw_hand_full` sorts only the `drawn` copy used for the log at lib.rs:411, not the hand). API consumers/bots relying on the documented ordering will get arbitrary order. Same defect pattern as found in lost-cities-2.
- recommendation: Either sort the hand in `player_state()` (`let mut hand = self.hands[player].clone(); hand.sort();`) or fix the two doc strings to say the hand is in arbitrary order.

### Stats.investments never written; Stats.expeditions write-only
- severity: minor
- category: quality
- location: game/lost-cities-1/src/lib.rs:44-45 (fields), lib.rs:376 (only expeditions write), lib.rs:448-482 (player_stats reads neither)
- finding: `Stats.investments` is declared, serialized, and defaulted but incremented nowhere — it is always 0. `Stats.expeditions` is incremented once (lib.rs:376) but `player_stats()` only surfaces Plays/Discards/Draws/Takes, so the value is never read. Both fields are dead weight carried in every serialized game state. Same defect pattern as found in lost-cities-2.
- recommendation: Remove both fields (serde `#[serde(default)]` for back-compat on load) or actually track and surface them in `player_stats()`.

### stats.expeditions increment condition does not match the field name
- severity: minor
- category: correctness
- location: game/lost-cities-1/src/lib.rs:370-377
- finding: In `play()`, `self.stats[player].expeditions += 1` fires only when the player's *entire* expedition tableau is empty, i.e. only on the very first card played in a round. A stat named "expeditions" would naturally count expeditions started (first card per expedition color, up to 5 per round). As written it counts "rounds in which the player played at least one card". Harmless today only because the stat is never displayed (see previous finding).
- recommendation: If the stat is kept, test per-expedition emptiness (`self.expeditions[player].iter().all(|c| c.expedition != c_expedition)` before the push) or rename the field to reflect what it counts.

### Hand-sized arithmetic `HAND_SIZE - hand.len()` can underflow
- severity: nit
- category: correctness
- location: game/lost-cities-1/src/lib.rs:401
- finding: `let mut num = HAND_SIZE - hand.len();` panics on usize underflow (debug builds) if a hand ever exceeds 8 cards. Not reachable through the normal turn cycle (every turn removes one card before adding one, and round-start deal begins empty), so this is latent rather than live. Flagging because CODING.md prohibits panicking code in runtime paths; a `hand.len().saturating_sub`-style or `HAND_SIZE.saturating_sub(hand.len())` formulation is panic-free. Judged against official rules (no Go port exists): the 8-card hand invariant holds in all reachable flows.
- recommendation: Use `HAND_SIZE.saturating_sub(hand.len())`.

### Hardcoded literal player count `2` instead of the PLAYERS const
- severity: nit
- category: consistency
- location: game/lost-cities-1/src/lib.rs:144 (`for p in 0..2`), lib.rs:230 (`% 2` in next_player), lib.rs:501 (`(player + 1) % 2` in opponent), lib.rs:616 (`(0..2)` in command's finished-scores)
- finding: The crate defines `const PLAYERS: usize = 2` (lib.rs:25) and uses it elsewhere (lib.rs:124, 509, 634), but four spots use bare `2`. Harmless while the game is 2-player-only (`player_counts()` returns `vec![2]`, lib.rs:637-639), but inconsistent within the file and easy to miss if the const-based code is ever refactored. Note: `status()` returning stats for exactly players 0 and 1 (lib.rs:531) is NOT a defect here — the crate only supports 2 players, unlike the lost-cities-2 finding.
- recommendation: Replace the literals with `PLAYERS` (or `self.hands.len()` / `self.scores.len()` where that's the real bound).

### score() uses unwrap() guarded by an is_none() check
- severity: nit
- category: quality
- location: game/lost-cities-1/src/lib.rs:680-687
- finding: `let cards = exp_cards.get(&e); if cards.is_none() { return acc; } ... if cards.unwrap() >= &8` — the `unwrap()` is safe only because of the early return. An `if let Some(&cards) = exp_cards.get(&e)` restructure removes the panic-shaped code per CODING.md's no-panicking-code-in-runtime-paths stance. Scoring math itself verified correct against official rules (no Go port exists): `(sum - 20) * (investments + 1)`, +20 bonus at >= 8 total cards including investments, unstarted expeditions score 0, all-isize arithmetic so no underflow. The `score_works` test (lib.rs:781-822) covers the main cases.
- recommendation: Restructure with `if let Some(...)` to drop the unwrap.

### render.rs builds temporary empty Vecs for map lookups
- severity: nit
- category: quality
- location: game/lost-cities-1/src/render.rs:185 and render.rs:196
- finding: `by_exp.get(&e).unwrap_or(&vec![])` allocates a throwaway empty `Vec` on every lookup (5 per call in `render_tableau_cards`, plus the `largest` loop). Render-only, tiny cost, but the idiom is wasteful and slightly noisy; a module-level `&[]` or `.map(Vec::as_slice).unwrap_or_default()` avoids it.
- recommendation: Use `.map(Vec::as_slice).unwrap_or(&[])` or restructure the loop.

## Areas reviewed and found clean

- **Rules correctness (judged against official Lost Cities rules — no Go port exists):** initial deck of 60 (5 expeditions x (3 investments + 9 numbered)), 8-card hands, 44-card draw pile (src/lib.rs:96-107, test at 702-709); strictly ascending play with investments locked out after any numbered card (lib.rs:332-363) — matches RULES.md:36-42; can't take the card you just discarded this turn via `discarded_expedition` (lib.rs:243-247, reset in `start_turn` lib.rs:234-237); round ends immediately when the deck empties (lib.rs:434); next round started by higher total score, tie goes to the other player (lib.rs:129-136) — matches RULES.md:84; scoring formula verified above (lib.rs:662-688); 3 rounds then finish (lib.rs:160-168, 527-539). No rules defects found.
- **Serde views / information leaks:** `PubState` exposes only deck *count* (lib.rs:546), the *top* discard value per expedition (lib.rs:547-555), played expedition cards, and scores — no deck order, no opponent hand. `PlayerState` carries only the owning player's hand. Clean.
- **Command parser (src/command.rs):** parser only offered to the current player (command.rs:22), card choices restricted via `Enum::exact` over the player's actual hand (command.rs:65-70), so crafted `Value::N(99)`-style inputs are unparseable; non-current players get `None` → "not your turn" (lib.rs:576-579). Out-of-range `player` in `command_parser` yields an empty hand enum → parse error, not a panic (command.rs:66). No input-validation gaps found beyond the `player_state` panic above.
- **Binaries (src/bin/*.rs) and Cargo.toml:** the 4 binaries (cli, fuzz, http, repl) are byte-for-byte the standard boilerplate for this workspace; Cargo.toml carries the standard boilerplate dependency set. NO deviations from the systemic pattern — the systemic issues (binary-only deps as library `[dependencies]`, http bin default `0.0.0.0:80`, tokio "full") were already captured by another worker and are not re-flagged.
- **card.rs:** clean — enums, Ord derivations (Investment sorts before N, correct for hand/log sorting), Display/From impls, and helpers (`by_expedition`, `of_expedition`, `last_expedition`) are all sound. The Expedition→NamedColor mapping (White→Grey, Yellow→Orange) is a display choice, not a defect.
- **render.rs (other than the nit above):** all indexing is guarded (`get`, `cmp::min(player.unwrap_or(0), 1)` at render.rs:116, `p < 2` guard at render.rs:39); no panic paths found.
- **tests:** in-crate tests cover start/end-round/game-end/play-order/scoring/placings, plus the standard `assert_gamer_contract` contract test in tests/contract.rs. Reasonable coverage for a deprecated crate.
