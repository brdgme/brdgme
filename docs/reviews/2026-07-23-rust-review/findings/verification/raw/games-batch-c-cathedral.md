# Verification: games batch C - cathedral-2 (F22-F28)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust (commit f8763a5).
Go original: /home/beefsack/Development/brdgme-review-snapshot/brdgme-go/cathedral_1.
All paths below relative to the snapshot rust/ root unless absolute.

## F22 (major, quality) - Box::leak per request in loc_name

**Verdict: CONFIRMED (major)**

The comment's "one-time allocation" claim is false; the leak repeats per request.

- The leak site and wrong comment, `game/cathedral-2/src/command.rs:24-28`:

```rust
// Leaked once per process; the location set is fixed (100 entries), so this
// is a bounded, one-time allocation rather than a per-parse leak.
fn loc_name(loc: Loc) -> &'static str {
    Box::leak(loc.to_key().into_boxed_str())
}
```

- `loc_parser()` calls `loc_name` for every one of the 100 locs each time it
  is constructed, `command.rs:98-106`:

```rust
fn loc_parser() -> impl Parser<T = Loc> {
    let values: Vec<LocChoice> = loc::all_locs()
        .into_iter()
        .map(|l| LocChoice {
            loc: l,
            name: loc_name(l),
        })
        .collect();
```

- `loc_parser()` is invoked inside `play_parser` (`command.rs:73`), which is
  built fresh by `command_parser` (`command.rs:52-58`). `command_parser` is
  called on every `Gamer::command` (`lib.rs:467`: `self.command_parser(player as i32)`)
  and every `Gamer::command_spec` (`lib.rs:500`: `self.command_parser(player as i32).map(|cp| cp.to_spec())`).
  The cmd harness calls `game.command(...)` per Play request
  (`lib/cmd/src/requester/gamer.rs:125-131`), so each request leaks ~100
  small strings. Unbounded growth under ordinary traffic; nothing caches or
  memoizes the parser.
- The `'static` requirement is self-imposed: `LocChoice.name` is declared
  `&'static str` (`command.rs:21`), but the `Enum` parser only requires
  `T: ToString + Clone` (`lib/game/src/command/parser/mod.rs:551-553`:
  `pub struct Enum<T> where T: ToString + Clone`). `name` could be a `String`
  (or `LocChoice`'s `Display` could call `loc.to_key()` directly) with no
  leak at all.

Severity: major (quality) stands. It is a genuine per-request memory leak in
a long-lived server process, misdescribed by its own comment.

## F23 (minor, correctness) - cathedral tiles do not block the capture flood-fill

**Verdict: ADJUSTED (minor correctness -> not a defect; nit documentation at most)**

All code observations verify, but the "undocumented preserved defect"
framing is wrong: the behaviour is the documented, intended rule.

- Walk condition, `game/cathedral-2/src/lib.rs:282-284`:

```rust
loc::walk(l, &all_dirs, |l2| {
    if visited.contains(&l2) || self.tile_at(l2).player == player {
        return loc::WALK_BLOCKED;
    }
```

  `PLAYER_CATHEDRAL` is `2` (`game/cathedral-2/src/tile.rs:9`:
  `pub const PLAYER_CATHEDRAL: i32 = 2;`), `player` here is `0` or `1`, so
  cathedral tiles never block the inner walk - confirmed.
- Go parity confirmed, `brdgme-go/cathedral_1/play_command.go:217-219`:

```go
Walk(l, Dirs, func(l Loc) int {
    if visited[l] || g.Board[l.String()].Player == player {
        return WalkBlocked
```

- No code comment documents this behaviour (grep for "defect" in
  `game/cathedral-2/src/` hits only defects #1 lib.rs:113/1236, #2
  lib.rs:241, #3 lib.rs:1144, #4 lib.rs:1199) - confirmed.
- However, the crate's own RULES.md documents the cathedral as a neutral
  piece that sits *inside* enclosed regions rather than forming walls:
  `RULES.md:20` ("One neutral 6-cell piece, belonging to neither player"),
  `RULES.md:64-68` ("enclosed region ... that contains **at most one**
  distinct piece identity is captured. The Cathedral counts as a piece
  identity"), `RULES.md:70-73` (captured regions include "Cathedral tiles,
  whose ownership flips like any other captured tile"). A cathedral tile
  being walkable-through/capturable is exactly this documented rule, and it
  matches the official Cathedral rule that the neutral cathedral cannot form
  part of a player's enclosing wall. The behaviour is intended, not a
  preserved defect, so there is nothing to add to the suspected-defects
  list. Residual value at most a nit: a one-line comment at the walk noting
  the cathedral is deliberately non-blocking.

## F24 (minor, simplicity) - parse_loc is dead code

**Verdict: CONFIRMED (minor)**

- Definition: `game/cathedral-2/src/loc.rs:167` `pub fn parse_loc(input: &str) -> Option<Loc>`
  (body lines 167-181, port of Go `ParseLoc`).
- Grep for `parse_loc` across the whole snapshot `rust/` tree (src, bins,
  `tests/contract.rs`, other crates) finds only the definition line - zero
  callers.
- Actual location parsing goes through the `Enum`-over-fixed-names
  `loc_parser` (`command.rs:98-107`).
- `pub` visibility suppresses the dead-code lint, as claimed.

## F25 (minor, correctness) - pieces() panics on invalid player, reachable from Gamer entry points

**Verdict: CONFIRMED (minor)**

- The panic, `game/cathedral-2/src/piece.rs:106-112`:

```rust
pub fn pieces(player: i32) -> Vec<Piece> {
    match player {
        0 => player_0_pieces(),
        1 => player_1_pieces(),
        _ => panic!("invalid player: {}", player),
    }
}
```

- Call sites confirmed: `piece_parser` (`command.rs:93`
  `let max = pieces(player).len() as i32;`), `can_play_piece`
  (`lib.rs:125`), `play` (`lib.rs:173`), `remaining_piece_size`
  (`lib.rs:344`), `can_play_something` (`lib.rs:360`),
  `render_player_remaining_tiles` (`render.rs:359`
  `let all_pieces = piece::pieces(p_num as i32);`).
- Reachability from a request: `Gamer::command(&mut self, player: usize, ...)`
  (`lib.rs:461`) does no bounds check on `player`; it calls
  `command_parser(player as i32)` -> `can_play(player)` (`lib.rs:103-109`).
  When `no_open_tiles` is true, `can_play` calls
  `can_play_something(player, ...)` which calls `pieces(player)` - so a Play
  request naming player 2+ panics in simultaneous mode. The cmd harness
  forwards the request player index unvalidated
  (`lib/cmd/src/requester/gamer.rs:130-131`:
  `match game.command(player, command, names)`). (When `no_open_tiles` is
  false, `player >= 2` falls out cleanly via `current_player == player`
  being false.)
- Same class confirmed: `ortho_dir_name` (`loc.rs:41`
  `_ => panic!("not an ortho dir: {}", dir)`, reachable from a direct
  `play()` call with a diagonal dir via the log at `lib.rs:192`, though not
  through the parser, which only offers ortho dirs) and `wall_char`
  (`render.rs:85` `(false, false) => panic!("wall_char: empty dir")`).
- Violates docs/CODING.md ("No panicking code in runtime paths",
  /home/beefsack/Development/brdgme-review-snapshot/docs/CODING.md:44-49).
  Minor is right: real panics require an out-of-range player index that the
  upstream server normally does not produce, but the game crate itself has
  no guard.

## F26 (nit, correctness) - to_key overflows for out-of-range y; Game::tile_at unguarded

**Verdict: CONFIRMED (nit)**

- `game/cathedral-2/src/loc.rs:113-115`:

```rust
pub fn to_key(self) -> String {
    format!("{}{}", (b'A' + self.y as u8) as char, self.x + 1)
}
```

  For `y < 0`, `self.y as u8` wraps and `b'A' + <large u8>` overflows:
  panic in debug, wrapped garbage char in release. Same for `y > 9` beyond
  the wrap threshold.
- Guarded impl, `game/cathedral-2/src/render.rs:38-49` (`Tiler for HashMap`):

```rust
        if !loc.valid() {
            return None;
        }
        self.get(&loc.to_key()).cloned()
```

  with a comment (render.rs:39-44) explaining exactly this hazard, added
  after a real panic found during render-parity checking (see the test
  comment at lib.rs:1246-1256).
- Unguarded impl, `game/cathedral-2/src/lib.rs:85-90`:

```rust
    fn tile_at(&self, loc: Loc) -> Tile {
        self.board
            .get(&loc.to_key())
            .cloned()
            .unwrap_or_else(empty_tile)
    }
```

  No `loc.valid()` guard. Current callers happen to pass only valid locs
  (`can_play_piece` checks `l.valid()` at lib.rs:143 before `tile_at`;
  `loc::walk` only queues `next_loc.valid()` neighbours, loc.rs:210-213),
  so no live panic path today - nit is the right severity.

## F27 (nit, dependencies) - rand declared but unused

**Verdict: CONFIRMED (nit)**

- `game/cathedral-2/Cargo.toml:14`: `rand = "0.10.2"`.
- Grep for `rand` across `game/cathedral-2/src/` (including all four
  `src/bin/` binaries) and `tests/contract.rs`: zero code usages (the only
  hit is the word "randomness" in a doc comment at lib.rs:417).
- `start` ignores its seed, `lib.rs:419`:
  `fn start(players: usize, _seed: u64) -> ...` with the doc comment
  "Cathedral has no randomness, so `seed` is accepted (per the `Gamer`
  trait) but unused."
- The fuzz binary needs no direct rand, `src/bin/cathedral_2_fuzz.rs`:
  `brdgme_fuzz::fuzz_gamer::<Game>();`.

## F28 (nit, simplicity) - Display for Loc unused

**Verdict: CONFIRMED (nit)**

- The impl, `game/cathedral-2/src/loc.rs:118-122`:

```rust
impl std::fmt::Display for Loc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_key())
    }
}
```

- Every consumer calls `to_key()` directly: board keying (lib.rs:87, 179,
  310, 429), play log (lib.rs:194), render label (render.rs:252), plus all
  test call sites. Grep across the crate for `format!`/`write!`/
  `to_string()` usages found no `"{}"`-formatting of a `Loc` value anywhere
  (the `format!` at loc.rs:114 is inside `to_key` itself, formatting a char
  and an i32; `LocChoice`'s Display at command.rs:30-34 prints its cached
  `name` field, not the `Loc`). The `Enum` parser's `ToString` bound
  operates on `LocChoice`, not `Loc`. Dead impl.
