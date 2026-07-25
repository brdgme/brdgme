# Findings — lib-support

Unit: `lib/cmd` (1,070), `lib/game_client` (738), `lib/markup` (2,709),
`lib/color` (4,119), `lib/cost` (492), `lib/rand_bot` (142). ~9.3k LOC.
Reviewed against the snapshot worktree at `f8763a5`. Lead-verified: all
critical/major findings were independently re-checked against snapshot
source before inclusion.

## lib/markup

### slice() indexes text by byte offset while all offsets are char counts
- severity: critical
- category: correctness
- location: lib/markup/src/transform.rs:274
- finding: Every offset flowing into `slice()` comes from `TNode::len`,
  which counts `text.chars().count()` (lib/markup/src/ast.rs:201). But
  `slice` does `text[start..cmp::min(text.len(), end)]` where `text.len()`
  is the byte length and the indices are used as byte offsets. For any
  multi-byte character (box-drawing glyphs in canvas boards, accented
  player names, emoji) this either panics (`byte index N is not a char
  boundary`) or, via the `min(text.len(), end)` clamp, silently slices the
  wrong byte range and corrupts output. The whole `canvas()`
  bg-inheritance/overlap pipeline routes through `slice`, so any
  non-ASCII glyph inside a `{{canvas}}` layer is a latent crash.
  Additionally the node-skip check at transform.rs:264 uses `n_len <
  start` rather than `<=`, so a node ending exactly at `range.start` is
  recursed into instead of skipped.
- recommendation: Slice by chars, e.g.
  `text.chars().skip(start).take(end - start).collect::<String>()`.
  Change `<` to `<=` at line 264. Add a regression test with a multi-byte
  character in a canvas layer.

### parse_u8 / parse_usize unwrap on overflow — malformed markup panics the process
- severity: major
- category: quality
- location: lib/markup/src/parser.rs:54
- finding: `many1(digit()).map(|s: String| s.parse::<u8>().unwrap())`
  (and the identical `parse_usize` at parser.rs:79) panics on any digit
  run that overflows the target type. `{{fg rgb(999,1,1)}}` or
  `{{player 99999999999999999999}}` are reachable inputs (a typo in
  game-authored markup) that abort the server process rather than
  producing a parse error. Violates the project no-unwrap-in-runtime-paths
  rule; the same file already contains the correct pattern in `parse_pct`
  (parser.rs:59-72, `and_then` + `ok_or_else`).
- recommendation: Rewrite both parsers in the `parse_pct` style so
  overflow becomes an ordinary parse failure.

### Malformed/unterminated markup silently truncates instead of erroring
- severity: major
- category: correctness
- location: lib/markup/src/lib.rs:37
- finding: `markup_` is `many(choice(...))`; when no alternative matches
  at a position — an unterminated tag, an unknown tag, or any literal `{`
  in text (`text()` is `many1(none_of("{"))` and can never consume `{`) —
  `many` stops and the parse "succeeds" with the entire tail in `rest`.
  `from_string` returns `Ok((nodes, rest))` and nothing forces callers to
  check `rest`: every web caller discards it (`let (nodes, _) = ...`,
  e.g. web/src/rules.rs:139, web/src/game/server_fns.rs:265,
  web/src/email/render.rs:72), so a single stray `{` in a markup string
  silently drops everything after it. The grammar also has no escape
  mechanism, so a literal `{` is unrepresentable.
- recommendation: In `from_string`, return `Err(MarkupError::Parse)` when
  `rest` is non-empty (audit the few callers that may legitimately want
  streaming). Longer term, add an escape for a literal `{` or make
  `text()` consume `{` not followed by a known tag name.

### to_string emits Node::Text raw — no round-trip, markup injection through text
- severity: minor
- category: correctness
- location: lib/markup/src/lib.rs:45
- finding: `Node::Text(ref t) => t.to_owned()` means any `{{...}}`
  sequence inside a text node is re-interpreted as markup on the next
  `from_string`, so `from_string(to_string(nodes)) != nodes` for such
  input, and text built from external strings can inject tags into a
  serialized document. Consequence of the missing escape hatch above, but
  the serializer side is where half the fix lives.
- recommendation: Once an escape convention exists, apply it here so
  serialization round-trips.

### Library eprintln!s and silently substitutes a color on unknown rgb() values
- severity: minor
- category: quality
- location: lib/markup/src/parser.rs:264
- finding: `rgb_reverse_map` does `eprintln!("warning: unknown rgb
  colour …")` and silently substitutes `Foreground`. A library printing
  to stderr on a parse path is un-idiomatic (callers cannot route or
  suppress it), and the silent colour swap hides authoring errors —
  inconsistent with sibling parsers like `col_type_named`, which fail the
  parse on unknown names.
- recommendation: Make unknown rgb values a parse error (as
  `col_type_named` does), or if the fallback is load-bearing for
  backwards compatibility, drop the `eprintln!` and document the fallback
  on `from_string`.

### Duplicated logic across renderers and transforms
- severity: minor
- category: simplicity
- location: lib/markup/src/html.rs:20
- finding: `escape` is byte-identical in html.rs:20-25 and
  html_class.rs:63-68; the `fg`/`bg`/`b` wrappers in html.rs:5-18 and
  html_class.rs:99-117 are near-identical; and the `player()`
  name-resolution/`<name>` formatting is duplicated between
  transform.rs:87-100 and semantic.rs:97-110. The HTML escaping is
  security-relevant code maintained in two places (player names flow
  through these renderers) — a future gap in one copy is an XSS hole.
- recommendation: Hoist `escape` into one shared function used by both
  HTML renderers; extract a shared `resolve_player_name(p, fallback) ->
  String` used by both `player()` fns.

### PLAYER_COUNT = 8 hardcoded, duplicating palette knowledge
- severity: minor
- category: consistency
- location: lib/markup/src/html_class.rs:7
- finding: The constant "matches `Palette::player_colors`" by comment
  convention only. If the palette grows a 9th player colour, the
  generated CSS silently omits `player-8` rules and games render unstyled
  for that player.
- recommendation: Export a `PLAYER_COUNT` const (or derive from
  `player_colors().len()`) from `brdgme_color` and use it here.

### word_wrap measures bytes, not chars, and collapses whitespace
- severity: minor
- category: correctness
- location: lib/markup/src/wrap.rs:16
- finding: `current.len() + 1 + word.len()` uses byte length, so
  non-ASCII text wraps earlier than `width` chars — inconsistent with
  `TNode::len`'s char counting used everywhere else in the crate. Also
  `s.split(' ')` collapses runs of spaces and drops leading spaces, so
  spacing-sensitive input isn't preserved; the docstring only discloses
  newline preservation.
- recommendation: Use `.chars().count()` for width; decide whether
  space-collapsing is intended and document it (or use
  `split_inclusive(' ')` if preservation is wanted).

### Error type discards all parse diagnostics
- severity: minor
- category: quality
- location: lib/markup/src/error.rs:3
- finding: `map_err(|_| MarkupError::Parse)` (lib/markup/src/lib.rs:38)
  throws away combine's position/expected-token information, so the only
  feedback an author gets for broken markup is "failed to parse input"
  with no location.
- recommendation: Carry the formatted combine error (or at least the
  position) in the error, e.g. `Parse(String)`.

### Player-supplied text passes through ANSI/plain renderers unescaped
- severity: minor
- category: correctness
- location: lib/markup/src/ansi.rs:18
- finding: The HTML renderers escape text, but `ansi`/`plain`
  (plain.rs:7) push raw text — a player name containing ESC/control bytes
  would inject terminal control sequences into other players' terminals
  (log rendering, CLI tools). Names are validated upstream, but the HTML
  side shows the crate already accepts the "renderers sanitize"
  responsibility.
- recommendation: Strip control characters from text nodes in
  `ansi::render`, or document that sanitization is the caller's job.

### panic!("invalid transform") and align_arg unwrap
- severity: nit
- category: quality
- location: lib/markup/src/parser.rs:306
- finding: Both this and the `Align::from_str(s).unwrap()` at
  parser.rs:459 are unreachable by construction (the preceding `choice`
  guarantees the matched string), but they are the last panic sites in
  the crate and the project rule is no panics in runtime paths. The
  choices can produce the enum directly, making unreachability
  structural.
- recommendation: Restructure as
  `choice((string("mono").map(|_| ColTrans::Mono), …))` and map the align
  strings to `Align` variants inside the choice.

### Stale doc comment on TNode::len
- severity: nit
- category: consistency
- location: lib/markup/src/ast.rs:197
- finding: "Panics if it detects an untransformed node" — `TNode` has no
  untransformed variants and the function cannot panic. Leftover from a
  previous design.
- recommendation: Delete the second sentence.

Clean areas checked: HTML/XSS escaping of `& < >` in text nodes (both
renderers), ANSI style save/restore nesting, table/align/canvas
arithmetic (no underflow), module split (ast/parser/renderers/transform
are coherent units), test coverage of happy paths. `combine` 4.6.7 is
mature but low-activity upstream (final 4.x line); for a grammar this
small a hand-rolled recursive-descent parser would be comparable in size
and would have avoided the overflow/diagnostics findings — keeping
`combine` is defensible, noted for the dependencies unit.

## lib/color

### regex + lazy_static exist solely to parse 6 hex digits, on an API nobody calls at runtime
- severity: major
- category: dependencies
- location: lib/color/src/lib.rs:51
- finding: `Color::from_hex` compiles an anchored regex via
  `lazy_static` (lib/color/Cargo.toml:9-10) to validate `#rrggbb`. Two
  compounding issues. (a) A regex engine is disproportionate for "strip
  `#`, check length 6, `u8::from_str_radix` on three 2-char slices" — and
  `brdgme_color` is the only crate in the workspace that depends on
  `regex`, so dropping it removes the whole regex/aho-corasick tree from
  every binary linking this crate. (b) The parse API it serves is
  effectively dead: outside the crate, `from_hex` is called only in
  brdgme_markup tests (lib/markup/src/transform.rs:455,478), and
  `Color::from_str` (lib.rs:69-85, the only consumer of the private
  `named()` alias table) has no runtime caller anywhere in the snapshot —
  markup resolves names through `NamedColor::from_str`
  (lib/markup/src/parser.rs:156) and web maps stored names through its
  own `PLAYER_COLOR_NAMES` table. The four `.unwrap()`s at
  lib.rs:54,58-60 (provably infallible, but non-locally so) also
  disappear with the regex.
- recommendation: Delete `Color::from_str`, `Color::from_hex`, and
  `named()` (or, if the API must stay, reimplement `from_hex` in ~10
  lines of std code). Either way drop `regex` and `lazy_static` from
  Cargo.toml.

### Color::mono computes the average with lossy per-channel division
- severity: minor
- category: correctness
- location: lib/color/src/lib.rs:28
- finding: `self.r / 3 + self.g / 3 + self.b / 3 >= 128` floors each
  channel before summing, losing up to 2 units versus the intended mean.
  Mid-grey `rgb(128,128,128)` computes `42+42+42 = 126 < 128` → black,
  whereas the true mean is exactly 128 → white. Boundary behaviour is
  systematically biased dark.
- recommendation: `let avg = (u16::from(self.r) + u16::from(self.g) +
  u16::from(self.b)) / 3;` then compare against 128 (and decide whether
  `>=` vs `>` at the exact midpoint is intended).

### Three divergent color-name alias tables
- severity: minor
- category: consistency
- location: lib/color/src/lib.rs:127
- finding: `named()` (lib.rs:127-158) knows aliases the other tables
  don't ("indigo"→blue, "lightblue"→blue, "teal"→cyan, "bluegrey"→cyan,
  white/black); `NamedColor::from_str` (palette.rs:61-83) has no aliases;
  markup's `resolve_named` (lib/markup/src/parser.rs:150-158) has a third
  set ("magenta", "amber", "black", "white"). Latent while `named()` is
  test-only (see previous finding), but any future caller of
  `Color::from_str` gets meaningfully different results than the markup
  parser.
- recommendation: If the parse API is kept, consolidate on one alias
  table (extend `NamedColor::from_str` and route everything through it).
  Disappears if the parse API is deleted per the major finding above.

### Palette data representation is ~4× more verbose than necessary
- severity: minor
- category: simplicity
- location: lib/color/src/palette.rs:138
- finding: ~3,000 of the 3,814 lines are `Color { r: 255, g: 85, b: 85 }`
  struct literals at 4-5 lines per color, 12 colors per palette, 34
  palettes. The data is not dead — all 34 statics are registered in
  `themes()` (palette.rs:3194-3237) and consumed by web theme CSS and the
  settings picker — but a `const fn rgb(r: u8, g: u8, b: u8) -> Color`
  would collapse each color to one line (~400 lines total) and make hex
  cross-checking against source specs easier. The field-by-field clones
  `LIGHT_PROTANOPIA`/`DARK_PROTANOPIA` (palette.rs:2707-2720, 2806-2819)
  could be struct-update syntax.
- recommendation: Add a const constructor and rewrite the literals
  mechanically (scripted transform, reviewable diff). No behavior change.

### themes() doc comment describes behavior the code doesn't have
- severity: nit
- category: quality
- location: lib/color/src/palette.rs:3190
- finding: "Light/Dark is assigned by each palette's actual `background`
  lightness, not by theme name" — but the category is hardcoded per entry
  in the THEMES array (e.g. `("alucard", Light, &ALUCARD)` at
  palette.rs:3198). Nothing computes anything from background lightness;
  the comment describes authoring rationale, not mechanism.
- recommendation: Reword to "categorised by each palette's background
  lightness (assigned manually at registration)".

### Inconsistent sRGB linearization thresholds, triplicated code
- severity: nit
- category: consistency
- location: lib/color/src/palette.rs:3266
- finding: The runtime `srgb_channel_to_linear` uses the precise IEC
  breakpoint 0.03928 while two test-only implementations
  (palette.rs:3438, 3655) use the rounded 0.04045, in three near-
  identical functions. Sub-quantization at 8-bit precision, so cosmetic.
- recommendation: Share one linearization helper (the runtime one) with
  the test module; note the chosen threshold in its doc comment.

### hex() and Display duplicate the same format string
- severity: nit
- category: quality
- location: lib/color/src/lib.rs:47
- finding: Two copies of `#{:02x}{:02x}{:02x}` (lib.rs:47-49 and the
  `Display` impl at lib.rs:119-123) that must stay in sync.
- recommendation: `pub fn hex(self) -> String { self.to_string() }`, or
  drop `hex()` in favour of `Display`.

Overall: the 4,119 LOC is justified in substance (~3,000 lines of palette
data for 34 themes, all reachable and guarded by unusually rigorous tests
— WCAG contrast floors, CIE76 player-colour distinctness, CVD-simulation
gates) but padded in form (const constructor would cut it to ~1,500-2,000
LOC). Core color math (`mix`/`soften` clamping, `contrast_ratio`, CVD
matrices) verified correct. The dependency footprint (regex/lazy_static
serving a dead API) is the main thing worth fixing.

## lib/cmd

### Panic on malformed request in the production HTTP path
- severity: major
- category: correctness
- location: lib/cmd/src/http.rs:54
- finding: `g.request(&req).unwrap()` panics inside the warp handler
  whenever `GameRequester::request` returns `Err`. Reachable with user
  input: `Request::Status`/`Play`/`PubRender`/`PlayerRender` all run
  `serde_json::from_str(game)?` (requester/gamer.rs:28,37,41,45), so any
  request with a syntactically invalid `game` string panics the
  connection task — the client gets a dropped connection instead of a
  `SystemError` JSON body, and the error bypasses the sentry transaction
  set up just above. Tellingly, `impl Reject for RequestError {}` exists
  at http.rs:17 but is never used — the error-to-rejection wiring was
  started and abandoned. This handler runs in every production game
  service.
- recommendation: Map the error into the wire protocol instead of
  panicking:
  `warp::reply::json(&g.request(&req).unwrap_or_else(|e| Response::SystemError { message: e.to_string() }))`,
  and delete the unused `impl Reject` (or actually reject with it).

### REPL undo/load leave stale renders
- severity: minor
- category: correctness
- location: lib/cmd/src/repl.rs:116
- finding: `:undo` (repl.rs:116-128) and `:load` (repl.rs:112-115)
  replace `game` but never update `public_render`/`player_renders`. On
  the next loop iteration the board, command spec, and whose-turn prompt
  render from the pre-undo state (repl.rs:91-98) while the subsequent
  `Play` request is built from the restored `game.state` — display and
  actual game diverge until the next successful play. Dev-tool only, but
  it is a real logic bug in a debugging tool, where trust in what's
  displayed matters most.
- recommendation: After undo/load, re-request renders (issue a
  `Request::Status { game: game.state.clone() }` and adopt its renders),
  or store renders on the undo stack alongside the game.

### bot_cli::cli and bot_cli::Response are dead code
- severity: minor
- category: simplicity
- location: lib/cmd/src/bot_cli.rs:20
- finding: Nothing in the workspace calls `bot_cli::cli` — the only
  consumer of the module is `rand_bot`, which uses only the `Request`
  struct (lib/rand_bot/src/lib.rs:107) and implements its own simplified
  CLI "because RandBot doesn't care about game / state". The `cli` fn
  (with its three `.unwrap()`s, e.g. bot_cli.rs:30) and the `Response`
  type alias are dead.
- recommendation: Delete `cli`/`Response` (keep `Request`), or move
  `Request` into `rand_bot` and delete the module.

### REPL spins forever on stdin EOF
- severity: minor
- category: correctness
- location: lib/cmd/src/repl.rs:226
- finding: `prompt()` ignores the byte count from `read_line`; at EOF it
  returns `""` immediately, forever. Mid-game an EOF (piping input, or
  the parent process of `tools/repl` dying) makes the loop fire empty
  `Play` commands in a hot spin. `read_line` errors are also `.unwrap()`ed
  (repl.rs:233).
- recommendation: Have `prompt` return `Option<String>`/`Result` and
  treat EOF as `:quit`.

### Panic-heavy runtime paths throughout the crate
- severity: minor
- category: consistency
- location: lib/cmd/src/requester/gamer.rs:67
- finding: Runtime `.unwrap()`/`panic!()` are pervasive and inconsistently
  applied. `cli.rs` carefully converts a request parse failure into
  `Response::SystemError` but then `.unwrap()`s the requester error and
  the stdout write (cli.rs:14-18); `gamer.rs` routes state-serialization
  failure through `GameResponseError` in `from_gamer` but `.unwrap()`s
  the identical serialization in `renders`/`handle_pub_render`/
  `handle_player_render` (gamer.rs:67,74,164,177); the repl unwraps
  markup parses of game-produced log content (repl.rs:177,219). Also
  repl.rs:55: `panic!("wrong reponse")` — typo and content-free message.
- recommendation: In `gamer.rs`, reuse the existing `GameResponseError`
  path for the render handlers; in `cli.rs`, mirror the `SystemError`
  conversion for requester errors. Leave the repl's panics if it is
  strictly a dev toy, but at least fix the panic message.

### term_size is unmaintained (RUSTSEC-2020-0163)
- severity: minor
- category: dependencies
- location: lib/cmd/Cargo.toml:16
- finding: `term_size` 0.3.2 is formally unmaintained; the advisory
  recommends `terminal_size` (RUSTSEC-2020-0163). Single call site
  (repl.rs:186), drop-in replacement.
- recommendation: Switch to `terminal_size`.

### warp vs axum stack drift
- severity: minor
- category: consistency
- location: lib/cmd/Cargo.toml:17
- finding: warp 0.4.3 itself is current, so this is not a currency issue —
  but the project runs two HTTP server stacks: axum 0.8.9 in `web` (and
  in game_client's dev-deps), warp here. Every game service binary links
  warp + tokio signal just for `serve()`. The handler is ~30 lines and
  would port to axum trivially, removing a framework from the tree.
- recommendation: Consolidation backlog item, not urgent. Note for the
  dependencies unit.

### Clippy comparison_to_empty in repl
- severity: nit
- category: consistency
- location: lib/cmd/src/repl.rs:147
- finding: `remaining_input.trim() != ""` should be
  `!remaining_input.trim().is_empty()`.
- recommendation: Use `is_empty()`.

### Redundant #[serde(default)] on Option field
- severity: nit
- category: quality
- location: lib/cmd/src/api.rs:14
- finding: `#[serde(default)]` on `seed: Option<u64>` is redundant —
  serde already defaults missing `Option` fields to `None`.
- recommendation: Remove the attribute.

### No content-length limit on the HTTP body
- severity: nit
- category: quality
- location: lib/cmd/src/http.rs:38
- finding: `warp::body::json()` has no content-length cap; game states
  are attacker-influenced in size. In-cluster and low risk, but
  `warp::body::content_length_limit(...)` is one line.
- recommendation: Add a content-length limit before `warp::body::json()`.

### Local requester never checks child exit status
- severity: nit
- category: quality
- location: lib/cmd/src/requester/local.rs:35
- finding: A crashed child surfaces as a confusing JSON parse error on
  empty stdout (local.rs:35-41) instead of "child exited with status N".
- recommendation: Check `ExitStatus` and include it in the error.

### First :undo is a silent no-op reset
- severity: nit
- category: correctness
- location: lib/cmd/src/repl.rs:59
- finding: `undo_stack` is seeded with the initial game, so the first
  `:undo` before any play silently resets instead of printing "No undos
  available".
- recommendation: Start the undo stack empty; push the pre-play state on
  each successful play.

Overall: sensible module split (api/requester/cli/repl/http, all small),
good `thiserror` usage in requester/error.rs, good Go-compat datetime
handling with tests in api.rs, and test_support.rs is a genuinely nice
contract harness. The one production-runtime concern is the http.rs:54
unwrap; the rest is dev-tool robustness and hygiene.

## lib/game_client

### No timeout enforced by the crate; one caller has none at all
- severity: major
- category: correctness
- location: lib/game_client/src/lib.rs:47
- finding: The retry policy's doc claims timeouts are retried and
  `send_with_retry` keys on `e.is_timeout()` (lib.rs:80) — but the crate
  never sets a timeout; it fully delegates to the caller's
  `reqwest::Client`. `web` (web/src/main.rs:32-34, 10s) and `bot`
  (bot/src/main.rs:786-788, 60s) configure one, but the operator uses
  bare `reqwest::Client::new()` (operator/src/controller.rs:230) — no
  timeout — so a game pod that accepts and hangs blocks that reconcile
  worker forever, and the documented timeout-retry path never fires.
- recommendation: Set a default per-request timeout inside
  `send_with_retry` (e.g. `request_builder.timeout(...)`, possibly on
  `RetryConfig`) so the crate's guarantee holds regardless of caller
  configuration.

### anyhow in a library crate; error kinds flattened to strings
- severity: minor
- category: consistency
- location: lib/game_client/src/lib.rs:102
- finding: Every other `lib/` crate (game, cmd, color, markup) uses
  `thiserror` with a typed error enum. Here transport failures, non-2xx
  statuses (lib.rs:102), `Response::SystemError` (lib.rs:107), and
  response-variant mismatches collapse into indistinguishable anyhow
  strings, so callers can't branch on error kind (e.g. "game rejected the
  command" vs "service unreachable") without string matching.
- recommendation: Define a `thiserror` enum (Transport, HttpStatus,
  SystemError, UnexpectedResponse, Parse) following the sibling-lib
  convention. Errors are at least propagated, never swallowed.

### Retry predicate misses other transient transport failures
- severity: minor
- category: correctness
- location: lib/game_client/src/lib.rs:80
- finding: `e.is_connect() || e.is_timeout()` covers connect-refused
  (KEDA scale-from-zero — the main case) and timeouts, but a connection
  accepted and then reset mid-request (pod killed between accept and
  response — plausible exactly during scale-up/down) surfaces as a
  request/body error and is not retried.
- recommendation: Consider also retrying `e.is_request()` or hyper
  connection-closed errors; at minimum document the deliberate narrowness.

### serde_yaml 0.9 is deprecated
- severity: minor
- category: dependencies
- location: lib/game_client/Cargo.toml:15
- finding: `serde_yaml` was officially deprecated/archived by its author
  in 2024. Only `bot` shares it, so migration cost is small.
- recommendation: Plan a move to a maintained fork (`serde_yml` /
  `serde_norway`) or reconsider whether YAML is needed at all. Not
  urgent; flag for the dependencies unit.

### version_name interpolated into the Host header with no validation
- severity: nit
- category: correctness
- location: lib/game_client/src/lib.rs:54
- finding: Current callers pass DB `game_versions.name` (web/bot) or k8s
  object names (operator) — none attacker-controlled today, and reqwest
  rejects invalid `HeaderValue`s rather than injecting, so there is no
  live vulnerability. But a malformed name fails deep inside reqwest with
  an opaque builder error.
- recommendation: Validate `version_name` (DNS-label charset) once in
  `request()` and return a clear error.

### fetch_game_data does 5 sequential round trips
- severity: nit
- category: simplicity
- location: lib/game_client/src/lib.rs:220
- finding: Status → DataDocs → BasicStrategy → AdvancedStrategy → Rules
  are all independent after Status; the bot calls this per turn through
  an interceptor with potential cold-start latency, so serialization
  multiplies p50 latency.
- recommendation: `tokio::join!` the four post-Status requests, or accept
  and document.

### Timing-sensitive retry test
- severity: nit
- category: quality
- location: lib/game_client/src/lib.rs:322
- finding: `test_retry_on_connect_refused_then_success` races a 15ms
  server spawn against a 20-40ms first backoff; on a loaded CI box the
  server may not bind in time (max_attempts=3 makes outright failure
  unlikely).
- recommendation: Acceptable as-is; bind the replacement listener first
  on a second port if flakiness appears.

Overall: good crate — bounded retry with capped equal-jitter backoff,
4xx/5xx correctly never retried (proven by test), no panics in runtime
paths, errors propagated with context, genuinely good test coverage, lean
deps (reqwest 0.13 rustls/no-default-features). The only substantive gap
is the missing crate-level timeout guarantee.

## lib/cost

### Cost::new() has a spurious K: Clone bound
- severity: nit
- category: quality
- location: lib/cost/src/lib.rs:15
- finding: `new()` just delegates to `Default` (which needs only
  `Hash + Eq`) but sits in the `Clone` impl block (lib.rs:15-19),
  needlessly restricting callers.
- recommendation: Move `new()` into the `Hash + Eq` impl, or drop it for
  `Default::default()`.

### Single-consumer crate while splendor-2 re-implements the same abstraction
- severity: minor
- category: consistency
- location: game/splendor-2/src/cost.rs:7
- finding: lib/cost's only consumer is seven-wonders-1
  (`Cost<Good>`/`Cost<MultiResource>` in game/seven-wonders-1/src/card.rs:83,99),
  while splendor-2 keeps a local `cost.rs` that is a strict subset of the
  same abstraction — its own header says "Ported from
  `brdgme-go/libcost/cost.go`", the same Go origin as lib/cost. Semantics
  match exactly: `from_resources` ≡ `from_keys`, plus identical
  `add`/`inv`/`sub`/`sum`, and splendor's `diff.values().all(>= 0)` is
  precisely lib/cost's `sub → pos_neg → neg.is_empty()`. The only
  genuinely Splendor-specific parts are `get`/`set` accessors and the
  gold-shortfall `can_afford(a, c)` free function (cost.rs:79-87).
- recommendation: Consolidate on `brdgme_cost`: splendor-2 depends on it,
  deletes its local cost.rs (keeping the gold-aware `can_afford` free
  function locally), and lib/cost optionally gains a `get(&self, k: &K)
  -> i32` convenience. Removes ~155 duplicated lines and unifies two
  divergent ports of the same Go code. (Inlining lib/cost into
  seven-wonders-1 is defensible but strictly worse given an identical
  second implementation already exists.) Also tracked by the dependencies
  unit.

Overall: otherwise clean — no panics in runtime paths, `#[must_use]` on
all pure ops, serde as the only dep, and a thorough test suite (15+
tests including permutation/irrelevant-source cases for
`can_afford_perm`, whose skip-a-source logic I traced and found sound).

## lib/rand_bot

### chrono is a completely unused dependency
- severity: minor
- category: dependencies
- location: lib/rand_bot/Cargo.toml:11
- finding: `chrono = { version = "0.4.45", features = ["serde"] }` is
  declared but never referenced anywhere in the crate. Dead weight in
  every fuzzer build, and doubly off-idiom since the project standardized
  on `time` (lib/cmd uses time 0.3).
- recommendation: Delete the dependency.

### Token join separator inconsistent with tools/fuzz
- severity: minor
- category: consistency
- location: lib/rand_bot/src/lib.rs:93
- finding: `spec_to_command` emits explicit `" "` tokens for
  `Spec::Space` (lib.rs:85), yet the two callers join differently —
  `commands()` here joins tokens with `" "` (producing double spaces
  around every Space token, e.g. `"roll  ,  2"`), while tools/fuzz
  (tools/fuzz/src/lib.rs:349) joins with `""`. Both presumably parse
  because game parsers are whitespace-tolerant, but the bot's output
  shape differs depending on which driver uses it.
- recommendation: Pick one join (joining with `""` respects the
  Space-token design) and share it — e.g. have tools/fuzz call
  `RandBot`'s `commands` instead of re-implementing the join.

### Pulls in the HTTP server stack it never uses
- severity: minor
- category: dependencies
- location: lib/rand_bot/Cargo.toml:9
- finding: `brdgme_cmd = { path = "../cmd" }` takes default features,
  which include `http-server` → rand_bot links warp, tokio, and sentry
  for a stdin/stdout bot. (tools/repl and tools/fuzz have the same
  pattern — noted for their own unit.)
- recommendation: `default-features = false`.

### Panics on degenerate command specs
- severity: minor
- category: correctness
- location: lib/rand_bot/src/lib.rs:50
- finding: An empty `OneOf` (lib.rs:50, `choose().unwrap()`), empty
  `players` (lib.rs:84), or malformed request JSON (lib.rs:107) panics
  the bot process instead of yielding an invalid command, which the fuzz
  harness would otherwise tolerate. `Enum` already handles empties
  gracefully (lib.rs:45-48); `OneOf` and `Player` don't. Generation is
  single-pass, so there is no infinite-loop risk — good.
- recommendation: Mirror the `Enum` pattern (`.map(...).unwrap_or_default()`)
  for `OneOf`; for `Player`, fall back to a placeholder string. Low
  priority — but the fuzzer's job is finding exactly these degenerate
  specs.

### Pre-2018 extern crate leftover
- severity: nit
- category: consistency
- location: lib/rand_bot/src/main.rs:1
- finding: `extern crate brdgme_rand_bot;` is meaningless under edition
  2024.
- recommendation: Delete the line.

### Mangled comment referencing dead API
- severity: nit
- category: quality
- location: lib/rand_bot/src/lib.rs:98
- finding: The comment at lib.rs:98-101 (`// / Most bots...`, bad
  line-wrap) references `brdgme_cmd::bot_cli` usage that no bot actually
  uses (see lib/cmd dead-code finding).
- recommendation: Reword or delete alongside the bot_cli cleanup.

Overall: tiny, focused crate that does its job — random generation is
single-pass with no invalid-command-spam risk, and `bounded_i32` is
careful about `i32::MIN/MAX`. Issues are dependency hygiene (unused
chrono, unneeded http-server feature) and the join-separator
inconsistency with tools/fuzz.
