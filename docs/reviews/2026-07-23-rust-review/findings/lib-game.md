# Findings: lib-game (`rust/lib/game`, 3,737 LOC)

Snapshot: `/home/beefsack/Development/brdgme-review-snapshot`, HEAD
`f8763a5ba9c0ce3d0e85d61db7133d19a26ed313`. All line numbers reference the
snapshot. The custom parser combinator and the duplicated `impl Parser for
CommandSpec` are deliberate per the handover and are NOT flagged for
existence — only for concrete correctness/robustness bugs.

The dominant theme: three parsers slice `&str` at byte indices computed from
char counts (or token byte lengths), panicking on non-ASCII input. These run
server-side on every command submission and client-side (WASM) on every
keystroke via the suggest engine, and the project's no-panics-in-runtime-paths
rule names exactly these two contexts.

### Space::parse panics on multi-byte whitespace (char count used as byte index)
- severity: critical
- category: correctness
- location: lib/game/src/command/parser/mod.rs:431
- finding: `let consumed = input.chars().take_while(|c| c.is_whitespace()).count();` counts chars, then lines 440-442 slice `&input[..consumed]` / `&input[consumed..]` as a byte index. `char::is_whitespace()` is true for multi-byte chars (U+00A0 NBSP is 2 bytes, U+2000-U+200A/U+3000 are 3; iOS autocorrect inserts NBSPs), so any such char in command input panics mid-char. Reachable from raw user input both server-side (game command parsing) and in the WASM suggest engine (suggest.rs:77, :112, :121 call `spec.parse` on the typed fragment on every keystroke); a WASM panic kills the frontend session, a server panic kills the request.
- recommendation: Compute a byte length instead of a char count, e.g. `let consumed = input.len() - input.trim_start().len();` (or iterate `char_indices`). Add a test with `"\u{a0}"` input.

### Token::parse panics when the token byte-length cuts a multi-byte char in the input
- severity: critical
- category: correctness
- location: lib/game/src/command/parser/mod.rs:50
- finding: `if input.len() < self.token.len() || UniCase::new(&input[..t_len]) != ...` — the length check is in bytes, so an input whose first char is multi-byte can pass the check while `&input[..t_len]` splits that char. Example: token `"no"`, user types `"nñ"` (3 bytes) → `&input[..2]` panics. Same server/WASM reachability as the Space finding.
- recommendation: Use `input.get(..t_len)` and treat `None` as a mismatch, or compare via `input.chars().zip(self.token.chars())`. Add a non-ASCII test.

### Enum::parse panics on multi-byte values (incl. player names); match_len is chars, slicing is bytes
- severity: critical
- category: correctness
- location: lib/game/src/command/parser/mod.rs:641
- finding: `shared_prefix` (parser/mod.rs:578-593) returns a char count, but lines 641-642 use `match_len` to byte-slice the original input: `consumed: &input[..match_len]`. When the matched prefix contains a multi-byte char (value `"café"`, input `"caféx"`), this panics. Highly reachable: `Player::parse` (parser/mod.rs:766) builds `Enum::partial` from user-chosen player names, so any non-ASCII player name turns command parsing/suggestion into a panic. `to_lowercase()` at line 605 can additionally change string length (e.g. `İ`), further decoupling the char count from the original input.
- recommendation: Make `shared_prefix` return a byte length (accumulate `c.len_utf8()`), or slice with `input.char_indices().nth(n)`. Test with a non-ASCII player name.

### Exact Enum with multi-byte values can never match; full-match detection mixes chars and bytes
- severity: major
- category: correctness
- location: lib/game/src/command/parser/mod.rs:622
- finding: `if self.exact && matching < v_len` compares `matching` (char count from `shared_prefix`) against `v_len` (`v_str.len()`, bytes). For any value containing a multi-byte char, `matching < v_len` always holds, so an `exact` Enum silently never matches that value. The same unit confusion breaks the full-match-priority check at lines 626-628 (`matching == v_len` never true for multi-byte values), corrupting ambiguity resolution.
- recommendation: After fixing `shared_prefix` to return bytes (see the Enum panic finding), compare in one unit throughout; add an exact-Enum test with a multi-byte value.

### Enum full-match priority is declaration-order dependent; prefix values can become unselectable
- severity: major
- category: correctness
- location: lib/game/src/command/parser/mod.rs:626
- finding: With `values = ["abc", "ab"]` and input `"ab"`: `"abc"` matches partially (2) first and sets `match_len = 2`; then `"ab"` fully matches (2) but `matching > match_len` is false, so it is pushed instead of replacing → `matched = ["abc", "ab"]` → spurious "matched ab and abc, more input is required" error. With `["ab", "abc"]` it works. A value that is a prefix of an earlier-declared value cannot be selected at its exact length. Contradicts the comment at lines 608-609 ("a shorter full match will happen over a longer partial match").
- recommendation: When a full match arrives (`matching == v_len`), replace same-length partials rather than appending (e.g. track full matches separately and prefer them), so the result is independent of declaration order. Add tests for both orderings.

### Many loops have no zero-progress guard; a zero-width item parser loops forever
- severity: major
- category: quality
- location: lib/game/src/command/parser/mod.rs:353
- finding: Both the typed `Many::parse` loop (353-381) and the spec `CommandSpec::Many` loop (918-954) assume each iteration consumes input; nothing enforces it. If the item parser succeeds consuming 0 bytes and `delim` is `None` (or itself zero-width), `offset`/`remaining` never advance and the loop pushes values forever with unbounded Vec growth when `max` is `None`. Zero-width success is easy to construct: `Opt` always succeeds, `Token::new("")` always succeeds, and `CommandSpec::Chain(vec![])` succeeds consuming nothing (902-917). The suggest engine's Many loop (suggest.rs:111-144) has the same flaw via its `continue` paths at 135-137. Latent today (no in-tree game builds such a spec — all use `*_spaced` helpers with a `Space` delim), but `Spec`'s fields are public and it is `Deserialize`, so one buggy game spec hangs a game-service thread or freezes the browser tab (the suggest memo runs on the WASM main thread).
- recommendation: In all three loops (typed Many, spec Many, suggest Many), track remaining-input length across iterations and break when an iteration makes no progress. Document the progress invariant on the `Many` combinator and add a termination test with a degenerate spec.

### The OneOf "furthest error wins" machinery is dead code — all offsets are always 0
- severity: major
- category: quality
- location: lib/game/src/command/parser/mod.rs:473
- finding: Both `OneOf` impls (typed 473-520, spec 854-901) rank child errors by `GameError::Parse.offset` to keep only the furthest error, but every `Parse` error constructed in the crate uses `offset: 0` (all 10 construction sites verified) and `Chain`/`Many` propagate child errors with `?` without adjusting the offset (e.g. spec Chain at line 907). So `e_consumed.cmp(&error_consumed)` is always `Equal` and the "keep only the furthest errors" logic silently degrades to "accumulate all errors in declaration order". The code implies a behavior — better error messages from the furthest-progress alternative — that does not exist.
- recommendation: Either implement offset propagation (leaf parsers report their failure offset; `Chain`/`Many` add bytes consumed so far to child offsets) so the ranking works as intended, or delete the ranking and just accumulate, with a comment that order is declaration order. The first option noticeably improves error messages for `OneOf`-heavy command grammars.

### Typed Many early-return bypasses the min check and diverges from the spec impl
- severity: minor
- category: correctness
- location: lib/game/src/command/parser/mod.rs:342
- finding: `if let Some(max) = self.max && (max == 0 || max < self.min.unwrap_or(0)) { return Ok(...) }` makes typed `Many { min: Some(2), max: Some(1) }` (or `max: Some(0)` with `min > 0`) succeed with an empty vec — the min check at line 382 is never reached. The spec impl (918-973) has no such early return: it breaks out of the loop and fails the min check at 955-967. So for degenerate configs the execution parser and the spec/suggest parser disagree on success/failure — exactly the drift the `assert_typed_spec_parity` tests are meant to catch, but they don't cover it.
- recommendation: Drop the early return (the loop plus min check already handle these configs identically to the spec impl), or make both impls match. Add a parity test for `min > max` and `max == 0` configs.

### suggest's Many arm ignores min/max and suggests items the parser will reject
- severity: minor
- category: correctness
- location: lib/game/src/command/suggest.rs:109
- finding: `Spec::Many { spec, delim, .. }` destructures away `min`/`max`. The parse-side Many enforces `max` (parser/mod.rs:929-933), but the suggest loop keeps offering items after `max` have been typed: with `Many { max: Some(2), delim: Space }` and input `"a b "`, it suggests all values again for a third item the real parser will never accept. User-visible today: bounded Many is used in production specs (game/sushi-go-2/src/command.rs:42 `Many::bounded_spaced(.., 1, 2)`, game/sushizock-2/src/command.rs:47).
- recommendation: Count consumed items in the loop and return `vec![]` (or stop advancing) once `max` is reached. Ignoring `min` is fine. Add a test asserting suggestions stop at the cap.

### Int suggestion range computation can overflow
- severity: minor
- category: quality
- location: lib/game/src/command/suggest.rs:87
- finding: `let end = max.map(|m| m.min(start + 4)).unwrap_or(start + 4);` — `start + 4` overflows when a spec supplies `min: Some(i32::MAX - 3..=i32::MAX)`: a debug-build panic, a wrap to a negative `end` (empty suggestion range) in release. Spec-supplied rather than user-input-supplied, hence minor.
- recommendation: `start.saturating_add(4)`.

### doc_int renders an open-ended minimum as 0, contradicting the parser
- severity: minor
- category: correctness
- location: lib/game/src/command/doc.rs:51
- finding: The `(min, Some(max))` arm renders `format!("{}-{}", min.unwrap_or(0), max)`, so `Spec::Int { min: None, max: Some(5) }` documents as `0-5` while the parser (parser/mod.rs:119-174) accepts negative values when `min` is `None`. Inconsistent with `Int::expected_output` (parser/mod.rs:113), which correctly says "number 5 or lower" for the same shape. User-facing: `Spec::doc()` feeds REPL help (lib/cmd/src/repl.rs:95) and email notifications (web/src/email/notify.rs:94), and at least one game uses this shape (game/for-sale-2/src/command.rs:43-45).
- recommendation: Match `expected_output`'s semantics, e.g. render `#-5` / `-5 or lower` when `min` is `None` rather than substituting 0.

### doc_many drops a bounded max in the common arms
- severity: minor
- category: correctness
- location: lib/game/src/command/doc.rs:134
- finding: The `(None, _) | (Some(0), _)` arm renders `doc*` (unbounded) and the `(Some(1), _)` arm renders `doc+` even when `max` is `Some(n)`, so `Many { min: Some(0), max: Some(3) }` documents as `thing*` instead of `thing(0-3)`. Latent: no game crate constructs `Spec::Many` directly today, but the function silently misdescribes any bounded-max spec it receives, and `to_spec` (parser/mod.rs:415-422) propagates `min`/`max` faithfully into the spec this renders.
- recommendation: Only take the `*`/`+` shortcuts when `max` is `None`; otherwise fall through to the range arm.

### Doc::expected diverges between the typed and spec impls
- severity: minor
- category: consistency
- location: lib/game/src/command/parser/mod.rs:718
- finding: Typed `Doc::expected` (718-720) delegates to the inner parser, but the spec impl (line 1031) returns `vec![name.clone()]`. Typed `Doc::name("tokens", Enum[..]).expected()` yields the enum values; the spec yields `["tokens"]`. Possibly deliberate (the doc name is a better hint in aggregated error messages), but it is undocumented and the parity tests only compare `parse`, never `expected`.
- recommendation: If deliberate, add a WHY comment at line 1031; otherwise align the two. Consider extending the parity tests to `expected` output.

### Many::expected diverges between the typed and spec impls
- severity: minor
- category: consistency
- location: lib/game/src/command/parser/mod.rs:402
- finding: Typed `Many::expected` (402-413) wraps each entry with cardinality ("any number of X" / "N or more X" / "up to N X" / "between N and M X"); the spec impl (line 1025) returns the bare inner `spec.expected(names)`. User-facing error messages for the same grammar differ depending on which impl produced them.
- recommendation: Align the spec impl with the typed one (it has `min`/`max` available in the match arm) or document the difference.

### `combine` is declared but unused in this crate
- severity: minor
- category: dependencies
- location: lib/game/Cargo.toml:12
- finding: `combine = "4.6.7"` is declared, but grep over `rust/lib/game/src` finds no `combine` use anywhere (the command parser is hand-rolled — deliberately; `lib/markup` is the actual `combine` consumer). Dead dependency in the crate every game compiles. (`unicase` is used only at parser/mod.rs:51, `log` only in bot.rs, `serde_json` genuinely in the CommandSpec impl — those stay.)
- recommendation: Remove `combine` from `lib/game/Cargo.toml`.

### Int::parse uses a char count as a byte index (safe today, fragile)
- severity: nit
- category: quality
- location: lib/game/src/command/parser/mod.rs:124
- finding: `consumed_count` comes from `.chars().enumerate()...count()` and then slices `&input[..consumed_count]` (line 145). Correct only because the accepted chars (`-`, ASCII digits) are all 1-byte — the same pattern that causes the Space/Enum panics above. One future edit (allowing `+`, or `is_numeric()` instead of `is_ascii_digit()`) turns it into a panic.
- recommendation: Compute the byte length directly (e.g. accumulate `c.len_utf8()` or use `char_indices`), or leave a one-line comment stating the ASCII-only invariant.

### Case-folding semantics differ between suggest and parse
- severity: nit
- category: consistency
- location: lib/game/src/command/suggest.rs:26
- finding: Suggest matches prefixes with `to_lowercase()` (suggest.rs:26, 36-39, 52-55, 98-101), as does `Enum::parse` (parser/mod.rs:605, 614) — but `Token::parse` uses `UniCase` (parser/mod.rs:51), which does full Unicode case folding (`ß`↔`ss`). For ASCII all three agree, so purely theoretical today, but suggest filtering and parse acceptance can diverge for non-ASCII tokens.
- recommendation: Use `UniCase` in suggest (already a dependency) or document "suggest is lowercase-prefix, parse is UniCase" in the module header.

### Suggestions are not deduplicated, unlike the parser
- severity: nit
- category: correctness
- location: lib/game/src/command/suggest.rs:37
- finding: The `Enum` arm (37-44) maps every value verbatim, so a spec with duplicate values yields duplicate suggestions — while `Enum::parse` explicitly dedupes via a `HashSet` (parser/mod.rs:612-619). The `OneOf` arm (46-49) likewise concatenates branch results blindly. Real specs shouldn't contain dups, but the asymmetry with the parser is undocumented.
- recommendation: Dedupe by value in the `Enum` arm (and optionally after `OneOf` concatenation), mirroring the parser.

### Unbounded recursion over spec nesting, no depth guard
- severity: nit
- category: quality
- location: lib/game/src/command/suggest.rs:23
- finding: `suggest_spec` recurses per nesting level (`OneOf`→`Chain`→`Doc`→`Opt`→`Many`…), as does the spec `parse` impl (parser/mod.rs:907, 945). Depth is bounded by the spec, not user input, and specs are built by trusted game crates — but a pathological spec (e.g. 10k nested `Opt`s, constructible via the public enum or serde) stack-overflows both paths. Given `Spec: Deserialize`, cheap insurance if a spec is ever deserialized from an untrusted source.
- recommendation: No action required for current callers; if `Spec` is ever deserialized from untrusted input, add a depth limit during deserialization or traversal.

### Token("") yields an empty suggestion and shadows later chain elements
- severity: nit
- category: quality
- location: lib/game/src/command/suggest.rs:26
- finding: `"".starts_with("")` is true, so `Spec::Token("")` on empty input returns `Suggestion { value: "" }`; inside a `Chain`, that non-empty result short-circuits advancement (suggest.rs:74-75) and prevents suggestions for later elements. Only constructible via a degenerate spec.
- recommendation: Guard `if token.is_empty() { return vec![] }` or document the non-empty-token invariant.

## Areas reviewed and found clean

- **src/rng.rs** — exemplary: deterministic serializable `GameRng(ChaCha8Rng)`, honest docs (including the `usize` portability caveat and the `Default`-seeds-0 derive shim), `TryRng` with `Infallible`, meaningful tests (same-seed stream, serde roundtrip resumption). No findings.
- **src/game.rs** — `Gamer`/`Renderer` traits, `Status`, `Stat`, and `gen_placings` are correct (competition ranking with ties verified: `cur_place += players.len()`; `cmp_fallback` is a valid total order). Only observation: the `None` arm at game.rs:149 is unreachable for `i32` (`partial_cmp` is total) — harmless, not flagged as a finding.
- **src/errors.rs** — clean. `GameError::Parse.offset` is NOT dead: it is read by the OneOf error-merging logic (parser/mod.rs:480, 862) — the problem is that nothing ever writes a non-zero value (covered by the OneOf finding).
- **src/game_log.rs**, **src/command/mod.rs** (Spec enum), **src/lib.rs** — clean.
- **src/bot.rs** — the `Fuzzer` harness contains `.expect()`/`panic!` (bot.rs:113-153), acceptable by design for a fuzz tool (not a server/WASM runtime path); no finding.
- **src/command/parser/chain.rs** — clean: byte-length arithmetic is consistent (all `consumed` values are slices of the same input), `expected()`/`to_spec()` shapes match what the spec impl handles.
- **No `unwrap`/`expect`/`panic!`/`unreachable!` outside `#[cfg(test)]`** in parser/mod.rs or suggest.rs — the only runtime panic vectors are the slicing bugs flagged above.
- **"No consumption on failure" invariant holds** structurally (all failures return `Err`; `Opt`/`Many` restore `remaining: input` at parser/mod.rs:265-269, 377-399) — though it is undocumented; a doc comment on the `Parser` trait would be worthwhile given how much depends on it.
- **Test suites are genuinely strong** (~70 suggest tests incl. faithful Acquire/Splendor spec mirrors; typed/spec parity drift-guard tests) — the material gap is that no test exercises non-ASCII input, which is exactly where the three critical panics live; the per-finding recommendations above call for those tests.
