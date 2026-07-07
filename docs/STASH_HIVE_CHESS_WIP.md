# Stash Analysis: wip-go-hive-chess-port

Analysis of `stash@{0}` ("On leptos: wip-go-hive-chess-port: libgrid+hex libs,
chess_1 WIP port"), created 2026-07-04. 23 new untracked files, ~1,000 lines,
all under `brdgme-go/`. Documented so the stash can be dropped without losing
the design learnings; the code targets the Go stack, which is being retired
(see GO_VS_RUST_PORTING.md), and GAME_PORTING_PLAN.md defers hive/chess as new
development rather than ports.

## Contents

- `brdgme-go/libgrid/` - minimal 2D grid primitives (`Loc{X,Y}` + `Add`)
- `brdgme-go/libgrid/hex/` - sparse hex grid library with tests (hive groundwork;
  contains zero hive-specific logic)
- `brdgme-go/chess_1/` - chess move-generation engine with tests (no game layer)

## Chess: direction and design

### Architecture

- Pure move-generation core, deliberately separated from any `Gamer`
  implementation. No commands, turn logic, or player handling exist yet.
- `Piecer` interface: `Rune()`, `AvailableMoves(from, board) []Move`,
  `GetTeam()`. Each piece is a struct embedding a shared `Piece{Team int}`.
- `Board` is a value type: `[8][8]Piecer` array. Value semantics make board
  copies cheap, presumably intended for "does this move leave me in check"
  simulation later.
- Teams are encoded as `TEAM_WHITE = 1`, `TEAM_BLACK = -1`. The sign doubles
  as movement direction: pawn advance is `Rank + p.Team`, pawn start rank is
  `StartRank(team) + team`, and `EndRank(team) = StartRank(-team)`. Compact
  and elegant; eliminates all per-team branching in movement math.

### Move representation

```go
type Move struct {
    From, To       Location
    TakeAt         *Location // capture square, decoupled from To
    SubsequentMove *Move     // chained move
}
```

Two deliberate choices worth keeping:

- `TakeAt` is separate from `To` specifically so en passant works (capture
  square differs from destination). `nil` means non-capturing.
- `SubsequentMove` anticipates castling (king move chains the rook move) even
  though castling itself was never implemented. `Rook.HasMoved` exists for the
  same reason.

### Per-piece state

- `Pawn.MoveWasAdvanceTwo` flags en passant eligibility. En passant generation
  is fully implemented and tested. The flag would need clearing after the
  opponent's next move - a game-layer responsibility that does not exist, so
  the lifecycle is unproven.
- Sliding pieces (rook, bishop) share the same pattern: iterate direction
  vectors, walk `dist` 1..7, stop at first blocker, capture if enemy. Queen is
  composed as `Rook.AvailableMoves + Bishop.AvailableMoves` - zero duplication.

### Test approach (the best idea in the stash)

Boards are constructed for tests by parsing a Unicode diagram:

```go
b := parseBoard(`
········
····♛···
··♗·····
·····♘··`, t)
```

This makes movement tests self-documenting and trivially reviewable. Rendering
is tested the same way in reverse (render then strip `{{...}}` markup, compare
to the expected diagram). Strongly worth replicating in any Rust
implementation - FEN parsing would serve the same role for chess, but the
diagram approach generalizes to hive.

### Completion state

Implemented and tested: board init/render, bishop, rook, queen, pawn
(advance one/two, capture, en passant). Not implemented:

- `King.AvailableMoves` and `Knight.AvailableMoves` are empty stubs
- Castling (scaffolding only: `Rook.HasMoved`, `SubsequentMove`)
- Pawn promotion
- Filtering moves that leave own king in check; checkmate/stalemate detection
- Entire game layer (Gamer, commands, turns, log, whose-move)

### Known bugs (do not port these)

1. `King.IsInCheck` dereferences `*move.TakeAt` without a nil check. Any
   enemy non-capturing move (TakeAt is nil) panics. It only "worked" because
   nothing calls it.
2. Check detection via enemy `AvailableMoves` will infinitely recurse once
   `King.AvailableMoves` is implemented naively (king asks "am I in check" ->
   enumerates enemy king moves -> enemy king asks "am I in check" -> ...).
   The Rust design needs a separate "squares attacked by" computation that
   does not filter for legality.

## Hex grid: direction and design

### Design

- Offset coordinates with column-parity neighbor tables
  (`neighbourOffsets[l.X&1][dir]`), flat-top hexes, six named directions
  (N, NE, SE, S, SW, NW). A large ASCII diagram in `hex.go` documents the
  layout - keep that habit.
- `Grid` is `map[int]map[int]interface{}`: sparse and unbounded, which is the
  right call for hive (board grows in any direction, no fixed extent).
- API: `SetTile`, `Tile`, `Find` (by `reflect.DeepEqual`), `Each` (callback
  with early-exit), `Neighbour`/`Neighbours`, `Bounds`.
- Tests cover set/get, the even-X neighbor ring, bounds, and find.

### Known bugs and weaknesses (do not port these)

1. `Each`'s early-exit `break` only exits the inner (row) loop; iteration
   continues over remaining rows. `Find` still terminates correctly only by
   luck (found/l already latched), but wastes work and the contract is broken.
2. `Bounds` initializes lower/upper to `{0,0}`, so a grid whose tiles are all
   at positive coordinates reports `lower = {0,0}` incorrectly (same for all
   negative and `upper`). The test happens to straddle the origin, masking it.
3. Map-of-maps iteration order is nondeterministic, so `Find` with duplicate
   tiles is nondeterministic - a real hazard for a deterministic game engine.
4. `interface{}` tiles + `reflect.DeepEqual` lookup is pre-generics Go
   awkwardness; irrelevant in Rust (generic `Grid<T>` or `HashMap<Coord, T>`).

### Lesson for a Rust hive attempt

Offset coordinates were the wrong choice even here: the parity-indexed
neighbor table is the classic offset-coordinate wart. Use axial or cube
coordinates (Red Blob Games hex guide) - uniform neighbor offsets, trivial
distance/rotation math (hive needs rotation-free sliding checks, ring walks,
and connectivity/one-hive checks, all of which are cleaner in cube coords).
Nothing hive-specific was ever written: no piece types, sliding rules,
one-hive rule, or placement rules. The hex library is the entirety of the
hive effort.

## Learnings summary for future (Rust) attempts

Worth keeping:

- Diagram-based board parsing in tests (self-documenting movement tests)
- `Move { from, to, take_at: Option<Loc>, subsequent: Option<Box<Move>> }`
  shape: capture square decoupled from destination handles en passant; chained
  move handles castling
- Team-as-sign (+1/-1) arithmetic for direction-symmetric pawn/rank math
  (in Rust, an enum with a `dir() -> i8` method achieves the same)
- Queen = rook moves + bishop moves composition
- Cheap-copy board value type to simulate moves for legality filtering
- Sparse map-backed grid for hive's unbounded board

Avoid:

- Deriving check detection from legal-move generation (recursion trap);
  compute attacked squares independently
- Offset hex coordinates; use axial/cube
- Origin-biased bounds computation; fold over actual keys with proper
  init-from-first-element
- Early-exit contracts that only break one loop level

Verdict: abandoned-direction experiment on the retired Go stack. All
transferable value is captured above; the stash itself can be dropped.
