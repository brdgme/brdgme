# Render Parity: Comparing Rust Ports Against Go

How to check that a Rust `_cli` port's plain-text output (wording, spacing,
column alignment, section ordering) matches the Go implementation it was
ported from. Do this for every in-repo Go port, not just at initial porting
time - `category-5-2` shipped with all table column spacing lost, and the
existing test suites did not catch it because they assert on state/logs, not
on rendered layout.

## Background: what the CLIs return

Both `brdgme-go/<game>_1/cmd` and the Rust `<name>_N_cli` binaries speak the
same one-shot JSON protocol on stdin/stdout (`brdgme-go/cmd/cli.go`,
`rust/lib/cmd/src/cli.rs`). Critically, **neither side pre-renders tables to
padded plain text**. Both `public_render.render` and
`player_renders[i].render` are raw markup strings using `{{...}}` tags -
`{{table}}{{row}}{{cell left}}...{{/cell}}{{/row}}{{/table}}`, `{{fg
rgb(r,g,b)}}...{{/fg}}`, `{{player N}}`, etc. Go's `brdgme-go/render`
package (`table.go`, `color.go`, `align.go`) builds these tag strings
directly; Rust's game crates build a `Vec<Node>` AST
(`rust/lib/markup/src/ast.rs`) which `brdgme_cmd`'s requester serializes
back to the same tag syntax via `brdgme_markup::to_string`
(`rust/lib/cmd/src/requester/gamer.rs`). Rust's markup parser
(`rust/lib/markup/src/parser.rs`) is explicitly written to parse both
dialects (see its "Backwards compatibility with Go brdgme" comment), so the
**same plain-text renderer works on markup from either language**.

Column/row spacing is not automatic in either language's `Table` type -
there is no spacing parameter on `Node::Table` (Rust) or on the markup
`{{table}}` tag itself. Go's `render.Table(cells, rowSpacing, colSpacing)`
helper (`brdgme-go/render/table.go`) inserts literal blank spacer cells
between columns to fake it. A Rust port must do the same by hand (empty
`N::text("  ")` cells between real cells) - if it doesn't, the table parses
and transforms fine, but the columns end up glued together. This is what
happened in `category-5-2`: see "Known bug" below.

RNGs differ between Go and Rust, so game states (and therefore card/dice
values) can never be seeded to match. Comparison is always structural: same
player count, same wording, same spacing/alignment/column widths, same
section ordering - never literal random values.

## Building the binaries

From the repo root:

```bash
# Go CLI for a game (e.g. category_5)
go build -o /tmp/render-parity/category_5_1_go ./brdgme-go/category_5_1/cmd

# Rust CLI for the matching port (e.g. category-5-2)
cd rust && cargo build --package category-5-2 --bin category_5_2_cli
# binary lands at rust/target/debug/category_5_2_cli
```

Also build the plain-text render helper once (see below):

```bash
cd rust && cargo build --package brdgme_render_plain
# binary lands at rust/target/debug/render_plain
```

## Sending requests

Both binaries read one JSON request from stdin and write one JSON response.
Shared request shapes:

```
{"New":{"players":2}}
{"PubRender":{"game":"<state string>"}}
{"PlayerRender":{"player":0,"game":"<state string>"}}
{"Play":{"player":0,"command":"...","names":["Alice","Bob"],"game":"<state string>"}}
```

The `game` state string comes from the `state` field of a previous
response's `game` object (`.New.game.state`, `.Play.game.state`, ...). It is
implementation-specific: a Go state string only round-trips through the Go
binary, a Rust state string only through the Rust binary. Use identical
`names` on both sides so player-name-length effects don't confound the
comparison.

Example - start a game and grab player 0's render on each side:

```bash
echo '{"New":{"players":2}}' | /tmp/render-parity/category_5_1_go | jq -r '.New.player_renders[0].render'
echo '{"New":{"players":2}}' | rust/target/debug/category_5_2_cli | jq -r '.New.player_renders[0].render'
```

(Go's JSON field names are also `snake_case`, e.g. `.New.player_renders[0].render`,
`.New.game.state`, `.New.public_render.render` - matching the Rust response shape.)

To play a command, extract a valid value from the current render/state (e.g.
a card number from the "Your hand" row) and feed the previous state back in:

```bash
STATE=$(echo '{"New":{"players":2}}' | /tmp/render-parity/category_5_1_go | jq -r '.New.game.state')
echo "{\"Play\":{\"player\":0,\"command\":\"play 21\",\"names\":[\"Alice\",\"Bob\"],\"game\":$(printf '%s' "$STATE" | jq -Rs .)}}" \
  | /tmp/render-parity/category_5_1_go
```

## Rendering markup to plain text

`rust/tools/render_plain` (workspace member `brdgme_render_plain`, binary
`render_plain`) reads a markup string on stdin and writes plain text to
stdout, substituting player names given as CLI args (arg order = player
index):

```bash
cd rust && cargo build --package brdgme_render_plain
echo '{{b}}Hi {{player 0}}{{/b}}' | ./target/debug/render_plain Alice Bob
# -> Hi <Alice>   (bold markers are stripped in plain output)
```

It works on markup from *either* language - it just runs
`brdgme_markup::from_string` -> `transform` -> `plain`
(`rust/lib/markup/src/lib.rs`, `transform.rs`, `plain.rs`), the same pipeline
`rust/tools/repl` uses for its terminal output, generalized to take player
names as arguments instead of an interactive session.

`scripts/render-compare/render.sh` chains a request through a binary, a jq
filter, and `render_plain` in one step:

```bash
echo '{"New":{"players":2}}' | scripts/render-compare/render.sh \
  /tmp/render-parity/category_5_1_go '.New.player_renders[0].render' Alice Bob

echo '{"New":{"players":2}}' | scripts/render-compare/render.sh \
  rust/target/debug/category_5_2_cli '.New.player_renders[0].render' Alice Bob
```

## Known bug: category-5-2 table spacing

Side-by-side plain output at `New` with 2 players (`Alice`, `Bob`), player 0's
render, using the procedure above:

```
=== Go (category_5_1) ===
#1  74                     1 pts
#2  77                     5 pts
#3  100                    3 pts
#4  5                      2 pts

Your hand:  4  10  14  44  80  83  89  98  99  103

Legend: 1 pts, 2 pts, 3 pts, 5 pts, 7 pts

Players  Taken  Pts
<Alice>    0     0
<Bob>      0     0

66 points until the end of the game.

=== Rust (category-5-2) ===
#137          1 pts
#256          1 pts
#341          1 pts
#428          1 pts

Your hand:9173435576272848789

Legend:1 pts, 2 pts, 3 pts, 5 pts, 7 pts

PlayersTakenPts
<Alice>0    0
<Bob>  0    0

66 points until the end of the game.
```

(Card values differ because RNGs differ - ignore them. The bug is that Rust's
columns have no gap between them at all, while Go's board/hand/score columns
are visibly separated.)

**Root cause: per-game render code, not the shared markup library.** The
shared `brdgme_markup::transform` table logic correctly pads every cell to
its column's max width (see the `table_align_works` unit test in
`rust/lib/markup/src/transform.rs`) - that part of the pipeline is not
broken. The bug is that `rust/game/category-5-2/src/render.rs`
(`render_board`, `render_hand`, `render_scores`) builds table rows with no
spacer cells between real columns, whereas the Go equivalent
(`brdgme-go/category_5_1/render.go`) calls
`render.Table(cells, rowSpacing, colSpacing)` with `colSpacing=2`, which
inserts literal blank spacer cells between every column
(`brdgme-go/render/table.go`). `Node::Table` in Rust has no such spacing
parameter - Rust game code must insert the blank cells itself. Because
`category-5-2` never does, its table columns are correctly *aligned* (each
column is padded to its own max width) but have zero gap between them, so
they read as glued together.

Implication for other per-game audits: check whether each Rust game's
render code inserts spacer cells matching the Go source's `colSpacing`
argument to `render.Table(...)`, not just whether it uses `N::Table` at all.

## What to compare

For **both** `public_render.render` and **every** `player_renders[i].render`,
at `New` and after a few representative `Play` commands (to reach non-trivial
mid-game states):

- Wording: labels, headers, punctuation, pluralization.
- Whitespace / column spacing: gaps between table columns, indentation.
- Alignment: left/center/right per column.
- Row/section ordering: which blocks appear, and in what order.
- Headers/legends present on one side but not the other.

Do **not** compare: random values (card/dice draws, shuffled order),
anything that depends on player name length (use identical names on both
sides to eliminate this).

Colors (`{{fg ...}}` / `{{bg ...}}`) are stripped by `render_plain` (it
targets plain text, matching the "no colors needed" scope of this
comparison) - if color parity ever matters, use `brdgme_markup::ansi`
instead of `plain` (see `rust/lib/markup/src/ansi.rs`, `rust/tools/repl`).
