# Raw findings: rust/game/sushizock-2

Review of `/home/beefsack/Development/brdgme-review-snapshot/rust/game/sushizock-2/`
(lib.rs 1739 lines, render.rs 196, command.rs 98, src/bin/* skimmed).
Go cross-reference: `/home/beefsack/Development/brdgme-review-snapshot/brdgme-go/sushizock_1/`
(sushizock.go, command.go, dice.go, tile.go all read in full).

### Steal with `n = i32::MIN` overflows `len as i32 - n` — panic in debug/overflow-check builds
- severity: major
- category: correctness
- location: game/sushizock-2/src/lib.rs:460 (and identically game/sushizock-2/src/lib.rs:502)
- finding: `steal_blue`/`steal_red` accept an arbitrary `i32` tile index from player
  input (the `steal` parser uses `Int::any()` at command.rs:75, which happily parses
  `-2147483648`). With 4+ matching chopsticks (`can_steal_*_n` passes) and a non-empty
  target stack, `let index = len as i32 - n;` computes `len - i32::MIN`, which overflows
  `i32` (len >= 1, so len + 2^31 > i32::MAX). In dev/`server-dev`/test/fuzz builds
  (overflow checks on) this panics — a crafted command string kills the process. In
  release it wraps to a large negative and is accidentally caught by the `index < 0`
  guard, so production release is safe only by luck of two's-complement wrapping. The
  Go original wraps silently (Go ints), so this is a port-introduced hazard. CODING.md
  forbids panic paths reachable from requests.
- recommendation: Validate `n` before the arithmetic, e.g.
  `if n < 1 || n as usize > len { return Err(GameError::invalid_input(...)); }`
  placed right after the empty-stack check, then compute `let idx = len - n as usize;`
  (also removes the double cast). Alternatively use `checked_sub`. Consider also
  bounding the parser int (`Int::bounded(1, ...)` can't know the stack len, so
  validation in the game fn is the right place).

### Game ending via forced `take_worst` (roll path) never emits the placings log
- severity: minor
- category: correctness
- location: game/sushizock-2/src/lib.rs:711-722
- finding: The `Command::Take` (lib.rs:732-737) and `Command::Steal` (lib.rs:753-758)
  arms both check `self.is_finished()` after the move and append
  `placings_log(&self.placings(), Some(&scores))`. The `Command::Roll` arm does not.
  The game can legitimately finish inside `roll_dice_cmd`: when rolls are exhausted
  and the player can neither take nor steal, `take_worst()` (lib.rs:612) removes the
  last tile of the last non-empty pile, making both piles empty; `next_player()` then
  emits the "game is now finished" scores table but the structured placings log entry
  is missing, unlike every other end-of-game path in this crate and the convention in
  sibling crates (zombie-dice-2, greed-2, etc. append it in all terminal arms).
- recommendation: After `self.roll_dice_cmd(player, &dice)?` in the Roll arm, add the
  same `if self.is_finished() { ... logs.push(placings_log(...)) }` block used by the
  other two arms (or hoist the check to run once after the match for all arms).

### `.unwrap()` in `roll_dice` runtime path
- severity: nit
- category: quality
- location: game/sushizock-2/src/lib.rs:151
- finding: `(0..n).map(|_| *DIE_FACES.choose(rng).unwrap()).collect()` — `choose`
  returns `None` only for an empty slice and `DIE_FACES` is a 6-element const, so it
  is unreachable, but CODING.md bans `.unwrap()` in request-reachable runtime paths
  outright and this one is trivially avoidable.
- recommendation: Use indexing with a ranged random, e.g.
  `DIE_FACES[rng.random_range(0..DIE_FACES.len())]` (matches the idiom allowed for
  rand 0.10), or `*DIE_FACES.choose(rng).unwrap_or(&DieFace::Sushi)`-style total
  fallback if you want to keep `choose`.

### `take_worst` hand-rolled min loops, duplicated red/blue branches, fragile direct indexing
- severity: nit
- category: simplicity
- location: game/sushizock-2/src/lib.rs:527-566
- finding: Both branches re-implement "find index of minimum value" with a manual
  loop (`min_idx`/`min_val`) instead of
  `tiles.iter().enumerate().min_by_key(|(_, t)| t.value)`, and the two branches
  differ only in which pile they drain. Additionally the else branch indexes
  `self.blue_tiles[0]` directly (lib.rs:549): safe today only because `take_worst` is
  unreachable once both piles are empty (game would be finished), but that invariant
  is implicit — a future caller change turns it into a panic on an empty pile.
- recommendation: Extract the min-index via `min_by_key`, and either share the branch
  body over `(&mut pile, &mut player_pile)` selected by `TileType`, or at minimum keep
  the direct index but note the non-empty precondition in a short comment.

### `take_blue`/`take_red` and `steal_blue`/`steal_red` are near-verbatim duplicates
- severity: nit
- category: simplicity
- location: game/sushizock-2/src/lib.rs:399-431 and game/sushizock-2/src/lib.rs:433-515
- finding: Each pair differs only in which pile vec (`blue_tiles` vs `red_tiles`,
  `player_blue_tiles` vs `player_red_tiles`) and which dice count it reads. This
  mirrors the Go original's duplication (port fidelity), but in Rust a small helper
  keyed on `TileType` returning `(&mut Vec<Tile>, &mut Vec<Tile>)` plus the relevant
  guard would halve ~120 lines and eliminate the risk of the pairs drifting (the
  i32::MIN overflow above already had to be spotted twice).
- recommendation: Low priority given Go parity is deliberate; if touched, factor the
  shared body into one private `take(kind)` / `steal(kind, target, n)` and keep the
  public wrappers as thin guards.

### `roll` command's bounded `Many` — user-visible impact of the tracked suggest bug
- severity: minor
- category: correctness
- location: game/sushizock-2/src/command.rs:47
- finding: CROSS-REFERENCE ONLY (the lib/game suggest bug itself is tracked by another
  unit). `roll_parser` uses `Many::bounded_spaced(Int::bounded(1, max), 1, max)` where
  `max = rolled_dice.len()`. Because the suggest engine's `Many` arm ignores `max`,
  tab-completion/suggest for `roll` will keep offering dice numbers past the legal
  count (and players frequently re-roll 1-2 dice of 5), so suggestions here are
  routinely wrong in the most common interaction of this game.
- recommendation: No crate-local fix; resolves when the tracked `Many`-ignores-`max`
  suggest bug in `rust/lib/game` is fixed.

## Verified clean / deliberate (no finding)

- Scoring rule (`score`, lib.rs:260-268): blue tiles score only up to the number of
  red tiles held. Matches Go `Score` (tile.go:112-123) exactly and is pinned by
  `test_scoring` — deliberate rule/port behavior, not a bug.
- Steal semantics: 3 chopsticks = top of stack only, 4+ = nth from top; no
  sushi-cancels-chopsticks variant. Matches Go (`StealBlueN`/`StealRedN`) including
  the `n == 1` routing. Deliberate.
- Tile decks (12 blue: 1-6 x2; 12 red: -1 x5, -2 x4, -3 x2, -4 x1), shuffled once at
  start; take index = sushi/bones count - 1 with pile-length guard in `can_take_*` —
  bounds-safe. Matches Go.
- Dice faces and render glyphs (blue Θ sushi, blue X chopsticks, red ¥ bones, red X
  chopsticks) match Go `dice.go` exactly.
- `take_blue`/`take_red` `remove(idx)`: idx = count-1 is guarded by
  `can_take_*` (count > 0 and pile len >= count) — safe.
- `roll_dice_cmd`: duplicate die numbers collapse via HashSet (same as Go map),
  out-of-range and "keep at least one die" validated, auto-keep at 1 die /
  0 remaining rolls, forced `take_worst` when no take/steal available. Matches Go.
- Turn advancement and game end (`next_player` -> `log_game_end`) reachable from all
  mutating paths; `command_parser` returns `None` once finished, so no post-finish
  commands.
- No panics reachable from crafted input other than the i32::MIN overflow above;
  all other indexing (`render.rs:146,161`, `player_score`) is guarded by emptiness
  checks or construction invariants.
- Serde views (`PubState`/`PlayerState`) expose no hidden info (game has none);
  `final_scores` only populated when finished; RNG migration shim documented.
- `src/bin/*`: all four binaries byte-identical (modulo crate name) to sibling crates
  (verified by diff against zombie-dice-2) — standard boilerplate, no deviations.
- Dependencies: all path-internal plus rand/serde/tokio, consistent with sibling game
  crates; nothing extraneous.

## Worker summary

Read in full: `src/lib.rs` (all 1739 lines, incl. the whole test module),
`src/render.rs`, `src/command.rs`, and skimmed `src/bin/*` (verified identical to
sibling boilerplate via diff). Cross-referenced the complete Go original
(`sushizock.go`, `command.go`, `dice.go`, `tile.go`). Confirmed `Int::any()` in
`rust/lib/game/src/command/parser/mod.rs:81-186` parses `-2147483648` to `i32::MIN`,
making the steal overflow reachable; checked workspace `Cargo.toml` profiles (dev
profiles have overflow checks on, release does not). Game rules (roll/keep up to 3
rolls, sushi/bones take from piles, 3 vs 4 chopstick steals, forced take-worst,
both-piles-empty end, capped scoring) all verified faithful to the Go port.

Findings: 1 major (i32::MIN steal overflow), 2 minor (missing placings log on the
roll/take_worst end-of-game path; suggest `Many` max cross-reference), 3 nit
(unwrap in roll_dice, take_worst min-loop/indexing, take/steal duplication).

Considered clean: scoring rule, steal/take/tile-deck/dice-glyph fidelity to Go,
bounds safety of all take paths, turn/end-of-game flow, serde views, tests
(reasonable coverage incl. Go-port tests and guard tests), dependencies.
