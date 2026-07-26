# WP-28: lost-cities-1 / lost-cities-2 shared fixes

> **CITATION WARNING - line numbers in this spec are approximate and unverified.**
> Corpus-wide they measured **33-46% wrong**, and two "delete lines A-B" ranges
> would have destroyed live code. **Navigate by the named function, type or
> symbol** - never by line number alone. If the code at a cited location does not
> match this spec's description, **STOP and report**; do not improvise a fix or
> guess at the intended target.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Fix the one real correctness bug in lost-cities-2's 3-player support — `Status::Finished` reports stats for players 0 and 1 only (e F17) — and then repair the defects that exist **verbatim in both crates** in a single pass so the two ports stop drifting: dropped draw logs at round end (e F19 / e F37), `PlayerState.hand` documented sorted but delivered unsorted (e F20 / e F38), draw-count `usize` underflow (e F26 / e F41), `is_none()`-guarded `unwrap()` in `score()` (e F43), throwaway empty `Vec`s in the renderer (e F44). Plus the crate-specific riders: -2's regressed game-over log (e F24) and its `% MAX_PLAYERS` perspective bug (e F23), -1's bare `2` literals where `PLAYERS` exists (e F42), and the deployed blurb that still advertises a two-player-only game (e F27).

**Architecture — how the two crates work (read this before editing):**

- Two sibling crates, `rust/game/lost-cities-1` (package `lost-cities-1`, lib `lost_cities_1`) and `rust/game/lost-cities-2` (package `lost-cities-2`, lib `lost_cities_2`) — both confirmed from `Cargo.toml:2`. **-2 is a 2-3 player generalization of -1**, produced by copying -1 and parameterizing the player count. -1 is `isDeprecated: true` in k8s but still serves live games, so both are in scope.
- `-1` is 2-player only: `const PLAYERS: usize = 2` (lib.rs:25), `HAND_SIZE: usize = 8` (lib.rs:28), `player_counts() -> vec![2]` (lib.rs:638). `-2` is parameterized: `MIN_PLAYERS = 2` / `MAX_PLAYERS = 3` (lib.rs:26-27), per-count constants `HAND_SIZE_2P/3P`, `EXP_COST_2P/3P`, `EXP_BONUS_SIZE_2P/3P` (lib.rs:30-35) resolved by the free functions `hand_size()`, `expedition_cost()`, `expedition_bonus_size()` (lib.rs:673-695), and `Game.players: usize` as a stored field (lib.rs:57).
- `src/lib.rs` in both: `Stats` (per-player counters), `Game` (serde-persisted, all fields `pub`, `#[serde(default = "GameRng::from_entropy")]` migration shim on `rng` — -1 lib.rs:62-63, -2 lib.rs:70-71), `PubState`, `PlayerState`, the `Gamer` impl, the turn/round engine, a free `score()` fn, and an inline `#[cfg(test)] mod test` (**note: `test`, not `tests`** — -1 lib.rs:690, -2 lib.rs:732; `src/card.rs` in both uses `mod tests`; that inconsistency is e F9, out of scope).
- Turn/round flow (identical in both): `start()` -> `start_round()` (fresh shuffled 60-card deck, empty hands/expeditions, `draw_hand_full` per player) -> per turn, `Phase::PlayOrDiscard` (`play`/`discard`) then `Phase::DrawOrTake` (`draw`/`take`) -> when a `draw` empties the deck, `draw_hand_full` calls `end_round()`, which scores the round and either `start_round()`s the next one or pushes `game_over_log()`. 3 rounds (`ROUNDS = 3`).
- `src/render.rs` in both: a `render(pub_state, player, hand)` free fn plus `impl PubState { fn render_tableau }`. **-2's renderer is genuinely rewritten** (331 lines vs -1's 235): named spacer constants, a 2p/3p branch in `render_tableau` (-2 render.rs:135-188), a generalized score table (`for p_offset in 0..pub_state.players`, -2 render.rs:77-78) where -1 iterates `&[persp, opponent(persp)]` (-1 render.rs:64).
- `src/command.rs` is **byte-identical** between the crates (verified: `diff` empty). `src/card.rs` differs only cosmetically (import order, `&self` -> `self` on `Copy` receivers, `mod tests` block ordering). `tests/contract.rs` differs only in the crate name. `Cargo.toml` differs only in `name`.
- `Card` derives `Ord` with `expedition` before `value` (card.rs:72), and `Expedition`'s variant order is Red, Green, White, Blue, Yellow (card.rs:12-19), `Value::Investment < Value::N(_)` (card.rs:6-10). So a plain `Vec<Card>::sort()` yields exactly "sorted by expedition then value" — empirically confirmed: `["Y9","RX","G2"]` sorts to `["RX","G2","Y9"]`.
- **Serialization:** `Game` is the blob persisted in `games.game_state` and shipped inside every `Play`/`Status` request; `PubState`/`PlayerState` are **transient view types** serialized per request by `lib/cmd/src/requester/gamer.rs:158-181` (`handle_pub_render`/`handle_player_render`) and never persisted. **No task in this package changes a serialized type, field name, or field type.** The only shape-visible change is e F17's `stats` array length (see below), which is a response field, not stored state.
- **`Status::Finished { placings, stats }`** (`lib/game/src/game.rs:21-29`): `stats: Vec<HashMap<String, Stat>>`, one map per player. Traced end to end: `Gamer::stats()` (game.rs:92-95) reads it, `GameResponse::from_gamer` carries it in the `Status`/`Play` response — and **`rust/web` consumes only `placings`** (`web/src/game/mod.rs:37`: `Status::Finished { placings, .. }`). `stats` is currently unread by the web tier and is not written to any DB column. Fixing e F17 therefore cannot break a consumer; it makes the field correct for whoever reads it next (bot/operator/LLM tooling).
- Bins (`src/bin/*.rs`) are the 4 standard boilerplate binaries; `tests/contract.rs` is `assert_gamer_contract::<Game>()`, which drives `New` for **every** advertised player count and asserts a non-empty public render plus one player render per player (`lib/cmd/src/test_support.rs`) — so it exercises `render_tableau` for 2p and 3p in -2, and both renderer tasks below are covered by it.

**Tech Stack:** Rust 1.97.0, edition 2024, workspace at `/home/beefsack/Development/brdgme/rust`. Two crates touched plus two k8s manifests. `let`-else, `saturating_sub`, `map_or`, and slice patterns are all available on this toolchain. Deps unchanged (no `Cargo.toml`, no `Cargo.lock` edits anywhere in this package).

**Global Constraints:**

- All cargo commands run from `/home/beefsack/Development/brdgme/rust`. **Per-crate only**: `cargo test -p lost-cities-1`, `cargo test -p lost-cities-2`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Each task ends with, for each crate it touched: `cargo clippy -p lost-cities-<n> --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- **Green baseline (measured 2026-07-25, live repo):** `cargo test -p lost-cities-1` = **7 lib tests + 1 integration test, 0 failures** (`card::tests::value_cmp_works`, `test::start_works`, `test::end_round_works`, `test::game_end_works`, `test::play_works`, `test::score_works`, `test::placings_works`; `tests/contract.rs::game_contract`). `cargo test -p lost-cities-2` = **identical 7 + 1**. Every one of those 16 tests MUST keep passing, unmodified, after every task. None of them constrains any fix below (verified empirically — see "Empirical validation" below).
- **Fix both crates in the same task and the same commit** wherever the defect is shared. That is the entire point of this package: -2 inherited its bugs from -1 by copy-paste, and fixing only one crate re-creates the drift the review found.
- Line numbers cited are LIVE-file numbers as of the drift check below. Tasks 1-8 shift line numbers in the files they touch by small amounts; later tasks locate by symbol name and by the quoted `old` text, never by line number alone.
- Production `kubectl`/deploy mutations are **out of scope for this spec**. Task 10 edits manifest YAML in git only; applying it is a separate, human-initiated deploy.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without containers is pre-existing, backlog #40 — the script supplies the containers).

**Non-Goals** (each verified against `planning/work-packages.md`; anything not in WP-28's 13-ID scope list is a non-goal):

- **e F18 / e F36 — `player_state()` unchecked `self.hands[player]`** (-2 lib.rs:570, -1 lib.rs:566): owned by **WP-09** (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36; the fix is one bounds check at `lib/cmd/src/requester/gamer.rs`). **Task 3 edits the very line that panics — it MUST keep the `self.hands[player]` indexing form and must NOT switch to `.get(player)`.** Writing `self.hands.get(player).cloned().unwrap_or_default()` here would silently discharge a major finding owned by another package, defeat WP-09's decision, and make WP-09's own red test un-reproducible.
- **e F22 — `unreachable!()` in `expedition_cost`/`hand_size`/`expedition_bonus_size`** (-2 lib.rs:673-695) and `render_tableau`'s `_ => unreachable!()` (-2 render.rs:187): owned by **WP-09**. Task 5 edits `render_tableau`'s first line and must leave the `_ => unreachable!()` arm exactly as it is.
- **e F21 / e F39 — `Stats.investments` dead, `Stats.expeditions` write-only** and **e F40 — `expeditions` increment condition mismatches its name** (-1 lib.rs:370-377, -2 lib.rs:382-384): owned by **WP-30** (batch-e rules and stats adjudication, BLOCKED-ON-DECISION D-29/D-40 — keep-or-drop is a product decision). Do NOT add, remove, or start incrementing any `Stats` field, and do NOT surface `investments`/`expeditions` in `player_stats()`. Task 1 changes only **which players** `player_stats()` is called for, never what it returns.
- **e F25 — discard piles expose only the top card**: owned by **WP-30**. `PubState.discards` stays `HashMap<Expedition, Value>` in both crates.
- **e F13 / e F14 — age-of-war-2 epilogue dedup**: owned by **WP-08**; different crate entirely. Both lost-cities crates do have the "placings log on the finishing `draw`" epilogue (-1 lib.rs:613-618, -2 lib.rs:617-623), but neither is in WP-08's scope list — do not refactor it.
- **e F28 — stale `build-release` / `.rls.toml` / `.gitignore` in lost-cities-2's crate root**: owned by the dependency-hygiene package (work-packages.md:502-503 lists `game/lost-cities-2/{build-release,.rls.toml}`). Leave those three files alone.
- **e F45 / e F46 — boilerplate binary deps and the port-80 HTTP default**: owned by work-packages.md:555. No `Cargo.toml` or `src/bin/` edits in this package.
- Implementing the 3-player variant's rules any further, changing scoring, or touching `RULES.md` in either crate. RULES.md is already correct in both (-2 RULES.md:3 says "A 2-3 player card game", :23 documents the 7-card 3p hand, :88 documents 3p round-start order).
- Unifying the two crates into one, extracting a shared library, or backporting -2's renderer to -1. The crates are intentionally separate deployed game versions.

**Snapshot drift:** **None, both crates.** `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/lost-cities-1 /home/beefsack/Development/brdgme/rust/game/lost-cities-1` and the same for `lost-cities-2` both exit 0 with empty output (verified 2026-07-25 against snapshot commit `f8763a5`). All line numbers below are live-repo numbers and match the findings' citations except where the verification pass already corrected them (e F27's blurb is at `game-version.yaml:9`, not `:1`).

**-1 vs -2 divergence audit (diffed, not assumed):**

| Concern | lost-cities-1 | lost-cities-2 | Fix text |
|---|---|---|---|
| `src/command.rs` | — | — | byte-identical (`diff` empty); untouched by this package |
| `src/card.rs` | `&self` receivers, import order | `self` receivers | cosmetic only; untouched |
| Dropped draw logs (F19/F37) | lib.rs:434-438 | lib.rs:441-445 | **identical fix**, identical surrounding code |
| `hand` doc vs delivery (F20/F38) | rustdoc lib.rs:92 + DATA_DOCS.md:18; `player_state` lib.rs:562-568 | rustdoc lib.rs:102-103 + DATA_DOCS.md:19; `player_state` lib.rs:566-572 | **identical fix** (`hand.sort()`); doc text already correct in both, no doc edit needed |
| Draw-count underflow (F26/F41) | `HAND_SIZE - hand.len()` lib.rs:401 | `hand_size(self.players) - hand.len()` lib.rs:408 | same shape, **different expression** — `HAND_SIZE.saturating_sub(..)` vs `hand_size(self.players).saturating_sub(..)` |
| `score()` guarded `unwrap()` (F43) | lib.rs:680-687, magic `20`/`8` | lib.rs:718-729, `exp_cost`/`exp_bonus_size` locals | **same restructure, different literals** — finding filed against -1 only; -2's site is identical in shape (see Task 7) |
| Renderer throwaway `Vec`s (F44) | render.rs:185, 196 | render.rs:264, 282 | **identical fix**; finding filed against -1 only; -2's sites are the same two expressions (see Task 8) |
| Finished stats (F17) | lib.rs:531 — **correct**, crate is 2p-only | lib.rs:534 — **wrong**, carried over verbatim | **-2 only** |
| Perspective index (F23) | render.rs:116 `cmp::min(player.unwrap_or(0), 1)` — **correct clamp** | render.rs:130 `player.unwrap_or(0) % MAX_PLAYERS` — **wrong** | **-2 only** |
| Game-over log (F24) | lib.rs:171-192 — announces winner + margin, or "scores tied at N" | lib.rs:198-200 — bare "The game is over." | **-2 only**, and the fix must be *generalized*, not copied (see Task 4) |
| Bare `2` literals (F42) | lib.rs:144, 230, 501, 511-512, 616, 638, 642 + render.rs:39 | n/a — `-2` has no `PLAYERS` const; uses `self.players`/`MIN_PLAYERS`/`MAX_PLAYERS` throughout | **-1 only** |
| Deployed blurb (F27) | `k8s/base/game/lost-cities-1/game-version.yaml:9` — same text, and **correct** for a 2p game | `k8s/base/game/lost-cities-2/game-version.yaml:9` — wrong | **both manifests must change** — see Task 10 |
| `command()` parse-error arm | lib.rs:625 `Err(e) => Err(GameError::invalid_input(e.to_string()))` — flattens the error kind | lib.rs:630 `Err(e) => Err(e)` — preserves it | divergence noted, **not** a filed finding; left alone (see Cross-package) |
| `score()` constants | hardcoded `20` and `8` | `expedition_cost()` / `expedition_bonus_size()` | divergence noted, not a filed finding; left alone |
| Renderer structure | 235 lines, 2p only | 331 lines, 2p/3p branch, named constants | intentional; not reconciled |

**Re-derivation notes (every claim below re-read from live source, not taken from the findings):**

- **e F17 (3p stats, major, -2 only).** `status()` at -2 lib.rs:530-542 returns `Status::Finished { placings: self.placings(), stats: vec![self.player_stats(0), self.player_stats(1)] }`. `placings()` (lib.rs:491-497) is properly generalized over `0..self.players`; the `stats` line is not. `player_counts()` (lib.rs:644-646) advertises `(MIN_PLAYERS..=MAX_PLAYERS)` = `[2, 3]`, and `Game::start` (lib.rs:504-511) accepts 3, so a 3-player game is reachable through the normal `New` request and its finished status silently omits player 2. `player_stats(p)` (lib.rs:455-489) already bounds-guards (`if player >= self.stats.len() { return stats; }`) so generalizing the caller cannot introduce a panic. **Shape impact:** the serialized `stats` array goes from always-2 to `self.players` entries. Safe because (a) `stats` is a response field on `Status`, never a stored column, and (b) the only in-repo consumer, `web/src/game/mod.rs:37`, destructures `Status::Finished { placings, .. }` and ignores `stats` entirely. For a 2-player game the output is byte-identical to today's. **Empirically confirmed red:** a 3p game driven to completion asserts `stats.len() == 3` and fails with `left: 3, right: 2`. -1's identical-looking line (lib.rs:531) is **correct** and must not be touched: `Game::start` rejects anything but 2 (lib.rs:509-514).
- **e F19 / e F37 (dropped draw logs, minor in both after verification aligned F37 down from major).** `draw_hand_full` accumulates a public "P drew N cards, M remaining" log and a private "You drew …" log into a local `logs` (-1 lib.rs:398-430, -2 lib.rs:405-437), then:

  ```rust
  if self.deck.is_empty() {
      self.end_round()          // <-- `logs` dropped on the floor
  } else {
      Ok(logs)
  }
  ```

  (-1 lib.rs:434-438, -2 lib.rs:441-445). So the **final draw of every round** — including the last draw of the whole game — is never logged. State is unaffected; this is a pure log-stream loss. The fix is `logs.extend(self.end_round()?); Ok(logs)`. **No duplication risk:** `end_round()` (-1 lib.rs:141-169, -2 lib.rs:168-196) returns only the round-score logs plus either `start_round()`'s logs or `game_over_log()`; it never re-emits the draw that triggered it. Chronology is correct (draw before scoring). Empirically confirmed: pre-fix the round-end `draw()` returns 10 logs starting with "P0 scored 0 points"; post-fix it returns 12, starting with "P2 drew a card, 0 remaining" + "You drew B5".
- **e F20 / e F38 (hand documented sorted, delivered unsorted, minor).** The rustdoc (-1 lib.rs:92, -2 lib.rs:102-103) and DATA_DOCS.md (-1:18, -2:19) both promise "sorted by expedition then value". `player_state()` returns `self.hands[player].clone()` — raw acquisition order. In -2 only the *per-draw batch* is sorted before being appended (`drawn.sort()`, lib.rs:418) and `render_hand` sorts a display copy (render.rs:304-305); in -1 the verification pass established it is **stronger** — `drawn.sort()` at lib.rs:411 sorts only the vector used for the private log, so the hand itself is never sorted at any point. **Chosen direction: make the code match the docs, not the reverse.** Reasons: DATA_DOCS.md is the published contract for bots/operator/LLM tooling (`Gamer::data_docs()` serves it verbatim); the renderer already sorts, so the sorted order is what humans see and the unsorted JSON is the odd one out; and `PlayerState` is a per-request view, so sorting it costs one `sort()` on a <=8-element `Vec` and changes no stored state. `Card: Ord` gives exactly the documented order (verified above). **Do not** change the `self.hands[player]` indexing — that panic is WP-09's (e F18/F36).
- **e F23 (perspective index, nit, -2 only).** -2 render.rs:130: `let p = player.unwrap_or(0) % MAX_PLAYERS;` — the modulus is the crate maximum (3), not `self.players`. In a 2-player game `player = Some(2)` survives as `2`, so `render_tableau` renders `expeditions.get(2)` = `None` for the bottom half and the viewer sees no own-tableau at all. The verification pass correctly flagged this as the outlier: the score section **in the same function** clamps properly (`Some(p) if p < pub_state.players => p, _ => 0`, render.rs:51-54). **The finding's recommended `% self.players` is OVERTURNED:** `PubState` derives `Deserialize` with `players` defaulting to 0, so `% self.players` introduces a *new* divide-by-zero panic on a degenerate `PubState` where `% MAX_PLAYERS` was at least panic-free. Use the clamp form already present at render.rs:51-54 instead — same semantics for valid input, no new panic, and it makes the two perspective computations in one file agree. Side effect: `MAX_PLAYERS` becomes unused in render.rs (grep confirms line 130 is its only use there), so it must be dropped from the `use crate::{...}` list at render.rs:9 or the build fails `-D warnings`. **Empirically confirmed red/green:** pre-fix `pub_state().render_tableau(Some(2))` on a 2p game emits `{{player 1}}` but no `{{player 0}}`; post-fix it emits both. Leave `_ => unreachable!()` (render.rs:187) alone — WP-09.
- **e F24 (game-over log regression, nit, -2 only).** -1's `game_over_log()` (lib.rs:171-192) builds "The game is over, {player} won by N points" or "The game is over, scores tied at N". -2 (lib.rs:198-200) is a bare `"The game is over."`. -1's implementation cannot be copied: it is hardwired to two players (`[isize; 2]` scores array, `opponent(p)` for the margin, and a tie branch that assumes a single shared score). **Re-derived generalization:** use the existing `leaders()` helper (lib.rs:120-134) — it returns the `HashSet<usize>` of top scorers over `0..self.players` — sort it into a `Vec` for determinism (`HashSet` iteration order is not stable), and compute the margin against the best *non-winner* score via `(0..self.players).filter(|&p| p != winner).map(...).max()`. Using `leaders()` rather than `placings()`/a new `winners()` avoids adding a helper and reuses the same notion of "leader" the round-start order already uses (lib.rs:156). Edge cases that must not panic: `players == 0` (degenerate deserialized state) makes `leaders()` empty, which falls into the tie arm and prints "scores tied at 0"; a 3-way tie prints one "scores tied at N". `.max().unwrap_or(winner_score)` covers the impossible "single leader but no other players" case, yielding a margin of 0 rather than a panic. **Empirically verified output:** scores `[30, 15, 0]` -> `{{b}}The game is over, {{player 0}} won by 15 points{{/b}}`; scores `[10, 10, 0]` -> `{{b}}The game is over, scores tied at 10{{/b}}`. Note this log is *additional* to the separate `placings_log` the finishing `draw` already appends (lib.rs:617-623) — that duplication of "who won" is pre-existing in both crates and is not this package's business.
- **e F26 / e F41 (draw-count underflow, nit).** `let mut num = HAND_SIZE - hand.len();` (-1 lib.rs:401) / `let mut num = hand_size(self.players) - hand.len();` (-2 lib.rs:408). Unreachable through normal play — `draw_hand_full` is only called from `start_round` (hands empty) and `draw()` (phase `DrawOrTake`, so the player has already shed a card and the hand is exactly `hand_size - 1`) — but the verification pass refined the impact: **debug builds panic** with "attempt to subtract with overflow", and in release the wrap is immediately clamped by the very next `if num > dl { num = dl }` check (-1 lib.rs:403-405, -2 lib.rs:410-412), so release *drains the deck* rather than misbehaving unboundedly. `saturating_sub` is free and removes the debug panic. **Empirically confirmed:** pushing a 9th card into a hand and calling `draw_hand_full` panics at lib.rs:408 pre-fix and returns `num = 0` post-fix.
- **e F43 (`score()` guarded `unwrap()`, nit).** -1 lib.rs:680-687 does `let cards = exp_cards.get(&e); if cards.is_none() { return acc; } … cards.unwrap()`. **The finding filed this against -1 only, but -2 lib.rs:718-729 is the identical construct** (`*cards.unwrap()`, plus `exp_cost`/`exp_bonus_size` locals instead of the literals `20`/`8`). Fixing only -1 would leave the exact drift this package exists to eliminate, and -2's file is already in WP-28's declared paths (`game/lost-cities-2/src`), so Task 7 fixes both. `let`-else is the cleanest restructure and keeps the `fold`'s early-return shape. Behaviour is provably identical: the guard and the `unwrap` read the same `Option`.
- **e F44 (throwaway `Vec`s in the renderer, nit).** `by_exp.get(&e).unwrap_or(&vec![])` allocates an empty `Vec` on every lookup, at -1 render.rs:185 and 196. **Same two expressions exist in -2 at render.rs:264 and 282** — again filed against -1 only; Task 8 fixes both. The finding's `.map(Vec::as_slice).unwrap_or(&[])` works but is clumsier than what each site actually needs: line 185/264 only wants a length (`map_or(0, Vec::len)`) and line 196/282 only wants an element (`and_then(|cards| cards.get(row_i))`). Both are `Option` combinators with no allocation and no temporary-lifetime subtleties. Verified clippy-clean and all tests green with these forms.
- **e F42 (bare `2` vs `PLAYERS`, nit, -1 only).** -1 defines `const PLAYERS: usize = 2` (lib.rs:25) and uses it at lib.rs:124, 509, 634. The verification pass established the finding **undercounts**: the bare `2`s are at lib.rs:144 (`for p in 0..2` in `end_round`), 230 (`(self.current_player + 1) % 2` in `next_player`), 501 (`(player + 1) % 2` in `pub fn opponent`), 511-512 (`min: 2, max: 2` in the `PlayerCount` error), 616 (`(0..2)` in the finished-scores map), 638 (`vec![2]` in `player_counts`) and 642 (`2` in `player_count`) — seven sites, not four. Additionally render.rs:39 (`Some(p) if p < 2`) is the same defect one file over; `PLAYERS` is crate-root-private and therefore visible to the `render` child module (exactly how -2's render.rs:9 imports the equally-private `MAX_PLAYERS`), so it is included. **Deliberately left alone:** render.rs:116's `cmp::min(player.unwrap_or(0), 1)` — the `1` is a last-*index*, not a player *count*, and `PLAYERS - 1` reads worse than the literal; and lib.rs:686's `20`/`8` — expedition cost and bonus size, not player counts (a real -1-vs--2 divergence, but not this finding and not filed anywhere).
- **e F27 (deployed blurb, minor).** `k8s/base/game/lost-cities-2/game-version.yaml:9` reads `blurb: "Fund expeditions to five lost cities, … A tense two-player card game of investment and restraint."` — verbatim from -1's manifest — while -2 advertises `[2, 3]` and RULES.md:3 says "A 2-3 player card game". **The finding's recommendation ("update the blurb in lost-cities-2's manifest") is INCOMPLETE and must be extended, for a reason the finding did not see:** the operator upserts the blurb onto `game_types`, whose conflict key is the game **type name** (`rust/operator/src/controller.rs:181-196`: `INSERT INTO game_types (name, player_counts, weight, blurb) … ON CONFLICT (name) DO UPDATE SET … blurb = EXCLUDED.blurb`), and **both** manifests declare `typeName: Lost Cities` (lost-cities-1/game-version.yaml:7, lost-cities-2/game-version.yaml:7). The two `GameVersion` CRs therefore write the *same* `game_types` row, last-reconcile-wins. Editing only -2's manifest works today (the reconciler skips unchanged specs via `generation == observed_generation`, controller.rs:107-111, so -1 will not immediately clobber it) but leaves a landmine: the next -1 spec edit, or any loss of `.status.observedGeneration`, restores the stale two-player text. Both manifests get the same corrected blurb. That is also *correct* for -1: users only ever see one "Lost Cities" card, and it now truthfully covers 2-3 players. See "Cross-package / newly discovered" for the same root cause corrupting `player_counts`, which this package does **not** fix.

**Empirical validation performed while writing this spec (2026-07-25):** every non-trivial fix and every red-first assertion below was applied to a scratch copy of `lost-cities-2`, run, and then reverted (`git status` clean, confirmed). Confirmed: the F17 test fails `left: 3, right: 2` pre-fix; the F19 fix adds exactly the 2 missing log entries with no duplication; the F24 generalization produces the two quoted strings; the F23 test fails on "no player 0" pre-fix and passes post-fix; the F26 test panics "attempt to subtract with overflow" at lib.rs:408 pre-fix; the F43 `let`-else and F44 combinator rewrites compile, keep all 7 lib tests green, and pass `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`.

---

### Task 1: report finished stats for every player (e F17, major, lost-cities-2 only)

**Problem (restated):** `-2 lib.rs:534` hardcodes `stats: vec![self.player_stats(0), self.player_stats(1)]` inside `Status::Finished`. The crate supports 3 players (`player_counts()` = `[2, 3]`, lib.rs:644-646; `Game::start` accepts 3, lib.rs:505). Every finished 3-player game silently drops player 2's stats while `placings()` correctly reports all three — the surrounding code was generalized and this line was not.

**Fix (re-derived, matches the finding):** `stats: (0..self.players).map(|p| self.player_stats(p)).collect()`, mirroring `placings()`' own `(0..self.players)` (lib.rs:493) and `points()`' (lib.rs:639).

**Edge cases:**
- 2-player games produce byte-identical output — no behaviour change for the overwhelming majority of live games.
- `player_stats(p)` already returns an empty `HashMap` when `p >= self.stats.len()` (lib.rs:457-459), so a short/corrupt `stats` vec yields empty maps, never a panic. Do not add a second guard.
- Serialization: the `stats` array length becomes `self.players`. This is a `Status` response field, never a persisted column, and `web/src/game/mod.rs:37` ignores it. No migration, no back-compat shim.
- **Do not** touch `player_stats()`' body — surfacing `investments`/`expeditions` is WP-30's decision.
- -1's identical-looking line (lib.rs:531) is CORRECT (2-player-only crate) and must not change.

**Files:**
- Modify: `rust/game/lost-cities-2/src/lib.rs` (`status()`, line 534; inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add to `mod test` in `rust/game/lost-cities-2/src/lib.rs`:

```rust
    #[test]
    fn finished_status_reports_stats_for_every_player() {
        // e F17: `status()` hardcoded stats for players 0 and 1, so finished
        // 3-player games silently omitted player 2. A 3p deck is 60 cards and
        // the 3x7 opening deal leaves 39, so 39 discard+draw turns end a round
        // and 39 * ROUNDS end the game.
        let mut game = Game::start(3, 1).expect("3 players must be supported").0;
        assert_eq!(39, game.deck.len(), "3p deal must leave 39 cards");
        for _ in 0..(39 * ROUNDS) {
            let p = game.current_player;
            let c = game.hands[p][0];
            game.discard(p, c).unwrap();
            game.draw(p).unwrap();
        }
        assert!(game.is_finished());
        match game.status() {
            Status::Finished { placings, stats } => {
                assert_eq!(3, placings.len(), "placings must cover all players");
                assert_eq!(3, stats.len(), "stats must cover all players");
                for (p, s) in stats.iter().enumerate() {
                    assert!(
                        s.contains_key("Discards"),
                        "player {} has no stats: {:?}",
                        p,
                        s
                    );
                }
            }
            s => panic!("expected Finished, got {:?}", s),
        }
    }
```

- [ ] Run: `cargo test -p lost-cities-2 finished_status_reports_stats` — expected FAIL: `assertion \`left == right\` failed: stats must cover all players / left: 3 / right: 2`.
- [ ] Implement: in `rust/game/lost-cities-2/src/lib.rs`, replace line 534

```rust
                stats: vec![self.player_stats(0), self.player_stats(1)],
```

  with

```rust
                stats: (0..self.players).map(|p| self.player_stats(p)).collect(),
```

- [ ] Run: `cargo test -p lost-cities-2` — new test PASSES, all 7 lib tests + `game_contract` PASS.
- [ ] `cargo clippy -p lost-cities-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-2/src/lib.rs` ; message: `fix(lost-cities-2): report finished stats for all players (e F17, WP-28)`

---

### Task 2: keep the round's final draw logs (e F19 + e F37, minor, BOTH crates)

**Problem (restated):** `draw_hand_full` builds a public and a private draw log into a local `logs`, then discards them whole when the draw empties the deck and it tail-calls `end_round()` (-1 lib.rs:434-438, -2 lib.rs:441-445). The last draw of every round — and of the game — is missing from the log stream. State is correct; only logs are lost. Identical defect and identical surrounding code in both crates (-2 inherited it).

**Fix (re-derived; the finding's shape is right, simplified):** extend `logs` with `end_round()`'s logs and return the combined vec. One `if` with no `else` reads better than the finding's `let mut l = logs; …` dance:

```rust
        if self.deck.is_empty() {
            logs.extend(self.end_round()?);
        }
        Ok(logs)
```

**Edge cases:**
- No duplication: `end_round()` emits per-player round scores plus either `start_round()`'s logs (which include the *next* round's opening draws) or `game_over_log()`. It never re-emits the draw that triggered it. Verified empirically — the count goes 10 -> 12 and the two new entries are exactly the missing public+private draw lines.
- Chronology: draw logs first, then round-end scoring. Correct.
- The private log (`Log::private(…, vec![player])`) is restored along with the public one — that is the one the drawing player actually needed.
- `logs` is already `let mut` in both crates (-1 lib.rs:398, -2 lib.rs:405). `end_round()` returns `Result`, hence the `?`.
- `start_round()` also calls `draw_hand_full` (-1 lib.rs:127, -2 lib.rs:153). During the opening deal the deck is 44/39 cards and cannot be emptied by a deal, so no recursion change. There is no new recursion path: `draw_hand_full -> end_round -> start_round -> draw_hand_full` already existed.

**Files:**
- Modify: `rust/game/lost-cities-1/src/lib.rs` (`draw_hand_full`, lines 434-438; inline `mod test`)
- Modify: `rust/game/lost-cities-2/src/lib.rs` (`draw_hand_full`, lines 441-445; inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add the following to `mod test` in **`rust/game/lost-cities-1/src/lib.rs`** (44 draws per round for 2 players):

```rust
    #[test]
    fn final_draw_of_a_round_keeps_its_logs() {
        // e F37: draw_hand_full dropped its accumulated logs when the draw
        // emptied the deck, so the last draw of every round was invisible.
        // Pre-fix the returned logs start with the round-score log instead of
        // the draw log.
        let mut game = Game::start(2, 1).unwrap().0;
        let mut logs: Vec<Log> = vec![];
        for _ in 0..44 {
            let p = game.current_player;
            let c = game.hands[p][0];
            game.discard(p, c).unwrap();
            logs = game.draw(p).unwrap();
        }
        assert_eq!(
            START_ROUND + 1,
            game.round,
            "the 44th draw must end the round"
        );
        let text: Vec<String> = logs
            .iter()
            .map(|l| brdgme_markup::to_string(&l.content))
            .collect();
        assert!(
            text[0].contains("drew a card"),
            "the final draw's public log must come first, got: {:?}",
            text
        );
        assert!(
            logs.iter().any(|l| !l.public),
            "the final draw's private log must be present, got: {:?}",
            text
        );
    }
```

  and the same test to `mod test` in **`rust/game/lost-cities-2/src/lib.rs`** with `44` replaced by `39` and `Game::start(2, 1)` replaced by `Game::start(3, 1)` (3-player deal leaves 39 cards; using 3p here also gives the -2 suite a second 3-player path). Rename it identically in both crates.

- [ ] Run: `cargo test -p lost-cities-1 final_draw_of_a_round` and `cargo test -p lost-cities-2 final_draw_of_a_round` — expected FAIL in both on `the final draw's public log must come first` (pre-fix `text[0]` is `"{{player 0}} scored {{b}}0{{/b}} points, now on {{b}}0{{/b}}"`).
- [ ] Implement, in **both** files, replacing

```rust
        if self.deck.is_empty() {
            self.end_round()
        } else {
            Ok(logs)
        }
```

  with

```rust
        if self.deck.is_empty() {
            logs.extend(self.end_round()?);
        }
        Ok(logs)
```

- [ ] Run: `cargo test -p lost-cities-1` and `cargo test -p lost-cities-2` — new tests PASS, all 7 + 1 existing tests PASS in each crate.
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`, `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-1/src/lib.rs rust/game/lost-cities-2/src/lib.rs` ; message: `fix(lost-cities-1,lost-cities-2): keep the round's final draw logs (e F19 e F37, WP-28)`

---

### Task 3: deliver `PlayerState.hand` sorted, as documented (e F20 + e F38, minor, BOTH crates)

**Problem (restated):** the rustdoc on `PlayerState.hand` (-1 lib.rs:92, -2 lib.rs:102-103) and `DATA_DOCS.md` (-1:18, -2:19) both state "sorted by expedition then value", but `player_state()` returns the hand in acquisition order (-1 lib.rs:562-568, -2 lib.rs:566-572). -1 is the worse of the two: nothing sorts the hand at any point (`drawn.sort()` at lib.rs:411 sorts only the private-log vector). Bots, the operator UI and LLM tooling read DATA_DOCS and get an order that isn't the documented one.

**Fix (re-derived; the finding offered "sort, or fix the docs" — sorting is chosen):** sort the cloned hand in `player_state()`. `Card` derives `Ord` over `(expedition, value)` (card.rs:72) with `Expedition` in Red/Green/White/Blue/Yellow declaration order and `Investment < N(_)`, which is exactly the documented order — so `hand.sort()` satisfies the doc with no custom comparator. Sorting (rather than rewriting two doc strings per crate) is right because the renderer already sorts for display (`render_hand`, -1 render.rs:206-217, -2 render.rs:302-313), making the unsorted JSON the odd one out, and because `DATA_DOCS.md` is a published contract served verbatim by `Gamer::data_docs()`.

**Edge cases:**
- **CRITICAL — do NOT change the indexing.** The replacement must keep `self.hands[player]`. `self.hands.get(player)…` would silently fix e F18 / e F36, the request-reachable panic owned by **WP-09** (BLOCKED-ON-DECISION D-36), destroying that package's red test and pre-empting its decision. The line stays panicky on purpose.
- `PlayerState` is a per-request view (`handle_player_render`, `lib/cmd/src/requester/gamer.rs:170-181`), never persisted, so reordering a field's contents cannot invalidate stored state. No serialized type, field name or field type changes.
- Cost: one `sort()` over at most 8 elements per render.
- No doc edits: the rustdoc and DATA_DOCS text are already the *desired* behaviour in both crates. Leave all four strings untouched.
- Empty hand sorts fine.

**Files:**
- Modify: `rust/game/lost-cities-1/src/lib.rs` (`player_state`, lines 562-568; inline `mod test`)
- Modify: `rust/game/lost-cities-2/src/lib.rs` (`player_state`, lines 566-572; inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add to `mod test` in **both** crates (identical text; `Expedition`/`Value` are already imported in both test modules):

```rust
    #[test]
    fn player_state_hand_is_sorted_as_documented() {
        // e F38 / e F20: PlayerState.hand's rustdoc and DATA_DOCS.md both
        // promise "sorted by expedition then value"; player_state() returned
        // acquisition order. Card's derived Ord is (expedition, value) with
        // Red < Green < White < Blue < Yellow and Investment < N(_).
        let mut game = Game::start(2, 1).unwrap().0;
        game.hands[0] = vec![
            (Expedition::Yellow, Value::N(9)).into(),
            (Expedition::Red, Value::Investment).into(),
            (Expedition::Green, Value::N(2)).into(),
            (Expedition::Red, Value::N(4)).into(),
        ];
        let hand = game.player_state(0).hand;
        let mut expected = hand.clone();
        expected.sort();
        assert_eq!(expected, hand, "hand must be sorted");
        assert_eq!(
            vec!["RX", "R4", "G2", "Y9"],
            hand.iter().map(|c| c.to_string()).collect::<Vec<String>>()
        );
    }
```

- [ ] Run: `cargo test -p lost-cities-1 player_state_hand_is_sorted` and the same for `-p lost-cities-2` — expected FAIL in both: the returned order is `["Y9", "RX", "G2", "R4"]`.
- [ ] Implement, in **both** files, replacing the `player_state` body's field initializer

```rust
            hand: self.hands[player].clone(),
```

  with

```rust
            // Documented (and DATA_DOCS.md) contract: sorted by expedition
            // then value, which is exactly Card's derived Ord. Indexing is
            // left unchecked deliberately - the bounds fix for a crafted
            // PlayerRender is WP-09's (e F18 / e F36), not ours.
            hand: {
                let mut hand = self.hands[player].clone();
                hand.sort();
                hand
            },
```

- [ ] Run: `cargo test -p lost-cities-1` and `cargo test -p lost-cities-2` — new tests PASS, all existing tests PASS in each crate.
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`, `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-1/src/lib.rs rust/game/lost-cities-2/src/lib.rs` ; message: `fix(lost-cities-1,lost-cities-2): sort PlayerState.hand as documented (e F20 e F38, WP-28)`

---

### Task 4: restore the game-over winner announcement, generalized to 3 players (e F24, nit, lost-cities-2 only)

**Problem (restated):** -1's `game_over_log()` (lib.rs:171-192) announces "The game is over, {player} won by N points" (or "scores tied at N"); -2's (lib.rs:198-200) is a bare "The game is over." — an accidental regression during the 3-player generalization. -2 already has the helper it needs: `leaders()` (lib.rs:120-134) returns the set of top scorers over `0..self.players`.

**Fix (re-derived — -1's implementation CANNOT be copied):** -1's version is hardwired to two players (`[isize; 2]` score array, `opponent(p)` for the margin, tie branch assuming one shared score). Generalize instead: take `leaders()`, sort it into a `Vec` (a `HashSet`'s iteration order is not deterministic and this text goes into the immutable log stream), and on a single leader compute the margin against the best score among the other players.

**Edge cases:**
- `leaders()` returns a `HashSet` — **must** be sorted before matching, or the tie branch's "scores tied at N" could read a different (equal-scoring, so same text) element and, more importantly, the single-leader branch's determinism would rest on luck. Sort unconditionally.
- `self.players == 0` (degenerate deserialized `Game`): `leaders()` is empty, the slice pattern `[winner]` does not match, the tie arm prints "scores tied at 0" via `unwrap_or_default()`. No panic, no `unreachable!()`.
- 2- or 3-way tie: one "scores tied at N" line, matching -1's wording. Player names are not listed — the separate `placings_log` appended by the finishing `draw` (lib.rs:617-623) already names them.
- Single leader with no other players (impossible: `players >= 1` implies the leader exists, `players == 1` is rejected by `start`): `.max().unwrap_or(winner_score)` yields margin 0 rather than panicking on `Option::unwrap`.
- Negative scores are normal in this game (expeditions cost points), so the margin is `isize` arithmetic; `winner_score - runner_up` is always `>= 0` by construction.
- The whole line stays wrapped in `N::Bold`, as -1 does and as -2 does today.
- -1's `game_over_log()` is **not** modified. It is correct for a 2-player crate, and rewriting it would be gratuitous churn on a deprecated-but-live crate.

**Files:**
- Modify: `rust/game/lost-cities-2/src/lib.rs` (`game_over_log`, lines 198-200; inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add to `mod test` in `rust/game/lost-cities-2/src/lib.rs`:

```rust
    #[test]
    fn game_over_log_announces_the_winner() {
        // e F24: lost-cities-2 regressed to a bare "The game is over."; -1
        // announced the winner and margin. Generalized here over self.players.
        let mut g = Game::start(3, 1).unwrap().0;
        g.scores = vec![vec![10, 10, 10], vec![5, 5, 5], vec![0, 0, 0]];
        let won = brdgme_markup::to_string(&g.game_over_log().content);
        assert!(won.contains("{{player 0}}"), "winner must be named: {}", won);
        assert!(won.contains("won by 15 points"), "got: {}", won);

        g.scores = vec![vec![10], vec![10], vec![0]];
        let tied = brdgme_markup::to_string(&g.game_over_log().content);
        assert!(tied.contains("scores tied at 10"), "got: {}", tied);
    }
```

- [ ] Run: `cargo test -p lost-cities-2 game_over_log_announces` — expected FAIL: the log is `{{b}}The game is over.{{/b}}`, so `winner must be named` fires first.
- [ ] Implement: replace `game_over_log` (lib.rs:198-200) with

```rust
    fn game_over_log(&self) -> Log {
        // Sorted for determinism: leaders() returns a HashSet and this text is
        // written into the permanent log stream.
        let mut leaders: Vec<usize> = self.leaders().into_iter().collect();
        leaders.sort_unstable();
        let mut log_text = vec![N::text("The game is over, ")];
        match leaders.as_slice() {
            [winner] => {
                let winner = *winner;
                let winner_score = self.player_score(winner);
                // Margin over the best non-winner. unwrap_or covers the
                // degenerate single-player state without panicking.
                let runner_up = (0..self.players)
                    .filter(|&p| p != winner)
                    .map(|p| self.player_score(p))
                    .max()
                    .unwrap_or(winner_score);
                log_text.push(N::Player(winner));
                log_text.push(N::text(format!(
                    " won by {} points",
                    winner_score - runner_up
                )));
            }
            tied => {
                log_text.push(N::text("scores tied at "));
                log_text.push(N::text(format!(
                    "{}",
                    tied.first()
                        .map(|&p| self.player_score(p))
                        .unwrap_or_default()
                )));
            }
        }
        Log::public(vec![N::Bold(log_text)])
    }
```

- [ ] Run: `cargo test -p lost-cities-2` — new test PASSES, all existing tests PASS (`game_end_works` drives a 2-player game to completion through this function).
- [ ] `cargo clippy -p lost-cities-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-2/src/lib.rs` ; message: `fix(lost-cities-2): announce the winner in the game-over log (e F24, WP-28)`

---

### Task 5: clamp the render perspective to the real player count (e F23, nit, lost-cities-2 only)

**Problem (restated):** `-2 render.rs:130`: `let p = player.unwrap_or(0) % MAX_PLAYERS;`. The modulus is the crate maximum (3), not `self.players`, so in a 2-player game a perspective index of `2` survives as `2`: the top half renders `next_player(2, 2) == 1`'s tableau and the bottom half renders `expeditions.get(2) == None`, i.e. the viewer's own tableau vanishes. The score section of the **same file** already clamps correctly (render.rs:51-54), making this the outlier. -1's equivalent (render.rs:116, `cmp::min(player.unwrap_or(0), 1)`) is correct and is not touched.

**Fix (re-derived — the finding's `% self.players` is OVERTURNED):** `PubState` derives `Deserialize` with `players` defaulting to `0`, so `% self.players` would introduce a **new** divide-by-zero panic where `% MAX_PLAYERS` was at least arithmetically safe. Use the clamp form already present at render.rs:51-54:

```rust
        let p = match player {
            Some(p) if p < self.players => p,
            _ => 0,
        };
```

Same result for every valid input, no new panic, and the two perspective computations in one file finally agree.

**Edge cases:**
- `MAX_PLAYERS` becomes unused in render.rs (grep: line 130 is its only use there). It **must** be removed from the `use crate::{…}` list at render.rs:9 or `-D warnings` fails on `unused_imports`. `MAX_PLAYERS` stays defined in lib.rs:27 and used at lib.rs:505, 508, 645.
- `player = None` (public render) still yields `0`, unchanged.
- The `_ => unreachable!()` arm at render.rs:187 (`match self.players`) is **e F22, WP-09's** — leave it exactly as is. This task changes only the first statement of `render_tableau`.
- `render_tableau` is private to the `render` module, so its test lives in a new `#[cfg(test)] mod tests` at the bottom of render.rs (`mod tests`, matching `card.rs`; the crate's lib.rs uses `mod test` — e F9 territory, not ours).

**Files:**
- Modify: `rust/game/lost-cities-2/src/render.rs` (import line 9, `render_tableau` line 130, new test module at end of file)

**Steps:**

- [ ] Write the failing test. Append to `rust/game/lost-cities-2/src/render.rs`:

```rust
#[cfg(test)]
mod tests {
    use brdgme_game::Gamer;

    use crate::Game;

    #[test]
    fn render_tableau_clamps_perspective_to_player_count() {
        // e F23: `% MAX_PLAYERS` (3) let a perspective index of 2 through in a
        // 2-player game, so `expeditions.get(2)` was None and the bottom half
        // - the viewer's own tableau - rendered as nothing.
        let g = Game::start(2, 1).unwrap().0;
        let out = brdgme_markup::to_string(&g.pub_state().render_tableau(Some(2)));
        assert!(
            out.contains("{{player 0}}"),
            "own tableau (clamped to player 0) missing: {}",
            out
        );
        assert!(
            out.contains("{{player 1}}"),
            "opponent tableau missing: {}",
            out
        );
    }
}
```

- [ ] Run: `cargo test -p lost-cities-2 render_tableau_clamps` — expected FAIL: `own tableau (clamped to player 0) missing` (pre-fix the output contains `{{player 1}}` only).
- [ ] Implement, in `rust/game/lost-cities-2/src/render.rs`:
  1. Line 9 becomes `use crate::{END_ROUND, PlayerState, PubState, ROUNDS, START_ROUND, next_player};` (drop `MAX_PLAYERS`).
  2. Replace line 130 with the four-line `match player { Some(p) if p < self.players => p, _ => 0 }` block shown above.
- [ ] Run: `cargo test -p lost-cities-2` — new test PASSES, all existing tests PASS (`game_contract` renders both 2p and 3p games through this function).
- [ ] `cargo clippy -p lost-cities-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean. (Confirmed empirically: no `unused_imports` warning remains after step 1.)
- [ ] Commit: `git add rust/game/lost-cities-2/src/render.rs` ; message: `fix(lost-cities-2): clamp render perspective to the player count (e F23, WP-28)`

---

### Task 6: saturating draw-count arithmetic (e F26 + e F41, nit, BOTH crates)

**Problem (restated):** `let mut num = HAND_SIZE - hand.len();` (-1 lib.rs:401) / `let mut num = hand_size(self.players) - hand.len();` (-2 lib.rs:408). If a hand ever exceeds the hand size, this underflows `usize`: **debug builds panic** ("attempt to subtract with overflow"); in release the wrapped value is immediately clamped by the next `if num > dl { num = dl }` check (-1 lib.rs:403-405, -2 lib.rs:410-412), so release drains the deck instead. Unreachable through the normal turn cycle (see re-derivation notes) but free to make panic-proof.

**Fix (matches the finding):** `saturating_sub`.

**Edge cases:**
- With an over-full hand the fixed code draws 0 cards and still emits a `"drew 0 cards, N remaining"` public log plus an empty `"You drew "` private log. Cosmetically odd, **accepted**: suppressing the log would add a branch to a path unreachable in normal play, and the finding asks only for panic-free arithmetic. Do not add logging logic.
- No behaviour change on any reachable path: `num` is 8 (or 7) at round start and exactly 1 on every `draw()`.
- The `if num > dl` deck clamp below stays exactly as it is.

**Files:**
- Modify: `rust/game/lost-cities-1/src/lib.rs` (line 401; inline `mod test`)
- Modify: `rust/game/lost-cities-2/src/lib.rs` (line 408; inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add to `mod test` in **both** crates (identical text):

```rust
    #[test]
    fn draw_hand_full_does_not_underflow_on_an_oversized_hand() {
        // e F41 / e F26: `HAND_SIZE - hand.len()` panics in debug builds if a
        // hand ever exceeds the hand size. Unreachable in normal play, so this
        // constructs the state directly.
        let mut game = Game::start(2, 1).unwrap().0;
        let extra = game.deck.pop().expect("deck must not be empty");
        game.hands[0].push(extra);
        let over = game.hands[0].len();
        let logs = game
            .draw_hand_full(0)
            .expect("drawing into an over-full hand must not error");
        assert_eq!(
            over,
            game.hands[0].len(),
            "no cards may be drawn into an over-full hand"
        );
        assert!(!logs.is_empty(), "the draw attempt must still be logged");
    }
```

- [ ] Run: `cargo test -p lost-cities-1 draw_hand_full_does_not_underflow` and the same for `-p lost-cities-2` — expected FAIL in both: `panicked at … attempt to subtract with overflow` (lib.rs:401 / lib.rs:408).
- [ ] Implement:
  - `rust/game/lost-cities-1/src/lib.rs:401` -> `let mut num = HAND_SIZE.saturating_sub(hand.len());`
  - `rust/game/lost-cities-2/src/lib.rs:408` -> `let mut num = hand_size(self.players).saturating_sub(hand.len());`
- [ ] Run: `cargo test -p lost-cities-1` and `cargo test -p lost-cities-2` — new tests PASS, all existing tests PASS.
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`, `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-1/src/lib.rs rust/game/lost-cities-2/src/lib.rs` ; message: `fix(lost-cities-1,lost-cities-2): saturating draw-count arithmetic (e F26 e F41, WP-28)`

---

### Task 7: restructure `score()`'s guarded `unwrap()` (e F43, nit, BOTH crates)

**Problem (restated):** the `expeditions().iter().fold(...)` in `score()` does `let cards = exp_cards.get(&e); if cards.is_none() { return acc; } … cards.unwrap()` — an `unwrap()` that is safe only because of the early return three lines up (-1 lib.rs:680-687). **The finding filed this against -1 only; -2 lib.rs:718-729 is the identical construct** (`*cards.unwrap()`, with `exp_cost`/`exp_bonus_size` locals instead of -1's literal `20`/`8`). Fixing one and not the other re-creates exactly the drift this package exists to remove, and -2's `src` is in WP-28's declared paths.

**Fix (re-derived):** `let`-else, which preserves the `fold`'s early-return shape and removes the `unwrap` entirely. Behaviour is provably unchanged: the guard and the `unwrap` read the same `Option`.

**Edge cases:**
- `exp_cards` values are `isize` in both crates; `let Some(&cards) = …` binds an `isize` by copy, so the later comparison becomes `cards >= 8` (-1) / `cards >= exp_bonus_size` (-2) instead of the reference comparisons `cards.unwrap() >= &8` / `*cards.unwrap() >= exp_bonus_size`.
- The scoring formula itself is verified correct against the official rules by the review and is **not** touched. `score_works` (7 assertions in each crate, including the +20 eight-card bonus case) is the regression guard.
- Do not "fix" -1's magic `20`/`8` into constants — that is a real -1-vs--2 divergence but not a filed finding (noted in Cross-package).

**Files:**
- Modify: `rust/game/lost-cities-1/src/lib.rs` (`score`, lines 680-687)
- Modify: `rust/game/lost-cities-2/src/lib.rs` (`score`, lines 718-729)

**Steps:**

- [ ] No new test: the change is behaviour-preserving and `score_works` already pins the arithmetic in both crates.
- [ ] Implement in `rust/game/lost-cities-1/src/lib.rs`, replacing lines 680-687's fold body:

```rust
    expeditions().iter().fold(0, |acc, &e| {
        let Some(&cards) = exp_cards.get(&e) else {
            return acc;
        };
        acc + (exp_sum.get(&e).unwrap_or(&0) - 20) * (exp_inv.get(&e).unwrap_or(&0) + 1)
            + if cards >= 8 { 20 } else { 0 }
    })
```

- [ ] Implement in `rust/game/lost-cities-2/src/lib.rs`, replacing lines 718-729's fold body:

```rust
    expeditions().iter().fold(0, |acc, &e| {
        let Some(&cards) = exp_cards.get(&e) else {
            return acc;
        };
        acc + (exp_sum.get(&e).unwrap_or(&0) - exp_cost) * (exp_inv.get(&e).unwrap_or(&0) + 1)
            + if cards >= exp_bonus_size { exp_cost } else { 0 }
    })
```

- [ ] Run: `cargo test -p lost-cities-1` and `cargo test -p lost-cities-2` — all existing tests PASS, `score_works` in particular (it asserts 0, -17, -34, -30, -37 and the 44-point eight-card bonus case).
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`, `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-1/src/lib.rs rust/game/lost-cities-2/src/lib.rs` ; message: `refactor(lost-cities-1,lost-cities-2): drop guarded unwrap in score() (e F43, WP-28)`

---

### Task 8: stop allocating throwaway `Vec`s in the renderer (e F44, nit, BOTH crates)

**Problem (restated):** `render_tableau_cards` calls `by_exp.get(&e).unwrap_or(&vec![])` twice, allocating a temporary empty `Vec` on every miss — once per expedition in the height scan and once per expedition per row in the cell loop. -1 render.rs:185 and 196. **The same two expressions exist in -2 at render.rs:264 and 282** (also filed against -1 only).

**Fix (re-derived; the finding's `.map(Vec::as_slice).unwrap_or(&[])` is ADJUSTED to what each site actually needs):**
- Height scan: `by_exp.get(&e).map_or(0, Vec::len)` — the site only wants a length.
- Cell lookup: `by_exp.get(&e).and_then(|cards| cards.get(row_i))` — the site only wants one element.

Both are allocation-free `Option` combinators and neither has a temporary-lifetime subtlety.

**Edge cases:**
- Behaviour is identical: a missing expedition key contributed length 0 and `None` before, and does so now. (In practice `by_expedition` (card.rs:93-99) inserts all five expeditions unconditionally, so the `unwrap_or` never even fired — the allocation was pure waste.)
- `largest` starts at 1 in both crates and `cmp::max` is unchanged, so a card-less tableau still renders its one header row.
- Covered by `game_contract`, which renders every advertised player count in each crate. No new test — nothing observable to assert.

**Files:**
- Modify: `rust/game/lost-cities-1/src/render.rs` (lines 185, 196)
- Modify: `rust/game/lost-cities-2/src/render.rs` (lines 264, 282)

**Steps:**

- [ ] In `rust/game/lost-cities-1/src/render.rs`:
  - line 185: `largest = cmp::max(largest, by_exp.get(&e).map_or(0, Vec::len));`
  - line 196: `match by_exp.get(&e).and_then(|cards| cards.get(row_i)) {`
- [ ] In `rust/game/lost-cities-2/src/render.rs`: the identical two replacements at lines 264 and 282 (same before/after text).
- [ ] Run: `cargo test -p lost-cities-1` and `cargo test -p lost-cities-2` — all existing tests PASS (plus Task 5's new render test in -2).
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`, `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-1/src/render.rs rust/game/lost-cities-2/src/render.rs` ; message: `refactor(lost-cities-1,lost-cities-2): drop throwaway Vecs in the renderer (e F44, WP-28)`

---

### Task 9: use the `PLAYERS` const instead of bare `2` (e F42, nit, lost-cities-1 only)

**Problem (restated):** -1 defines `const PLAYERS: usize = 2` (lib.rs:25) and uses it at lib.rs:124, 509, 634, but seven other sites hardcode `2` — the verification pass established the finding's list of four was an undercount. Harmless while the crate is 2-player-only; a trap for anyone reading or refactoring it (and precisely how -2 inherited a hardcoded `stats: vec![p0, p1]`, i.e. e F17).

**Fix:** replace the literals with `PLAYERS`. Purely mechanical, zero behaviour change.

**Edge cases:**
- All seven sites, verified live: lib.rs:144 `for p in 0..2` -> `0..PLAYERS`; lib.rs:230 `(self.current_player + 1) % 2` -> `% PLAYERS`; lib.rs:501 `pub fn opponent`'s `(player + 1) % 2` -> `% PLAYERS`; lib.rs:511-512 `min: 2, max: 2` -> `min: PLAYERS, max: PLAYERS`; lib.rs:616 `(0..2)` -> `(0..PLAYERS)`; lib.rs:638 `vec![2]` -> `vec![PLAYERS]`; lib.rs:642 `2` -> `PLAYERS`.
- Plus render.rs:39 `Some(p) if p < 2` -> `Some(p) if p < crate::PLAYERS`. `PLAYERS` is crate-root-private, which makes it visible to the `render` child module — the same arrangement by which -2's render.rs:9 imports the equally-private `MAX_PLAYERS`. Import it (`use crate::PLAYERS;` or extend the existing `use super::{…}` at render.rs:3) rather than path-qualifying, to match the file's style.
- **Deliberately NOT changed:** render.rs:116's `cmp::min(player.unwrap_or(0), 1)` — that `1` is a last-*index*, not a player count, and `PLAYERS - 1` reads worse than the literal. And lib.rs:686's `20`/`8` — expedition cost and bonus size, not player counts; a genuine divergence from -2 (which uses named constants) but not a filed finding, so out of scope.
- `GameError::PlayerCount { min, max, given }`'s fields are `usize`, so `min: PLAYERS` type-checks directly.
- No new test: `player_counts_works` does not exist in this crate, but `game_contract` asserts `player_counts()` is non-empty and that an unadvertised count is rejected — which covers lib.rs:509-514 and 638 — and `start_works`/`end_round_works` cover the rest.

**Files:**
- Modify: `rust/game/lost-cities-1/src/lib.rs` (lines 144, 230, 501, 511-512, 616, 638, 642)
- Modify: `rust/game/lost-cities-1/src/render.rs` (line 3 import, line 39)

**Steps:**

- [ ] Apply the seven lib.rs replacements listed above, one at a time, re-reading each line before editing (several `2`s in this file are NOT player counts — the round/score literals and the `% 2` inside `opponent` are, the `8`/`20` in `score()` are not).
- [ ] Extend render.rs's import at line 3 to include `PLAYERS` and change line 39 to `Some(p) if p < PLAYERS => p,`.
- [ ] Run: `cargo test -p lost-cities-1` — all existing tests PASS (this is a no-op refactor; any failure means a wrong `2` was replaced).
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/lost-cities-1/src/lib.rs rust/game/lost-cities-1/src/render.rs` ; message: `refactor(lost-cities-1): use the PLAYERS const instead of bare 2 (e F42, WP-28)`

---

### Task 10: correct the deployed "two-player" blurb (e F27, minor, BOTH manifests) + final gate

**Problem (restated):** `k8s/base/game/lost-cities-2/game-version.yaml:9` advertises "A tense two-player card game of investment and restraint" — copied verbatim from -1's manifest — while lost-cities-2 offers `player_counts() = [2, 3]` (lib.rs:644-646) and RULES.md:3 documents "A 2-3 player card game". Players browsing the new-game page are told the game is 2-player-only.

**Fix (re-derived — the finding's single-file recommendation is EXTENDED to both manifests):** the operator upserts the blurb onto the `game_types` row keyed by **type name** (`rust/operator/src/controller.rs:181-196`, `ON CONFLICT (name) DO UPDATE SET … blurb = EXCLUDED.blurb`), and **both** `GameVersion` CRs declare `typeName: Lost Cities` (lost-cities-1/game-version.yaml:7, lost-cities-2/game-version.yaml:7). They therefore write the same row, last-reconcile-wins. Editing only -2's manifest happens to work right now — the reconciler skips specs whose `generation` matches `observedGeneration` (controller.rs:107-111), so -1 will not immediately overwrite it — but any future -1 spec edit, or any loss of `.status.observedGeneration`, restores the stale text. Give both manifests the same corrected blurb. It is also *correct* for -1: users see a single "Lost Cities" card, and the game type does now support 2-3 players.

**Edge cases:**
- Only the `blurb` value changes. Do **not** touch `typeName`, `weight`, `interfaceVersion`, or -1's `isDeprecated: true`.
- Keep the two blurbs byte-identical, so whichever CR reconciles last produces the same row.
- Keep it one line of YAML in double quotes, as both files have today (the CRD field is a plain string; no folding).
- **No `kubectl`, no `argocd`, no deploy.** This task edits YAML in git only. Rolling it out is a separate human-initiated action.
- No Rust change, so no crate test is affected; run both suites anyway as the package's final gate.

**Files:**
- Modify: `k8s/base/game/lost-cities-2/game-version.yaml` (line 9)
- Modify: `k8s/base/game/lost-cities-1/game-version.yaml` (line 9)

**Steps:**

- [ ] In **both** files, replace line 9 with the identical:

```yaml
  blurb: "Fund expeditions to five lost cities, committing cards to routes that must pay off before the deck runs out. A tense card game of investment and restraint for 2-3 players."
```

- [ ] Verify they match: `diff <(sed -n 9p k8s/base/game/lost-cities-1/game-version.yaml) <(sed -n 9p k8s/base/game/lost-cities-2/game-version.yaml)` — must be empty.
- [ ] Run: `cargo test -p lost-cities-1` and `cargo test -p lost-cities-2` — full suites PASS (unrelated, but this is the package's last commit).
- [ ] `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`, `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`, `cargo fmt --all -- --check` clean.
- [ ] Run the full pre-commit gate before this final commit: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass.
- [ ] Commit: `git add k8s/base/game/lost-cities-1/game-version.yaml k8s/base/game/lost-cities-2/game-version.yaml` ; message: `fix(k8s): Lost Cities blurb says 2-3 players (e F27, WP-28)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| e F17 finished stats hardcoded to players 0/1 (-2) | major | `stats: (0..self.players).map(\|p\| self.player_stats(p)).collect()` | **CONFIRMED** | Re-read lib.rs:534 and 644-646: 3p games are reachable and lose player 2's stats while `placings()` is generalized. Rec applied verbatim. Shape change (array length 2 -> `players`) traced as safe: `stats` is a response field, not a stored column, and `web/src/game/mod.rs:37` ignores it (Task 1). |
| e F19 draw logs dropped on deck-empty (-2) | minor | `if self.deck.is_empty() { let mut l = logs; l.extend(self.end_round()?); Ok(l) } else { Ok(logs) }` | **CONFIRMED (form simplified)** | Defect and mechanism confirmed at lib.rs:441-445; verified `end_round()` cannot duplicate the draw log. Written as `if … { logs.extend(self.end_round()?); } Ok(logs)` — same semantics, no shadowing (Task 2). |
| e F20 `PlayerState.hand` documented sorted (-2) | minor | Sort in `player_state()` (`hand.sort()`), or fix the docs | **CONFIRMED (sort chosen)** | `Card`'s derived `Ord` is exactly "expedition then value" (card.rs:6-19, 72), the renderer already sorts, and DATA_DOCS is a published contract — so the code moves, not the doc. Kept `self.hands[player]` indexing to avoid discharging WP-09's e F18 (Task 3). |
| e F23 perspective `% MAX_PLAYERS` (-2) | nit | `% self.players` | **OVERTURNED** | `PubState.players` defaults to 0 on deserialize, so `% self.players` adds a divide-by-zero panic where `% MAX_PLAYERS` was arithmetically safe. Used the clamp form already correct at render.rs:51-54 instead. Also requires dropping `MAX_PLAYERS` from render.rs:9's import or `-D warnings` fails — an effect the rec did not mention (Task 5). |
| e F24 game-over log regressed vs -1 (-2) | nit | Generalize -1's winner log, or accept the regression | **CONFIRMED (implementation re-derived)** | Fixed, not accepted. -1's version is uncopyable (2-element score array, `opponent()`, single-score tie branch); re-derived over `leaders()` with a sorted `Vec` for log determinism and a `.max()`-based margin, and made non-panicking for `players == 0` (Task 4). |
| e F26 draw-count usize underflow (-2) | nit | `hand_size(self.players).saturating_sub(hand.len())` | **CONFIRMED** | Applied verbatim; reproduced the debug panic at lib.rs:408 with an over-full hand (Task 6). |
| e F27 k8s blurb still says two-player | minor | Update the blurb in `k8s/base/game/lost-cities-2/game-version.yaml` | **ADJUSTED (extended)** | Rec is right but incomplete: `game_types` is keyed on `typeName`, which both manifests set to "Lost Cities" (controller.rs:181-196), so the two CRs write the same row last-writer-wins. Both blurbs updated to identical text. Verification's line correction (`:9`, not `:1`) confirmed (Task 10). |
| e F37 draw logs dropped on deck-empty (-1) | major -> **minor** (verification) | Extend the draw logs with `end_round()`'s and return the combined vec | **CONFIRMED** | Byte-for-byte the same defect as e F19 at lib.rs:434-438; fixed in the same task and commit as -2 so the two cannot drift again (Task 2). |
| e F38 `PlayerState.hand` never sorted (-1) | minor | Sort in `player_state()`, or fix the two doc strings | **CONFIRMED** | Verification's strengthening reconfirmed: `drawn.sort()` (lib.rs:411) sorts only the private-log vector, so -1's hand is never sorted at all. Same fix as e F20, same commit (Task 3). |
| e F41 `HAND_SIZE - hand.len()` underflow (-1) | nit | `HAND_SIZE.saturating_sub(hand.len())` | **CONFIRMED** | Applied verbatim; verification's refinement (release wrap is immediately re-clamped by the `num > dl` check at lib.rs:403-405) re-verified and recorded (Task 6). |
| e F42 bare `2` vs `PLAYERS` const (-1) | nit | Replace the literals at lib.rs:144, 230, 501, 616 with `PLAYERS` | **ADJUSTED (widened)** | Verification's undercount confirmed: also lib.rs:511-512, 638, 642, plus render.rs:39 one file over (`PLAYERS` is crate-visible). Seven lib.rs sites + one render.rs site. Explicitly excluded render.rs:116's `min(.., 1)` (a last-index, not a count) and lib.rs:686's `20`/`8` (cost/bonus size, not a filed finding) (Task 9). |
| e F43 `score()` `is_none()`-guarded `unwrap()` (-1) | nit | Restructure with `if let Some(&cards) = exp_cards.get(&e)` | **ADJUSTED (widened to both crates)** | Rec's direction taken as `let`-else (keeps the `fold`'s early return). Widened because **-2 lib.rs:718-729 has the identical construct** and the review filed it only against -1 — fixing one crate is exactly the drift WP-28 exists to prevent (Task 7). |
| e F44 throwaway empty `Vec`s in renderer (-1) | nit | `.map(Vec::as_slice).unwrap_or(&[])` | **ADJUSTED (widened + form changed)** | Widened to -2 render.rs:264/282, the same two expressions. Form changed to `map_or(0, Vec::len)` (height scan wants a length) and `and_then(\|cards\| cards.get(row_i))` (cell loop wants an element) — simpler than slice-coercion and equally allocation-free (Task 8). |
| e F18 / e F36 `player_state()` unchecked index | major | `hands.get(player).cloned().unwrap_or_default()`, or bounds-check in the requester | **OUT OF SCOPE** | Owned by WP-09 (D-36). Task 3 edits the same line and deliberately preserves the panicking `self.hands[player]` form. |
| e F21 / e F39 / e F40 `Stats` dead & mis-named fields | minor | Increment/surface, or remove the fields | **OUT OF SCOPE** | Owned by WP-30 (D-29/D-40). No `Stats` field or increment is touched; Task 1 changes only which players `player_stats()` is called for. |
| e F22 `unreachable!()` outside players 2..=3 | minor | Return `GameError::internal` / clamp / document | **OUT OF SCOPE** | Owned by WP-09 (D-36). Task 5 leaves `render.rs:187`'s arm and lib.rs:673-695 untouched. |
| e F25 discard piles expose top card only | nit | Expose full ordered discard lists, or document the deviation | **OUT OF SCOPE** | Owned by WP-30. `PubState.discards` shape unchanged in both crates. |
| e F28 stale `build-release` / `.rls.toml` (-2) | nit | Delete both; trim `.gitignore` | **OUT OF SCOPE** | Owned by the dependency-hygiene package (work-packages.md:502-503). |
| e F13 / e F14 epilogue dedup | nit | — | **OUT OF SCOPE** | age-of-war-2, owned by WP-08; neither lost-cities crate is in that scope list. |

## Test plan summary

| Crate | Baseline (live, measured) | New tests | Location |
|---|---|---|---|
| lost-cities-1 | 7 lib + 1 integration, all green | `final_draw_of_a_round_keeps_its_logs`, `player_state_hand_is_sorted_as_documented`, `draw_hand_full_does_not_underflow_on_an_oversized_hand` | inline `mod test`, `src/lib.rs` |
| lost-cities-2 | 7 lib + 1 integration, all green | `finished_status_reports_stats_for_every_player`, `final_draw_of_a_round_keeps_its_logs`, `player_state_hand_is_sorted_as_documented`, `game_over_log_announces_the_winner`, `draw_hand_full_does_not_underflow_on_an_oversized_hand` | inline `mod test`, `src/lib.rs` |
| lost-cities-2 | — | `render_tableau_clamps_perspective_to_player_count` | **new** `#[cfg(test)] mod tests` at the end of `src/render.rs` |

- Final expected counts: `cargo test -p lost-cities-1` = **10 lib tests + 1 integration** (7 baseline + 3 new in `mod test`). `cargo test -p lost-cities-2` = **13 lib tests + 1 integration** (7 baseline + 5 new in lib.rs's `mod test` + 1 new in render.rs's `mod tests`).
- Red-first is required and achievable for Tasks 1, 2, 3, 4, 5, 6 — each has a stated, empirically observed failure mode. Tasks 7, 8, 9 are behaviour-preserving refactors with no observable defect; they rely on the existing suite (`score_works` for Task 7, `game_contract`'s per-player-count renders for Task 8, the whole suite for Task 9) and add no tests. Task 10 is YAML.
- Run commands (from `/home/beefsack/Development/brdgme/rust`, per-crate only — never workspace-wide):
  - `cargo test -p lost-cities-1`
  - `cargo test -p lost-cities-2`
  - `cargo clippy -p lost-cities-1 --all-targets -- -D warnings`
  - `cargo clippy -p lost-cities-2 --all-targets -- -D warnings`
  - `cargo fmt --all -- --check`
  - Final package gate: `/home/beefsack/Development/brdgme/scripts/rust-test.sh`
- Test-module naming: use the crate's existing `mod test` in `lib.rs` (do not rename it — e F9 is a workspace-wide naming question, out of scope) and `mod tests` for the new render.rs module, matching `card.rs` in the same crate.

## Cross-package coordination points

- **WP-09 (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36)** owns e F18/e F36 (`player_state()` unchecked index) and e F22 (`unreachable!()` on out-of-range player counts) for both crates. **Task 3 edits the exact line WP-09 must fix** and preserves the `self.hands[player]` panic on purpose; whoever executes WP-09 will replace the indexing inside the block Task 3 introduces — a one-line rebase. Task 5 similarly leaves `render.rs:187`'s `_ => unreachable!()` in place. If WP-09 lands first, Task 3 must adapt to whatever accessor it introduced and keep the `sort()`.
- **WP-30 (batch-e rules and stats adjudication, BLOCKED-ON-DECISION D-29/D-40)** owns e F21/e F39/e F40 (`Stats.investments`/`expeditions`) and e F25 (discard-pile visibility) in both crates. Task 1 touches `status()`'s call site but never `player_stats()`' body or any `Stats` field, so WP-30 can add/remove fields or start surfacing them without conflict. If WP-30 decides to *remove* `Stats` fields, that is a serialized-`Game` change requiring `#[serde(default)]` care — unaffected by anything here.
- **Dependency-hygiene package (work-packages.md:502-503)** owns e F28 and will delete `game/lost-cities-2/{build-release,.rls.toml}` and trim `.gitignore`. No file overlap with this package.
- **work-packages.md:555 package** owns e F45/e F46 (binary-only deps, port-80 default). This package makes **zero** `Cargo.toml` and `src/bin/` edits, so there is no interaction — note in particular that e F45's `[dev-dependencies]` recommendation is already flagged invalid by verification.
- **Anything touching `rust/operator/src/controller.rs` or `k8s/base/game/*/game-version.yaml`** must coordinate with Task 10 and read the newly-discovered defect below.

## Cross-package / newly discovered (NOT fixed here — report to the orchestrator)

1. **`game_types` is a shared, last-writer-wins row across game versions of the same type — `player_counts` is silently wrong for Lost Cities.** `upsert_game_type_and_version` (`rust/operator/src/controller.rs:181-196`) upserts `game_types (name, player_counts, weight, blurb)` with `ON CONFLICT (name)`, where `name` is the CR's `typeName`. `lost-cities-1` and `lost-cities-2` both declare `typeName: Lost Cities`, so both write that row — including `player_counts`, which the operator fetches per-version from the game service (controller.rs:117-128). -1 reports `[2]`, -2 reports `[2, 3]`, so the stored `game_types.player_counts` is whichever version reconciled last. The new-game page reads it (`web/src/db.rs:296`, `web/src/new_game.rs`), so **Lost Cities may offer only 2 players even though lost-cities-2 supports 3** — the exact user-visible symptom e F27 describes, from a second, deeper cause that the blurb fix does not address. The blurb has the same nondeterminism, which is why Task 10 writes both manifests. **Owner: WP-62 (operator, READY — paths `operator/src/{controller.rs,crd.rs}`), routed there by the unit-3 Lead.** WP-62's spec writer must treat this as an added item beyond its bo F18-F25 scope: it is a new defect discovered during WP-28 spec-writing, it lives in exactly WP-62's file, and it needs the same design judgement as the rest of that package. A correct fix is probably to derive `game_types.player_counts` (and `blurb`, `weight`) from the non-deprecated versions only, or to union the counts across versions of a type — a design decision, not a mechanical fix. Not in WP-28's scope list, not attempted here, and no test was written that would bake in the current behaviour.
2. **-1's `command()` flattens parser errors, -2's preserves them.** `-1 lib.rs:625` is `Err(e) => Err(GameError::invalid_input(e.to_string()))`; `-2 lib.rs:630` is `Err(e) => Err(e)`. -1 therefore reports every parse failure as `InvalidInput`, losing e.g. `NotYourTurn`-style kinds and turning what could be a structured error into a string. Not a filed finding in batch e; -2's form is the better one. **Owner: whoever does a batch-e follow-up or a lost-cities-1 cleanup pass.** Not fixed here (it is a behaviour change to a deprecated-but-live crate with no finding backing it).
3. **-1's `score()` hardcodes the expedition cost (`20`) and bonus size (`8`)** where -2 uses `expedition_cost()` / `expedition_bonus_size()` (lib.rs:686 vs -2's named constants at lib.rs:32-35). Cosmetic divergence, no filed finding, deliberately left alone by Task 7. Same class as e F42 but in a different dimension (rules constants, not player counts).
4. **Both crates append a `placings_log` on the finishing `draw` in addition to `game_over_log()`** (-1 lib.rs:613-618, -2 lib.rs:617-623), so a finished game emits "who won" twice with different wording. Pre-existing in both, similar in kind to WP-08's epilogue-duplication class but neither crate is in WP-08's scope list. Task 4 makes -2's second line informative rather than removing the redundancy. **Owner: WP-08's follow-up, if the sweep is ever widened.**
