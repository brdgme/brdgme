# Raw findings: games-batch-c — `cathedral-2`

Crate reviewed: `/home/beefsack/Development/brdgme-review-snapshot/rust/game/cathedral-2/`
Files covered in full: `src/lib.rs` (1,275), `src/render.rs` (434), `src/loc.rs` (221),
`src/command.rs` (119), `src/piece.rs` (112), `src/tile.rs` (50), `tests/contract.rs`,
`Cargo.toml`, all 4 bins under `src/bin/`.
Cross-referenced against `/home/beefsack/Development/brdgme-review-snapshot/brdgme-go/cathedral_1/`
(`game.go`, `play_command.go`, `command.go`, `board.go`, `piece.go`, `tile.go`, `render.go`).

---

### Per-request memory leak in `loc_name` (Box::leak per parser construction)
- severity: major
- category: quality
- location: game/cathedral-2/src/command.rs:26
- finding: `loc_name` does `Box::leak(loc.to_key().into_boxed_str())` and the comment
  claims "Leaked once per process; the location set is fixed (100 entries), so this is a
  bounded, one-time allocation". That is wrong: `loc_parser()` (command.rs:98-107) calls
  `loc_name` for all 100 locs *every time it is constructed*, and `loc_parser` is built
  fresh on every `command_parser()` call — i.e. every `command()` and every
  `command_spec()` invocation. Each parse/suggest request leaks 100 freshly-allocated
  strings (~4–8 KB with allocator overhead) that are never reclaimed. On a long-running
  HTTP service where `command_spec` is hit per page load / suggest keystroke this is an
  unbounded leak driven by ordinary traffic. The `&'static str` is only needed because
  `LocChoice.name` was typed that way; nothing in the `Enum` parser requires `'static`.
- recommendation: Change `LocChoice.name` to `String` (store `loc.to_key()` directly) and
  drop `loc_name`/`Box::leak` entirely; or, if a static table is preferred, build the 100
  `LocChoice`s once in a `std::sync::OnceLock` and clone from it. Fix the stale comment
  either way.

### Cathedral is traversable by the capture flood-fill (not treated as a wall)
- severity: minor
- category: correctness
- location: game/cathedral-2/src/lib.rs:283
- finding: The inner area walk in `check_captures` blocks only on
  `self.tile_at(l2).player == player` (the capturing player's own pieces). Cathedral
  tiles (`player == PLAYER_CATHEDRAL == 2`) do NOT block the walk — the flood-fill passes
  through the cathedral square and merges areas on both sides of it. Per official
  Cathedral rules an area enclosed by one player's pieces *and/or the board edge and/or
  the cathedral* is captured; here the cathedral cannot serve as part of an enclosure
  wall, so an "enclosure" completed via the cathedral instead floods through it and (if
  the merged area then contains ≥2 distinct pieces) no capture happens where the official
  rules would grant one. Verified this is inherited verbatim from Go's `CheckCaptures`
  (`play_command.go`: same `g.Board[...].Player == player` block condition), so it is a
  Go-parity behaviour — but unlike preserved defects #1–#4 it is NOT flagged in any code
  comment or the suspected-defects list, so it reads as intentional-correct rather than
  preserved-quirk. (Related, also undocumented: the official-rules "cathedral must be
  placed within the central area" restriction some editions have is absent — matching Go.)
- recommendation: Decide explicitly whether this is preserved-defect #5. If yes, add a
  comment at lib.rs:283 noting the cathedral deliberately does not block the area walk
  (Go parity, deviates from official enclosure rules). If it should match official rules,
  also block on `t.player == PLAYER_CATHEDRAL` (and check the `pieces_found` counting
  still handles a cathedral enclosed alone, per defect #3's carve-out).

### Dead code: `parse_loc` is never called
- severity: minor
- category: simplicity
- location: game/cathedral-2/src/loc.rs:167
- finding: `parse_loc` (loc.rs:167-181, port of Go's `ParseLoc`) has no callers anywhere
  in the crate. Go used it for command parsing; the Rust port parses locations via the
  `Enum`-over-fixed-names parser in `command.rs` (`loc_parser`, command.rs:98), so the
  free-text parser is vestigial. It is `pub` so the compiler emits no dead-code warning.
  It is also the only `Option`-returning loc parser and would diverge from the actual
  accepted command syntax if anyone did wire it up.
- recommendation: Delete `parse_loc` (it remains in git history if a future free-text
  input path needs it), or add a unit test exercising it if it is being kept deliberately
  for port completeness — in which case say so in its doc comment.

### `pieces()` panics on out-of-range player index (request-adjacent)
- severity: minor
- category: correctness
- location: game/cathedral-2/src/piece.rs:110
- finding: `pieces(player)` ends in `_ => panic!("invalid player: {}", player)`. It is
  called from `piece_parser` (command.rs:93), `can_play_piece` (lib.rs:125), `play`
  (lib.rs:173), `remaining_piece_size`, `can_play_something`, and
  `render_player_remaining_tiles` (render.rs:359) with `player` derived from the
  `player: usize` argument of the `Gamer` methods `command`/`command_spec`/`player_state`.
  Today the service layer only passes valid player indices, but per `docs/CODING.md`
  ("no `.unwrap()`/`.expect()`/`panic!()` in runtime paths reachable from requests") this
  invariant is enforced only by an upstream contract, and a panic in a game crate kills
  the shared HTTP service. Same class: `ortho_dir_name` (loc.rs:41, panics on non-ortho
  dir — parser-constrained today) and `wall_char` (render.rs:85, panics on empty dir —
  structurally unreachable as `render_corner` always accumulates ≥1 nonzero component).
- recommendation: For `pieces()` specifically, return an empty `Vec` (or make it
  `Option`/`Result`) for out-of-range players so a bad index degrades to "no playable
  pieces" instead of a service panic. The `ortho_dir_name`/`wall_char` invariant panics
  are acceptable as-is but could be `debug_assert!` + safe fallback for full CODING.md
  compliance.

### `Loc::to_key` arithmetic overflow on out-of-range coordinates
- severity: nit
- category: correctness
- location: game/cathedral-2/src/loc.rs:114
- finding: `to_key` computes `(b'A' + self.y as u8) as char`; for `y < 0` or `y > 9` this
  panics on overflow in debug builds and silently produces a garbage key in release. All
  current callers validate first — `render.rs`'s `Tiler::tile_at` (render.rs:45) has an
  explicit `loc.valid()` guard added after a real panic was caught in render parity
  testing — but `Game::tile_at` (lib.rs:85-90) does NOT guard and relies on every caller
  having checked `valid()` beforehand (`can_play_piece` does at lib.rs:143;
  `check_captures` only ever sees walk-validated locs). The invariant is currently upheld
  everywhere but is invisible and fragile against future callers.
- recommendation: Add the same `loc.valid()` early-return guard to `Game::tile_at`
  (returning `empty_tile()` for off-board locs, mirroring Go's missing-map-key
  behaviour), so the two `tile_at` implementations share one defensive contract; or at
  minimum document the "callers must validate" invariant on `to_key`.

### Unused `rand` dependency
- severity: nit
- category: dependencies
- location: game/cathedral-2/Cargo.toml:14
- finding: `rand = "0.10.2"` is declared in `[dependencies]` but nothing in the crate
  references `rand` (grep of `src/` and `tests/` finds no `rand::`/`use rand`); the fuzz
  binary goes through `brdgme_fuzz`, which declares its own `rand`. The game itself is
  fully deterministic (`start` ignores `seed`). (`tokio` is legitimately used by the
  `cathedral_2_http` bin.) This may be uniform boilerplate across game-crate Cargo.tomls,
  in which case fold it into that tracked cleanup.
- recommendation: Remove `rand` from cathedral-2's `[dependencies]` (or handle as part of
  the cross-crate boilerplate cleanup if it is uniform).

### Dead code: `impl Display for Loc` is never used
- severity: nit
- category: simplicity
- location: game/cathedral-2/src/loc.rs:118
- finding: The `Display` impl just forwards to `to_key()`, and no call site uses it —
  every consumer calls `to_key()` directly (log rendering at lib.rs:194,
  `render_empty_tile` at render.rs:252, board keying throughout). It exists only because
  Go's `Loc.String()` existed.
- recommendation: Delete the `Display` impl, or keep it and replace direct `to_key()`
  calls in display contexts with `{}` formatting — one idiom, not two.

---

## Things checked and considered clean (not findings)

- **Preserved Go defects #1–#4 (documented, per review instructions not findings):**
  #1 `can_play_piece` bounds check `piece > len` off-by-one (lib.rs:126, documented
  doc-comment, unreachable via `Int::bounded(1, max)` parser); #2 cathedral placement not
  advancing the turn (lib.rs:239-245, documented); #3 cathedral tile wiped to owned-empty
  when its area is captured (test-documented at lib.rs:1174-1195); #4 captured pieces
  returned to hand and replayable (test-documented).
- **`walk`'s never-updated `queued` set (loc.rs:203-209):** duplicate-callback quirk is
  explicitly documented as preserved-from-Go, and both `check_captures` callbacks guard
  with their own `visited` set. Verified the Go source has the identical defect.
- **Placement legality:** `can_play_piece` checks off-board, overlap, and enemy-owned
  territory per rotated cell; rotation math (`Loc::rotate` n=2/-1/1/0) verified against
  Go's `Loc.Rotate` and covered by `play_rotation_produces_correct_offsets`.
- **Capture semantics:** 8-way area flood-fill with ortho outer walk, `pieces_found`
  keyed by `PlayerType`, capture only when ≤1 distinct piece type, cathedral excluded
  from return-count/size, ownership flip of the whole area — all verified line-by-line
  against Go `CheckCaptures`. Cathedral-first rule, capture-only-after-cathedral-played
  gate (lib.rs:198), and skipping capture check on the cathedral play itself all match Go.
- **End of game / scoring:** game ends only when neither player has any playable piece
  (pass handled by `next_player` skipping a stuck opponent, lib.rs:335-340); placings by
  lowest remaining-piece-area via `-remaining_piece_size` (lib.rs:393-398);
  `points()` returning raw remaining size (higher = worse) matches Go `Points()` exactly.
- **Simultaneous mode:** `no_open_tiles` transition, `whose_turn_players`, and
  `can_play` dual-player logic match Go (`CanPlay`/`WhoseTurn`).
- **Panic reachability from crafted input:** none found. Piece index is clamped by
  `Int::bounded` before the (buggy) bounds check; loc/dir come from fixed `Enum` values;
  all board indexing in `play`/`check_captures` operates on parser- or walk-validated
  locs. The only input-independent panic paths are the internal-invariant ones listed
  above.
- **Serde:** `Game`/`PubState`/`PlayerState` round-trip test present; `HashMap<String,
  Tile>` board keyed by `to_key()` matches Go's `map[string]Tile`; no hidden state.
- **Render:** edge-of-board panic from parity testing is guarded and regression-tested
  (lib.rs:1257-1274); `render_corner`'s HashMap iteration order is nondeterministic but
  the accumulated `corner` bitmask is order-independent; `render_piece`'s
  `unwrap_or_default()` is on `Option<Vec<N>>` and cannot panic.
- **Bins:** all 4 are the standard boilerplate with no deviations; `tests/contract.rs`
  wires the shared `assert_gamer_contract`.

## Worker summary

Reviewed all of `cathedral-2`: full read of `src/lib.rs`, `src/render.rs`, `src/loc.rs`,
`src/command.rs`, `src/piece.rs`, `src/tile.rs`, skimmed the 4 boilerplate bins (no
deviations), `Cargo.toml`, and `tests/contract.rs`. Cross-referenced the capture,
placement, turn, scoring, and points logic line-by-line against the Go original
(`brdgme-go/cathedral_1`). Documented preserved Go defects #1–#4 and the `walk` quirk
were treated as deliberate cross-references per instructions and verified against the Go
source rather than reported. Result: 1 major (per-request `Box::leak` in the command
parser, contradicting its own "leaked once per process" comment), 3 minor (undocumented
cathedral-not-a-wall capture semantics inherited from Go; dead `parse_loc`;
request-adjacent panic in `pieces()`), 3 nit (`to_key` overflow fragility, unused `rand`
dep, unused `Display for Loc`). Core game logic — placement legality, rotation, capture
counting, end-of-game, scoring — is otherwise a faithful, well-tested port with no
player-input-reachable panics.
