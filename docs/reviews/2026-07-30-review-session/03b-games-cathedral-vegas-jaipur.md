# Unit 03b review - cathedral-2, sushizock-2, lords-of-vegas-1, jaipur-2

Reviewed commits (Unit 03 subset, excluding splendor-2/texas-holdem-2/acquire-1
which Unit 03a covered):

- `f5472388` WP-21 - cathedral-2 (`Box::leak`) + sushizock-2 (overflow)
- `7337c7ac` WP-22 - lords-of-vegas-1
- `a692b638` WP-23 - jaipur-2

Crates: `rust/game/cathedral-2`, `rust/game/sushizock-2`,
`rust/game/lords-of-vegas-1`, `rust/game/jaipur-2`.

Files reviewed in final form: `cathedral-2/src/{lib,piece,command,render}.rs`
(+ `loc.rs`, `tile.rs`, `Cargo.toml` via evidence extraction);
`sushizock-2/src/lib.rs`, `sushizock-2/{RULES,DATA_DOCS}.md`;
`lords-of-vegas-1/src/{lib,board,card,render,tile}.rs`,
`lords-of-vegas-1/{Cargo.toml,RULES.md}`;
`jaipur-2/src/{lib,command,render}.rs`, `jaipur-2/RULES.md`;
`rust/lib/game/src/game.rs` (trait defaults).

Findings continue from F-47; this unit numbers from **F-48** (12 findings,
F-48..F-59).

Method: recovered each WP spec from `git show 868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/...`,
checked each acceptance criterion against the **end state** of the code, then
hand-audited every `Log::public` site, `validate()` presence, and `pub_state`
redaction shape per crate. No tests, benchmarks or lints were run.

## Findings

### F-48 (Medium) cathedral-2: WP-21's recorded efficiency observation was closed by a hoist that the inner call site defeats - the catalogue is still rebuilt in the innermost loop

- `rust/game/cathedral-2/src/piece.rs:110` (`pieces`), `:58`, `:79`
- `rust/game/cathedral-2/src/lib.rs:106` (`player_pieces`), `:146`
  (`can_play_piece`), `:402-432` (`can_play_something`)
- `rust/game/cathedral-2/src/command.rs:80` (`piece_parser`)

First, the `Box::leak` itself (c F22) is genuinely fixed: it lived in
`fn loc_name(loc: Loc) -> &'static str { Box::leak(loc.to_key().into_boxed_str()) }`
in `command.rs`, called once per location per `loc_parser()` construction, and
the replacement `Enum::partial(loc::all_locs())` (command.rs:89-91) retains
nothing. `pieces()` was never the leak - it already returned an allocating
`Vec<Piece>` before the commit. That half is clean.

The finding is the adjacent item WP-21's spec recorded as
"Cross-package / newly discovered, item 2":

> **`can_play_something` rebuilt the piece catalogue inside its 100-iteration
> location loop (cathedral-2, efficiency).** Not a filed finding; Task 3
> hoists it out of the loop as a required consequence of the Option shaping.

The hoist landed (lib.rs:407, comment: "rebuilding it 100 times per call was
pure waste") but does not achieve what it claims, because the loop body's own
call at lib.rs:425 goes through `can_play_piece`, which calls
`self.player_pieces(player)` again at lib.rs:146. Each such call rebuilds all
14 (player 0) or 15 (player 1) `Piece` values, every one owning its own
`Vec<Loc>`.

Counting the calls per `can_play_something`: before the hoist, 100 (loop head)
+ up to 100 x 14 x 4 = ~5,600 (inner) = ~5,700 rebuilds. After the hoist, 1 +
~5,600 = ~5,601. The hoist removed under 2% of the cost, and the comment reads
as if the problem is gone.

Why it matters: `can_play_something` is not a cold path. `whose_turn_players`
(lib.rs:435) calls it once per player when `no_open_tiles` is set and
`status()` (lib.rs:574) calls `whose_turn_players()`; `can_play` (lib.rs:126)
calls it from `command_parser`, which `command_spec()` (lib.rs:563) builds -
and the web layer calls `status()` and `command_spec()` on every render.
`play()` calls it up to four more times per move (lib.rs:237-242, 263-268) and
`next_player()` once (lib.rs:380). At ~5,600 rebuilds x ~15 `Vec`s each, that
is ~10^5 allocations per call. `piece_parser` (command.rs:80) also builds the
whole catalogue solely to read `.len()`.

Remediation: make the catalogue genuinely static without leaking -
`static PIECES: LazyLock<[Vec<Piece>; 2]> = LazyLock::new(|| [player_0_pieces(), player_1_pieces()]);`
with `pieces(player) -> Option<&'static [Piece]>`. That preserves the
`Option` "not a player" distinction c F25 required, costs nothing per call,
and makes the hoist (and its comment) unnecessary. `LazyLock` is in std and
lords-of-vegas-1 already uses exactly this pattern for `TILES`
(`lords-of-vegas-1/src/tile.rs:22`), so it is the in-repo idiom.

### F-49 (Medium) cathedral-2: `check_captures` indexes `played_pieces` raw with two untrusted values from the board - reachable panic, and cathedral-2 has no `validate()` (F-06)

- `rust/game/cathedral-2/src/lib.rs:343`

```rust
self.played_pieces[pt.player as usize][(pt.typ - 1) as usize] = false;
```

`pt` is a `PlayerType` read straight off a board tile (`t.player_type()`,
lib.rs:333). Both components are unvalidated `i32`s deserialized from stored
state:

- `pt.player as usize` - any board tile whose `player` is not `0`/`1`/
  `PLAYER_CATHEDRAL` (or a `played_pieces` outer vec shorter than
  `self.players`) indexes out of bounds.
- `(pt.typ - 1) as usize` - a tile with `typ == 0` wraps to
  `usize::MAX` on the cast; a tile with `typ` greater than the player's piece
  count is simply out of range.

Nothing between deserialization and this line checks either value:
cathedral-2 is one of the missing-13 crates with **no `Gamer::validate`
override** (sweep 1), so `Gamer::validate`'s fail-open `Ok(())` default
(F-06) is what runs. The path is command-reachable:
`command()` -> `play()` -> `check_captures()` (lib.rs:232).

The same class appears at lib.rs:154/:157/:217/:394/:416, where
`played_pieces[player][piece_idx]` and `all_pieces[piece_idx]` are indexed
raw. lib.rs:150 makes one of these directly reachable even from clean state
via the `pub` API: the bounds check is `piece as usize > all_pieces.len()`
(deliberately preserving Go's off-by-one, per the comment at lib.rs:134-138),
which **admits `piece == all_pieces.len()`** and then panics at lib.rs:154 and
lib.rs:157. The comment argues the parser bounds `piece` to
`1..=len` before subtracting 1, which is true today, but the port note
documents the off-by-one as a preserved defect rather than closing it, so the
panic is one refactor away.

Remediation: add a `Gamer::validate` override asserting
`played_pieces.len() == players`, `played_pieces[p].len() == pieces(p).len()`
for each `p`, and that every board tile's `player`/`owner` is in
`{NO_PLAYER, 0, 1, PLAYER_CATHEDRAL}` with `typ` in
`1..=pieces(player).len()`. Independently, change lib.rs:150 to `>=` (the
off-by-one is not observable behaviour worth porting - Go panics there too)
and switch lib.rs:343 and lib.rs:394/:416 to checked `get`/`get_mut`.

### F-50 (Medium) lords-of-vegas-1: `build()` indexes `self.players` raw from `current_player`; no `validate()` (F-06), and `next_player()` divides by `players.len()`

- `rust/game/lords-of-vegas-1/src/lib.rs:298`, `:308`, `:353`

`build()` reaches `self.players[p].cash` (lib.rs:298) and
`self.players[p].cash -= ...` (lib.rs:308) with `p` guarded only by
`can_build` -> `p == self.current_player` (lib.rs:242-244). `current_player`
is a plain deserialized `usize` field with no bound relative to
`players.len()`, and lords-of-vegas-1 has **no `Gamer::validate` override**
(sweep 1), so nothing rejects `current_player >= players.len()`. A stored
state with a short/empty `players` vector and a board tile
`BoardTile::Owned { player: current_player }` panics inside `command()`.

`next_player()` (lib.rs:353) computes `% self.players.len()`, which is a
divide-by-zero panic on an empty `players` vector - and that one is reachable
from the `Done` command with no board precondition at all.

Note the contrast: `player_state` (lib.rs:171) *is* written defensively
(`self.players.get(player).cloned()`), which shows the author was aware of the
hazard on the render path but not on the command path.

Remediation: add a `Gamer::validate` override checking
`(2..=6).contains(&players.len())`, `current_player < players.len()`, and that
every `BoardTile::Owned`/`Built` owner index is `< players.len()`; that single
hook closes every site above at once and is the WP-09b pattern the other 15
crates already follow.

### F-51 (Medium) sushizock-2: `status()` itself panics on a short per-player vector - the most directly exposed instance of F-06 in this batch

- `rust/game/sushizock-2/src/lib.rs:278-283` (`player_score`), `:326`, `:330`,
  `:376`, `:673-685` (`status`), `:700-704` (`pub_state`)

`player_score(player)` indexes `self.player_blue_tiles[player]` and
`self.player_red_tiles[player]` raw. Both are `Vec<Vec<Tile>>` sized from
`players` in `start()` (lib.rs:662-663) but with no post-deserialization
check - sushizock-2 has **no `Gamer::validate` override** (sweep 1).

Unlike the cathedral-2 and lords-of-vegas-1 cases, this one does not need a
command to trigger. `status()` (lib.rs:674-678) calls `placings()`, which maps
`player_score` over `0..self.players`, and `pub_state()` (lib.rs:701) does the
same for `final_scores`. The web layer calls both on every render, so a stored
state whose `players` count exceeds either vector's length makes the game
permanently unrenderable rather than merely uncommandable. `next_player()`
(lib.rs:376) additionally divides by `self.players`, a divide-by-zero on
`players == 0`, and `another_player_has_blue`/`_red` (lib.rs:326, :330) index
the same vectors over `0..self.players` from the command path.

Remediation: `validate()` asserting
`(MIN_PLAYERS..=MAX_PLAYERS).contains(&self.players)`,
`player_blue_tiles.len() == self.players`,
`player_red_tiles.len() == self.players`, and `current_player < self.players`.

### F-52 (Low) sushizock-2: F-18 confirmed - copy-pasted three-arm epilogue with no `!was_finished` gate, and the game-end log is emitted twice in two formats

- `rust/game/sushizock-2/src/lib.rs:372-378` (`next_player`), `:380-404`
  (`log_game_end`), `:737-745`, `:761-766`, `:781-790` (the three `command`
  arms)

Confirms F-18 for sushizock-2. Each of the three `command` arms carries the
same six-line block:

```rust
if self.is_finished() {
    let scores: Vec<(usize, i32)> = (0..self.players)
        .map(|p| (p, self.player_score(p)))
        .collect();
    logs.push(placings_log(&self.placings(), Some(&scores)));
}
```

with no `let was_finished = ...` captured before the mutation, i.e. WP-08's
transition gate is absent. Compare jaipur-2, which was migrated and does
capture it (jaipur-2/src/lib.rs:759, `if !was_finished && self.is_finished()`).

Not currently exploitable into a double epilogue, because `command_parser`
returns `None` once the game is finished (sushizock-2/src/command.rs, and
`command()` rejects at lib.rs:721-728), so no arm can run on an
already-finished game. The finding is that the guard is a
`self.is_finished()` post-condition rather than a transition, which is only
sound by that external invariant; any future command that is legal after
finish re-emits the epilogue.

Separately, and independent of F-18: the finishing move already emits an
end-of-game log *before* the arm runs. `next_player()` (lib.rs:373-375)
returns `self.log_game_end()` when the game has finished, which pushes a
public per-player table of tiles and scores. Every path that finishes the game
(`take`, `steal`, `take_worst`) goes through `next_player()`, so the resulting
log stream always contains `log_game_end`'s score table *and* then
`placings_log`'s placings+scores line - the same scores twice, in two formats.

Remediation: migrate sushizock-2 to WP-08's shared epilogue helper, capture
`was_finished` before dispatch, and drop either `log_game_end` or the
`placings_log` call so end-of-game is reported once.

### F-53 (Low) sushizock-2: RULES.md says the tile rows are face down; `PubState` publishes every value in them

- `rust/game/sushizock-2/RULES.md:7-8` vs
  `rust/game/sushizock-2/DATA_DOCS.md:7-8` and
  `rust/game/sushizock-2/src/lib.rs:89-96`, `:692-693`

RULES.md setup reads "12 blue tiles: two each valued 1 through 6, shuffled
face down" / "12 red tiles: ... shuffled face down", while DATA_DOCS.md and
the `PubState` doc comments declare the full ordered central row - values
included - to be public, which is what `pub_state()` returns.

I am not treating this as a leak: DATA_DOCS.md is the redaction contract per
D-33, it explicitly declares the row public, the mechanic (`take` the Nth tile
where N is your dice count) is forced rather than chosen so visibility grants
no illegitimate advantage, and `take_worst` needs the values engine-side
anyway. But the two rules documents disagree, and the sweep's classification
of sushizock-2 as "documented as having no hidden information" rests on the
document that happens to agree with the code. Someone auditing from RULES.md
would reasonably file this as a High leak.

Remediation: reword RULES.md:7-8 to "shuffled, then laid out face up in a
row" (or whatever the intended physical setup is) so the two documents state
the same thing.

### F-54 (Medium) jaipur-2: `current_player` is unbounded on load and every per-player access is a fixed-array raw index - no `validate()` (F-06)

- `rust/game/jaipur-2/src/lib.rs:161` (`current_player`), `:340`, `:380`,
  `:426`, `:459`, `:462`, `:504`, `:515`, `:566`, `:737` (`player_state`)
- `rust/game/jaipur-2/src/render.rs:209`, `:216`, `:231`, `:235`, `:239`

jaipur-2's per-player state is fixed-size (`[Vec<Good>; 2]`, `[u32; 2]`), so a
corrupt state cannot make a vector short - but every access is a raw index and
`current_player` is a bare deserialized `usize` with no bound. jaipur-2 has
**no `Gamer::validate` override** (sweep 1).

The reachable path is short. `can_take`/`can_sell` (lib.rs:313-315, :481-483)
only test `self.current_player == player`, so a state with
`current_player: 2` lets `command(2, "take camel", ..)` through
`command_parser` (command.rs:15 also only compares against `current_player`)
straight into `self.camels[2] += ...` at lib.rs:340. `player_state(2)` panics
at `self.hands[2].clone()` (lib.rs:737), and `render.rs`'s
`you_have_rows`/`opponent_rows` index `camels`/`hand_sizes`/`token_counts` with
the same value.

Remediation: `validate()` asserting `current_player < NUM_PLAYERS` and
`round_wins[p] <= 2`. Because `NUM_PLAYERS` is a compile-time 2 and the state
is fixed-arrays, this is a two-line hook.

### F-55 (Medium) cathedral-2: WP-21 explicitly routed the `played_pieces` truncated-row indexing to WP-09, and WP-09b never picked cathedral-2 up - the handoff dropped on the floor

- WP-21 spec, Non-Goals (recovered from
  `868094a6:docs/reviews/2026-07-23-rust-review/planning/specs/archive/WP-21-cathedral-sushizock-fixes.md`):

  > **Deserialized/foreign-state trust hardening** - owned by **WP-09**
  > (BLOCKED-ON-DECISION D-36). In scope here is only the *player-index*
  > boundary (c F25). Out of scope: `played_pieces[player][i]` indexing
  > surviving a truncated row, `render_player_remaining_tiles`' indexing,
  > sushizock's `player_blue_tiles[target]`/`[player]` indexing.

- WP-21 spec, Cross-package item 1: "**LEAD RULING: ROUTED TO WP-09**;
  sushizock-2 must be ADDED to WP-09's crate list."

Both routings were recorded, and neither arrived. WP-09b (`c078c3ee`, 16
files) added `validate()` to 15 crates; cathedral-2 and sushizock-2 are not
among them (sweep 1). So the exact indexing WP-21 deferred is still
unguarded, and the crate that WP-21's own lead ruled "must be ADDED to WP-09's
crate list" was not added.

Concretely still live in cathedral-2:
`rust/game/cathedral-2/src/render.rs:378` - `state.played_pieces[p_num][i]`.
The guard added just above it (render.rs:363-371) checks the **outer** length
(`p_num < state.played_pieces.len()`) but not the **inner** one, so a
`played_pieces[p_num]` row shorter than `pieces(p_num).len()` panics on the
render path - which is the case WP-21's non-goal names verbatim ("surviving a
truncated row"). The same inner-length gap is at lib.rs:154, :217, :394, :416.

Why it matters beyond F-49: this is a process failure, not just a code gap. Two
findings were closed by declaring another work package the owner, that package
shipped without them, and nothing reconciled the two. Any audit that checks
"was c F25 fixed?" reads clean, because c F25 *was* fixed - the routed
remainder is invisible.

Remediation: fold both crates into the F-06 remediation (see F-49, F-51) and,
process-side, treat "routed to WP-NN" as a tracked obligation on WP-NN rather
than a closure note on the routing package.

### F-56 (Low) cathedral-2: `can_play` returns true on a finished game when the game ended before `no_open_tiles` was set, so `command_spec` keeps advertising `play`

- `rust/game/cathedral-2/src/lib.rs:124-130` (`can_play`), `:243-258`,
  `:562-564` (`command_spec`), `:518-560` (`command`)

`play()` sets `self.finished = true` (lib.rs:244) inside the
`if !playable_piece` branch, and that branch **skips** the `else if
!self.no_open_tiles` branch that would have set `no_open_tiles`. So the state
`finished == true && no_open_tiles == false` is reachable.

In that state `can_play(player)` takes its else arm and returns
`self.current_player as i32 == player` - true for the last player to move. Two
consequences:

- `command_spec(player)` (lib.rs:563) returns `Some(spec)`, so the web layer
  keeps offering a `play <piece> <loc> [<dir>]` command on a finished game.
- `command()` (lib.rs:518) never calls the trait's `assert_not_finished`
  helper (`rust/lib/game/src/game.rs:96-104`), so the resulting attempt fails
  inside `can_play_piece` with a piece-level message ("that is not a valid
  piece number", "there is already a piece there") rather than
  `GameError::Finished`.

Compare jaipur-2, whose `command_parser` short-circuits on
`self.is_finished()` (jaipur-2/src/command.rs:15), and sushizock-2, whose
`whose_turn_inner` returns `vec![]` when finished.

Remediation: add `if self.finished { return false; }` at the top of
`can_play`, or call `self.assert_not_finished()?` as the first line of
`Gamer::command` and have `command_parser` return `None` when
`self.finished`.

### F-57 (Low) lords-of-vegas-1: `.unwrap()` on a deserialization-reachable path in `Loc::parse_str`, and d F5's `neighbours()` half is still unfixed (also a dropped WP-09 routing)

- `rust/game/lords-of-vegas-1/src/board.rs:84`, `:103-118`, `:151-166`

```rust
let block = Block::parse_char(chars.next().unwrap())?;
```

This is the only panic operator left outside `#[cfg(test)]` in the crate. It
is provably safe (the `value.is_empty()` check two lines above guarantees
`chars.next()` is `Some`), but `parse_str` is the `Deserialize` impl's entry
point for every `Loc` key in a stored `Board` (board.rs:151-166) as well as
the command-parse path, so it is squarely "request-reachable", where
`docs/CODING.md` bans `.unwrap()`. sushizock-2 has an explicit in-code comment
citing that same rule as the reason for avoiding `choose(..).unwrap()`
(sushizock-2/src/lib.rs:151-155), so the crates are inconsistent about a rule
they both cite.

Remediation: `let Some(c) = chars.next() else { return Err("Loc string is
empty".to_string()) };` - which also lets the separate `is_empty` check go.

Related, and the reason this is worth a line at all: WP-22's Non-Goals routed
d F5 to WP-09 with "Do NOT add validation to `Loc::parse_str` or touch
`neighbours()`". Of the two halves:

- The out-of-range-lot half **is** now closed - board.rs:89-91 rejects
  `lot < 1 || lot > block.max_lot()`, and because `Deserialize` funnels
  through `parse_str`, that also validates every `Loc` arriving from stored
  state.
- The `neighbours()` underflow half is **not**. With `lot == 0`, board.rs:108
  evaluates `0 % 3 != 1` as true and board.rs:109 computes `self.lot - 1`,
  a `usize` underflow panic. It is unreachable today only because
  `parse_str` now rejects lot 0; nothing in `neighbours()` or the
  `From<(Block, Lot)>` impl (board.rs:96-100) enforces it, so the guard is
  one construction site away from being bypassed.

This is the same dropped-routing pattern as F-55: WP-22 closed d F5 by naming
WP-09 the owner, and WP-09b (`c078c3ee`) did not add a `validate()` to
lords-of-vegas-1 (sweep 1).

Remediation: either bound `lot` in `From<(Block, Lot)>` (making `Loc`
construction total) or make `neighbours()` use `checked_sub`.

### F-58 (informational) F-35 occurrences in this unit - recorded, not re-raised

Per the unit brief, `Status::Finished { stats: vec![] }` is parked in WP-20
(`c F12`). Occurrences in this batch, for the F-35 tally only:

| File:line | Crate |
|---|---|
| `rust/game/cathedral-2/src/lib.rs:570` | cathedral-2 |
| `rust/game/sushizock-2/src/lib.rs:677` | sushizock-2 |
| `rust/game/jaipur-2/src/lib.rs:707` | jaipur-2 |
| `rust/game/lords-of-vegas-1/src/lib.rs:201` | lords-of-vegas-1 |

All four crates also implement `Gamer::points()` with real values
(cathedral-2:580, jaipur-2:797, sushizock-2 via `player_score`), so the
information `stats` would carry is partly available by another route. No fix
demanded.

## Verified good

### cathedral-2 (WP-21, `f5472388`)

Every WP-21 acceptance criterion for this crate checks out against the end
state:

- **Task 1 / c F22 (the `Box::leak`, major)** - genuinely fixed, not papered
  over. The leak was `fn loc_name` in `command.rs` calling
  `Box::leak(loc.to_key().into_boxed_str())` once per location per
  `loc_parser()` construction; `LocChoice` and its `Display` impl are gone;
  `loc_parser()` is `Enum::partial(loc::all_locs())` (command.rs:89-91), which
  retains nothing. `impl Display for Loc` was correctly **kept**
  (loc.rs:118) - Task 1 depends on it. The grammar is pinned by
  `loc_parser_spec_is_every_board_location_in_row_major_order`
  (command.rs:114), which asserts 100 values, `A1` first, `J10` last, and
  full equality with `all_locs()`. This is the "confirm the replacement
  doesn't reintroduce it under a different shape" check the unit brief asked
  for, and it passes.
- **Task 2 / c F26** - `tile_at` has the permissive `if !loc.valid() { return
  empty_tile(); }` guard (lib.rs:93-96), the separate `!l.valid()` legality
  check in `can_play_piece` was **not** removed (lib.rs:167-169), and the
  factually-false "mirrors Go's missing-map-key behaviour" rationale the spec
  forbade is absent. Test `tile_at_returns_empty_for_off_board_locations`
  (lib.rs:1348) present.
- **Task 3 / c F25** - `pieces` is `Option`-shaped (piece.rs:110),
  catalogue builders are `pub`, `player_pieces` does the game-aware bound
  check (lib.rs:106-111), `command_parser` is the single choke point for both
  `Gamer::command` and `command_spec` and returns `None` out of range
  (command.rs:31-42), `Gamer::command` returns `GameError::internal` rather
  than `invalid_input` (lib.rs:524-529), `remaining_piece_size` returns
  `Option<i32>` (lib.rs:390), and the render-side guard the spec specified in
  lieu of a `Result`-returning `player_state` is in place with the exact
  "not a player in this game" marker (render.rs:363-371). All three named
  tests present (lib.rs:1372, :1394, :1405).
- **Task 4 / c F24** - `parse_loc` is gone crate-wide.
- **Task 5 / c F23** - comment-only, at the correct site (lib.rs:317-324),
  citing RULES.md, and the walk condition was **not** changed - exactly what
  the spec's OVERTURNED disposition required.
- **Task 6 / c F27** - `rand` gone from `Cargo.toml` and `Cargo.lock`.
- **Skipped rider** - `wall_char` (render.rs:85) and `ortho_dir_name`
  (loc.rs:41) invariant panics are untouched, as the spec decided.
- Test count is 31, above the spec's 27 target - nothing was weakened or
  deleted to make things pass.

### sushizock-2 (WP-21, `f5472388`)

- **Task 7 / c F29 (the overflow, major)** - correct and complete. The
  validation is genuinely *before* any arithmetic (lib.rs:504-509), accepts
  exactly `1..=len`, and the message is preserved; `idx = len - n as usize`
  (lib.rs:510) then maps `n == 1` to the last element, i.e. the top of the
  stack, matching RULES.md. The `target >= self.players` check (lib.rs:486)
  is correctly ordered *before* the `player_*_tiles[target]` accesses
  (lib.rs:492-493), which incidentally closes WP-21's "newly discovered
  unbounded `target`" item in-crate.
- **Task 8 / c F30** - the Roll arm now carries the placings log
  (lib.rs:737-745) with the comment explaining why.
- **Task 9 / c F32** - `roll_dice` is index-based
  (`DIE_FACES[rng.random_range(0..DIE_FACES.len())]`, lib.rs:156-158) with a
  comment recording the RNG-stream-equivalence argument. This matters because
  `Game.rng` is persisted, and the argument is stated rather than assumed.
- **Task 10 / c F33, c F34** - `take_blue`/`take_red` and
  `steal_blue`/`steal_red` are thin `pub` wrappers over private `take`/`steal`
  (lib.rs:406-544), preserving the public API; `take_worst` uses
  `min_by_key` with a `let ... else` instead of the old unguarded
  `blue_tiles[0]` (lib.rs:571-573), and the comment correctly notes
  `min_by_key` keeps the first minimum, matching the strict-`<` loop it
  replaced.

### lords-of-vegas-1 (WP-22, `7337c7ac`)

All nine WP-22 tasks land, including the two the spec rated major:

- **Task 1 / d F1** - the five `unimplemented!()` arms are `GameError::
  InvalidInput` (lib.rs:249-263), dispatch is extracted into a testable
  private `dispatch` (lib.rs:246), the `..` patterns removed the
  `player`-shadowing hazard and the `#[allow(unused_variables)]`, and
  `unimplemented_commands_error_instead_of_panicking` (lib.rs:376) drives all
  five variants.
- **Task 2 / d F2 (determinism, major)** - `casino_at`'s BFS queue is a
  transient `BTreeSet<Loc>` popped with `pop_first()` (board.rs:244-249), the
  `expect("queue shouldn't be empty")` is gone, and `casinos()` sorts
  `TILES.keys()` into a `Vec` before iterating (board.rs:278-280). The
  serialized `Board` stayed a `HashMap`, per the spec's rejection of the
  `BTreeMap` alternative. Both pinning tests exist
  (`resolve_boss_ties_is_deterministic_per_seed` board.rs:562,
  `casinos_iterates_in_sorted_loc_order` board.rs:650). This is the fix that
  matters most in the crate - unordered iteration feeding a seeded RNG makes
  replay and undo incorrect - and it was done the right way round.
- **Task 3 / d F4 (verification-upgraded to major)** - both halves landed, and
  the second half is the one that is easy to skip: `build()` rejects an
  exhausted colour supply *before* deducting cash (lib.rs:303-307), and
  `saturating_sub` is applied at all three render sites (render.rs:82, :90,
  :123) even though (1) makes them unreachable for new games - because
  already-persisted games may hold more than nine tiles of a colour. Both
  tests present (lib.rs:416, :478), and `render_saturates_when_supplies_
  exceeded` builds a state that genuinely exceeds both the 9-tile and 12-die
  supplies. No bare subtraction on a supply count remains in render.rs.
- **Task 4 / d F3** - one public log per reroll, naming casino, player,
  location and new die (board.rs:332-346); test at board.rs:613.
- **Tasks 5-9** - `TILES` is `LazyLock` (tile.rs:22) with `lazy_static` gone
  from `Cargo.toml`; `serde_json` moved to `[dev-dependencies]`;
  `thiserror` correctly left alone as the spec instructed; the
  `Card::GameEnd` `unreachable!()` carries its invariant comment
  (lib.rs:118-122) with the verification-corrected number (position >= 38),
  which I re-derived independently from `card.rs:20-35` and the 48-entry
  `TILES` - minimum insertion index is 38 at two players and 40 at six;
  `use std::iter::FromIterator;` is gone; `BLOCK_WIDTH` is `pub` and used
  twice in `render_block` with no literal `3` left (render.rs:160-161);
  RULES.md has Sphinx = Orange, Pioneer = Brown plus the approximation note.
- `player_state` is written defensively (`self.players.get(player).cloned()`,
  lib.rs:171) - worth noting as the one crate in this batch that got the
  render-path index right.

### jaipur-2 (WP-23, `a692b638`)

WP-23 is the cleanest of the three packages. All five tasks land, including
the two where the spec had to reject the finding's own recommended mechanism:

- **Task 1 / d F14 (major)** - `let bonus_key = quantity.min(MAX_TRADE_BONUS);`
  (lib.rs:524) uses the constant rather than a literal 5, as specified;
  `quantity` itself is **not** clamped, so the log text, `good_tokens` count
  and hand removal all still use the true quantity (lib.rs:513-517, :535-556,
  :565-569). The exhausted-pile case is handled without a panic by the
  chained `if let Some(bonuses) = ... && let Some(bonus) = bonuses.first()`
  (lib.rs:525-527), and quantities 1 and 2 still map to keys 1 and 2, which
  are absent from `bonuses`, so they correctly earn nothing. The comment at
  lib.rs:521-523 explains why the 7-card hand limit makes 6- and 7-card sales
  reachable at all, which is the non-obvious part.
- **Task 2 / d F18 + d F20** - the spec correctly identified that the
  finding's recommendation ("validate inside the `Map` closure") is impossible
  because `Map`'s closure is infallible, and the replacement
  `SellGoodsParser` (command.rs:64-115) is the right shape: it errors on mixed
  types with `offset = out.consumed.len()` so `OneOf` prefers its message over
  the sibling parser's offset-0 failure (command.rs:87-97), errors via
  `let ... else` rather than defaulting on an empty list (command.rs:77-86,
  retiring d F20's `unwrap_or(Good::Diamond)`), and delegates `to_spec()` and
  `expected()` straight to the inner `Many` (command.rs:108-114) so the
  advertised `CommandSpec` and autocomplete are unchanged. The doc comment
  states both reasons it cannot be a `Map`.
- **Task 3 / d F19** - `command_parser` builds the vec directly and returns
  `Some` unconditionally; the only `None` path is the finished-game/wrong-
  player early return (command.rs:15-17).
- **Task 4 / d F22** - "N rounds remaining" is gone; `common_rows`
  (render.rs:173-200) states "First to 2 round wins takes the game." plus a
  leader row carrying the actual counts, and the comment
  (render.rs:175-177) records why no count derived from `round_wins` is
  trustworthy (a fully tied round replays without incrementing either
  counter). The rejected "round N of 3" alternative is not present.
- **Task 5 / d F17** - RULES.md is a real 91-line rules document. Both hard
  constraints hold: it documents the 8-in-deck + 3-in-market camel split
  explicitly (RULES.md:30-32), which is the durable fix for the d F13
  misreading that would have created 14 camels; and it stays silent on all
  three WP-26 questions (who starts the next round, whether the camel token
  counts toward the bonus-token tie-break, opponent camel visibility). I
  checked it line-by-line against `lib.rs` and found no divergence.
- **Hidden information (the standing per-crate check)** - jaipur-2's
  redaction is correct. `PubState` (lib.rs:176-196) exposes only counts where
  contents are secret: `deck_len` not `deck`, `hand_sizes` not `hands`,
  `token_counts` not `tokens`, and `bonuses` as `HashMap<usize, usize>`
  (pile *lengths*) rather than the `Vec<u32>` values (lib.rs:718-719).
  `PlayerState` adds only the viewer's own `hand` (lib.rs:737).
  I audited every `Log::public` site by hand:
  - `start_round` (lib.rs:214-220) - announces 3 camels, public by setup.
  - `replenish_market` (lib.rs:271) - names the drawn cards, which go
    face-up into the public market.
  - `receive_cards` (lib.rs:294-309) - correctly **`Log::private`** twice:
    the full card list to the drawing player, and counts only
    (`N goods and M camels`) to the opponent. This is the one place a leak
    would be easy and it is right.
  - `take_camels` (lib.rs:327-339) - camel count, and camels are a public
    herd.
  - `take_goods` (lib.rs:441-445) - `"took N cards"` only, no identities;
    and the identities are already derivable from the public market delta,
    so nothing is added.
  - `sell` (lib.rs:535-556) - good, quantity and points, all public;
    critically the bonus token's **value** goes out as `Log::private` to the
    seller only (lib.rs:558-563) while the public line says merely "and took
    a bonus token".
  - `end_round` (lib.rs:592-640) - reveals totals and per-player bonus/goods
    token counts, which is the end-of-round scoring reveal.
  - `finish_epilogue` (lib.rs:666-676) - final placings and scores.
  No `Log::public` carries a card identity or token value that is not already
  public. The redaction test `pub_state_does_not_leak_hand_contents`
  (lib.rs:1244) exists, so jaipur-2 is WP-10-3a compliant.
- The command parsers are state-independent (command.rs:24-30, :117-141), so
  `command_spec` cannot leak state either - worth stating because a
  hand-derived `Enum` of sellable goods would have been the natural
  implementation and would have leaked the hand into the spec.

### Hidden-information audit, remaining three crates

**cathedral-2 - no hidden information, correctly implemented.** `PubState`
(lib.rs:44-57) and `PlayerState` (lib.rs:63-68) carry the same data, and
`player_render` renders both players' remaining-piece catalogues
unconditionally (render.rs:408-429), which is right: piece inventories are
derivable from the board. All eight `Log::public` sites audited
(lib.rs:218-228, :245-258, :271-273, :361-372, and `placings_log` at :550) -
none carries anything not on the public board. No redaction is owed, so the
absence of a redaction test is correct rather than a gap.

**sushizock-2 - everything public by design.** Per DATA_DOCS.md the whole
state is public; `PubState` mirrors `Game` minus `rng`, and `final_scores` is
correctly gated on `finished` (lib.rs:700-704) rather than always populated.
All five `Log::public` sites audited (lib.rs:365-369, :398-403, :437-441,
:547-553, :579-583). See F-53 for the RULES.md wording contradiction.

**lords-of-vegas-1 - the hidden state is the deck, and it is not leaked.** The
secrets are the undealt deck's order and, specifically, the position of the
`Card::GameEnd` card. `PubState` exposes `remaining_deck: self.deck.len()`
rather than `deck` (lib.rs:161), and the doc comment on `played` correctly
records that the `GameEnd` card is never listed there (lib.rs:48). The one
`Log::public` that looks risky is `start`'s per-player
`"<player> drew <cards> and will start with <cash>"` (lib.rs:124-130), which
does publish each player's starting cards - but the same loop calls
`board.set(loc, BoardTile::Owned { player: p })` for each of those cards
(lib.rs:115), and `board` is published verbatim in `PubState` (lib.rs:163), so
the log adds nothing that is not already public. The other two sites
(lib.rs:139-142 start player, :320-326 build) are public actions. `deck` is
never read after `start` (see Coverage gaps), so there is no draw log to leak.

### F-59 (Low) lords-of-vegas-1: real hidden state, and still no `pub_state` redaction test - WP-10 3a compliance gap

- `rust/game/lords-of-vegas-1/src/lib.rs:157-166` (`pub_state`), `:41-54`
  (`PubState`)

WP-10 3a declared the `pub_state` redaction shape mandatory "for every game
crate" but only 3 of 28 were done, and no later WP swept the rest. Of the four
crates in this unit, lords-of-vegas-1 is the only one that both holds genuinely
secret state (the deck order and the `Card::GameEnd` position) and has **no
test calling `pub_state()` at all** (sweep 3).

The redaction is currently *correct* (see the audit above) - this is a
regression-risk finding, not a live leak. But the invariant "`PubState` must
never carry `deck`" is held only by nobody having added the field, and the
crate is an explicitly partial port whose unimplemented commands (card draws,
payouts, scoring) will need to touch the deck when they land. A three-line
test asserting `serde_json::to_string(&game.pub_state())` does not contain the
undealt cards would pin it.

Remediation: add `pub_state_does_not_leak_the_deck`, matching the shape of the
15 crates that already have one (e.g.
`jaipur-2/src/lib.rs:1244`). Note the crate already has `serde_json` as a
dev-dependency, so there is no new dependency cost.

## Coverage gaps

- **`Gamer::validate` (F-06) is unfixed in all four crates.** Confirmed
  individually: cathedral-2 (F-49), lords-of-vegas-1 (F-50), sushizock-2
  (F-51), jaipur-2 (F-54). All four are in the missing-13. The exploitable
  shape the brief predicted - parallel per-player vectors indexed raw - is
  present in three of the four; jaipur-2's variant is fixed-size arrays
  indexed by an unbounded `current_player`, which is the same bug with a
  different surface. sushizock-2's is reachable from `status()` itself, so it
  is the worst of the four. WP-09b (`c078c3ee`) touched 16 files and skipped
  all four despite WP-21's spec explicitly ruling that cathedral-2's and
  sushizock-2's cases "must be ADDED to WP-09's crate list" (F-55).
- **No crate in this unit tests the log layer.** As with every earlier unit,
  the `Log::public` vs `Log::private` split is untested everywhere. My audit
  found the split correct in all four crates (jaipur-2's is the only one where
  it does real work), but that is a hand check with no regression barrier.
  jaipur-2's `receive_cards` is the highest-value place to add one: two
  `Log::private` calls with different audiences, and flipping either to
  `Log::public` would leak an opening hand with no test failing - the exact
  defect F-22 recorded in alhambra-1.
- **lords-of-vegas-1 can never finish, and I could not determine whether it is
  reachable by users.** `finished` is never assigned `true` anywhere in the
  crate (grep-confirmed), `deck` is never read after `start()`, `Card::GameEnd`
  is inserted but never drawn, and `Player.points` is documented as "always 0
  as scoring is not yet implemented" (lib.rs:70-71) yet is the primary
  `status()` placings metric (lib.rs:198). WP-22's spec pre-ruled this
  ("`status()` can never reach `Finished` (no endgame trigger) - documented
  behavior, not a defect"), so per the brief I am not filing it. I flag it
  because the ruling turns on whether players can start such a game: game
  registration appears to be DB-driven (`rust/web/migrations/`), not code, and
  the only non-crate references to lords-of-vegas-1 are the workspace member
  list in `rust/Cargo.toml` and `rust/Cargo.lock`. If the version row is
  enabled in production, "documented" is not an adequate disposition for a
  game that accepts players and never ends. **Worth an explicit
  Orchestrator/owner question rather than a Lead ruling.**
- **Test-count gates not verifiable.** Both WP-21's and WP-23's package gates
  are stated in terms of `cargo test -p <crate>` counts (and WP-23 targets
  exactly 68). Tests may not be run in this session, so I checked only that
  nothing was weakened or deleted: cathedral-2 has 31 `#[test]` fns against a
  spec target of 27, and every named test the three specs require is present
  and asserts what the spec says it should. No test in any of the four crates
  is `#[ignore]`d and none carries an `#[allow(...)]`.
- **sushizock-2's scoring is order-dependent and I did not adjudicate it.**
  `score()` (lib.rs:267-275) pairs blue tiles with red tiles by *acquisition
  order* - `blue.iter().enumerate()`, scoring only `i < red.len()` - so which
  blue tiles count depends on the sequence in which they were collected and
  stolen, and an opponent's `steal <n>` mutates that sequence. RULES.md:43 and
  DATA_DOCS.md:36 both document exactly this, and rules parity is parked in
  WP-20, so I have not filed it. Recording it so it is not re-derived: the
  open question for whoever owns WP-20 is whether the physical game lets a
  player choose their pairing, and whether unpaired *red* tiles should score
  (the code sums all reds unconditionally while discarding unpaired blues,
  which is asymmetric).
- **`Gamer::points()`' ordering contract is undocumented, and cathedral-2's
  sign is the opposite of its own placings metric.** `rust/lib/game/src/game.rs:117-119`
  defines `fn points(&self) -> Vec<f32> { vec![] }` with no doc comment.
  cathedral-2 returns `+remaining_piece_size` (lib.rs:580-584) while
  `calc_placings` ranks on `-remaining_piece_size` (lib.rs:448-450), i.e.
  `points()` is higher-is-worse there while jaipur-2's and sushizock-2's are
  higher-is-better. I did not trace the consumers (they are outside this
  unit's crates), so I am not filing it - but if any web surface sorts or
  displays `points()`, cathedral-2 is inverted. **Carry to whichever unit owns
  `lib/game`'s trait surface or the stats/display path (Unit 08).**
- **Not re-raised, per the brief:** F-06 itself, F-18 as a systemic item
  (F-52 is the sushizock-2 confirmation only), F-19 (jaipur-2's duplicated
  placings metric, already filed), F-35 (recorded in F-58), and the parked
  WP-20/WP-26 items - jaipur-2's `d F15` next-round starter, `d F16` camel
  token in the tie-break, `d F23` opponent camel display and the routed
  `render.rs` literal-`"Player 0"`-instead-of-`N::Player` nit; sushizock-2's
  and lords-of-vegas-1's player-count questions; cathedral-2's four
  deliberately-preserved Go defects (documented in-code at lib.rs:134-138 and
  :274-278).
