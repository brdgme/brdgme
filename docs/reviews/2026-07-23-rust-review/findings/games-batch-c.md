# Findings: games-batch-c — texas-holdem-2, acquire-1, cathedral-2, sushizock-2

Review of the detached snapshot worktree (`/home/beefsack/Development/brdgme-review-snapshot`,
HEAD `f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`). Paths are relative to `rust/`;
line numbers match the snapshot. Review-only; no code was changed.

Cross-reference material: `brdgme-go/texas_holdem_1` + `brdgme-go/libpoker`,
`brdgme-go/cathedral_1`, and `brdgme-go/sushizock_1` exist in the snapshot and
were diffed against. **No Go source exists for Acquire** — acquire-1 is a fresh
implementation, so its rules findings are judged against the official Acquire
rulebook and say so explicitly.

Deliberately preserved, in-code-documented Go quirks were verified against the
Go source and are NOT reported as findings (notably texas-holdem-2's
`can_raise`/`largest_raise`, `cards_by_suit` sort skip, `pop_n` semantics,
fixed-width rank padding, fold loop flattening; cathedral-2's preserved
defects #1–#4 and the `walk` queued-set quirk). The four boilerplate binaries
per crate were verified byte-identical (modulo crate name) to the standard
pattern in all four crates — no deviations; the systemic duplication itself is
tracked in the dependencies unit.

Totals: **34 findings — 0 critical, 4 major, 15 minor, 15 nit.**
By category: correctness 16, simplicity 6, quality 5, consistency 5, dependencies 2.

## texas-holdem-2

Core logic (hand evaluation, betting rounds, blinds, side-pot construction,
folded-player absorption, uncalled-bet return, uneven-split remainder, heads-up
dealer/blind order) was traced line-by-line against `texas_holdem_1` and
`libpoker` and is a faithful 1:1 port. No panic path is reachable from crafted
player input; serde views leak nothing private. 6 findings (2 minor, 4 nit).

### Raise parser min bound diverges from Go, and the "Go quirk preserved" comment is factually wrong
- severity: minor
- category: correctness
- location: game/texas-holdem-2/src/command.rs:41-51
- finding: The doc comment on `raise_parser` claims "Go quirk preserved: the `Int` bound's `min` is `g.LargestRaise`, not `g.MinRaise()`". This is false: Go's `RaiseParser` (`brdgme-go/texas_holdem_1/command.go:174`) uses `min := g.MinRaise()`. (The comment's author appears to have conflated it with Go's `CanRaise`, `texas_holdem.go:328`, which genuinely does use `g.LargestRaise` — that quirk is real and correctly preserved in `lib.rs:298-310`.) So the Rust parser uses `self.largest_raise` (line 51) where Go used `max(minimum_bet, largest_raise)`. Concrete effect: pre-flop after blinds, `largest_raise` is 5 (big blind over small blind) while `MinRaise()` is 10, so the parser accepts `raise 5`..`raise 9` which `Game::raise` (lib.rs:282-287) then rejects with "Your raise must be at least 10". Go rejected these at parse time. No state corruption (the action re-validates), but it is a real behavioural divergence from the source, and the incorrect comment will mislead anyone auditing port fidelity.
- recommendation: Change `let min = self.largest_raise;` to `let min = self.min_raise();` and rewrite the comment to state that the parser bound matches Go's `g.MinRaise()` (optionally noting the genuine `CanRaise`/`LargestRaise` quirk lives in `lib.rs`).

### Max player count is 8, Go original supports 9 — undocumented divergence
- severity: minor
- category: consistency
- location: game/texas-holdem-2/src/lib.rs:33
- finding: `MAX_PLAYERS: usize = 8`, so `Game::start` rejects 9 players and `player_counts()` returns 2..=8. The Go original allows 2-9 (`texas_holdem.go:58` "Texas hold 'em is limited to 2 - 9 players", and `PlayerCounts()` returns 2..=9). Nothing in the deck math requires 8 (9 players needs only 18 hole + 5 community cards). If the reduction is deliberate (e.g. UI constraints) it is undocumented — there is no comment, and the crate doc header claims a straight port of `texas_holdem_1`.
- recommendation: Either restore `MAX_PLAYERS = 9` for Go parity, or add a short comment documenting why the port deliberately caps at 8.

### `bet_up_to` uses `.expect()` in a runtime path
- severity: nit
- category: consistency
- location: game/texas-holdem-2/src/lib.rs:158
- finding: `self.bet(player_num, bet_amount).expect("BetUpTo always bets an affordable amount")`. The invariant holds today (`bet_amount = amount.min(player_money)`), so the panic is unreachable, and Go panicked in the equivalent spot (`texas_holdem.go:205-212`). But docs/CODING.md says no `.expect()` in runtime paths; the invariant is also only locally true — a future edit to `bet_up_to` could make it reachable from blinds posting during `new_hand`.
- recommendation: Either restructure so `bet` cannot fail here (e.g. an infallible internal clamping helper), or keep as Go-mirroring with an explicit comment noting the style-rule exception.

### Documented Go-mirroring panics in `next_player_in_set` and `pop_n`
- severity: nit
- category: consistency
- location: game/texas-holdem-2/src/lib.rs:144
- finding: `next_player_in_set` has `assert!(!set.is_empty(), "No players in set")` (lib.rs:144) and a trailing `panic!("Could not find any valid players")` (lib.rs:151); `card::pop_n` panics `"Not enough cards to pop"` (card.rs:106). All three mirror Go panics and are documented as such. Every call site was traced: `next_player_in_set` is only invoked with non-empty sets guarded by callers, and `pop_n` underflow is impossible (52-card deck, <= 8 players * 2 hole + 5 community = 21 max). Not reachable from crafted player input — noted only because CODING.md discourages panics in runtime paths.
- recommendation: No change required for correctness; if the panic-free rule is ever enforced strictly, convert to `Result`/debug_assert. Low priority.

### `HandResult.category: Option<Category>` is redundant with the `Category::None` variant
- severity: nit
- category: simplicity
- location: game/texas-holdem-2/src/poker.rs:31
- finding: `Category` has a `None` variant (mirroring Go's `CATEGORY_NONE = 0`, needed for `hand_score` to produce 0), yet `HandResult.category` wraps it in `Option`, forcing `unwrap_or(Category::None)` at both use sites (poker.rs:39, 54). Two representations of "no category" in one type.
- recommendation: Drop the `Option`, default `HandResult.category` to `Category::None` (via `#[derive(Default)]` on the enum with `#[default]` on `None`).

### Placings-log block duplicated across all five `command()` arms
- severity: nit
- category: quality
- location: game/texas-holdem-2/src/lib.rs:719
- finding: The identical 8-line `if self.is_finished() { ... placings_log ... }` block plus the `Ok(CommandResponse { ... })` construction is copy-pasted into each of the five match arms (lib.rs:719-803), with the only differences being the action called and `can_undo` (false everywhere except Raise). Any future change to finished-game logging has to be made in five places.
- recommendation: Restructure to bind `(logs, can_undo)` per arm in a small match, then run the finished/placings logic and `CommandResponse` construction once.

## acquire-1

No Go port exists; rules were judged against the official Acquire rulebook.
Merger resolution (largest-wins, tie broken by tile-placer, multi-chain
sequential resolution, safe-corp rules), majority/minority bonus splitting
across all tie shapes (incl. sole-shareholder double bonus, rounded UP to the
nearest $100 per the rulebook), pricing tiers, SAFE_SIZE=11/GAME_END_SIZE=41,
founder free share, 3-share buy limit, 2-for-1 trades bounded by bank stock,
final scoring, and tile privacy were all verified correct. 15 findings
(2 major, 8 minor, 5 nit).

### player_counts() excludes 6 players despite MAX_PLAYERS = 6
- severity: major
- category: correctness
- location: game/acquire-1/src/lib.rs:313
- finding: `fn player_counts()` returns `(2..6).collect()`, i.e. `[2, 3, 4, 5]`. The half-open range excludes 6, but `MAX_PLAYERS` is 6 (lib.rs:25) and `start()` accepts 6 (`(MIN_PLAYERS..=MAX_PLAYERS).contains(&players)`, lib.rs:186). `player_counts()` is the `Gamer` trait's advertised set of supported player counts, so the lobby/service layer will never offer or allow a 6-player acquire game even though the engine fully supports it. Acquire is a 3-6 player game (2 with the dummy variant), so 6 is the headline player count.
- recommendation: Change to `(MIN_PLAYERS..=MAX_PLAYERS).collect()`.

### 2-player dummy shareholder die roll can never be a 6
- severity: major
- category: correctness
- location: game/acquire-1/src/lib.rs:902
- finding: In 2-player games `bonus_players()` rolls the dummy shareholder's holding with `self.rng.random_range(1..=5)`, i.e. a uniform 1-5. The game's own start log says "A dice (D6) is rolled to determine the dummy player's shares" (lib.rs:221-223), and the official Acquire 2-player variant rolls one standard six-sided die per chain when bonuses are calculated. As written the dummy is systematically weaker than the rules state (never holds 6 shares), which shifts majority/minority outcomes in 2-player games. Judged against the official rulebook (no Go port exists).
- recommendation: Use `self.rng.random_range(1..=6)`.

### panic! in pay_bonuses on empty major-bonus list
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:841
- finding: `pay_bonuses()` does `if major_len == 0 { panic!("expected some major bonus players") }`. This is a runtime path executed during merges and game end; a panic in the game service kills the HTTP worker, and the project style rule forbids `panic!` in request-reachable paths. In practice it appears unreachable (a corp on the board always has its founder holding >=1 share at bonus time, and 2-player games always push the dummy into `major`), but "appears unreachable" is exactly what `GameError::Internal` is for.
- recommendation: Return `Result` and surface `GameError::Internal { message: "no major bonus players" }` instead of panicking.

### expect() cluster panics on legacy/corrupt state missing HashMap keys
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:683 (also 1006, 1026, 1076, 1087, 1122, 1139); game/acquire-1/src/command.rs:59 and :163; game/acquire-1/src/render.rs:78
- finding: Numerous `.expect("could not get player shares")` / `.expect("could not get corp share count")` calls on `players[p].shares.get(&corp)` and `shares.get(&corp)`. Fresh games pre-populate all 7 corp keys (`corp_hash_map`, lib.rs:1224), so these only fire on deserialized states that lack a key — but serde happily deserializes a HashMap with missing keys, and this crate already carries a migration shim for legacy games (`rng` field, lib.rs:156-159), so legacy states are a real concern. render.rs:78 (`self.shares.get(c).expect("expected corp to have shares")`) is in the render path, where a panic is even less acceptable. The code is also internally inconsistent: `handle_buy_command` (lib.rs:616) and the render player table (render.rs:138) use `.get(&corp).cloned().unwrap_or(0)` for the same lookups. Also note the typo "could not et player shares" at command.rs:163.
- recommendation: Standardize on `.get(&corp).copied().unwrap_or_default()` everywhere; fix the typo.

### "Trades" stat reports the merge count
- severity: minor
- category: correctness
- location: game/acquire-1/src/stats.rs:46
- finding: `s.insert("Trades".to_string(), Stat::Int(self.merges as i32));` — copy-paste of the line above; should be `self.trades`. `stats.trades` is maintained in `handle_trade_command` (lib.rs:1095) but never surfaced correctly.
- recommendation: Use `self.trades as i32`.

### Stats are tracked but never surfaced (dead code)
- severity: minor
- category: quality
- location: game/acquire-1/src/lib.rs:238; game/acquire-1/src/stats.rs:27
- finding: `status()` returns `Status::Finished { placings, stats: vec![] }`, and `Stats::to_brdgme_stats()` has no callers anywhere in the workspace. The entire per-player stats bookkeeping (`Stats` struct, ~15 fields updated across lib.rs) is write-only dead weight as shipped. Either wire it into `status()` stats or delete it; as-is it also hides the "Trades" bug above.
- recommendation: Return the stats from `status()` via `to_brdgme_stats()`, or remove the stats machinery.

### Start player chosen randomly instead of by initial tile draw
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:213
- finding: Official Acquire setup: each player draws one tile and places it on the board; the player whose tile is closest to 1-A (row letter then number) plays first. The code places one random tile per player (lib.rs:200-202) but then picks the start player with `g.rng.random_range(0..players)`. A common digital simplification, but it is a deviation from the rulebook. Judged against the official rulebook (no Go port exists).
- recommendation: Either derive the start player from the initially placed tiles (lowest row, then lowest col), or note the deviation in RULES.md.

### Full-hand redraw discards temporarily-unplayable tiles
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:693-735; game/acquire-1/src/board.rs:130-142
- finding: `start_turn()` treats a tile as unplayable via `assert_loc_playable()`, which rejects both (a) tiles merging two safe corps (permanently unplayable per the rulebook) and (b) tiles that would found a chain when all 7 corps are on the board (only *temporarily* unplayable — they become legal again after any merger frees a corp). If every tile in hand is unplayable, `redraw_hand()` permanently discards the whole hand (`set_discarded`) and redraws, including type-(b) tiles. The end-of-turn discard in `draw_replacement_tiles()` (lib.rs:377-380) correctly discards only type-(a) tiles. The wholesale redraw rule itself exists in later Hasbro editions but not the classic 3M/AH rulebooks, so this is an edition choice worth confirming. Judged against the official rulebook (no Go port exists).
- recommendation: Confirm the intended edition; if keeping the redraw, consider only discarding permanently-unplayable tiles and redrawing the rest, or document the house rule in RULES.md.

### Tile-bag exhaustion ends the game immediately
- severity: minor
- category: correctness
- location: game/acquire-1/src/lib.rs:403-408
- finding: `draw_replacement_tiles()` calls `self.end()` when the bag can't refill the hand to 6, ending the game mid-turn. The official rulebook ends the game by player declaration once end conditions are met; behavior on bag exhaustion differs by edition (some end the game, some continue without drawing). Worth an explicit decision since there is no Go port to match.
- recommendation: Verify against the chosen rulebook edition; document in RULES.md.

### Unused thiserror dependency
- severity: minor
- category: dependencies
- location: game/acquire-1/Cargo.toml:14
- finding: `thiserror = "2.0.18"` is declared but never used — grep for `thiserror` across the crate matches only Cargo.toml. It is also beyond the standard game-crate dep set.
- recommendation: Remove the dependency.

### can_undo in handle_found_command is a tautology
- severity: nit
- category: simplicity
- location: game/acquire-1/src/lib.rs:586
- finding: Returns `matches!(self.phase, Phase::Buy { .. })` immediately after `self.buy_phase(player)` unconditionally set the phase to `Phase::Buy` — always `true`.
- recommendation: Return `true` (or restructure so the value is meaningful).

### unwrap() on single-element neighbouring_corps set
- severity: nit
- category: consistency
- location: game/acquire-1/src/lib.rs:466
- finding: `neighbouring_corps.iter().next().unwrap()` in the `1 =>` match arm. Safe by construction (arm guarded on `len() == 1`) but the project idiom forbids `.unwrap()` in runtime paths; a `let Some(n_corp) = ... else { return Err(GameError::Internal{..}) }` or iterating the set costs nothing.
- recommendation: Replace with a fallible extraction returning `GameError::Internal`.

### unwrap() in board render row-run logic
- severity: nit
- category: consistency
- location: game/acquire-1/src/render.rs:268-270
- finding: `start.unwrap()` (twice) inside the corp-text width scan. Safe by construction (`start` set to `Some(col)` earlier in the same branch) but sits in the render path; `if let Some(s) = start` expresses it without panic paths.
- recommendation: Restructure with `if let`.

### Full-game clone for can_end checks
- severity: nit
- category: quality
- location: game/acquire-1/src/lib.rs:1201 (also 1184); lib.rs:259
- finding: `player_can_end()` calls `self.pub_state().can_end()`, and `pub_state()` is `self.to_owned().into()` — a deep clone of the entire game (players, board, maps) just to compute three integers. This runs on every `command_parser()` build (i.e. every command/spec request). The logic in `PubState::can_end` only needs `board`, `finished`, and `last_turn`.
- recommendation: Move `can_end` onto a shared helper taking `(&Board, finished, last_turn)` and call it directly from `Game`.

### Nondeterministic corp ordering in found parser
- severity: nit
- category: consistency
- location: game/acquire-1/src/command.rs:34
- finding: `self.board.available_corps()` returns a `HashSet<Corp>` whose iteration order feeds `found_parser(Enum::partial(...))`, so suggestion / spec ordering of foundable corps varies run to run. Cosmetic only.
- recommendation: Sort by `CORPS` order before building the parser.

## cathedral-2

Placement legality, rotation math, capture counting, end-of-game and scoring
were verified line-by-line against `brdgme-go/cathedral_1`; the four documented
preserved Go defects and the `walk` quirk were confirmed in the Go source and
excluded per the review rules. No panic path is reachable from crafted player
input. 7 findings (1 major, 3 minor, 3 nit).

### Per-request memory leak in `loc_name` (Box::leak per parser construction)
- severity: major
- category: quality
- location: game/cathedral-2/src/command.rs:26
- finding: `loc_name` does `Box::leak(loc.to_key().into_boxed_str())` and the comment claims "Leaked once per process; the location set is fixed (100 entries), so this is a bounded, one-time allocation". That is wrong: `loc_parser()` (command.rs:98-107) calls `loc_name` for all 100 locs *every time it is constructed*, and `loc_parser` is built fresh on every `command_parser()` call — i.e. every `command()` and every `command_spec()` invocation. Each parse/suggest request leaks 100 freshly-allocated strings (~4-8 KB with allocator overhead) that are never reclaimed. On a long-running HTTP service where `command_spec` is hit per page load / suggest keystroke this is an unbounded leak driven by ordinary traffic. The `&'static str` is only needed because `LocChoice.name` was typed that way; nothing in the `Enum` parser requires `'static`.
- recommendation: Change `LocChoice.name` to `String` (store `loc.to_key()` directly) and drop `loc_name`/`Box::leak` entirely; or, if a static table is preferred, build the 100 `LocChoice`s once in a `std::sync::OnceLock` and clone from it. Fix the stale comment either way.

### Cathedral is traversable by the capture flood-fill (not treated as a wall)
- severity: minor
- category: correctness
- location: game/cathedral-2/src/lib.rs:283
- finding: The inner area walk in `check_captures` blocks only on `self.tile_at(l2).player == player` (the capturing player's own pieces). Cathedral tiles (`player == PLAYER_CATHEDRAL == 2`) do NOT block the walk — the flood-fill passes through the cathedral square and merges areas on both sides of it. Per official Cathedral rules an area enclosed by one player's pieces *and/or the board edge and/or the cathedral* is captured; here the cathedral cannot serve as part of an enclosure wall, so an "enclosure" completed via the cathedral instead floods through it and (if the merged area then contains >=2 distinct pieces) no capture happens where the official rules would grant one. Verified this is inherited verbatim from Go's `CheckCaptures` (`play_command.go`: same block condition), so it is Go-parity behaviour — but unlike preserved defects #1-#4 it is NOT flagged in any code comment or the suspected-defects list, so it reads as intentional-correct rather than preserved-quirk. (Related, also undocumented: the "cathedral must be placed within the central area" restriction some editions have is absent — matching Go.)
- recommendation: Decide explicitly whether this is preserved-defect #5. If yes, add a comment at lib.rs:283 noting the cathedral deliberately does not block the area walk (Go parity, deviates from official enclosure rules). If it should match official rules, also block on `t.player == PLAYER_CATHEDRAL` (and check the `pieces_found` counting still handles a cathedral enclosed alone, per defect #3's carve-out).

### Dead code: `parse_loc` is never called
- severity: minor
- category: simplicity
- location: game/cathedral-2/src/loc.rs:167
- finding: `parse_loc` (loc.rs:167-181, port of Go's `ParseLoc`) has no callers anywhere in the crate. Go used it for command parsing; the Rust port parses locations via the `Enum`-over-fixed-names parser in `command.rs` (`loc_parser`, command.rs:98), so the free-text parser is vestigial. It is `pub` so the compiler emits no dead-code warning. It would also diverge from the actual accepted command syntax if anyone wired it up.
- recommendation: Delete `parse_loc` (it remains in git history if a future free-text input path needs it), or document in its doc comment why it is being kept for port completeness and add a unit test exercising it.

### `pieces()` panics on out-of-range player index (request-adjacent)
- severity: minor
- category: correctness
- location: game/cathedral-2/src/piece.rs:110
- finding: `pieces(player)` ends in `_ => panic!("invalid player: {}", player)`. It is called from `piece_parser` (command.rs:93), `can_play_piece` (lib.rs:125), `play` (lib.rs:173), `remaining_piece_size`, `can_play_something`, and `render_player_remaining_tiles` (render.rs:359) with `player` derived from the `player: usize` argument of the `Gamer` methods `command`/`command_spec`/`player_state`. Today the service layer only passes valid player indices, but per `docs/CODING.md` this invariant is enforced only by an upstream contract, and a panic in a game crate kills the shared HTTP service. Same class: `ortho_dir_name` (loc.rs:41, panics on non-ortho dir — parser-constrained today) and `wall_char` (render.rs:85, structurally unreachable).
- recommendation: For `pieces()` specifically, return an empty `Vec` (or make it `Option`/`Result`) for out-of-range players so a bad index degrades to "no playable pieces" instead of a service panic. The `ortho_dir_name`/`wall_char` invariant panics are acceptable as-is but could be `debug_assert!` + safe fallback for full CODING.md compliance.

### `Loc::to_key` arithmetic overflow on out-of-range coordinates
- severity: nit
- category: correctness
- location: game/cathedral-2/src/loc.rs:114
- finding: `to_key` computes `(b'A' + self.y as u8) as char`; for `y < 0` or `y > 9` this panics on overflow in debug builds and silently produces a garbage key in release. All current callers validate first — `render.rs`'s `Tiler::tile_at` (render.rs:45) has an explicit `loc.valid()` guard added after a real panic was caught in render parity testing — but `Game::tile_at` (lib.rs:85-90) does NOT guard and relies on every caller having checked `valid()` beforehand. The invariant is currently upheld everywhere but is invisible and fragile against future callers.
- recommendation: Add the same `loc.valid()` early-return guard to `Game::tile_at` (returning `empty_tile()` for off-board locs, mirroring Go's missing-map-key behaviour), so the two `tile_at` implementations share one defensive contract; or at minimum document the "callers must validate" invariant on `to_key`.

### Unused `rand` dependency
- severity: nit
- category: dependencies
- location: game/cathedral-2/Cargo.toml:14
- finding: `rand = "0.10.2"` is declared in `[dependencies]` but nothing in the crate references `rand` (grep of `src/` and `tests/` finds no `rand::`/`use rand`); the fuzz binary goes through `brdgme_fuzz`, which declares its own `rand`. The game itself is fully deterministic (`start` ignores `seed`). This may be uniform boilerplate across game-crate Cargo.tomls, in which case fold it into that tracked cleanup.
- recommendation: Remove `rand` from cathedral-2's `[dependencies]` (or handle as part of the cross-crate boilerplate cleanup if it is uniform).

### Dead code: `impl Display for Loc` is never used
- severity: nit
- category: simplicity
- location: game/cathedral-2/src/loc.rs:118
- finding: The `Display` impl just forwards to `to_key()`, and no call site uses it — every consumer calls `to_key()` directly (log rendering at lib.rs:194, `render_empty_tile` at render.rs:252, board keying throughout). It exists only because Go's `Loc.String()` existed.
- recommendation: Delete the `Display` impl, or keep it and replace direct `to_key()` calls in display contexts with `{}` formatting — one idiom, not two.

## sushizock-2

Scoring (blue capped by red count), steal semantics (3 chopsticks = top of
stack, 4+ = nth from top), tile decks, dice glyphs, turn flow, and all
take-path bounds checks were verified faithful to `brdgme-go/sushizock_1`;
serde views leak nothing. 6 findings (1 major, 2 minor, 3 nit).

### Steal with `n = i32::MIN` overflows `len as i32 - n` — panic in debug/overflow-check builds
- severity: major
- category: correctness
- location: game/sushizock-2/src/lib.rs:460 (and identically game/sushizock-2/src/lib.rs:502)
- finding: `steal_blue`/`steal_red` accept an arbitrary `i32` tile index from player input (the `steal` parser uses `Int::any()` at command.rs:75, which happily parses `-2147483648`). With 4+ matching chopsticks (`can_steal_*_n` passes) and a non-empty target stack, `let index = len as i32 - n;` computes `len - i32::MIN`, which overflows `i32` (len >= 1, so len + 2^31 > i32::MAX). In dev/`server-dev`/test/fuzz builds (overflow checks on) this panics — a crafted command string kills the process. In release it wraps to a large negative and is accidentally caught by the `index < 0` guard, so production release is safe only by luck of two's-complement wrapping. The Go original wraps silently (Go ints), so this is a port-introduced hazard. CODING.md forbids panic paths reachable from requests.
- recommendation: Validate `n` before the arithmetic, e.g. `if n < 1 || n as usize > len { return Err(GameError::invalid_input(...)); }` placed right after the empty-stack check, then compute `let idx = len - n as usize;` (also removes the double cast). Alternatively use `checked_sub`. Bounding the parser int can't know the stack len, so validation in the game fn is the right place.

### Game ending via forced `take_worst` (roll path) never emits the placings log
- severity: minor
- category: correctness
- location: game/sushizock-2/src/lib.rs:711-722
- finding: The `Command::Take` (lib.rs:732-737) and `Command::Steal` (lib.rs:753-758) arms both check `self.is_finished()` after the move and append `placings_log(&self.placings(), Some(&scores))`. The `Command::Roll` arm does not. The game can legitimately finish inside `roll_dice_cmd`: when rolls are exhausted and the player can neither take nor steal, `take_worst()` (lib.rs:612) removes the last tile of the last non-empty pile, making both piles empty; `next_player()` then emits the "game is now finished" scores table but the structured placings log entry is missing, unlike every other end-of-game path in this crate and the convention in sibling crates (zombie-dice-2, greed-2, etc. append it in all terminal arms).
- recommendation: After `self.roll_dice_cmd(player, &dice)?` in the Roll arm, add the same `if self.is_finished() { ... logs.push(placings_log(...)) }` block used by the other two arms (or hoist the check to run once after the match for all arms).

### `roll` command's bounded `Many` — user-visible impact of the tracked suggest bug (cross-reference)
- severity: minor
- category: correctness
- location: game/sushizock-2/src/command.rs:47
- finding: CROSS-REFERENCE ONLY (the lib/game suggest bug itself is tracked by another unit — do not re-fix here). `roll_parser` uses `Many::bounded_spaced(Int::bounded(1, max), 1, max)` where `max = rolled_dice.len()`. Because the suggest engine's `Many` arm ignores `max`, tab-completion/suggest for `roll` keeps offering dice numbers past the legal count (and players frequently re-roll 1-2 dice of 5), so suggestions here are routinely wrong in the most common interaction of this game.
- recommendation: No crate-local fix; resolves when the tracked `Many`-ignores-`max` suggest bug in `rust/lib/game` is fixed.

### `.unwrap()` in `roll_dice` runtime path
- severity: nit
- category: quality
- location: game/sushizock-2/src/lib.rs:151
- finding: `(0..n).map(|_| *DIE_FACES.choose(rng).unwrap()).collect()` — `choose` returns `None` only for an empty slice and `DIE_FACES` is a 6-element const, so it is unreachable, but CODING.md bans `.unwrap()` in request-reachable runtime paths outright and this one is trivially avoidable.
- recommendation: Use indexing with a ranged random, e.g. `DIE_FACES[rng.random_range(0..DIE_FACES.len())]`, or a total fallback like `unwrap_or(&DieFace::Sushi)` if keeping `choose`.

### `take_worst` hand-rolled min loops, duplicated red/blue branches, fragile direct indexing
- severity: nit
- category: simplicity
- location: game/sushizock-2/src/lib.rs:527-566
- finding: Both branches re-implement "find index of minimum value" with a manual loop (`min_idx`/`min_val`) instead of `tiles.iter().enumerate().min_by_key(|(_, t)| t.value)`, and the two branches differ only in which pile they drain. Additionally the else branch indexes `self.blue_tiles[0]` directly (lib.rs:549): safe today only because `take_worst` is unreachable once both piles are empty (game would be finished), but that invariant is implicit — a future caller change turns it into a panic on an empty pile.
- recommendation: Extract the min-index via `min_by_key`, and either share the branch body over pile references selected by `TileType`, or at minimum note the non-empty precondition in a short comment.

### `take_blue`/`take_red` and `steal_blue`/`steal_red` are near-verbatim duplicates
- severity: nit
- category: simplicity
- location: game/sushizock-2/src/lib.rs:399-431 and game/sushizock-2/src/lib.rs:433-515
- finding: Each pair differs only in which pile vec (`blue_tiles` vs `red_tiles`, `player_blue_tiles` vs `player_red_tiles`) and which dice count it reads. This mirrors the Go original's duplication (port fidelity), but in Rust a small helper keyed on `TileType` returning `(&mut Vec<Tile>, &mut Vec<Tile>)` plus the relevant guard would halve ~120 lines and eliminate the risk of the pairs drifting (the i32::MIN overflow above had to be spotted twice).
- recommendation: Low priority given Go parity is deliberate; if touched, factor the shared body into one private `take(kind)` / `steal(kind, target, n)` and keep the public wrappers as thin guards.
