# WP-21: cathedral-2 + sushizock-2 fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Stop the traffic-driven `Box::leak` memory leak in cathedral-2's location parser (c F22, which also retires the dead-`Display` finding c F28), reject out-of-range player indices at cathedral-2's `Gamer` boundary instead of panicking the shared game process (c F25), close the latent `Loc::to_key` overflow in `Game::tile_at` (c F26), delete the vestigial `parse_loc` (c F24), document the intended cathedral-through-flood-fill behaviour at the walk site (c F23 — **comment only**, verification refuted the premise), drop the unused `rand` dependency (c F27); and in sushizock-2: reject `n = i32::MIN` in the steal path before it overflows `len as i32 - n` (d/c F29), emit the missing placings log when the game ends inside the roll path (c F30), remove the `.unwrap()` from `roll_dice` (c F32), and de-duplicate `take_worst`'s hand-rolled min loops plus the `take_*`/`steal_*` colour pairs (c F33, c F34).

**Architecture — how these two crates work (read this before editing):**

`rust/game/cathedral-2` (package `cathedral-2`, lib `cathedral_2`, edition 2024, `Cargo.toml:2,6`). A faithful port of `brdgme-go/cathedral_1`: 2-player, zero randomness (`start` ignores `seed`, lib.rs:419), no hidden information.

- `src/lib.rs` (1275 lines): `Game` (serde-persisted; `players`, `board: HashMap<String, Tile>` keyed by `Loc::to_key()`, `played_pieces: Vec<Vec<bool>>`, `current_player`, `no_open_tiles`, `finished` — lib.rs:30-41), `PubState` (lib.rs:43-57), `PlayerState { public, player }` (lib.rs:62-68), the `impl Game` core (`tile_at` :85, `loc_filter_matches` :92, `can_play` :103, `can_play_piece` :118, `play` :158, `check_captures` :251, `next_player` :335, `remaining_piece_size` :343, `can_play_something` :355, `whose_turn_players` :382, `calc_placings` :393), the `Gamer` impl (:413-546), and an inline **`#[cfg(test)] mod test`** (lib.rs:548-1275; note the module is named `test`, singular — not `tests`).
- `src/command.rs` (119 lines): `Command::Play { piece, loc, dir }`, the private `LocChoice`/`DirChoice` wrapper structs, `Game::command_parser` (:52), `play_parser` (:61), `piece_parser` (:92), `loc_parser` (:98), `dir_parser` (:110). **No test module today.** Glob-imports `brdgme_game::command::parser::*` at command.rs:4.
- `src/loc.rs` (221 lines): `Dir` bit-flags + `ORTHO_DIRS`/`DIAG_DIRS`/`dirs()`, `ortho_dir_name` (:35, panics on non-ortho), `Loc { x: i32, y: i32 }` with `valid()` (:108, `0..=9` on both axes), `to_key()` (:113), `impl Display for Loc` (:118, forwards to `to_key`), `all_locs()` (:149, 100 locs row-major so `all_locs()[0] == Loc::new(0,0)` -> `"A1"` and `[99] == Loc::new(9,9)` -> `"J10"`), `locs_by_row()` (:160), `parse_loc` (:167), `walk` (:190). No test module.
- `src/piece.rs` (112 lines): `Piece { player_type, positions, directional }`, the two private catalogues `player_0_pieces()` (:58, 14 pieces) and `player_1_pieces()` (:79, 15 pieces, index 0 is the Cathedral), and `pub fn pieces(player: i32) -> Vec<Piece>` (:106) whose `_` arm is `panic!("invalid player: {}", player)` (:110). No test module.
- `src/render.rs` (434 lines): private `Tiler` trait (:33), `impl Tiler for HashMap<String, Tile>` **with an explicit `loc.valid()` guard** (:37-50 — added after a real render panic, see the comment at :39-44), `wall_char` (:78, panics on dir 0), `render_board` (:274), `render_player_remaining_tiles` (:358, calls `piece::pieces(p_num as i32)` at :359 and indexes `state.played_pieces[p_num][i]` at :366), `player_render` (:396), `Renderer` impls for `PubState`/`PlayerState` (:424-434).
- `src/tile.rs` (50 lines): `NO_PLAYER = -1`, `PLAYER_CATHEDRAL = 2`, `Tile`, `empty_tile()` (`{player: -1, typ: 0, owner: -1, text: ""}`).
- `tests/contract.rs`: `assert_gamer_contract::<Game>()`. `src/bin/`: the four standard boilerplate bins (cli/fuzz/http/repl), none of which references `rand` or any symbol this package changes.
- Four Go defects are deliberately preserved verbatim and documented in code (piece-index `>` vs `>=` bounds check lib.rs:112-117; cathedral placement not advancing the turn lib.rs:241-243; cathedral never returned to a hand; captured pieces replayable) plus the `walk` double-visit quirk (loc.rs:203-209). **None of them is in scope.**

`rust/game/sushizock-2` (package `sushizock-2`, lib `sushizock_2`, edition 2024). Port of `brdgme-go/sushizock_1`: 2-5 players, seeded `GameRng` stored in `Game` (lib.rs:79-80 with a `#[serde(default = "GameRng::from_entropy")]` migration shim).

- `src/lib.rs` (1739 lines): `DieFace`, the 6-entry `DIE_FACES` const (:34), `TileType`, `Tile { kind, value }`, `Game` (:66-81), `PubState` (:83-107), `PlayerState` (:109-115), `DiceCounts` (:117), free fns `dice_counts` (:131), `all_dice` (:144), `roll_dice` (:150, `*DIE_FACES.choose(rng).unwrap()`), `blue_tiles`/`red_tiles` deck data (:154/:207), `score` (:260); `impl Game` (`player_score` :271, `is_finished` :282 — inherent, shadows the trait default, `can_roll` :294, `can_take_*` :298-316, `can_steal_*` :326-352, `start_turn` :354, `next_player` :365, `log_game_end` :373, `take_blue` :399, `take_red` :417, `steal_blue` :433, `steal_red` :475, `steal_log` :517, `take_worst` :527, `roll_dice_cmd` :568, `placings` :618); the `Gamer` impl (:626-802) whose `command` (:695) has three arms — Roll (:711-722), Take (:723-743), Steal (:744-764); and an inline **`#[cfg(test)] mod test`** (:804-1739, 33 tests, again named `test` singular). Test-local constants `MICK = 0`, `STEVE = 1`, `BJ = 2` and `fn names()` (3 names) live at :808-814.
- `src/render.rs` (196 lines) and `src/command.rs` (98 lines: `Command::{Roll, Take, Steal}`, `Game::command_parser` :18, `roll_parser` :40, `steal_parser` :54 whose tile-index arm is `Int::any()` at :75, `take_parser` :86).
- `tests/contract.rs`: `assert_gamer_contract::<Game>()`.

**Serialization contract (both crates):** `Game`, `PubState` and `PlayerState` round-trip through the DB as serde JSON, and live saved games must keep deserializing. **No task in this package changes any serialized type, field name, field type or field order.** Every change below is to function signatures, function bodies, comments, or `Cargo.toml`.

**Tech Stack:** Rust 1.97.0, edition 2024, workspace at `/home/beefsack/Development/brdgme/rust` (`rust-toolchain.toml` pins the channel plus rustfmt/clippy). Two crates touched: `cathedral-2`, `sushizock-2`. `let ... else`, let-chains and `Option::map_or` are all available and already used in these crates.

**Global Constraints:**

- Run all commands from `/home/beefsack/Development/brdgme/rust`. **Per-crate only**: `cargo test -p cathedral-2`, `cargo test -p sushizock-2`. NEVER workspace-wide `cargo build`/`check`/`test` (AGENTS.md "Resource constraints": a workspace build links ~30 binaries and spikes RAM/disk).
- Each task ends with `cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- Every existing test must keep passing **unmodified**, except the two cathedral test-file edits that are forced by a signature change and are called out explicitly in Task 3 (lib.rs:1037-1038). cathedral-2 has 22 inline tests plus `game_contract`; sushizock-2 has 33 inline tests plus `game_contract`.
- `assert_gamer_contract` (`rust/lib/cmd/src/test_support.rs:20-152`) asserts, for **player 0**, that `Request::Play` with garbage input returns `Response::UserError` and **never** `SystemError`. Task 3 introduces a `GameError::Internal` return (-> `SystemError`) but only for players **outside** `0..players`, so the contract test is unaffected. Do not widen that check to valid players.
- Line numbers below are LIVE-file numbers as of the drift check. Tasks that shift numbering say so; later tasks locate by symbol name.
- Run the full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` before the **final** commit of the package (it provisions the throwaway Postgres/NATS containers; DB-backed web test failures in a bare local run without it are pre-existing, backlog #40 — not a regression).

**Non-Goals (owned elsewhere — do NOT absorb):**

- **c F31** (sushizock `roll`'s bounded `Many` offering dice numbers past the legal count) — cross-reference only. The defect is `Spec::Many` losing `min`/`max` in the suggest engine (`rust/lib/game/src/command/suggest.rs:109`), tracked as **lg F9** and owned by **WP-03** (lib-game parser mechanical). Do NOT touch `roll_parser` (`sushizock-2/src/command.rs:40-52`) or `suggest.rs`; c F31 discharges when WP-03 lands.
- **Deserialized/foreign-state trust hardening** — owned by **WP-09** (BLOCKED-ON-DECISION D-36). In scope here is only the *player-index* boundary (c F25). Out of scope: cathedral's `self.played_pieces[player][i]` indexing surviving a truncated `played_pieces` row, `render_player_remaining_tiles`' `state.played_pieces[p_num][i]`, and sushizock's `player_blue_tiles[target]`/`[player]` indexing for a state whose per-player vectors are shorter than `players`.
- **Manifest / workspace hygiene sweeps** — owned by **WP-64/WP-65**. Task 6 removes exactly one line (`rand` from `cathedral-2/Cargo.toml:14`, c F27's own recommendation) and nothing else. Do not reorder, retemplate or prune any other dependency in either `Cargo.toml`, and do not touch `sushizock-2/Cargo.toml` (its `rand` **is** used, at `sushizock-2/src/lib.rs:15,151,647-648`).
- **Rules adjudication** — cathedral's player count, the missing "cathedral must be placed in the central area" edition restriction, and sushizock's scoring/steal semantics are all verified-faithful or belong to a rules package. No RULES.md change in this package.
- **The four preserved Go defects and the `walk` double-visit quirk** in cathedral-2 — deliberately verbatim, already commented. Leave them.
- **`ortho_dir_name` (loc.rs:41) and `wall_char` (render.rs:85) invariant panics** — c F25 names them and rates them acceptable as-is; re-derived below and confirmed unreachable. **Skipped by decision, no code change.**
- **c F23's flood-fill behaviour itself.** Verification refuted the premise. Task 5 lands a comment and nothing else. Do NOT add `t.player == PLAYER_CATHEDRAL` to the walk block condition at lib.rs:283.
- **The newly-discovered sushizock `target` bound defect** described under "Cross-package / newly discovered" — NOT in this package's scope. Do not add that guard unless the Lead rules it in.

**Snapshot drift:** None, for both crates. `diff -ru /home/beefsack/Development/brdgme-review-snapshot/rust/game/cathedral-2 /home/beefsack/Development/brdgme/rust/game/cathedral-2` and the same for `sushizock-2` both produce empty output and exit 0 (verified 2026-07-25 against snapshot commit `f8763a5`). All line numbers below are live-file numbers and match the findings' citations.

**Prior art:** a stray subagent produced partial cathedral fixes during the read-only review phase; they were captured to `docs/reviews/2026-07-23-rust-review/planning/raw/cathedral-stray-edits.diff` and **reverted** (the tree contains none of them). Edit A is adopted as-is (Task 1), Edit B is adopted as-is with a corrected rationale (Task 2), Edit C is **rejected as insufficient** (Task 3 replaces it). Every change below is re-derived from live source; the diff is reference only.

**Re-derivation notes (verified by reading live source):**

- **c F22 (`Box::leak` leak) — CONFIRMED, and the recommendation is superseded by a strictly better fix.** `loc_name` (command.rs:26-28) leaks `loc.to_key()` per call; `loc_parser()` (command.rs:98-107) calls it for all 100 locs on **every construction**; `loc_parser()` is built inside `play_parser` (command.rs:73), which `command_parser` (command.rs:52-58) builds fresh on every call, and `command_parser` is called from both `Gamer::command` (lib.rs:467) and `Gamer::command_spec` (lib.rs:500). So every parse and every command-spec fetch leaks 100 strings permanently. The finding suggested making `LocChoice.name` a `String` or caching in a `OnceLock`; both are unnecessary because `LocChoice` itself is redundant: `Enum<T>` requires only `T: ToString + Clone` (parser/mod.rs:551-576), `Loc` is `Copy + Clone` and implements `Display` (loc.rs:118-122) forwarding **verbatim** to `to_key()`, and `Enum::parse`/`expected`/`to_spec` reach the string exclusively through `v.to_string()` (parser/mod.rs:614, 665-673, 675-681). Therefore `loc_parser()` can be exactly `Enum::partial(loc::all_locs())`: same accepted grammar, byte-identical `Spec::Enum { values, exact: false }`, zero allocation retained. Dropping the `Map` wrapper is safe — `Map` stays in use at command.rs:62 (`play_parser`), :94 (`piece_parser`) and :118 (`dir_parser`), and command.rs:4 is a glob import so nothing needs an import change.
- **c F28 (dead `impl Display for Loc`) — RESOLVED BY the c F22 fix, not a separate change.** After Task 1, `Enum<Loc>`'s `to_string()` calls are the impl's live callers. The finding's two options were "delete it" or "use it everywhere"; Task 1 takes the second, in the one place that matters. **Do not delete `impl Display for Loc` — Task 1 depends on it.**
- **c F26 (`to_key` overflow) — CONFIRMED; adopt the guard, but the finding's rationale is wrong.** `to_key` computes `(b'A' + self.y as u8) as char` (loc.rs:114): for `y = -1` the cast yields `255` and `65 + 255` overflows `u8` — panic in any overflow-checked build; for `y = 10` it silently produces the garbage key `"K…"`. `render.rs`'s `Tiler::tile_at` guards with `loc.valid()` (render.rs:45) after a real panic was caught in render parity testing; `Game::tile_at` (lib.rs:85-90) does not. **No live caller is masked**, verified path by path: `can_play_piece` checks `!l.valid()` at lib.rs:143 *before* `tile_at` at :146; `check_captures`' outer walk starts from a loc that `can_play_piece` already validated; every walk callback loc comes from `loc::walk`, which only enqueues `next_loc.valid()` neighbours (loc.rs:210-215); `loc_filter_matches` (:92) is only driven from `all_locs()` (:356). **Correct rationale to record in the commit and the comment:** the guard removes a latent overflow / garbage-key hazard and unifies `Game::tile_at`'s contract with `render.rs`'s `Tiler` guard. **Do NOT write "mirrors Go's missing-map-key behaviour"** — that claim is factually false: Go's zero `Tile` is `{Player: 0, Owner: 0}` while `empty_tile()` is `{-1, -1}`, so a Go off-board read yields a *player-0* tile, not an empty one. The Rust guard is deliberately the *permissive* direction (off-board reads as empty **and** unowned), which means a future caller that forgets `valid()` would see a *placeable* square; the real safety net is the separate `if !l.valid()` check at **lib.rs:143, which must stay**.
- **c F25 (`pieces()` panic) — CONFIRMED and the finding's own "return an empty Vec" option is REJECTED.** The panic is concretely reachable: `rust/lib/cmd/src/requester/gamer.rs:125-135` forwards the request's `player` to `Gamer::command` unvalidated, and `gamer.rs:170-182` forwards it to `Gamer::player_state` + `Gamer::command_spec` unvalidated. In cathedral, `command(2, …)` -> `command_parser(2)` -> `can_play(2)`: with `no_open_tiles == false` this short-circuits on `self.current_player as i32 == player` (false, so no panic), but with `no_open_tiles == true` it calls `can_play_something(2, …)` -> `pieces(2)` -> **panic**, which kills the shared game process. `player_state(2)` -> `Renderer::render` -> `player_render` -> `render_player_remaining_tiles(state, 2)` -> `piece::pieces(2)` -> **panic**, unconditionally. The stray Edit C (`_ => vec![]`) closes both panics and produces no secondary panic (traced: `can_play_something` -> `for i in (0..0).rev()` never runs -> `false`; `can_play` -> `false`; `command_parser` -> `None` -> a clean `GameError`; `remaining_piece_size` -> `0`; `render_player_remaining_tiles` -> `has_tiles == false` -> renders "None"), **but it makes `pieces()` total-but-lying**: it cannot distinguish "player 2 does not exist" from "this player has no pieces left". That silently reshapes `remaining_piece_size(2) == 0` — a scoring function feeding `points()` (lib.rs:519), `calc_placings()` (lib.rs:395) and the final-scores log (lib.rs:485) — and renders a plausible-looking player panel headed "None" for a player who does not exist. So Task 3 does real boundary validation instead: `pieces()` becomes `Option`-shaped, `Game` gains a `players`-aware `player_pieces` accessor, and the out-of-range player is rejected at the `Gamer` boundary.
  - Boundary placement, re-derived: `Gamer::command` and `Gamer::command_spec` both funnel through `Game::command_parser` (lib.rs:467, :500), so one range guard there covers both; `command` additionally gets an explicit first-line check so the caller gets a precise message instead of the generic "not expecting any commands at the moment". `Gamer::player_state` **cannot** be guarded this way — the trait signature is `fn player_state(&self, player: usize) -> Self::PlayerState` (`rust/lib/game/src/game.rs:52`), with no `Result` — so its guard lives at the only place that dereferences the index, `render_player_remaining_tiles`, and renders an explicit "not a player in this game" marker rather than a bogus empty-hand panel. `PlayerState`'s serialized shape is untouched.
  - Every `pieces()` call site, and what it does after the change (grepped, complete): `command.rs:93` `piece_parser` -> `map_or(0, len)`; `lib.rs:125` `can_play_piece` -> `Err(String)`; `lib.rs:173` `play` -> `Err(GameError)` (unreachable in practice: `can_play` at :165 already rejects); `lib.rs:344` `remaining_piece_size` -> `None`; `lib.rs:360` `can_play_something` -> `false`, **hoisted out of the 100-iteration loc loop**; `lib.rs:431` `start` -> switches to the two catalogue fns directly (see the newly-discovered note); `render.rs:359` `render_player_remaining_tiles` -> the "not a player" marker; test-only `lib.rs:1037-1038` -> `.unwrap()` (test code, permitted).
  - `ortho_dir_name` (loc.rs:35-43) is reached from a runtime path only at lib.rs:192 (`play`'s log), where `dir` comes from `dir_parser` (an `Enum` over `ORTHO_DIRS`, command.rs:110-119) or the `DIR_DOWN` default at command.rs:84 — parser-constrained, no reachable panic. `wall_char` (render.rs:78-87) panics only for `dir == 0`; every call site passes a nonzero literal combination, and `render_corner` (render.rs:121-164) builds `corner` from a 2-entry `corner_map` where each entry contributes either `d` or `dir_inv(other)`, both nonzero. Both are **skipped**, per the finding's own rating.
- **c F24 (dead `parse_loc`) — CONFIRMED, delete.** `grep -rn "parse_loc" --include=*.rs` over the whole workspace returns exactly one hit: the definition at loc.rs:167. It is `pub`, so no dead-code lint fires, and its free-text `^[a-j]\d+$` grammar would diverge from the `Enum`-over-fixed-names grammar the crate actually accepts. The finding's alternative (keep it plus a unit test) is rejected: keeping an unreachable second parser for the same concept is exactly the divergence hazard the finding describes.
- **c F23 (cathedral traversable by the capture flood-fill) — OVERTURNED as a code change; COMMENT ONLY.** Every code fact holds (the inner walk at lib.rs:283 blocks only on `self.tile_at(l2).player == player`, so `PLAYER_CATHEDRAL == 2` tiles do not block; verbatim Go parity), but verification refuted the finding's framing: the crate's own `RULES.md` "3. Captures resolve automatically" (RULES.md:59-83) explicitly documents the cathedral as a *capturable piece identity inside* an enclosed region ("The Cathedral counts as a piece identity for this limit exactly like an opponent piece — a region containing the Cathedral plus one opponent piece has two distinct identities and is NOT captured"), which is exactly what the code does. Verification downgraded minor -> nit with residual "add a walk-site comment referencing RULES.md". Changing the walk would contradict the crate's documented rules and break the existing `capture_returns_piece_but_never_counts_cathedral` test (lib.rs:1174-1195).
- **c F27 (unused `rand`) — CONFIRMED.** `grep -rn rand game/cathedral-2/src/ game/cathedral-2/tests/` finds no `use rand`/`rand::` at all (the single textual hit, lib.rs:417, is the word "randomness" in a doc comment). `start` ignores `seed`; the fuzz bin goes through `brdgme_fuzz`, which carries its own `rand`.
- **c F29 (steal `i32::MIN` overflow) — CONFIRMED; the recommendation is correct and adopted verbatim in shape.** `steal_parser`'s tile-index arm is `Int::any()` (command.rs:75) and `Int::parse` accepts a leading `-` (parser/mod.rs:122-150), so `-2147483648` parses to `i32::MIN`. Neither `can_steal_blue_n` nor `can_steal_red_n` (lib.rs:338-348) inspects `n`. With a non-empty target stack, `let index = len as i32 - n;` (lib.rs:460, identically :502) computes `len + 2^31`, which exceeds `i32::MAX` for any `len >= 1` -> **panic in every default dev/test/fuzz build** (no profile in `rust/Cargo.toml` overrides `overflow-checks`); release wraps to a large negative and is caught by the `index < 0` guard only by luck of two's-complement. The replacement guard `if n < 1 || n as usize > len` accepts **exactly the same** `n` set as the old `index < 0 || index as usize >= len` (old: reject `n > len` via `index < 0`, reject `n <= 0` via `index >= len`), evaluates no arithmetic on the hostile value (`n < 1` short-circuits before the `as usize` cast), and keeps the existing user-facing message so no existing test changes.
- **c F30 (roll path never logs placings) — CONFIRMED.** The Take arm (lib.rs:732-737) and Steal arm (lib.rs:753-758) both append `placings_log(&self.placings(), Some(&scores))` on finish; the Roll arm (lib.rs:711-722) does not. The Roll arm can finish the game: `roll_dice_cmd` (lib.rs:568-616) reaches `logs.extend(self.take_worst())` at :612 when the rolls are exhausted and the player can neither take nor steal, and `take_worst` removes the last tile of the last non-empty pile, making `is_finished()` (lib.rs:282-284) true. `next_player()` then emits `log_game_end()`'s scores table but not the structured placings entry. The finding's "or hoist the check to run once after the match" alternative is **rejected**: the three arms differ in whether they own `logs` mutably and the Roll arm's response fields are otherwise identical, so hoisting would require restructuring all three arms for no benefit; add the same block, matching the sibling arms exactly.
- **c F32 (`.unwrap()` in `roll_dice`) — CONFIRMED, and the finding's suggested replacement is verified RNG-stream-identical.** `roll_dice` (lib.rs:150-152) does `*DIE_FACES.choose(rng).unwrap()`; `choose` returns `None` only for an empty slice and `DIE_FACES` is a 6-element const, so the `unwrap` is infallible but banned on request-reachable paths. Critically, `IndexedRandom::choose`'s body in the pinned `rand 0.10.2` is `Some(&self[rng.random_range(..self.len())])` (`~/.cargo/registry/src/*/rand-0.10.2/src/seq/slice.rs:52-61`), and `SampleRange for RangeTo<usize>` forwards to `sample_single(0, end)` exactly as `SampleRange for Range<usize>` does (`src/distr/uniform.rs:447-483`). So `DIE_FACES[rng.random_range(0..DIE_FACES.len())]` samples the *same* distribution with the *same* number of RNG words: no in-flight saved game's future dice change, and no existing seeded test outcome shifts. (This mattered enough to verify: `Game.rng` is persisted, so a stream change would silently alter live games.)
- **c F33 (`take_worst` hand-rolled min loops) — CONFIRMED.** Both branches (lib.rs:529-546, :547-565) re-implement find-index-of-minimum with `min_idx`/`min_val` and strict `<`, so they keep the **first** minimum on ties — which is exactly `Iterator::min_by_key`'s documented tie behaviour, so the swap is behaviour-preserving (this is what keeps `test_take_worst_red_picks_minimum`, lib.rs:1686-1713, green). The else branch's `self.blue_tiles[0]` (lib.rs:549) is safe today only via the non-local invariant "`take_worst` is unreachable once both piles are empty because the game would be finished and `command_parser` returns `None`" (command.rs:19-21); `min_by_key` returning `Option` removes the indexing entirely.
- **c F34 (`take_*`/`steal_*` near-verbatim duplicates) — CONFIRMED; ADJUSTED to keep the public API.** `take_blue`/`take_red` (lib.rs:399-431) differ only in the guard, the message, whether the removal index comes from `sushi` or `bones`, and which two pile vectors are touched; `steal_blue`/`steal_red` (lib.rs:433-515) differ only in the guards, the two messages and the pile pair. All four are `pub` and are called from `Gamer::command` (:729-730, :750-751) and from existing tests, so the fix keeps them as thin one-line wrappers over private `take(kind)`/`steal(kind, …)` helpers. Doing this **after** c F29 means the validated steal guard exists in exactly one place afterwards, which is the concrete reason the finding gives for de-duplicating at all ("the i32::MIN overflow above had to be spotted twice").

---

### Task 1: drop the `Box::leak` location-name table (c F22 major; resolves c F28 nit)

**Problem (restated):** `loc_parser()` builds 100 `LocChoice`s, each holding a `&'static str` produced by `Box::leak` (command.rs:26-28), and it is rebuilt on **every** `command()` and `command_spec()` call. Each parse or suggest request permanently leaks 100 heap strings (~4-8 KB with allocator overhead) on a long-running HTTP game service. The `'static` lifetime is self-imposed by `LocChoice.name`'s type; nothing in the parser needs it.

**Fix (re-derived):** delete `LocChoice`, its `Display` impl and `loc_name`, and make `loc_parser()` return `Enum::partial(loc::all_locs())` directly. `Loc: Copy + Clone + Display` satisfies `Enum`'s `T: ToString + Clone` bound, and `Loc`'s `Display` (loc.rs:118-122) writes exactly `to_key()`, so the accepted grammar and the emitted `Spec::Enum` are byte-identical. This also gives `impl Display for Loc` its first real caller, which is what closes c F28 — **keep that impl**.

**Edge cases:**
- `Map` must stay imported/used: it is still used at command.rs:62, :94, :118. The glob import at command.rs:4 covers `Enum` already.
- `DirChoice` (command.rs:38-48) is **not** touched: its `name` comes from `ortho_dir_name`, a `match` returning real `&'static str` literals — no leak there.
- `Enum::expected()` sorts its value strings (parser/mod.rs:665-673) and `to_spec()` does not (:675-681). Both behaviours are unchanged because the input strings are unchanged.
- No `deny(warnings)` and no `[lints]` table in `rust/Cargo.toml` or `game/cathedral-2/Cargo.toml`, but clippy still runs at `-D warnings` in the checkpoint, so no unused item may be left behind.
- The leak itself is not observable from a unit test (leaked memory is not reclaimed and not counted); the test below is a **grammar/spec lock-in** that proves the refactor did not change what the parser accepts or advertises. It passes before and after — that is intentional and is why the red-first step is a compile-level one instead.

**Files:**
- Modify: `rust/game/cathedral-2/src/command.rs` (delete lines 15-34, rewrite `loc_parser` at :98-107, add a test module)

**Steps:**

- [ ] Delete command.rs lines 15-34 in full — the `LocChoice` doc comment, the struct, the `loc_name` comment, `loc_name` itself, and `impl std::fmt::Display for LocChoice`. Leave the `DirChoice` block (:36-48) exactly as it is.
- [ ] Replace `loc_parser` (command.rs:98-107) with:

```rust
/// Port of `LocParser` (`command.go`): an `Enum` over every `AllLocs[i].String()`.
///
/// `Loc`'s `Display` impl forwards verbatim to `to_key()`, and `Enum` only
/// needs `ToString + Clone`, so the locations go in directly - no wrapper
/// struct and no leaked `&'static str` name table (c F22).
fn loc_parser() -> impl Parser<T = Loc> {
    Enum::partial(loc::all_locs())
}
```

- [ ] Run: `cargo test -p cathedral-2` — everything compiles and all 22 inline tests plus `game_contract` PASS. If it does not compile, the most likely cause is a leftover reference to `LocChoice`; there should be none.
- [ ] Add the spec lock-in test. Append to the end of `rust/game/cathedral-2/src/command.rs`:

```rust
#[cfg(test)]
mod test {
    use brdgme_game::command::Spec;
    use brdgme_game::command::parser::Parser;

    use super::loc_parser;
    use crate::loc;

    #[test]
    fn loc_parser_spec_is_every_board_location_in_row_major_order() {
        // c F22 lock-in: dropping the leaked `&'static str` name table must
        // not change the accepted grammar or the advertised command spec.
        match loc_parser().to_spec() {
            Spec::Enum { values, exact } => {
                assert!(!exact, "locations are matched by prefix");
                assert_eq!(100, values.len());
                assert_eq!("A1", values[0]);
                assert_eq!("J10", values[99]);
                let expected: Vec<String> =
                    loc::all_locs().iter().map(|l| l.to_key()).collect();
                assert_eq!(expected, values);
            }
            s => panic!("expected an Enum spec, got {:?}", s),
        }
    }

    #[test]
    fn loc_parser_parses_a_full_and_a_partial_location() {
        let names: Vec<String> = vec!["mick".to_string(), "steve".to_string()];
        let out = loc_parser().parse("f6", &names).expect("f6 must parse");
        assert_eq!(loc::Loc::new(5, 5), out.value);
        let out = loc_parser().parse("j10 rest", &names).expect("j10 must parse");
        assert_eq!(loc::Loc::new(9, 9), out.value);
        assert_eq!(" rest", out.remaining);
    }
}
```

- [ ] Run: `cargo test -p cathedral-2` — the 2 new tests PASS along with everything else. (`play 1 e5 down` / `play 9 f9 down` style commands in the existing tests are the end-to-end proof that the grammar still works.)
- [ ] `cargo clippy -p cathedral-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/cathedral-2/src/command.rs` ; message: `fix(cathedral-2): remove per-request Box::leak in the location parser (c F22 c F28, WP-21)`

NOTE: this task shortens command.rs by ~20 lines above the old line 98; later cathedral tasks cite command.rs symbols by name.

---

### Task 2: guard `Game::tile_at` against off-board locations (c F26 nit)

**Problem (restated):** `Game::tile_at` (lib.rs:85-90) calls `loc.to_key()` unconditionally. `to_key` computes `(b'A' + self.y as u8) as char` (loc.rs:113-115): for `y < 0` the cast wraps to a large `u8` and the addition overflows — panic in every overflow-checked build; for `y > 9` it silently produces a garbage key. `render.rs`'s `Tiler::tile_at` already guards with `loc.valid()` (render.rs:45-47), added after a real panic caught in render parity testing; the `Game` version relies on every caller having checked first.

**Fix (re-derived):** add the same early return. No live caller is masked (see the re-derivation notes: `can_play_piece` validates at lib.rs:143, walk callbacks only ever see `valid()` locs, `loc_filter_matches` is driven from `all_locs()`), so this changes no legality outcome — it removes a latent hazard and gives both `tile_at` implementations one defensive contract.

**Edge cases:**
- The guard is **permissive**: an off-board loc now reads as `empty_tile()`, i.e. `player == NO_PLAYER` **and** `owner == NO_PLAYER`, which `loc_filter_matches` would report as *placeable*. The check that actually keeps off-board placements illegal is the separate `if !l.valid()` at **lib.rs:143**; it must stay. Do not remove it as "now redundant" — it is the real safety net and it produces the user-facing "playing there would go off the board" message.
- Do **not** write "mirrors Go's missing-map-key behaviour" in the comment or the commit message: Go's zero `Tile` is `{Player: 0, Owner: 0}`, not `empty_tile()`'s `{-1, -1}`.
- `y > 9` produced a garbage key that simply missed the map and already returned `empty_tile()`; the observable change is confined to the panic case.

**Files:**
- Modify: `rust/game/cathedral-2/src/lib.rs` (`tile_at`, lines 85-90; new test in the inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add to `mod test` in `rust/game/cathedral-2/src/lib.rs` (the module starting at lib.rs:548):

```rust
    #[test]
    fn tile_at_returns_empty_for_off_board_locations() {
        // c F26: `Loc::to_key` computes `b'A' + y as u8`, which overflows for
        // negative y (panic in overflow-checked builds) and yields a garbage
        // key above row J. `Game::tile_at` must reject off-board locations
        // before keying, matching render.rs's `Tiler` guard.
        let (g, _) = Game::start(2, 1).unwrap();
        for l in [
            Loc::new(-1, -1),
            Loc::new(0, -1),
            Loc::new(-1, 0),
            Loc::new(0, 10),
            Loc::new(10, 0),
            Loc::new(-5, 42),
        ] {
            let t = g.tile_at(l);
            assert_eq!(NO_PLAYER, t.player, "{:?} must read as empty", l);
            assert_eq!(NO_PLAYER, t.owner, "{:?} must read as unowned", l);
        }
        // On-board reads are unaffected.
        assert_eq!(NO_PLAYER, g.tile_at(Loc::new(0, 0)).player);
        assert_eq!(NO_PLAYER, g.tile_at(Loc::new(9, 9)).player);
    }
```

- [ ] Run: `cargo test -p cathedral-2 tile_at_returns_empty` — expected FAIL with `panic: attempt to add with overflow` inside `Loc::to_key` on the very first `Loc::new(-1, -1)` (cargo test builds with overflow checks on).
- [ ] Implement: replace `tile_at` (lib.rs:85-90) with:

```rust
    /// The tile at `loc`, or `empty_tile()` if `loc` is off the board.
    ///
    /// The off-board guard mirrors `render.rs`'s `Tiler` implementation so
    /// both `tile_at`s share one contract: `Loc::to_key` assumes `0..=9` on
    /// both axes and overflows on a negative `y`. Note this is the permissive
    /// direction - off-board reads as empty AND unowned - so placement
    /// legality still depends on the separate `!l.valid()` check in
    /// `can_play_piece` (c F26).
    fn tile_at(&self, loc: Loc) -> Tile {
        if !loc.valid() {
            return empty_tile();
        }
        self.board
            .get(&loc.to_key())
            .cloned()
            .unwrap_or_else(empty_tile)
    }
```

- [ ] Run: `cargo test -p cathedral-2` — the new test PASSES and all previous tests PASS (including `render_does_not_panic_on_board_edge_tiles`, lib.rs:1257, and `play_rejects_off_board_occupied_and_owned_by_other`, lib.rs:835, which still asserts the exact string "playing there would go off the board").
- [ ] `cargo clippy -p cathedral-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/cathedral-2/src/lib.rs` ; message: `fix(cathedral-2): guard Game::tile_at against off-board locations (c F26, WP-21)`

NOTE: this task shifts lib.rs line numbers below ~line 90 by about +10; Task 3 and Task 5 locate their edits by symbol name.

---

### Task 3: validate the player index at the `Gamer` boundary and make `pieces()` `Option`-shaped (c F25 minor)

**Problem (restated):** `piece::pieces(player)` ends in `_ => panic!("invalid player: {}", player)` (piece.rs:106-112). The request harness forwards the request's player index unvalidated — `rust/lib/cmd/src/requester/gamer.rs:125-135` into `Gamer::command`, and `gamer.rs:170-182` into `Gamer::player_state` and `Gamer::command_spec` — so a request naming player 2 in a 2-player game panics the shared game process. Two concrete paths: `command(2, …)`/`command_spec(2)` when `no_open_tiles == true` (via `can_play` -> `can_play_something` -> `pieces`), and `player_state(2)`'s render **unconditionally** (via `render_player_remaining_tiles` -> `piece::pieces`).

**Fix (re-derived, rejecting the finding's empty-`Vec` option):** returning `vec![]` would close both panics but leave `pieces()` unable to distinguish "player 2 does not exist" from "player 2 has no pieces left" — silently turning `remaining_piece_size(2)` into a legitimate-looking `0` in a scoring function and rendering a plausible but fictitious player panel. Instead:

1. `piece::pieces` returns `Option<Vec<Piece>>`; the two catalogue builders become `pub` so `start` can size `played_pieces` without going through the fallible API.
2. `Game` gains `player_pieces(player)`, which additionally requires `0 <= player < self.players` — the game-aware check `piece::pieces` alone cannot make.
3. The `Gamer` boundary rejects out-of-range players: `command_parser` (which both `Gamer::command` and `Gamer::command_spec` funnel through) returns `None`, and `Gamer::command` returns a precise `GameError::Internal` first.
4. `Gamer::player_state` cannot return a `Result` (trait signature, `rust/lib/game/src/game.rs:52`), so its guard lives where the index is actually dereferenced — `render_player_remaining_tiles` — and renders an explicit "not a player in this game" marker rather than an empty-hand panel.
5. `remaining_piece_size` returns `Option<i32>` so no caller can be handed a fabricated score.

**Why `GameError::Internal` and not `invalid_input`:** an out-of-range player index is a platform contract violation, not player input. `Internal` maps to `Response::SystemError` in the harness (`gamer.rs:151-155`), which surfaces in monitoring instead of being shown to a user as "invalid input". This does not affect `assert_gamer_contract`, which only exercises player 0.

**Why `unwrap_or(0)` at the four internal `remaining_piece_size` call sites is not the same lie:** all four iterate `0..self.players` (lib.rs:215, :395, :484, :518), so `None` is structurally impossible there; the API is now honest for every *external* caller, which is where the hazard was.

**Edge cases:**
- Negative `player`: `player_pieces` checks `player < 0` before the `as usize` cast, so `-1` is rejected rather than wrapping to a huge index.
- `player == self.players` and beyond: rejected by every path above.
- `opponent(p)` is `(p + 1) % 2` (lib.rs:71-73), so `player_render(state, 2)` still renders player 1's panel in the opponent slot. That is acceptable — the invalid player's own panel carries the marker, and the render must not panic. Do not change `opponent`.
- `can_play_something` returning `false` for a non-player is not a lie: "this non-player can play nothing" is the correct answer to a predicate.
- Serialized shapes: unchanged. `PlayerState` still serializes `{ public, player }` for any `player` value; `Game`/`PubState` are untouched.
- Out of scope (WP-09): a *deserialized* `Game` whose `played_pieces` has fewer than `players` rows, or whose row is shorter than the catalogue. Keep the existing `self.played_pieces[player as usize][i]` indexing exactly as it is; the boundary check here is about the player index only.
- The `pieces(0)`/`pieces(1)` calls in the existing test `points_returns_raw_remaining_piece_size` (lib.rs:1037-1038) must gain `.unwrap()`. This is the only permitted existing-test edit in the package.

**Files:**
- Modify: `rust/game/cathedral-2/src/piece.rs` (`player_0_pieces` :58, `player_1_pieces` :79, `pieces` :106-112)
- Modify: `rust/game/cathedral-2/src/lib.rs` (new `player_pieces`; `can_play_piece`, `play`, `remaining_piece_size`, `can_play_something`, `calc_placings`, `Gamer::start`, `Gamer::command`, `Gamer::points`; test module)
- Modify: `rust/game/cathedral-2/src/command.rs` (`Game::command_parser`, `piece_parser`)
- Modify: `rust/game/cathedral-2/src/render.rs` (`render_player_remaining_tiles`)

**Steps:**

- [ ] Write the failing tests. Add to `mod test` in `rust/game/cathedral-2/src/lib.rs`:

```rust
    #[test]
    fn out_of_range_player_is_rejected_not_panicked() {
        // c F25: the request harness forwards the player index unvalidated
        // (rust/lib/cmd/src/requester/gamer.rs:125-135, :170-182), so an
        // out-of-range index must produce an error, not kill the process.
        let (mut g, _) = Game::start(2, 1).unwrap();
        // `no_open_tiles` routes `can_play` through `can_play_something`,
        // which is the path that reaches the piece catalogue.
        g.no_open_tiles = true;

        let err = g.command(2, "play 1 a1 down", &players()).unwrap_err();
        assert!(
            err.to_string().contains("not a player in this game"),
            "unexpected error: {}",
            err
        );
        assert!(g.command_spec(2).is_none());
        assert!(g.command_spec(0).is_some());
        assert!(!g.can_play_something(2, LocFilter::Playable));
        assert!(!g.can_play(2));
    }

    #[test]
    fn remaining_piece_size_is_none_for_a_non_player() {
        // c F25: scoring must not fabricate a 0 for a player who does not
        // exist - `None` forces the caller to decide.
        let (g, _) = Game::start(2, 1).unwrap();
        assert!(g.remaining_piece_size(0).is_some());
        assert!(g.remaining_piece_size(1).is_some());
        assert_eq!(None, g.remaining_piece_size(2));
        assert_eq!(None, g.remaining_piece_size(-1));
    }

    #[test]
    fn player_state_render_survives_a_non_player_index() {
        // c F25: `Gamer::player_state` cannot return a Result, so the render
        // must degrade to an explicit marker instead of panicking or showing
        // a fictitious empty hand.
        let (g, _) = Game::start(2, 1).unwrap();
        let markup = brdgme_game::Renderer::render(&g.player_state(2));
        let text = brdgme_markup::plain(&brdgme_markup::transform(&markup, &[]));
        assert!(
            text.contains("not a player in this game"),
            "expected the non-player marker, got: {}",
            text
        );
        // A real player still renders their catalogue, not the marker.
        let markup = brdgme_game::Renderer::render(&g.player_state(0));
        let text = brdgme_markup::plain(&brdgme_markup::transform(&markup, &[]));
        assert!(!text.contains("not a player in this game"));
    }
```

- [ ] Run: `cargo test -p cathedral-2 out_of_range_player` then `cargo test -p cathedral-2 non_player` — expected FAILURES, all three panicking with `invalid player: 2` from `piece.rs`.
- [ ] Implement, in `rust/game/cathedral-2/src/piece.rs`: change `fn player_0_pieces()` (piece.rs:58) to `pub fn player_0_pieces()`, change `fn player_1_pieces()` (piece.rs:79) to `pub fn player_1_pieces()`, and replace `pieces` (piece.rs:104-112) with:

```rust
/// Port of the `Pieces` package var (`piece.go`), the full piece catalogue
/// keyed by player index (0 or 1).
///
/// Returns `None` for any other index rather than panicking: the request
/// harness forwards player indices unvalidated, and an `Option` keeps
/// "not a player" distinguishable from "no pieces left" (c F25).
pub fn pieces(player: i32) -> Option<Vec<Piece>> {
    match player {
        0 => Some(player_0_pieces()),
        1 => Some(player_1_pieces()),
        _ => None,
    }
}
```

- [ ] Implement, in `rust/game/cathedral-2/src/lib.rs`:
  1. Add `player_pieces` to `impl Game`, immediately after `tile_at`:

```rust
    /// The piece catalogue for `player`, or `None` if `player` is not a
    /// player in this game. This is the single game-aware player-index check
    /// (`piece::pieces` alone cannot know `self.players`) (c F25).
    fn player_pieces(&self, player: i32) -> Option<Vec<Piece>> {
        if player < 0 || player as usize >= self.players {
            return None;
        }
        pieces(player)
    }
```

  2. Add `Piece` to the piece imports at lib.rs:25: `use piece::{Piece, pieces};`
  3. In `can_play_piece`, replace `let all_pieces = pieces(player);` with:

```rust
        let all_pieces = match self.player_pieces(player) {
            Some(p) => p,
            None => return Err("that is not a player in this game".to_string()),
        };
```

  4. In `play`, replace `let all_pieces = pieces(player);` with:

```rust
        // Unreachable in practice: `can_play` above already rejects an
        // out-of-range player. Kept total so no future reordering can panic.
        let all_pieces = match self.player_pieces(player) {
            Some(p) => p,
            None => {
                return Err(GameError::invalid_input("that is not a player in this game"));
            }
        };
```

  5. Replace `remaining_piece_size` (lib.rs:342-352) with:

```rust
    /// Port of `Game.RemainingPieceSize` (`game.go`).
    ///
    /// `None` when `player` is not a player in this game - this feeds
    /// scoring (`points`, `calc_placings`, the final-scores log), so a
    /// fabricated `0` would be a silent wrong answer (c F25).
    pub fn remaining_piece_size(&self, player: i32) -> Option<i32> {
        let all_pieces = self.player_pieces(player)?;
        let mut sum = 0i32;
        for (i, p) in all_pieces.iter().enumerate() {
            if !self.played_pieces[player as usize][i] {
                sum += p.positions.len() as i32;
            }
        }
        Some(sum)
    }
```

  6. In `can_play_something` (lib.rs:355-379), hoist the catalogue out of the location loop and guard it. Replace the function body's opening — the `for l in loc::all_locs() {` line, the `if !self.loc_filter_matches(...) { continue; }` block and the `let all_pieces = pieces(player);` line — with:

```rust
        // Hoisted out of the location loop: the catalogue is identical for
        // every location, and rebuilding it 100 times per call was pure
        // waste. `None` means `player` is not a player in this game, which
        // can play nothing (c F25).
        let Some(all_pieces) = self.player_pieces(player) else {
            return false;
        };
        for l in loc::all_locs() {
            if !self.loc_filter_matches(filter, player, l) {
                continue;
            }
```

  (the rest of the loop body — the `for i in (0..all_pieces.len()).rev()` block — is unchanged.)
  7. In `calc_placings` (lib.rs:393-398), change the metric line to
     `.map(|p| vec![-self.remaining_piece_size(p as i32).unwrap_or(0)])`
     and add above the `let metrics` line: `// `p` is always in `0..self.players`, so `None` is unreachable.`
  8. In `Gamer::start`, replace the `played_pieces` line (lib.rs:431) with:

```rust
        let played_pieces = vec![
            vec![false; piece::player_0_pieces().len()],
            vec![false; piece::player_1_pieces().len()],
        ];
```

  9. In `Gamer::command`, insert as the first statement of the body (before `let output = …`):

```rust
        if player >= self.players {
            return Err(GameError::internal(format!(
                "player {} is not a player in this game ({} players)",
                player, self.players
            )));
        }
```

  10. In `Gamer::command`'s finished block, change the score line to
      `.map(|p| (p, -self.remaining_piece_size(p as i32).unwrap_or(0)))`.
  11. In `Gamer::points`, change the map to
      `.map(|p| self.remaining_piece_size(p as i32).unwrap_or(0) as f32)`.
  12. In the finish log inside `play` (lib.rs:219-221), change the pushed value to
      `self.remaining_piece_size(pl as i32).unwrap_or(0).to_string()`.
  13. In the existing test `points_returns_raw_remaining_piece_size`, change lib.rs:1037-1038 to use `pieces(0).unwrap()` and `pieces(1).unwrap()`.
- [ ] Implement, in `rust/game/cathedral-2/src/command.rs`:
  1. Add the range guard as the first statement of `Game::command_parser`:

```rust
        // Single choke point for both `Gamer::command` and
        // `Gamer::command_spec`, which each build their parser here (c F25).
        if player < 0 || player as usize >= self.players {
            return None;
        }
```

  2. Replace `piece_parser`'s first line (`let max = pieces(player).len() as i32;`) with:

```rust
    // `command_parser` rejects out-of-range players before this is built, so
    // the catalogue is always present here; `0` degrades to a parser that
    // matches nothing rather than panicking.
    let max = pieces(player).map_or(0, |p| p.len() as i32);
```

- [ ] Implement, in `rust/game/cathedral-2/src/render.rs`: replace the first line of `render_player_remaining_tiles` (render.rs:359, `let all_pieces = piece::pieces(p_num as i32);`) with:

```rust
    // `Gamer::player_state` cannot reject an out-of-range player (the trait
    // returns `Self::PlayerState`, not a `Result`), so the render is where
    // the index is checked. Say so explicitly rather than showing an empty
    // hand for a player who does not exist (c F25).
    let all_pieces = match piece::pieces(p_num as i32) {
        Some(p) if p_num < state.players && p_num < state.played_pieces.len() => p,
        _ => {
            return vec![N::Bold(vec![N::Fg(
                NamedColor::Grey.into(),
                vec![N::text("not a player in this game")],
            )])];
        }
    };
```

- [ ] Run: `cargo test -p cathedral-2` — the 3 new tests PASS and every previous test PASSES, including `game_contract` (which only drives player 0) and `points_returns_raw_remaining_piece_size` (now with `.unwrap()`).
- [ ] `cargo clippy -p cathedral-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/cathedral-2/src/piece.rs rust/game/cathedral-2/src/lib.rs rust/game/cathedral-2/src/command.rs rust/game/cathedral-2/src/render.rs` ; message: `fix(cathedral-2): validate player index at the Gamer boundary, Option-shaped pieces() (c F25, WP-21)`

---

### Task 4: delete the vestigial `parse_loc` (c F24 minor)

**Problem (restated):** `parse_loc` (loc.rs:166-181), a port of Go's `ParseLoc`, has zero callers anywhere in the workspace — `grep -rn "parse_loc" --include=*.rs` over `/home/beefsack/Development/brdgme` returns only the definition. It is `pub`, so no dead-code lint fires. Its free-text `^[a-j]\d+$` grammar is not what the crate accepts (locations come from the `Enum`-over-fixed-names `loc_parser`), so wiring it up later would silently diverge from the real command syntax.

**Fix:** delete it. Git history keeps it if a free-text input path is ever wanted. The finding's alternative (keep it and add a unit test) is rejected: a tested-but-unreachable second grammar for the same concept is precisely the divergence hazard.

**Edge cases:** none — nothing references it, and it references only `Loc::new`/`Loc::valid`, which stay.

**Files:**
- Modify: `rust/game/cathedral-2/src/loc.rs` (delete lines 166-181, i.e. the `/// Port of `ParseLoc` …` doc comment and the whole function, plus the blank line left behind)

**Steps:**

- [ ] Before deleting, confirm zero callers: `grep -rn "parse_loc" --include=*.rs /home/beefsack/Development/brdgme` — expected output: exactly one line, `rust/game/cathedral-2/src/loc.rs:167:pub fn parse_loc(input: &str) -> Option<Loc> {`. If any other hit appears, STOP and report instead of deleting.
- [ ] Delete loc.rs lines 166-181 (the doc comment and the function). Leave `WALK_CONTINUE`/`WALK_BLOCKED`/`WALK_FINISH` (:183-185) and `walk` (:190) intact.
- [ ] Run: `cargo test -p cathedral-2` — full suite PASSES.
- [ ] `cargo clippy -p cathedral-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/cathedral-2/src/loc.rs` ; message: `refactor(cathedral-2): delete unused parse_loc (c F24, WP-21)`

---

### Task 5: document the cathedral's role in the capture walk (c F23 nit, COMMENT ONLY)

**Problem (restated):** the inner area walk in `check_captures` blocks only on `self.tile_at(l2).player == player` (lib.rs:283), so `PLAYER_CATHEDRAL == 2` tiles do not block the flood-fill. The original finding read this as an undocumented preserved defect. **Verification refuted that premise:** the crate's own `RULES.md` "3. Captures resolve automatically" (RULES.md:59-83) documents the cathedral as a capturable piece *identity inside* an enclosed region, not as an enclosure wall — exactly what the code does. Severity was downgraded minor -> nit with the sole residual being a walk-site comment.

**Fix:** add a comment. **Do NOT change the walk condition.** Adding `|| self.tile_at(l2).player == PLAYER_CATHEDRAL` would contradict the crate's documented rules, break Go parity, and break the existing `capture_returns_piece_but_never_counts_cathedral` test (lib.rs:1174-1195).

**Edge cases:** none — this task adds no executable code. No test is added: the behaviour is already covered by `capture_returns_piece_but_never_counts_cathedral` and `capture_with_two_distinct_pieces_does_not_capture` (lib.rs:1060).

**Files:**
- Modify: `rust/game/cathedral-2/src/lib.rs` (the inner `loc::walk` closure inside `check_captures` — locate by the literal `loc::walk(l, &all_dirs, |l2| {`)

**Steps:**

- [ ] Insert directly above the `loc::walk(l, &all_dirs, |l2| {` line inside `check_captures`:

```rust
            // The area walk blocks only on the capturing player's own
            // pieces. Cathedral tiles (`PLAYER_CATHEDRAL`) deliberately do
            // NOT block it: per RULES.md "3. Captures resolve
            // automatically", the Cathedral counts as a piece *identity*
            // found inside an enclosed region, not as part of the enclosing
            // wall. Verbatim Go parity with `CheckCaptures`
            // (`play_command.go`). Intended behaviour, not a preserved
            // defect (c F23).
```

- [ ] Run: `cargo test -p cathedral-2` — full suite PASSES (comment-only change).
- [ ] `cargo clippy -p cathedral-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/cathedral-2/src/lib.rs` ; message: `docs(cathedral-2): explain the cathedral's role in the capture walk (c F23, WP-21)`

---

### Task 6: remove cathedral-2's unused `rand` dependency (c F27 nit)

**Problem (restated):** `rand = "0.10.2"` sits in `game/cathedral-2/Cargo.toml:14` but nothing in the crate uses it: `grep -rn rand game/cathedral-2/src/ game/cathedral-2/tests/` finds only the word "randomness" in a doc comment at lib.rs:417. The game is fully deterministic (`start` ignores `seed`), and the fuzz bin goes through `brdgme_fuzz`, which declares its own `rand`.

**Fix:** delete that one line. Nothing else in either manifest is touched (broader manifest hygiene is WP-64/WP-65). **Do not** touch `sushizock-2/Cargo.toml` — its `rand` is genuinely used (`sushizock-2/src/lib.rs:15,151,647-648`).

**Edge cases:**
- `Cargo.lock` will drop cathedral-2's `rand` edge (the `rand` package itself stays, used by many crates). Include `rust/Cargo.lock` in the commit if it changed.
- The 4 bins (`src/bin/cathedral_2_{cli,fuzz,http,repl}.rs`) were read: none references `rand`.

**Files:**
- Modify: `rust/game/cathedral-2/Cargo.toml` (delete line 14), possibly `rust/Cargo.lock`

**Steps:**

- [ ] Delete the line `rand = "0.10.2"` from `rust/game/cathedral-2/Cargo.toml` (line 14). Leave every other dependency line exactly as it is, in place.
- [ ] Run: `cargo build -p cathedral-2` — compiles (proves no non-test use), then `cargo test -p cathedral-2` — full suite PASSES (proves no test use either).
- [ ] `cargo clippy -p cathedral-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/cathedral-2/Cargo.toml rust/Cargo.lock` ; message: `refactor(cathedral-2): drop the unused rand dependency (c F27, WP-21)`

---

### Task 7: reject out-of-range steal tile numbers before the subtraction (c F29 major)

**Problem (restated):** `steal_parser`'s tile-index argument is `Int::any()` (`sushizock-2/src/command.rs:75`), and `Int::parse` accepts a leading `-` (`rust/lib/game/src/command/parser/mod.rs:122-150`), so a player can send `steal <opponent> blue -2147483648`. Neither `can_steal_blue_n` nor `can_steal_red_n` (lib.rs:338-348) inspects `n`. With 4+ matching chopsticks and a non-empty target stack, `let index = len as i32 - n;` (lib.rs:460 and identically :502) computes `len + 2^31 > i32::MAX` — **panic** in every default dev/test/fuzz build (nothing in `rust/Cargo.toml` overrides `overflow-checks`). Release wraps to a large negative and is caught by the `index < 0` guard only by accident of two's-complement.

**Fix (re-derived):** validate `n` first and drop the signed intermediate entirely. `if n < 1 || n as usize > len` rejects exactly the same `n` set the old post-hoc guard did (old: `n > len` fails `index < 0`; `n <= 0` fails `index >= len`), and `n < 1` short-circuits so the hostile value never reaches the `as usize` cast. The user-facing message is kept verbatim so no existing test changes.

**Edge cases:**
- `n = i32::MIN`, `n = 0`, negatives generally: caught by `n < 1`, no arithmetic executed.
- `n = len` (bottom of the stack) must still be **accepted** -> `idx = 0`. `n = len + 1` must be rejected.
- `n = 1` (default when omitted, lib.rs:439/:481) -> `idx = len - 1`, the top of the stack — unchanged.
- The guard stays **after** the `can_steal_*` / self-steal / empty-stack checks so error precedence and messages are unchanged.
- No behaviour change for any legal input; existing `test_steal_blue_n` (lib.rs:1045) and `test_steal_red_n` (lib.rs:1088) pin the accepted cases.

**Files:**
- Modify: `rust/game/sushizock-2/src/lib.rs` (`steal_blue` lines 459-467, `steal_red` lines 501-509; new tests in the inline `mod test`)

**Steps:**

- [ ] Write the failing tests. Add to `mod test` in `rust/game/sushizock-2/src/lib.rs`:

```rust
    #[test]
    fn test_steal_blue_rejects_extreme_negative_tile_number() {
        // c F29: `Int::any()` parses i32::MIN, and `len as i32 - n` then
        // overflows - a crafted command string panics the process in every
        // overflow-checked build.
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        g.player_blue_tiles[BJ] = vec![Tile {
            kind: TileType::Blue,
            value: 3,
        }];
        g.kept_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Bones,
        ];
        g.rolled_dice = vec![];
        let err = g
            .command(MICK, "steal bj blue -2147483648", &n)
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid tile number"),
            "unexpected error: {}",
            err
        );
        assert_eq!(1, g.player_blue_tiles[BJ].len(), "no tile may move");
        assert!(g.player_blue_tiles[MICK].is_empty());
    }

    #[test]
    fn test_steal_red_rejects_extreme_negative_tile_number() {
        // c F29, the identical duplicated branch in `steal_red`.
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        g.player_red_tiles[STEVE] = vec![Tile {
            kind: TileType::Red,
            value: -3,
        }];
        g.kept_dice = vec![
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::RedChopsticks,
            DieFace::Sushi,
        ];
        g.rolled_dice = vec![];
        let err = g
            .command(MICK, "steal ste red -2147483648", &n)
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid tile number"),
            "unexpected error: {}",
            err
        );
        assert_eq!(1, g.player_red_tiles[STEVE].len(), "no tile may move");
    }

    #[test]
    fn test_steal_tile_number_bounds_are_inclusive() {
        // c F29 regression fence: 1..=len must stay accepted and len+1
        // rejected, so the new guard cannot drift from the old one.
        let (mut g, _) = Game::start(3, 1).unwrap();
        let n = names();
        let stack = vec![
            Tile {
                kind: TileType::Blue,
                value: 3,
            },
            Tile {
                kind: TileType::Blue,
                value: 1,
            },
        ];
        g.player_blue_tiles[BJ] = stack.clone();
        g.kept_dice = vec![
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::BlueChopsticks,
            DieFace::Bones,
        ];
        g.rolled_dice = vec![];
        assert!(g.command(MICK, "steal bj blue 3", &n).is_err(), "len + 1");
        assert!(g.command(MICK, "steal bj blue 0", &n).is_err(), "zero");
        // n == len is the bottom of the stack and must succeed.
        g.command(MICK, "steal bj blue 2", &n).unwrap();
        assert_eq!(vec![stack[0]], g.player_blue_tiles[MICK]);
    }
```

- [ ] Run: `cargo test -p sushizock-2 extreme_negative` — expected FAILURES: both tests panic with `attempt to subtract with overflow` at `lib.rs:460` / `lib.rs:502`.
- [ ] Implement: in `steal_blue`, replace lines 459-467

```rust
        let len = self.player_blue_tiles[target].len();
        let index = len as i32 - n;
        if index < 0 || index as usize >= len {
            return Err(GameError::invalid_input(format!(
                "invalid tile number, you need to pick something between 1 and {}",
                len
            )));
        }
        let idx = index as usize;
```

  with

```rust
        let len = self.player_blue_tiles[target].len();
        // Validate before any arithmetic: `n` comes straight from
        // `Int::any()`, so `len as i32 - n` overflows for n = i32::MIN and
        // panics in overflow-checked builds. `n < 1` short-circuits so the
        // hostile value never reaches the cast. Accepts exactly 1..=len,
        // the same set the old post-hoc guard accepted (c F29).
        if n < 1 || n as usize > len {
            return Err(GameError::invalid_input(format!(
                "invalid tile number, you need to pick something between 1 and {}",
                len
            )));
        }
        let idx = len - n as usize;
```

- [ ] Implement: apply the byte-identical replacement to `steal_red` lines 501-509, with `self.player_red_tiles[target]` in place of `self.player_blue_tiles[target]`.
- [ ] Run: `cargo test -p sushizock-2` — the 3 new tests PASS and all 33 previous tests plus `game_contract` PASS.
- [ ] `cargo clippy -p sushizock-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/sushizock-2/src/lib.rs` ; message: `fix(sushizock-2): reject out-of-range steal tile numbers before subtracting (c F29, WP-21)`

---

### Task 8: emit the placings log when the roll path ends the game (c F30 minor)

**Problem (restated):** `Gamer::command`'s Take arm (lib.rs:732-737) and Steal arm (lib.rs:753-758) both append `placings_log(&self.placings(), Some(&scores))` when the move finished the game. The Roll arm (lib.rs:711-722) does not — yet the game can legitimately finish there: `roll_dice_cmd` calls `take_worst()` at lib.rs:612 when the rolls are exhausted and the player can neither take nor steal, and `take_worst` can remove the last tile of the last non-empty pile, making `is_finished()` (lib.rs:282-284) true. The player then sees `log_game_end()`'s scores table but the structured placings entry — which every other terminal path in this crate and in sibling crates emits — is missing.

**Fix (re-derived):** add the same block to the Roll arm. The finding's "hoist the check after the match" alternative is rejected: the arms differ in ownership of `logs`, and duplicating six lines that already appear twice verbatim is lower-risk than restructuring all three arms.

**Edge cases:**
- The Roll arm's `let logs = …` must become `let mut logs = …`.
- The vast majority of rolls do not finish the game; `is_finished()` gates the whole block, so nothing changes for them.
- `placings()`/`player_score` are read-only, so appending the log cannot change game state or the RNG stream.
- `can_undo: false` stays `false` (every sushizock arm sets it false).

**Files:**
- Modify: `rust/game/sushizock-2/src/lib.rs` (`Gamer::command`'s Roll arm, lines 711-722; new test in the inline `mod test`)

**Steps:**

- [ ] Write the failing test. Add to `mod test` in `rust/game/sushizock-2/src/lib.rs`:

```rust
    #[test]
    fn test_roll_finishing_via_take_worst_logs_placings() {
        // c F30: the game can finish inside `roll_dice_cmd` via the forced
        // `take_worst`, and that path must emit the placings log like the
        // take and steal arms do.
        let (mut g, _) = Game::start(2, 1).unwrap();
        let n = names();
        // One blue tile left, no red tiles.
        g.blue_tiles = vec![Tile {
            kind: TileType::Blue,
            value: 2,
        }];
        g.red_tiles = vec![];
        g.player_blue_tiles = vec![vec![], vec![]];
        g.player_red_tiles = vec![vec![], vec![]];
        g.current_player = MICK;
        g.remaining_rolls = 1;
        // Two sushi already kept means `sushi >= 2 > blue_tiles.len()`, so
        // `can_take_blue` stays false whatever the re-roll produces; red is
        // empty so `can_take_red` is false; nobody else holds tiles so
        // `can_steal` is false. That forces `take_worst`.
        g.kept_dice = vec![DieFace::Sushi, DieFace::Sushi];
        g.rolled_dice = vec![DieFace::Bones, DieFace::Bones];

        let resp = g.command(MICK, "roll 1", &n).unwrap();
        assert!(g.is_finished(), "the forced take must empty the last pile");
        let text = resp
            .logs
            .iter()
            .map(|l| brdgme_markup::plain(&brdgme_markup::transform(&l.content, &[])))
            .collect::<Vec<String>>()
            .join("\n");
        assert!(
            text.contains("Final scores:"),
            "the roll path must emit the placings log, got: {}",
            text
        );
    }
```

- [ ] Run: `cargo test -p sushizock-2 finishing_via_take_worst` — expected FAIL on the `text.contains("Final scores:")` assertion (the game does finish and `log_game_end`'s table is present, but the placings entry is not).
- [ ] Implement: in `Gamer::command`'s Roll arm (lib.rs:711-722), change `let logs = self.roll_dice_cmd(player, &dice)?;` to `let mut logs = …` and insert the finish block after it, so the arm reads:

```rust
            Ok(ParseOutput {
                remaining,
                value: Command::Roll(dice),
                ..
            }) => {
                let mut logs = self.roll_dice_cmd(player, &dice)?;
                // The game can finish here via the forced `take_worst`
                // inside `roll_dice_cmd`; emit the same placings log as the
                // take and steal arms (c F30).
                if self.is_finished() {
                    let scores: Vec<(usize, i32)> = (0..self.players)
                        .map(|p| (p, self.player_score(p)))
                        .collect();
                    logs.push(placings_log(&self.placings(), Some(&scores)));
                }
                Ok(CommandResponse {
                    logs,
                    can_undo: false,
                    remaining_input: remaining.to_string(),
                })
            }
```

- [ ] Run: `cargo test -p sushizock-2` — the new test PASSES and everything else PASSES (in particular `test_roll`, `test_roll_must_keep_one`, `test_roll_invalid_die_number`, `test_roll_wrong_player`).
- [ ] `cargo clippy -p sushizock-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/sushizock-2/src/lib.rs` ; message: `fix(sushizock-2): log placings when the roll path ends the game (c F30, WP-21)`

---

### Task 9: remove the `.unwrap()` from `roll_dice` (c F32 nit)

**Problem (restated):** `roll_dice` (lib.rs:150-152) is `(0..n).map(|_| *DIE_FACES.choose(rng).unwrap()).collect()`. `choose` returns `None` only for an empty slice and `DIE_FACES` is a 6-element const, so it is infallible — but `docs/CODING.md` bans `.unwrap()` outright on request-reachable runtime paths, and `roll_dice` is on one (`start_turn`, `roll_dice_cmd`).

**Fix (re-derived, and verified stream-safe):** index with a ranged random, exactly as sibling crates do (`game/zombie-dice-2/src/lib.rs:80`, `game/roll-through-the-ages-2/src/lib.rs:645`). This is safe for **live saved games** — `Game.rng` is persisted, so a change in RNG consumption would silently alter every future roll of every in-flight game. Verified against the pinned `rand 0.10.2` source that it does not: `IndexedRandom::choose`'s body is `Some(&self[rng.random_range(..self.len())])` (`src/seq/slice.rs:52-61`), and `SampleRange for RangeTo<usize>` forwards to `sample_single(0, end)` identically to `SampleRange for Range<usize>` (`src/distr/uniform.rs:447-483`). Same distribution, same draw count.

**Edge cases:**
- `DIE_FACES.len()` is used rather than a literal `6`, so adding or removing a face cannot desynchronize the range.
- `DieFace` is `Copy`, so indexing yields a value directly and the leading `*` goes away.
- No new test: there is no observable behaviour change. The whole existing suite exercises `roll_dice` through `start_turn` on every `Game::start`, and the seeded tests (`test_take_blue` compares against `g.blue_tiles[1]`, `test_roll` counts rolls) are the regression fence for the stream claim — **if any seeded test changes outcome, the change is wrong; revert rather than updating the expectation.**

**Files:**
- Modify: `rust/game/sushizock-2/src/lib.rs` (`roll_dice`, lines 150-152)

**Steps:**

- [ ] Replace `roll_dice` (lib.rs:150-152) with:

```rust
fn roll_dice(rng: &mut GameRng, n: usize) -> Vec<DieFace> {
    // Indexed rather than `choose(..).unwrap()`: CODING.md bans `.unwrap()`
    // on request-reachable paths. `IndexedRandom::choose` is itself
    // `self[rng.random_range(..self.len())]` in rand 0.10, so this draws
    // identically and does not shift the dice of any in-flight saved game
    // (c F32).
    (0..n)
        .map(|_| DIE_FACES[rng.random_range(0..DIE_FACES.len())])
        .collect()
}
```

- [ ] Run: `cargo test -p sushizock-2` — full suite PASSES with **no** expectation changes. If a seeded test fails, STOP: the draw pattern changed and the fix must be reverted, not the test.
- [ ] Check whether `rand::prelude::*` (lib.rs:15) still has a live user after `choose` is gone: `shuffle` at lib.rs:647-648 and `SeedableRng::seed_from_u64` at :644 both come from it, so the import stays. If clippy reports an unused import, remove only what it names.
- [ ] `cargo clippy -p sushizock-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/sushizock-2/src/lib.rs` ; message: `refactor(sushizock-2): drop the unwrap in roll_dice (c F32, WP-21)`

---

### Task 10: de-duplicate `take_worst` and the colour-paired take/steal bodies (c F33 + c F34 nits) + final gate

**Problem (restated):**
- c F33 — both branches of `take_worst` (lib.rs:529-546 and :547-565) hand-roll "index of the minimum value" with `min_idx`/`min_val`, and the else branch indexes `self.blue_tiles[0]` (lib.rs:549), safe today only via the non-local invariant that `take_worst` is unreachable once both piles are empty (`command_parser` returns `None` for a finished game, `command.rs:19-21`).
- c F34 — `take_blue`/`take_red` (lib.rs:399-431) and `steal_blue`/`steal_red` (lib.rs:433-515) are near-verbatim duplicates differing only in guard, message, pile pair, and (for take) whether the removal index comes from `sushi` or `bones`. The finding's stated motivation is concrete: the c F29 overflow had to be found and fixed twice.

**Fix (re-derived, ADJUSTED to preserve the public API):** the four `pub fn`s stay as one-line wrappers over two private helpers keyed on `TileType`, so `Gamer::command` (lib.rs:729-730, :750-751) and all existing tests are unaffected. `take_worst` uses `min_by_key`, whose documented tie behaviour ("if several elements are equally minimum, the first is returned") matches the existing strict-`<` loop exactly, and whose `Option` return removes the `blue_tiles[0]` indexing.

**Edge cases:**
- **Tie behaviour must not change**: `min_by_key` returns the first minimum, as the strict-`<` loop did. `test_take_worst_red_picks_minimum` (lib.rs:1686) and `test_take_worst_blue_when_no_red` (lib.rs:1715) are the fence.
- Both piles empty: previously a latent panic, now an explicit early `return vec![]` with a comment. This is not a silent reshape — it is unreachable (the game would already be finished and `command_parser` returns `None`), and the alternative would be to invent a log for a take that did not happen.
- Borrow checker: compute the `can_*` booleans and the removal index **before** taking a `&mut` to the pile vectors, otherwise `self` is borrowed twice.
- Error message strings must be preserved **verbatim** — `test_take_blue`/`test_take_red`/`test_steal_from_self`/`test_steal_from_empty_player` and the c F29 tests match on them.
- Do **not** collapse the `n == 1` vs `n != 1` guard structure in `steal`: the two branches select different `can_steal_*` predicates and produce different messages.
- Running this task **after** Task 7 means the validated `n` guard exists in exactly one place afterwards. Task 7's three tests are the safety net for this refactor.

**Files:**
- Modify: `rust/game/sushizock-2/src/lib.rs` (`take_blue`, `take_red`, `steal_blue`, `steal_red`, `take_worst`)

**Steps:**

- [ ] Replace `take_blue` and `take_red` (lib.rs:399-431) with a shared private helper plus two wrappers:

```rust
    fn take(&mut self, player: usize, kind: TileType) -> Result<Vec<Log>, GameError> {
        let counts = self.dice_counts_all();
        let (allowed, idx, message) = match kind {
            TileType::Blue => (
                self.can_take_blue(player),
                counts.sushi,
                "unable to take blue at the moment",
            ),
            TileType::Red => (
                self.can_take_red(player),
                counts.bones,
                "unable to take red at the moment",
            ),
        };
        if !allowed {
            return Err(GameError::invalid_input(message));
        }
        // `can_take_*` guarantees `idx >= 1` and `pile.len() >= idx`.
        let idx = idx - 1;
        let t = match kind {
            TileType::Blue => {
                let t = self.blue_tiles.remove(idx);
                self.player_blue_tiles[player].push(t);
                t
            }
            TileType::Red => {
                let t = self.red_tiles.remove(idx);
                self.player_red_tiles[player].push(t);
                t
            }
        };
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" took "),
            N::Bold(vec![render::tile(&t)]),
        ])];
        logs.extend(self.next_player());
        Ok(logs)
    }

    pub fn take_blue(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.take(player, TileType::Blue)
    }

    pub fn take_red(&mut self, player: usize) -> Result<Vec<Log>, GameError> {
        self.take(player, TileType::Red)
    }
```

- [ ] Run: `cargo test -p sushizock-2` — full suite PASSES (this step is behaviour-preserving; if `test_take_blue`/`test_take_red`/`test_take_advances_turn` fail, the index or pile selection was mismatched).
- [ ] Replace `steal_blue` and `steal_red` (lib.rs:433-515, post-Task-7 bodies) with a shared private helper plus two wrappers:

```rust
    fn steal(
        &mut self,
        player: usize,
        target: usize,
        kind: TileType,
        n: Option<i32>,
    ) -> Result<Vec<Log>, GameError> {
        let n = n.unwrap_or(1);
        let (allowed, message) = match (kind, n == 1) {
            (TileType::Blue, true) => (
                self.can_steal_blue(player),
                "can't steal a blue tile at the moment",
            ),
            (TileType::Blue, false) => (
                self.can_steal_blue_n(player),
                "can't steal a hidden blue tile at the moment",
            ),
            (TileType::Red, true) => (
                self.can_steal_red(player),
                "can't steal a red tile at the moment",
            ),
            (TileType::Red, false) => (
                self.can_steal_red_n(player),
                "can't steal a hidden red tile at the moment",
            ),
        };
        if !allowed {
            return Err(GameError::invalid_input(message));
        }
        if player == target {
            return Err(GameError::invalid_input("can't steal from yourself"));
        }
        let len = match kind {
            TileType::Blue => self.player_blue_tiles[target].len(),
            TileType::Red => self.player_red_tiles[target].len(),
        };
        if len == 0 {
            return Err(GameError::invalid_input(match kind {
                TileType::Blue => "they don't have any blue tiles to steal",
                TileType::Red => "they don't have any red tiles to steal",
            }));
        }
        // Validate before any arithmetic: `n` comes straight from
        // `Int::any()`, so `len as i32 - n` overflows for n = i32::MIN.
        // Accepts exactly 1..=len (c F29).
        if n < 1 || n as usize > len {
            return Err(GameError::invalid_input(format!(
                "invalid tile number, you need to pick something between 1 and {}",
                len
            )));
        }
        let idx = len - n as usize;
        let t = match kind {
            TileType::Blue => {
                let t = self.player_blue_tiles[target].remove(idx);
                self.player_blue_tiles[player].push(t);
                t
            }
            TileType::Red => {
                let t = self.player_red_tiles[target].remove(idx);
                self.player_red_tiles[player].push(t);
                t
            }
        };
        let mut logs = self.steal_log(player, target, &t);
        logs.extend(self.next_player());
        Ok(logs)
    }

    pub fn steal_blue(
        &mut self,
        player: usize,
        target: usize,
        n: Option<i32>,
    ) -> Result<Vec<Log>, GameError> {
        self.steal(player, target, TileType::Blue, n)
    }

    pub fn steal_red(
        &mut self,
        player: usize,
        target: usize,
        n: Option<i32>,
    ) -> Result<Vec<Log>, GameError> {
        self.steal(player, target, TileType::Red, n)
    }
```

- [ ] Run: `cargo test -p sushizock-2` — full suite PASSES, including Task 7's three tests and `test_steal_blue`, `test_steal_red`, `test_steal_red_n_not_allowed`, `test_steal_blue_n`, `test_steal_red_n`, `test_steal_from_self`, `test_steal_from_empty_player`, `test_steal_advances_turn`.
- [ ] Replace `take_worst` (lib.rs:527-566) with:

```rust
    pub fn take_worst(&mut self) -> Vec<Log> {
        let player = self.current_player;
        let kind = if self.red_tiles.is_empty() {
            TileType::Blue
        } else {
            TileType::Red
        };
        let pile = match kind {
            TileType::Blue => &mut self.blue_tiles,
            TileType::Red => &mut self.red_tiles,
        };
        // Unreachable with both piles empty: the game would be finished and
        // `command_parser` returns `None`. `min_by_key` keeps the FIRST
        // minimum, matching the hand-rolled strict-`<` loop it replaces
        // (c F33).
        let Some((min_idx, _)) = pile.iter().enumerate().min_by_key(|(_, t)| t.value) else {
            return vec![];
        };
        let t = pile.remove(min_idx);
        match kind {
            TileType::Blue => self.player_blue_tiles[player].push(t),
            TileType::Red => self.player_red_tiles[player].push(t),
        }
        let mut logs = vec![Log::public(vec![
            N::Player(player),
            N::text(" is forced to take "),
            N::Bold(vec![render::tile(&t)]),
        ])];
        logs.extend(self.next_player());
        logs
    }
```

- [ ] Run: `cargo test -p sushizock-2` — full suite PASSES, in particular `test_take_worst_red_picks_minimum`, `test_take_worst_blue_when_no_red`, `test_force_take_most_negative_red` (lib.rs:866), `test_force_take_lowest_blue` (lib.rs:897) and Task 8's `test_roll_finishing_via_take_worst_logs_placings`.
- [ ] `cargo clippy -p sushizock-2 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Final package gate: run `/home/beefsack/Development/brdgme/scripts/rust-test.sh` — must pass (it provisions the Postgres/NATS containers the DB-backed web tests need).
- [ ] Commit: `git add rust/game/sushizock-2/src/lib.rs` ; message: `refactor(sushizock-2): share the take/steal colour bodies and take_worst min search (c F33 c F34, WP-21)`

---

## Findings disposition

| Finding | Severity | Original recommendation | Disposition | Reason |
|---|---|---|---|---|
| c F22 `Box::leak` per parser construction | major | Make `LocChoice.name` a `String`, or cache 100 `LocChoice`s in a `OnceLock` | ADJUSTED | Both offered options keep a wrapper struct that is not needed at all: `Enum` requires only `ToString + Clone` (parser/mod.rs:551-576) and `Loc: Copy + Display` already forwards to `to_key()` (loc.rs:118-122), so `Enum::partial(loc::all_locs())` is byte-identical in grammar and spec with zero retained allocation (Task 1). |
| c F23 cathedral traversable by the capture flood-fill | minor -> **nit** (verification) | Decide whether this is preserved-defect #5; comment it, **or** block the walk on `PLAYER_CATHEDRAL` | OVERTURNED (code change) / CONFIRMED (comment) | The "undocumented" premise is refuted: the crate's own RULES.md:59-83 documents the cathedral as a capturable piece *identity inside* an enclosed region, exactly matching the code. Blocking the walk would contradict the crate's rules docs, break Go parity, and break `capture_returns_piece_but_never_counts_cathedral`. **Comment only** (Task 5). |
| c F24 dead `parse_loc` | minor | Delete it, **or** keep it with a doc comment plus a unit test | ADJUSTED | Deleted. Re-verified zero callers workspace-wide. The keep-and-test option is rejected: an unreachable second grammar for locations is the exact divergence hazard the finding names (Task 4). |
| c F25 `pieces()` panics on a bad player index | minor | Return an empty `Vec` (or make it `Option`/`Result`) for out-of-range players | ADJUSTED (the empty-`Vec` half REJECTED) | The empty-`Vec` form (which the stray Edit C implemented) closes the panic but makes `pieces()` total-but-lying: it cannot distinguish "player 2 does not exist" from "no pieces left", silently turning `remaining_piece_size(2)` into a scoring `0` and rendering a fictitious player panel. Spec takes the `Option` half plus real boundary validation: `player_pieces` gates on `self.players`, `command_parser` (shared by `Gamer::command`+`command_spec`) returns `None`, `Gamer::command` returns `GameError::Internal`, `remaining_piece_size` returns `Option<i32>`, and the render — which cannot error, since `player_state` returns no `Result` — shows an explicit marker (Task 3). |
| c F25 rider: `ortho_dir_name` / `wall_char` panics | nit (within F25) | Acceptable as-is; optionally `debug_assert!` + safe fallback | SKIPPED | Re-derived unreachable: `ortho_dir_name`'s only runtime caller is `play`'s log with a `dir` from `dir_parser` (an `Enum` over `ORTHO_DIRS`) or the `DIR_DOWN` default; `wall_char` panics only for `dir == 0`, and every call site — including `render_corner`'s `corner_map` construction — passes a nonzero combination. No code change. |
| c F26 `Loc::to_key` overflow | nit | Add the `loc.valid()` early return to `Game::tile_at` "mirroring Go's missing-map-key behaviour", or document the invariant | CONFIRMED (fix) / OVERTURNED (rationale) | The guard is adopted verbatim (it is also stray Edit B). The stated rationale is factually wrong and must not be carried into the code or the commit: Go's zero `Tile` is `{Player: 0, Owner: 0}`, whereas `empty_tile()` is `{-1, -1}`, so Go off-board reads yield a *player-0* tile. Correct rationale recorded instead: removes a latent overflow/garbage-key hazard and unifies the contract with render.rs's `Tiler` guard; the real legality safety net remains the separate `!l.valid()` check in `can_play_piece` (Task 2). |
| c F27 unused `rand` dependency | nit | Remove `rand` from cathedral-2's `[dependencies]`, or fold into the cross-crate boilerplate cleanup | CONFIRMED | Re-verified unused across `src/`, the 4 bins and `tests/`. Removed as a single-line edit; the broader manifest sweep stays with WP-64/WP-65 (Task 6). |
| c F28 dead `impl Display for Loc` | nit | Delete the impl, **or** keep it and use `{}` formatting in display contexts | RESOLVED BY c F22 | Not a separate fix. Task 1's `Enum::partial(loc::all_locs())` makes `Enum`'s `to_string()` the impl's live caller — the finding's second option, applied where it matters. **The impl must NOT be deleted; Task 1 depends on it.** |
| c F29 steal `n = i32::MIN` overflow | major | Validate `n` before the arithmetic (`n < 1 \|\| n as usize > len`), then `len - n as usize`; or `checked_sub` | CONFIRMED | Adopted verbatim in shape, in both copies. Proved the new guard accepts exactly the same `n` set as the old post-hoc one, that `n < 1` short-circuits before the `as usize` cast, and that the user-facing message is unchanged (Task 7). |
| c F30 roll arm misses the placings log | minor | Add the same `if self.is_finished() { … }` block to the Roll arm, **or** hoist the check after the match | ADJUSTED | Block added to the Roll arm only. Hoisting is rejected: the three arms differ in `logs` ownership, so restructuring all three to save six duplicated lines is more risk than value (Task 8). |
| c F31 `roll`'s bounded `Many` suggest overrun | minor | No crate-local fix; resolves with the tracked lib/game `Many`-ignores-`max` suggest bug | FENCED OUT | Owned by **WP-03** via **lg F9** (`rust/lib/game/src/command/suggest.rs:109` drops `min`/`max`). Do not touch `roll_parser` or `suggest.rs` here. |
| c F32 `.unwrap()` in `roll_dice` | nit | `DIE_FACES[rng.random_range(0..DIE_FACES.len())]`, or `unwrap_or(&DieFace::Sushi)` | CONFIRMED | Took the indexing form after **verifying it is RNG-stream-identical** against the pinned rand 0.10.2 source (`choose` is itself `self[rng.random_range(..len)]`; `RangeTo<usize>` and `Range<usize>` both reach `sample_single(0, end)`). This mattered because `Game.rng` is persisted — a stream change would silently alter in-flight games (Task 9). |
| c F33 `take_worst` hand-rolled min loops | nit | Extract the min index via `min_by_key`; share the branch bodies; or at minimum comment the non-empty precondition | CONFIRMED | All three, in one edit. Verified `min_by_key`'s first-minimum tie behaviour matches the strict-`<` loop, so `test_take_worst_red_picks_minimum` stays green, and the `Option` return removes the `blue_tiles[0]` indexing (Task 10). |
| c F34 take/steal colour pairs are duplicates | nit | Low priority; if touched, factor into private `take(kind)`/`steal(kind, target, n)` and keep the public wrappers | ADJUSTED | Done exactly as suggested, but **sequenced after c F29** so the validated steal guard ends up in one place — which is the finding's own stated motivation ("the i32::MIN overflow had to be spotted twice"). All four `pub fn`s survive as one-line wrappers so `Gamer::command` and every existing test are untouched (Task 10). |

## Cross-package / newly discovered

Recorded, **not fixed** here. **The Lead's rulings are recorded inline below and are binding.**

1. **`Player {}` parser yields an unbounded target index (sushizock-2, potential panic).** `Parser for Player` (`rust/lib/game/src/command/parser/mod.rs:743-776`) builds its `Enum` from the `names` slice and returns that index, bounded only by `names.len()` — never by `self.players`. `steal_blue`/`steal_red` then index `self.player_blue_tiles[target]` / `self.player_red_tiles[target]` (lib.rs:454, :496) and push into `[player]`. If the platform ever passes more names than the game has players, a legal-looking `steal <extra-name> blue` panics with an index-out-of-bounds. Existing tests never expose it (`names()` returns 3 names and those games are started with 3 players). The fix is one guard — `if target >= self.players { return Err(GameError::invalid_input("that is not a player in this game")); }` right after the self-steal check — and Task 10 restructures that exact function, so it is cheap to fold in **if ruled in**. Same defect class as c F25 (player-index boundary), but it is a *different crate* and a *different index source* (parser output, not the request's player field), and it is not in any finding. **LEAD RULING: ROUTED TO WP-09** (deserialized-state trust hardening, BLOCKED-ON-DECISION D-36) — its declared paths already include `lib/cmd/src/requester/gamer.rs` plus a per-crate sweep, and its scope note already covers "the two request-reachable `player_state` panics ... covered by one bounds check at gamer.rs". **`sushizock-2` must be ADDED to WP-09's crate list**, with this `target`-index site named. It is explicitly **NOT** folded into WP-21 Task 10: absorbing an unfiled fix into a fixed-scope package is the scope drift this review discipline forbids (same ruling as WP-19's two `panic!("must be Phase::SellOrTrade")` sites). Task 10 must NOT add the guard.
2. **`can_play_something` rebuilt the piece catalogue inside its 100-iteration location loop (cathedral-2, efficiency).** `pieces(player)` at lib.rs:360 sits inside `for l in loc::all_locs()`, allocating a 14- or 15-element `Vec<Piece>` (each with its own `positions: Vec<Loc>`) up to 100 times per call — and `can_play_something` is itself called up to twice per `can_play`, plus from `next_player`, `whose_turn_players` and `status`. Not a filed finding. Task 3 **hoists it out of the loop as a required consequence** of the `Option` shaping (the guard has to run once, before the loop), and the step calls this out explicitly rather than smuggling it in. Flagged here so the Lead knows Task 3 carries a performance change as well as a correctness one; no separate routing needed.
3. **`Gamer::player_state` has no way to reject an invalid player, in any game crate.** The trait signature is `fn player_state(&self, player: usize) -> Self::PlayerState` (`rust/lib/game/src/game.rs:52`), and `rust/lib/cmd/src/requester/gamer.rs:170-182` forwards the request's player straight into it plus `Renderer::render`. Every game crate therefore has to make its renderer total for arbitrary indices, or panic. cathedral-2 gets the marker treatment in Task 3, but the structural gap is workspace-wide. **LEAD RULING: ROUTED TO WP-09**, which already owns `lib/cmd/src/requester/gamer.rs` and whose scope note already frames it as "one bounds check at gamer.rs" for the request-reachable `player_state` panics (e F18/F36). WP-09's spec writer should treat this as the general statement of that item: validate `player < game.player_count()` in `handle_player_render` (and the command path) so the class retires in one place. Note that **WP-06's spec (lib/cmd tools and http) is already finalized and must NOT be retro-edited** to carry this. cathedral-2 still takes Task 3's local marker regardless — WP-21 must not depend on WP-09 landing.
