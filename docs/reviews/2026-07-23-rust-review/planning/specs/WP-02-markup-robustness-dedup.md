# WP-02: markup robustness and dedup

**Findings:** ls F2 (major), ls F3 (major), ls F4, F5, F6, F7, F8, F9 (minor),
ls F10, F11 (nit) - all numbered against `findings/verification/lib-support.md`
(F10 = raw F11 `panic!`/`align_arg`, F11 = raw F12 stale doc; raw F10, the
ANSI/plain escaping item, is REJECTED-by-omission and out of scope).
**Decision:** D-37 answered option **A** - error on a non-empty parse
remainder AND escape braces in `to_string`.

**Rebase:** WP-01 already landed `ls F1` in `rust/lib/markup/src/transform.rs`
(`slice`). Do not revisit it. WP-01/WP-06 left `parser.rs`, `wrap.rs`, `lib.rs`
untouched for this package.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

- **ls F2** - `parse_u8` and `parse_usize` (`rust/lib/markup/src/parser.rs`)
  end in `.parse::<T>().unwrap()`, so any overflowing digit run panics.
- **ls F3** - `markup_` is `many(choice(...))` and `from_string`
  (`rust/lib/markup/src/lib.rs`) returns `Ok((nodes, rest))`. A literal `{` in
  text is unrepresentable (`text()` is `many1(none_of("{"))`), so parsing stops
  there and every caller discards `rest` - output silently truncates.
- **ls F4** - `to_string` (`lib.rs`) emits `Node::Text` verbatim, so braces in
  text re-parse as markup and `from_string(to_string(n)) != n`.

## 2. Why it's wrong

- **ls F2 is correct as written.** Verified live: both helpers unwrap;
  `parse_pct` in the same file already shows the correct `and_then` +
  `ok_or_else` pattern. `{{player 99999999999999999999}}` reaches it.
- **ls F3 is correct as written.** Verified live: `text()` cannot consume `{`,
  there is no escape, and every caller (`web/src/rules.rs`,
  `web/src/game/server_fns.rs` x4, `web/src/email/render.rs`,
  `web/src/theme.rs`, `lib/cmd/src/repl.rs`, `tools/render_plain`) drops `rest`.
- **ls F4 is correct as written** and is the serializer half of the same hole.
- The escape token must be **`{{lbrace}}`**, not a bare doubled `{{`. A bare
  `{{` escape would match the leading `{{` of every closing tag (`{{/b}}`),
  so nested `markup()` would consume the terminator and every container tag
  would fail to parse. `{{lbrace}}` stays in the `{{...}}` family D-37 asked
  for and cannot collide with any tag or closing tag.

## 3. Required end state

### 3a. `parser.rs::parse_u8` / `parser.rs::parse_usize` (F2)

Both rewritten in the `parse_pct` shape: `many1(digit()).and_then(...)`,
mapping the `Err` from `str::parse` to
`<StreamErrorFor<Input>>::message_static_message("...")`. No `unwrap`. Overflow
becomes an ordinary parse failure; valid inputs behave exactly as today.

### 3b. `parser.rs::text` - the `{{lbrace}}` escape (F3)

`text()` becomes `many1` over a `choice` of `none_of("{".chars())` and
`attempt(string("{{lbrace}}")).map(|_| '{')`, still collecting into one
`String` and one `Node::Text`. Producing a single char (not a separate node)
is what makes the round trip in 3d exact. `markup_`'s alternative list is
unchanged - `text()` already sits after every tag alternative.

### 3c. `lib.rs::from_string` - hard error on leftover input (F3)

Keep the existing `Result<(Vec<Node>, &str), MarkupError>` signature (callers
are spread across web/, lib/cmd and tools; changing it is a non-goal). After a
successful `markup().parse(input)`, return `Err` when `rest` is non-empty. On
`Ok`, `rest` is now always `""`.

### 3d. `lib.rs::to_string` - escape braces in text (F4)

The `Node::Text` arm emits `t.replace('{', "{{lbrace}}")`. `}` needs no
escaping (`text()` consumes it). No other arm changes.

### 3e. Stored-content risk assessment (D-37 user flag) - **reading only**

Before landing 3c, write a short risk note into this package's commit message
or `planning/` (implementer's choice), derived **by reading code and
migrations only - do NOT query any database**: enumerate the stored markup
columns (start from `rust/web/migrations/001_initial_schema.sql`; game renders
are recomputed at request time, log bodies are persisted) and list, for each
`from_string` caller, what a newly-failing parse does today
(`unwrap_or_default`, `unwrap_or_else(|_| (vec![], ""))`, `RenderError`, or
`.unwrap()`). Report to the Lead if any caller would panic rather than degrade.

### 3f. `error.rs::MarkupError` (F9)

`Parse` gains a `String` payload (`#[error("failed to parse input: {0}")]`).
`from_string` fills it with the `Display` of combine's error for a parse
failure, and with the byte offset (`input.len() - rest.len()`) plus a short
truncated snippet of `rest` for the new leftover-input error. **The finding's
"combine position/expected info" is not obtainable here**: `&str`'s
`StreamOnce::Error` is `StringStreamError`, which carries neither; switching
to `combine::easy` is a non-goal. Update the two `MarkupError::Parse`
construction/match sites the compiler points at.

## 4. Non-goals

- Anything in `rust/lib/color` beyond adding the player-count const (WP-05 owns
  lib/color, BLOCKED-ON-DECISION D-39). Do not touch `palette.rs` colour data.
- `lib/cmd`'s `from_string(...).unwrap()` calls in `repl.rs` - WP-06.
- Changing `from_string`'s return type, adopting `combine::easy`, or replacing
  `combine`. Do not touch `transform.rs::slice` (WP-01) or the ANSI/plain
  renderers. Do not unify the `fg`/`bg` HTML wrappers - the two renderers take
  different colour types; only `escape` is shared (F6).
- Do not change `word_wrap`'s space-splitting behaviour, only its width unit
  and docstring (F8, verification ADJUSTED: mid-line space runs are preserved).

## 5. Regression test cases

- `parser.rs` `mod tests`: `{{fg rgb(999,1,1)}}x{{/fg}}` and
  `{{player 99999999999999999999}}` return `Err` from `markup().parse`, no
  panic. `{{lbrace}}` inside text yields one `Node::Text` containing `{`; a
  `{{lbrace}}` inside a `{{b}}...{{/b}}` body still terminates correctly.
- `lib.rs` `mod tests`: `from_string("a{b")` is `Err`; `from_string("{{b}}x")`
  (unterminated) is `Err`; a well-formed document still returns `Ok` with an
  empty `rest`. Round trip: for nodes containing `Text("a{{b}}c")`,
  `from_string(&to_string(nodes)).unwrap().0 == nodes`.
- `wrap.rs` `mod tests`: a line of multi-byte chars wraps at `width` **chars**,
  not bytes (e.g. accented words at width 10); existing ASCII tests unchanged.
- `html_class.rs` `mod tests`: `markup_class_css()` emits one `player-{n}` rule
  per `Palette::player_colors()` slot (drift guard for F7).

## 6. Riders

| Finding | File / fn | Fix | Test |
|---|---|---|---|
| F5 | `parser.rs::rgb_reverse_map` | Delete the `eprintln!`; **keep** the `Foreground` fallback (it is the legacy Go `rgb()` compat path and making it an error would newly break stored logs, doubly so with 3c) and document it on `from_string`'s doc comment | n |
| F6 | `html.rs::escape`, `html_class.rs::escape` | Make `html::escape` `pub(crate)`; delete the copy in `html_class.rs` and `use crate::html::escape` | n |
| F6 | `transform.rs::player`, `semantic.rs::player` | Extract `pub(crate) fn player_name(name: Option<&str>, p: usize) -> String` (returns the name or `format!("Player {}", p)`) into `ast.rs`; both call it. Colour resolution stays per-module | n |
| F7 | `lib/color/src/palette.rs` + `lib/color/src/lib.rs` | Add `pub const PLAYER_COUNT: usize = 8;` next to `Palette::player_colors`, re-export it from `lib.rs`, change `player_colors` to return `[Color; PLAYER_COUNT]`, and replace `html_class.rs`'s local `PLAYER_COUNT` with the import | y (see 5) |
| F8 | `wrap.rs::wrap_segment` | Width check uses `current.chars().count() + 1 + word.chars().count()`; extend `word_wrap`'s docstring to state that runs of spaces at line starts/wrap points are collapsed | y (see 5) |
| F10 | `parser.rs::col_trans`, `parser.rs::align_arg` | Replace `choice([string(..)])` + `.map(match)`/`from_str().unwrap()` with `choice((attempt(string("mono")).map(\|_\| ColTrans::Mono), ...))` so the variant comes from the branch; no `panic!`, no `unwrap`. Keep alternative order (`mono`, `inv`, `contrast`; `left`, `center`, `right`) | n |
| F11 | `ast.rs::TNode::len` | Delete the stale second sentence "Panics if it detects an untransformed node." | n |

**Open question:** D-37 named "`{{` or backslash" as the escape. A bare `{{` is
unworkable (see section 2) and backslash would additionally require escaping
`\` itself, so this spec pins `{{lbrace}}`, matching D-37's stated preference
for `{{`-family syntax. Flag to the Lead if a different token is wanted before
3b lands.
