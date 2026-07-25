# W1 triage rows: lib-game + lib-support

## lib-game: 20 findings (3c / 4M / 8m / 5n) - matches expected; all CONFIRMED, none rejected/adjusted
## lib-support: 45 findings (1c / 5M / 23m / 16n) - matches expected; 43 CONFIRMED, 2 ADJUSTED (F8, F15 - severities unchanged), none rejected

lib-game F1 | critical | Space::parse counts chars but byte-slices; multi-byte whitespace (NBSP) panics | rust/lib/game/src/command/parser/mod.rs | M | char-byte-panics
lib-game F2 | critical | Token::parse byte-length check lets &input[..t_len] split a multi-byte char and panic | rust/lib/game/src/command/parser/mod.rs | M | char-byte-panics
lib-game F3 | critical | Enum::parse shared_prefix returns chars, sliced as bytes; non-ASCII player names panic | rust/lib/game/src/command/parser/mod.rs | M | char-byte-panics
lib-game F4 | major | Exact Enum compares char count vs byte len; multi-byte values can never match | rust/lib/game/src/command/parser/mod.rs | M | char-byte-panics
lib-game F5 | major | Enum full-match priority is declaration-order dependent; prefix values unselectable | rust/lib/game/src/command/parser/mod.rs | M | enum-match-priority
lib-game F6 | major | Many loops (typed, spec, suggest) lack zero-progress guard; zero-width item loops forever | rust/lib/game/src/command/parser/mod.rs,rust/lib/game/src/command/suggest.rs | M | many-progress-guard
lib-game F7 | major | OneOf furthest-error ranking is dead code; all Parse offsets are always 0 | rust/lib/game/src/command/parser/mod.rs | D | oneof-error-offsets: implement offset propagation vs delete ranking
lib-game F8 | minor | Typed Many early-return bypasses min check; diverges from spec impl on degenerate configs | rust/lib/game/src/command/parser/mod.rs | M | typed-spec-parity
lib-game F9 | minor | suggest Many destructures away min/max; keeps suggesting past max cap | rust/lib/game/src/command/suggest.rs | M | suggest-many-max
lib-game F10 | minor | Int suggestion range start + 4 can overflow near i32::MAX | rust/lib/game/src/command/suggest.rs | M | suggest-misc
lib-game F11 | minor | doc_int renders open-ended min as 0, contradicting parser and expected_output | rust/lib/game/src/command/doc.rs | M | doc-render
lib-game F12 | minor | doc_many */+ arms drop a bounded max; Many{0,3} documents as thing* | rust/lib/game/src/command/doc.rs | M | doc-render
lib-game F13 | minor | Doc::expected diverges: typed delegates to inner, spec returns vec![name] | rust/lib/game/src/command/parser/mod.rs | D | typed-spec-parity: deliberate doc-name hint or align impls
lib-game F14 | minor | Many::expected diverges: typed wraps cardinality phrases, spec returns bare inner | rust/lib/game/src/command/parser/mod.rs | D | typed-spec-parity: align spec impl or document difference
lib-game F15 | minor | combine declared in Cargo.toml but unused anywhere in the crate | rust/lib/game/Cargo.toml | M | deps-hygiene
lib-game F16 | nit | Int::parse uses char count as byte index; safe today (ASCII-only) but fragile | rust/lib/game/src/command/parser/mod.rs | M | char-byte-panics
lib-game F17 | nit | Case folding differs: suggest/Enum use to_lowercase, Token::parse uses UniCase | rust/lib/game/src/command/suggest.rs,rust/lib/game/src/command/parser/mod.rs | D | suggest-parse-consistency: adopt UniCase in suggest or document split
lib-game F18 | nit | Suggestions not deduplicated; Enum::parse dedupes via HashSet but suggest does not | rust/lib/game/src/command/suggest.rs | M | suggest-misc
lib-game F19 | nit | Unbounded recursion over spec nesting; no depth guard despite Spec: Deserialize | rust/lib/game/src/command/suggest.rs,rust/lib/game/src/command/parser/mod.rs | D | spec-hardening: only needed if specs ever deserialized from untrusted input
lib-game F20 | nit | Token("") yields empty suggestion that shadows later Chain elements | rust/lib/game/src/command/suggest.rs | M | spec-hardening
lib-support F1 | critical | markup slice() byte-indexes char-count offsets; non-ASCII in canvas panics or corrupts | rust/lib/markup/src/transform.rs | M | markup-slice
lib-support F2 | major | parse_u8/parse_usize unwrap on digit overflow; malformed markup panics the process | rust/lib/markup/src/parser.rs | M | markup-parser-panics
lib-support F3 | major | Unmatched markup silently truncates: many() succeeds with tail in rest, callers discard it | rust/lib/markup/src/lib.rs | D | markup-rest-handling: error on non-empty rest + design a literal-{ escape
lib-support F4 | minor | to_string emits Node::Text raw; no round-trip, markup injection through text | rust/lib/markup/src/lib.rs | D | markup-rest-handling: depends on escape convention from F3
lib-support F5 | minor | rgb_reverse_map eprintln!s and silently substitutes Foreground on unknown rgb values | rust/lib/markup/src/parser.rs | D | markup-parser-panics: parse error vs documented back-compat fallback
lib-support F6 | minor | escape/fg-bg-b wrappers/player() duplicated across html, html_class, transform, semantic | rust/lib/markup/src/html.rs,rust/lib/markup/src/html_class.rs,rust/lib/markup/src/transform.rs,rust/lib/markup/src/semantic.rs | M | markup-dedup
lib-support F7 | minor | PLAYER_COUNT = 8 hardcoded, duplicating palette knowledge by comment convention | rust/lib/markup/src/html_class.rs | M | markup-dedup
lib-support F8 | minor | word_wrap measures bytes not chars; drops leading/wrap-boundary spaces (mid-line runs OK) | rust/lib/markup/src/wrap.rs | D | markup-wrap: char width fix is mechanical; decide if space handling is intended
lib-support F9 | minor | MarkupError::Parse discards all combine position/expected diagnostics | rust/lib/markup/src/error.rs,rust/lib/markup/src/lib.rs | M | markup-diagnostics
lib-support F10 | nit | panic!("invalid transform") and Align::from_str unwrap; restructure choices to be panic-free | rust/lib/markup/src/parser.rs | M | markup-parser-panics
lib-support F11 | nit | Stale TNode::len doc claims a panic that cannot happen | rust/lib/markup/src/ast.rs | M | doc-comments
lib-support F12 | major | regex + lazy_static exist solely for from_hex; parse API has no runtime caller | rust/lib/color/src/lib.rs,rust/lib/color/Cargo.toml | D | color-dead-api: delete from_str/from_hex/named() vs keep with std reimpl
lib-support F13 | minor | Color::mono floors each channel before summing; midpoint biased dark | rust/lib/color/src/lib.rs | M | color-math
lib-support F14 | minor | Three divergent color-name alias tables (named, NamedColor::from_str, markup resolve_named) | rust/lib/color/src/lib.rs,rust/lib/color/src/palette.rs,rust/lib/markup/src/parser.rs | D | color-dead-api: moot if parse API deleted per F12
lib-support F15 | minor | Palette struct literals ~4x more verbose than a const fn rgb() constructor | rust/lib/color/src/palette.rs | M | palette-const-fn
lib-support F16 | nit | themes() doc describes computed lightness categorisation that is actually hardcoded | rust/lib/color/src/palette.rs | M | doc-comments
lib-support F17 | nit | sRGB linearization threshold inconsistent (0.03928 vs 0.04045) across three copies | rust/lib/color/src/palette.rs | M | color-math
lib-support F18 | nit | hex() and Display duplicate the same format string | rust/lib/color/src/lib.rs | M | color-math
lib-support F19 | major | g.request(&req).unwrap() panics warp handler on malformed request in production HTTP path | rust/lib/cmd/src/http.rs | M | cmd-http
lib-support F20 | minor | REPL :undo/:load leave stale renders; display diverges from actual state | rust/lib/cmd/src/repl.rs | M | cmd-repl
lib-support F21 | minor | bot_cli::cli and Response are dead; rand_bot uses only the Request struct | rust/lib/cmd/src/bot_cli.rs | D | cmd-dead-code: delete cli/Response vs move Request into rand_bot
lib-support F22 | minor | REPL prompt() ignores read_line byte count; stdin EOF causes hot spin | rust/lib/cmd/src/repl.rs | M | cmd-repl
lib-support F23 | minor | Panic-heavy runtime paths: gamer.rs render unwraps, cli.rs unwraps, repl typo panic | rust/lib/cmd/src/requester/gamer.rs,rust/lib/cmd/src/cli.rs,rust/lib/cmd/src/repl.rs | M | cmd-runtime-panics
lib-support F24 | minor | term_size 0.3.2 unmaintained (RUSTSEC-2020-0163); drop-in terminal_size replacement | rust/lib/cmd/Cargo.toml | M | deps-hygiene
lib-support F25 | minor | warp vs axum: two HTTP server stacks in the workspace | rust/lib/cmd/Cargo.toml,rust/lib/cmd/src/http.rs | D | http-stack-consolidation: whether to port game-service handler to axum
lib-support F26 | nit | remaining_input.trim() != "" instead of !...is_empty() | rust/lib/cmd/src/repl.rs | M | cmd-repl
lib-support F27 | nit | Redundant #[serde(default)] on Option field | rust/lib/cmd/src/api.rs | M | cmd-hygiene
lib-support F28 | nit | No content-length limit on HTTP body before warp::body::json() | rust/lib/cmd/src/http.rs | M | cmd-http
lib-support F29 | nit | Local requester never checks child exit status; crash surfaces as JSON parse error | rust/lib/cmd/src/requester/local.rs | M | cmd-hygiene
lib-support F30 | nit | undo_stack seeded with initial game; first :undo is a silent no-op reset | rust/lib/cmd/src/repl.rs | M | cmd-repl
lib-support F31 | major | Crate never sets a request timeout; operator uses bare Client::new() and can hang forever | rust/lib/game_client/src/lib.rs | M | client-timeout-retry
lib-support F32 | minor | anyhow in a library crate; error kinds flattened to strings vs sibling thiserror convention | rust/lib/game_client/src/lib.rs | M | client-errors
lib-support F33 | minor | Retry predicate only is_connect/is_timeout; mid-request resets not retried | rust/lib/game_client/src/lib.rs | D | client-timeout-retry: widen retry policy vs document narrowness
lib-support F34 | minor | serde_yaml 0.9 deprecated/archived | rust/lib/game_client/Cargo.toml | D | deps-hygiene: pick fork (serde_yml/serde_norway) or drop YAML
lib-support F35 | nit | version_name interpolated into Host header unvalidated; opaque failure on bad names | rust/lib/game_client/src/lib.rs | M | client-hygiene
lib-support F36 | nit | fetch_game_data does 5 sequential round trips; 4 post-Status calls are independent | rust/lib/game_client/src/lib.rs | M | client-hygiene
lib-support F37 | nit | Timing-sensitive retry test races 15ms server spawn vs 20-40ms backoff | rust/lib/game_client/src/lib.rs | M | client-hygiene
lib-support F38 | nit | Cost::new() sits in Clone impl block; spurious K: Clone bound | rust/lib/cost/src/lib.rs | M | cost-consolidation
lib-support F39 | minor | splendor-2 re-implements lib/cost (same Go origin, identical semantics) | game/splendor-2/src/cost.rs,rust/lib/cost/src/lib.rs | D | cost-consolidation: consolidate on brdgme_cost, API additions needed
lib-support F40 | minor | chrono declared but never referenced; project standardized on time | rust/lib/rand_bot/Cargo.toml | M | deps-hygiene
lib-support F41 | minor | Token join separator inconsistent: rand_bot joins " ", tools/fuzz joins "" | rust/lib/rand_bot/src/lib.rs | M | randbot-fuzz-join
lib-support F42 | minor | brdgme_cmd default features pull warp/tokio/sentry into a stdio bot | rust/lib/rand_bot/Cargo.toml | M | deps-hygiene
lib-support F43 | minor | rand_bot panics on degenerate specs (empty OneOf, empty players, bad JSON) | rust/lib/rand_bot/src/lib.rs | M | randbot-panics
lib-support F44 | nit | extern crate leftover under edition 2024 | rust/lib/rand_bot/src/main.rs | M | cmd-hygiene
lib-support F45 | nit | Mangled comment references dead bot_cli API | rust/lib/rand_bot/src/lib.rs | M | cmd-dead-code

## Grouping notes

- char-byte-panics is the flagship package: lib-game F1-F4/F16 (parser slicing) and lib-support F1 (markup slice) plus F8's width measure share one root pattern - char counts used as byte indices. Same fix idiom (accumulate len_utf8 / char_indices / trim-based byte math) and the same missing-test gap (no non-ASCII inputs anywhere); one work package can land the fix pattern plus a shared non-ASCII test convention across both crates.
- markup-rest-handling (F3, F4) is one design decision - an escape convention for literal { - that gates both the truncation fix and round-trip serialization; fix together, not separately.
- color-dead-api (F12, F14) is one decision: deleting the dead parse API resolves F14 for free and drops regex/lazy_static workspace-wide. Decide F12 first.
- deps-hygiene rows (lib-game F15, lib-support F24, F34, F40, F42) are independent one-line Cargo.toml edits, safe to batch into a single mechanical PR; F34 alone needs a fork-choice decision.
- typed-spec-parity (lib-game F8, F13, F14) is one theme: the typed and spec Parser impls drift; F8 is mechanical, F13/F14 need one call on whether expected() divergence is deliberate, and all three suggest extending parity tests beyond parse().
- cmd-repl (F20, F22, F26, F30) and cmd-runtime-panics/cmd-http (F19, F23, F28) split naturally into dev-tool robustness vs the one production-path fix; F19 is the single prod-urgent item in lib-support.
- Verification flagged no invalid fix recommendations. The two ADJUSTED verdicts only narrowed claims: F8 (word_wrap) - mid-line space runs are preserved, only leading/wrap-boundary spaces collapse; F15 (palette) - line-count savings overstated (~2,300 post-fix, not ~400). Both fixes stand as recommended.
- Cross-unit dependency: lib-support F45 (mangled comment) should ride along with the F21 bot_cli dead-code decision; markup F7 (PLAYER_COUNT) touches lib/color's public API (export a count const), linking markup-dedup to the color package.
