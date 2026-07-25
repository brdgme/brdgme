# Verification: lib/markup findings (lib-support.md F1-F11)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot/rust @ f8763a5. All paths relative to that root.

## F1 - slice() byte/char offset mismatch - CONFIRMED (critical)

Evidence:
- ast.rs:198-207 `TNode::len` counts chars: line 201 `TNode::Text(ref text) => text.chars().count(),`
- ast.rs:211-248 `TNode::bg_ranges` also uses `t.chars().count()` (line 217), so all offsets in the canvas pipeline are char counts.
- transform.rs:273-275 slices by byte offset:
  ```rust
  TNode::Text(ref text) => {
      TNode::Text(text[start..cmp::min(text.len(), end)].to_string())
  }
  ```
  `text.len()` is bytes; `text[start..end]` indexes bytes. With multi-byte chars, char offsets and byte offsets diverge: indexing on a non-char-boundary panics; where boundaries coincidentally align, wrong content is extracted.
- Reachability: transform.rs:316-408 `canvas()` calls `slice` in the bg-inheritance path (lines 337, 341-344) and the overlap-trimming path (lines 368, 373-377). `canvas` is invoked from both `transform_with_palette` (transform.rs:79-81) and `transform_semantic` (semantic.rs:89-91), i.e. every render of a `{{canvas}}` containing non-ASCII text in an overlapping/bg-inheriting position.
- Node-skip sub-claim: transform.rs:264 is `if n_len < start`. A node with `n_len == start` ends exactly at `range.start` (it covers offsets `[0, n_len)`) and should be skipped, but the `<` comparison lets it fall through to the slice arms - Text yields `text[start..]` with `start == char count` (itself a byte/char hazard on multi-byte text) and container variants recurse, pushing spurious empty nodes. Claim accurate.

Severity: critical is appropriate - reachable runtime panic in the shared render path (docs/CODING.md:46 forbids panicking code in runtime paths).

## F2 - parse_u8/parse_usize unwrap on overflow - CONFIRMED (major)

Evidence:
- parser.rs:54 `many1(digit()).map(|s: String| s.parse::<u8>().unwrap())`
- parser.rs:79 `many1(digit()).map(|s: String| s.parse::<usize>().unwrap())`
- `"999".parse::<u8>()` is `Err` -> unwrap panics. combine's `attempt` backtracks parse errors, not panics, so the process aborts.
- Reachable: `col_type_rgb` (parser.rs:281-290) feeds `{{fg rgb(999,1,1)}}` digits into parse_u8; `player` (parser.rs:353), `layer`, `align`, `indent` feed parse_usize - a 20-digit number (> u64::MAX ~ 1.8e19) panics. Both reachable from `from_string` (lib.rs:37-39), called on stored log bodies and render output (web/src/game/server_fns.rs:265,392,411,699; web/src/rules.rs:139; web/src/email/render.rs:72).
- Same file has the correct pattern: parse_pct at parser.rs:64-71 uses `.and_then(...ok()...ok_or_else(StreamError::message_static_message))`.

Severity: major fits. Input is normally produced by trusted game crates via to_string, so not directly attacker-controlled in the common path; still a process-abort on malformed stored data, against the no-panic rule.

## F3 - many(choice) silently truncates at unparseable input - CONFIRMED (major)

Evidence:
- parser.rs:18-30 `markup_` = `many(choice((attempt(bold()), ..., attempt(text()), ...)))`.
- parser.rs:467 `text()` = `many1(none_of("{".chars()))` - cannot consume `{`.
- On input like `"a{b"`: text() consumes `"a"`, every alternative then fails at `{` (all tag parsers need `{{<known tag>`), `many` stops with success. `markup().parse(input)` returns `Ok((nodes, "{b"))` - no eof requirement anywhere.
- lib.rs:37-39 `from_string` returns `Ok((Vec<Node>, &str))` and all callers discard the remainder:
  - web/src/rules.rs:139 `let (nodes, _) = brdgme_markup::from_string(content).map_err(...)?;`
  - web/src/game/server_fns.rs:265 (and 392, 411, 699) `let (nodes, _) = ...`
  - web/src/email/render.rs:72 `let (nodes, _) = ...unwrap_or_default();`
- No escape syntax exists for a literal `{` (parser has no escape rule; to_string at lib.rs:45 emits text verbatim).

So a stray `{`, unknown tag, or unterminated tag silently drops the rest of the document with an Ok result. Major correctness defect, severity appropriate.

## F4 - to_string emits raw text, no round-trip / markup injection - CONFIRMED (minor)

Evidence:
- lib.rs:45 `Node::Text(ref t) => t.to_owned(),` - no escaping of `{{`.
- A `Node::Text("{{b}}x{{/b}}")` serializes to the literal tag string, which parser.rs re-interprets as `Node::Bold` on the next from_string. Round-trip is broken for any text containing tag syntax; text content (e.g. player-influenced strings embedded by game crates) can inject markup.

Minor is defensible given text rarely contains `{{` in practice, though combined with F3 (no escape mechanism exists at all) it is a real design gap.

## F5 - rgb_reverse_map warns and substitutes instead of failing - CONFIRMED (minor)

Evidence:
- parser.rs:263-272: unknown rgb triple hits `eprintln!("warning: unknown rgb colour rgb({},{},{}), falling back to foreground", ...)` and returns `ColType::Named { color: NamedColor::Foreground, soften: None }`.
- Contrast: col_type_named at parser.rs:165-172 fails the parse via `and_then` + `message_static_message("unknown named colour")`.
- Inconsistent error strategy within one file; eprintln from a library parse path is also poor. Minor/quality is right.

## F6 - duplicated escape/fg/bg/b/player code - CONFIRMED (minor)

Evidence:
- html.rs:20-25 and html_class.rs:63-68 `escape` are byte-identical (`&`->`&amp;`, `<`->`&lt;`, `>`->`&gt;`).
- html.rs:5-18 fg/bg/b vs html_class.rs:99-117: same structure, differing only in style-vs-class attribute; `b()` identical.
- transform.rs:87-100 `player()` vs semantic.rs:97-110 `player()`: identical name-resolution (`players.get(p).map(...).unwrap_or_else(|| format!("Player {}", p))`) and identical Bold(Fg(text("<name>"))) shape; only colour resolution differs.
- HTML escaping maintained in two places is the sharpest point (a fix in one can miss the other). Minor/simplicity is right.

## F7 - PLAYER_COUNT hardcoded to 8 - CONFIRMED (minor)

Evidence:
- html_class.rs:6-7:
  ```rust
  /// Number of player colour slots (matches `Palette::player_colors`).
  const PLAYER_COUNT: usize = 8;
  ```
- lib/color/src/palette.rs:119 `pub fn player_colors(&self) -> [Color; 8]`. No code-level linkage (could be `LIGHT.player_colors().len()` or a shared const in brdgme_color). Growing the palette array would compile clean while `markup_class_css` omits the new `player-8` rules; test at html_class.rs:264 even asserts `!css.contains("player-8")`. Minor/consistency is right.

## F8 - word_wrap byte measurement / space handling - ADJUSTED (minor)

Correct parts:
- wrap.rs:16 `current.len() + 1 + word.len() <= width` measures bytes; multi-byte text wraps earlier than `width` display columns. Accurate.
- Leading spaces are dropped: `"  a".split(' ')` yields `["", "", "a"]`; empty words hit the `current.is_empty()` branch (wrap.rs:14-15) and leave `current` empty.
- Docstring (wrap.rs:1) only mentions newline preservation. Accurate.

Correction:
- "s.split(' ') collapses space runs" is overstated. Mid-line runs are preserved: for `"a  b"` the empty token takes the `else if` branch and appends `' ' + ""`, then `"b"` appends `' ' + "b"`, producing `"a  b"` unchanged. Runs collapse only at line starts (leading spaces) and when a wrap point lands inside a run (the empty token starts a new line as `""` and is dropped). Also, interior empty tokens still consume width budget (each contributes `+1`), a subtlety the finding doesn't mention but which doesn't change severity.

Severity minor stands.

## F9 - MarkupError::Parse discards combine diagnostics - CONFIRMED (minor)

Evidence:
- error.rs:3-7: `MarkupError` has the single unit variant `Parse` ("failed to parse input").
- lib.rs:38 `markup().parse(input).map_err(|_| MarkupError::Parse)` - combine's error (position, expected tokens, the `message_static_message` texts carefully added in parse_pct/col_type_named/col_type_mix/col_type_soften) is thrown away, so callers can never surface why or where parsing failed. Minor/quality is right.

## F10 - residual panic sites in parser - CONFIRMED (nit)

Evidence:
- parser.rs:298-307 `col_trans`: input restricted by `choice([string("mono"), string("inv"), string("contrast")])`, then `_ => panic!("invalid transform")` at line 306 - unreachable by construction.
- parser.rs:459 `choice([string("left"), string("center"), string("right")]).map(|s| Align::from_str(s).unwrap())` - `Align::from_str` (ast.rs:31-41) accepts exactly those three strings, so unwrap is unreachable.
- The project rule exists: docs/CODING.md:46 "No panicking code in runtime paths. `.unwrap()`, `.expect()`, `panic!()`, ..."
- Unreachable today but fragile against edits to the choice lists. Nit is the right severity.

## F11 - stale TNode::len doc comment - CONFIRMED (nit)

Evidence:
- ast.rs:197 `/// Calculates the length of the containing text.  Panics if it detects an untransformed node.`
- ast.rs:198-207: the body matches exhaustively on TNode's four variants (Text/Fg/Bg/Bold, ast.rs:182-187) and contains no panic path. TNode has no "untransformed" variants (that concept belongs to `Node`). Doc is stale/wrong. Nit is right.

## Summary table

| Finding | Verdict | Severity |
|---|---|---|
| F1 slice byte/char | CONFIRMED | critical |
| F2 parse_u8/usize unwrap | CONFIRMED | major |
| F3 silent tail truncation | CONFIRMED | major |
| F4 to_string no escaping | CONFIRMED | minor |
| F5 rgb fallback + eprintln | CONFIRMED | minor |
| F6 duplicated escape/render/player | CONFIRMED | minor |
| F7 PLAYER_COUNT hardcode | CONFIRMED | minor |
| F8 word_wrap bytes/spaces | ADJUSTED (space-run collapse only at line starts and wrap points; byte-measure and leading-space claims correct) | minor |
| F9 error discards diagnostics | CONFIRMED | minor |
| F10 unreachable panics | CONFIRMED | nit |
| F11 stale len() doc | CONFIRMED | nit |
