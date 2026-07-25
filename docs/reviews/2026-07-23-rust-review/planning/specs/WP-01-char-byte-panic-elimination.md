# WP-01: char/byte panic elimination

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to execute this plan task-by-task with review checkpoints.

**Goal:** Eliminate every panic caused by char counts (or unchecked byte lengths) being used as byte slice indices on user-supplied strings: `Space::parse`, `Token::parse`, `Enum::parse`/`shared_prefix` in the core command parser (lg F1, F2, F3, F4), the fragile-but-safe `Int::parse` (lg F16), `slice()` in the markup transformer (ls F1), and red7-1's `CardParser` (e F29). Also establish a small workspace convention for non-ASCII input testing - the core libs currently have zero non-ASCII coverage.

**Architecture - the defect class, restated:**

Rust `&str` is UTF-8. `s[a..b]` indexes by BYTES and panics ("byte index N is not a char boundary") if `a` or `b` falls inside a multi-byte character. `s.chars().count()` counts CHARACTERS. The seven findings in scope all compute a length in one unit and use it in the other:

- `rust/lib/game/src/command/parser/mod.rs` - the hand-rolled command parser combinator. `Output { consumed: &'a str, remaining: &'a str }` values are real subslices of the input, and every offset passed between combinators is a BYTE length (`consumed.len()` accumulated in typed `Many` at lines 358/370, spec `Chain` at 909, spec `Many` at 940/948, and `chain_2` in `chain.rs:19`). The unit convention of this crate is therefore BYTES end to end. The bugs are local sites that compute a CHAR count and then use it as a byte index (`Space`, `Enum`) or byte-slice without a boundary check (`Token`). The `GameError::Parse { offset }` field is always 0 everywhere (dead ranking machinery, lg F7 - owned by WP-04, do NOT touch it here).
- `rust/lib/markup/src/transform.rs` - the markup layout pipeline. Here the unit convention is CHARS end to end: `TNode::len` (`rust/lib/markup/src/ast.rs:198-207`) counts `text.chars().count()`, `TNode::bg_ranges` (ast.rs:211+) accumulates char counts, and all canvas x-offsets and align widths are char counts. The one deviant site is `slice()`'s `Text` arm (transform.rs:274), which byte-indexes with those char offsets.
- `rust/game/red7-1/src/command.rs` - a crate-local hand-written `Parser` impl with the same bug: char-count guard, byte slice.

**How user input reaches these functions (why the panics are live):**

- Server side: a player submits a command; the web service sends it to the per-game service, whose HTTP handler dispatches `Request::Play { command, names, .. }` (`rust/lib/cmd/src/requester/gamer.rs:31-43`) into `game.command(player, command, names)` (gamer.rs:131), which every game crate implements by calling `command_parser(...).parse(input, names)` - the lib/game combinators (and, for red7-1, `CardParser`). A panic aborts the request and kills the game-service process's request handling. There is no `catch_unwind` anywhere in `lib/` (verified during review).
- Client side (WASM): `rust/web/src/components/game.rs:575-580` runs `spec.suggest(&current_input, &player_names)` in a `Memo` on EVERY keystroke; `suggest_spec`'s `Chain` and `Many` arms (`rust/lib/game/src/command/suggest.rs`, the `spec.parse(rem, names)` calls) invoke the same spec parsers on the typed fragment. A panic in WASM kills the frontend session.
- Markup: game renders route through `brdgme_markup::transform` (e.g. `rust/lib/cmd/src/requester/gamer.rs` render paths and web-side rendering). Any non-ASCII glyph inside a `{{canvas}}` layer (box-drawing characters, accented player names, emoji) reaches `slice()` via the canvas bg-inheritance/overlap pipeline (transform.rs:337, 341, 368, 373).
- Live triggers are ordinary: iOS autocorrect inserts U+00A0 NBSP (2 bytes, and `char::is_whitespace()` is true for it), player names may contain accents/emoji (`Player::parse` at parser/mod.rs:766 builds `Enum::partial` from player names), and any typo with a multi-byte char (`nñ`, `play r€`) triggers the Token/red7 sites. Per triage: "iOS NBSP is a live trigger" - it reaches both the server and the WASM suggest path.
- `docs/CODING.md:46-49` states the project rule: no panicking code in runtime paths; a server panic kills the request, a WASM panic kills the frontend session. These findings are direct violations.

**Tech Stack:** Rust 1.97.0 workspace at `/home/beefsack/Development/brdgme/rust`. Crates touched: `brdgme_game` (`rust/lib/game`), `brdgme_markup` (`rust/lib/markup`), `red7-1` (`rust/game/red7-1`) - names confirmed from each `Cargo.toml`. Tests are inline `#[cfg(test)]` modules per repo convention (notes-conventions.md; `parser/mod.rs` and `transform.rs` already have them; red7-1 `command.rs` gets a new one).

**Global Constraints:**

- All commands run from `/home/beefsack/Development/brdgme/rust`. Per-crate only: `cargo test -p brdgme_game`, `cargo test -p brdgme_markup`, `cargo test -p red7-1`. NEVER workspace-wide builds/tests (AGENTS.md resource constraints).
- Unit discipline: in `lib/game` every fix must produce BYTE lengths (matching `consumed.len()` accumulation); in `lib/markup` the fix must stay in CHAR units (matching `TNode::len`). Do not convert either crate to the other convention.
- All-ASCII behavior must be byte-for-byte identical to today. The existing test suites (parser tests incl. the typed/spec parity tests, `slice_works`, red7 game tests) must keep passing unmodified.
- Each task ends with `cargo clippy -p <crate> --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- The full pre-commit gate `/home/beefsack/Development/brdgme/scripts/rust-test.sh` MUST pass before the final commit of the package (DB-backed web tests failing without the containers is pre-existing, backlog #40 - run the script, which provides them).

**Non-Goals:**

- Other lib-game parser findings: enum match priority order-dependence (lg F5), Many zero-progress guards (lg F6), typed-Many min bypass (lg F8), suggest min/max (lg F9), doc rendering (lg F10-F12), combine dep (lg F15), suggest dedup (lg F18), Token("") (lg F20) - WP-03; OneOf offset propagation (lg F7), expected() divergences (lg F13/F14), case-folding convention (lg F17), depth guard (lg F19) - WP-04. [Corrected 2026-07-25 by the unit-3b Lead: this line previously mis-assigned F13/F14 to WP-03 and F18/F20 to WP-04; `planning/work-packages.md` is authoritative and WP-03's spec follows it. Routing-label correction only - no task in this spec changed.] Do NOT implement offset propagation or change any `offset: 0`.
- Markup robustness beyond ls F1: parser unwraps (ls F2), silent truncation (ls F3), round-trip (ls F4), word_wrap (ls F8), etc. - WP-02. Do not touch `parser.rs`, `wrap.rs`, `lib.rs`.
- Other red7 findings: zero-rule-fulfilling leader (e F30) and doc fixes - WP-29/WP-30. Do not touch `card.rs` logic.
- Unicode display-width correctness (CJK double-width cells in tables) - not a finding; char counting stays.

**Snapshot drift:** None. All six relevant live files (`rust/lib/game/src/command/parser/mod.rs`, `rust/lib/game/src/command/suggest.rs`, `rust/lib/markup/src/transform.rs`, `rust/lib/markup/src/ast.rs`, `rust/game/red7-1/src/command.rs`, `rust/game/red7-1/src/card.rs`) are byte-identical to the review snapshot (`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit f8763a5); verified by diff on 2026-07-25. All finding line numbers cited below are valid against the live files.

**Chosen fix pattern (consistent across all sites):** compute byte lengths directly at the point of consumption - via `str::trim_start` (Space), `str::get` (Token), per-char `len_utf8()` accumulation (`Enum`/`shared_prefix`, `Int`, red7 `CardParser`) - so every value used to slice `input` is a byte offset on a char boundary of THAT string. The one char-unit crate (`lib/markup`) instead slices by chars (`chars().skip().take()`), because there the surrounding offsets are all char counts. Helper-free: each site is a 1-5 line local change; a shared boundary-safe-slice helper would need to live in a common crate and is not warranted (per-site fixes are smaller than the plumbing).

---

### Task 1: Space::parse - byte-safe whitespace consumption (lg F1)

**Problem (restated):** `rust/lib/game/src/command/parser/mod.rs:431`:

```rust
let consumed = input.chars().take_while(|c| c.is_whitespace()).count();
```

counts CHARS, then lines 440-442 use `consumed` as a BYTE index: `input[..consumed].to_owned()`, `consumed: &input[..consumed]`, `remaining: &input[consumed..]`. `char::is_whitespace()` is true for multi-byte whitespace: U+00A0 NBSP (2 bytes, inserted by iOS autocorrect), U+2000-U+200A and U+3000 (3 bytes). Input `"\u{a0}x"` gives `consumed = 1`, and `&input[..1]` is inside the 2-byte NBSP: panic `byte index 1 is not a char boundary`. Reachable from every command submission (`Space` is the delimiter in every `*_spaced` helper and `AfterSpace`) and on every WASM keystroke via suggest.

**Fix (re-derived):** `str::trim_start` removes leading chars matching exactly `char::is_whitespace` - the identical predicate - so `input.len() - input.trim_start().len()` is the BYTE length of the same whitespace prefix, always on a char boundary. This matches the crate's byte-unit convention (the value feeds `consumed.len()` accumulation in `Many`/`Chain` consumers unchanged, since `consumed` remains a real subslice).

**Edge cases:** empty string -> 0 consumed -> existing `Err` branch (unchanged); all-ASCII spaces/tabs/newlines -> byte len == char count, identical to today; NBSP-only `"\u{a0}"` -> consumes 2 bytes, `remaining = ""`; mixed `" \u{a0}\t x"` -> consumes the full 4-byte run; combining char after space (`" e\u{301}"`) -> consumes 1 byte (U+0301 is not whitespace); 4-byte emoji after space -> consumes 1 byte.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (line 431 in `Space::parse`)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test. Add to `mod tests` in `rust/lib/game/src/command/parser/mod.rs`:

```rust
    #[test]
    fn space_parser_handles_multibyte_whitespace() {
        // U+00A0 NBSP is 2-byte whitespace; iOS autocorrect inserts it in
        // place of a regular space. Must not panic (char count != byte len).
        let parser = Space {};
        assert_eq!(
            Output {
                value: "\u{a0}".to_string(),
                consumed: "\u{a0}",
                remaining: "x",
            },
            parser
                .parse("\u{a0}x", &[])
                .expect("expected NBSP to parse as whitespace")
        );
        // Mixed ASCII + NBSP + ideographic space run.
        assert_eq!(
            Output {
                value: " \u{a0}\u{3000}".to_string(),
                consumed: " \u{a0}\u{3000}",
                remaining: "go",
            },
            parser
                .parse(" \u{a0}\u{3000}go", &[])
                .expect("expected mixed whitespace run to parse")
        );
        // Non-whitespace multi-byte char must still error, not panic.
        parser
            .parse("é", &[])
            .expect_err("expected 'é' to produce an error");
    }
```

- [ ] Run: `cargo test -p brdgme_game space_parser_handles_multibyte_whitespace`. Expected: FAIL - the test panics with `byte index 1 is not a char boundary; it is inside '\u{a0}'` (from `Space::parse`, not an assert failure).
- [ ] Implement. In `Space::parse` replace line 431:

```rust
        let consumed = input.chars().take_while(|c| c.is_whitespace()).count();
```

  with:

```rust
        // Byte length of the leading whitespace run. trim_start strips the
        // same set of chars as char::is_whitespace, so this is always a char
        // boundary; a char count here would byte-slice mid-char on multi-byte
        // whitespace such as U+00A0 NBSP.
        let consumed = input.len() - input.trim_start().len();
```

  Lines 432-443 (the `consumed == 0` guard and the `Output` construction) are unchanged - they now receive a byte length.

- [ ] Run: `cargo test -p brdgme_game space_parser_handles_multibyte_whitespace` - PASS. Then `cargo test -p brdgme_game` - full crate suite PASS (ASCII behavior identical: for ASCII whitespace, byte len == char count).
- [ ] `cargo clippy -p brdgme_game --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs` ; message: `fix(parser): byte-safe whitespace length in Space::parse (lg F1, WP-01)`

---

### Task 2: Token::parse - boundary-checked prefix slice (lg F2)

**Problem (restated):** `rust/lib/game/src/command/parser/mod.rs:49-51`:

```rust
let t_len = self.token.len();
if input.len() < self.token.len()
    || UniCase::new(&input[..t_len]) != UniCase::new(&self.token)
```

The length guard is in BYTES, so it passes whenever the input has at least `t_len` bytes - but byte position `t_len` may fall inside a multi-byte char of the INPUT. Token `"no"` (`t_len = 2`), input `"nñ"` (3 bytes: `n`, then 2-byte `ñ`): guard passes, `&input[..2]` cuts `ñ` mid-char: panic `byte index 2 is not a char boundary`. Any command grammar with a `Token` (all of them) is exposed to any typo whose byte layout straddles the token length; same server/WASM reachability as Task 1.

**Fix (re-derived):** `input.get(..t_len)` returns `Some(prefix)` only when `t_len` is both within bounds AND a char boundary of `input`; `None` otherwise. Treating `None` as a mismatch preserves today's semantics exactly (any input where `t_len` splits a char could never have case-insensitively equaled the token as a str of that byte length) while removing the panic. Note the finding's alternative (`chars().zip()` comparison) is NOT chosen: `UniCase` performs full Unicode case folding (`ß` vs `ss`), which is not expressible as a per-char zip; keeping the existing `UniCase` comparison on the same byte-length prefix preserves current matching behavior bit-for-bit (lg F17, the suggest/parse folding divergence, is WP-04's concern).

**Edge cases:** empty input + non-empty token -> `get` returns `None` -> `Err` (today: length guard -> `Err`, same); all-ASCII -> identical to today; input shorter in bytes than token -> `None` -> `Err` (same as old length guard); multi-byte exactly at `t_len` (`"nñ"` vs `"no"`) -> was panic, now `Err`; input whose prefix case-folds to the token across different byte lengths (`"STRASSE"` vs token `"straße"`, both 7 bytes) -> unchanged from today (same prefix bytes compared); token itself multi-byte (`"sí"`, 3 bytes) with input `"sí!"` -> `get(..3)` is a boundary -> `Ok`, unchanged.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (`Token::parse`, lines 48-64)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing test:

```rust
    #[test]
    fn token_parser_handles_multibyte_input() {
        // "nñ" is 3 bytes; byte index 2 (the token's length) is inside 'ñ'.
        // Must be a mismatch, not a panic.
        let parser = Token::new("no");
        parser
            .parse("nñ", &[])
            .expect_err("expected 'nñ' to produce an error for token 'no'");
        // Multi-byte input longer than the token still mismatches cleanly.
        parser
            .parse("ñofurther", &[])
            .expect_err("expected 'ñofurther' to produce an error for token 'no'");
        // A multi-byte token still matches multi-byte input exactly.
        let parser = Token::new("sí");
        assert_eq!(
            Output {
                value: "sí".to_string(),
                consumed: "sí",
                remaining: "!",
            },
            parser.parse("sí!", &[]).expect("expected 'sí!' to parse")
        );
    }
```

- [ ] Run: `cargo test -p brdgme_game token_parser_handles_multibyte_input`. Expected: FAIL - panics with `byte index 2 is not a char boundary` on the first assertion.
- [ ] Implement. Replace the body of `Token::parse` (lines 48-64) with:

```rust
    fn parse<'a>(&self, input: &'a str, names: &[String]) -> Result<Output<'a, String>, GameError> {
        let t_len = self.token.len();
        // get() returns None when t_len exceeds the input or is not a char
        // boundary of the input; both are mismatches, never panics.
        match input.get(..t_len) {
            Some(prefix) if UniCase::new(prefix) == UniCase::new(&self.token) => Ok(Output {
                value: self.token.to_owned(),
                consumed: prefix,
                remaining: &input[t_len..],
            }),
            _ => Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            }),
        }
    }
```

  (`&input[t_len..]` is safe inside the `Some` arm: `get(..t_len)` succeeding proves `t_len` is a char boundary.)

- [ ] Run: `cargo test -p brdgme_game token_parser_handles_multibyte_input` - PASS. Then `cargo test -p brdgme_game` - full suite PASS (includes `token_parser_works` and the parity tests).
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs` ; message: `fix(parser): boundary-checked prefix slice in Token::parse (lg F2, WP-01)`

---

### Task 3: Enum::parse / shared_prefix - byte-length matching, unit-consistent full-match detection (lg F3 + lg F4)

**Problem (restated):**

- lg F3 (critical): `shared_prefix` (`rust/lib/game/src/command/parser/mod.rs:578-593`) returns a CHAR count. `Enum::parse` stores it in `match_len` and byte-slices the ORIGINAL input at lines 641-642: `consumed: &input[..match_len]`, `remaining: &input[match_len..]`. Value `"café"` (5 bytes, 4 chars), input `"caféx"`: `shared_prefix = 4` chars, `&input[..4]` cuts the 2-byte `é`: panic. Highly reachable: `Player::parse` (line 766) builds `Enum::partial` from player names, so one accented player name makes every command mentioning them a panic - server-side and on every WASM keystroke.
- lg F4 (major): line 622 `if self.exact && matching < v_len` compares `matching` (CHARS) against `v_len = v_str.len()` (BYTES). For any multi-byte value `matching < v_len` always holds, so an exact Enum silently never matches that value. The same mixed-unit comparison poisons the full-match priority logic at lines 626-628 (`matching == v_len` never true for multi-byte values), corrupting ambiguity resolution.
- Additional trap identified during re-derivation: line 605 `input.to_lowercase()` can CHANGE the byte and char length relative to `input` (e.g. `İ` U+0130, 2 bytes, lowercases to `i` + combining dot, 3 bytes/2 chars). So the finding's recommendation - "make shared_prefix return a byte length (accumulate len_utf8)" - is INSUFFICIENT as written: a byte length measured on the lowercased strings is not a valid index into the original `input`. The fix below therefore compares the ORIGINAL strings char-by-char with per-char case folding, and measures bytes of each original string separately.

**Fix (re-derived):** Rewrite `shared_prefix` to walk `input` and the ORIGINAL value string in parallel, comparing `char::to_lowercase()` expansions per position, and return a PAIR of byte lengths: `(bytes matched in input, bytes matched in value)`. Each is by construction a char boundary of its own string. Then in `Enum::parse`:

- slice `input` with the input-byte length (fixes lg F3);
- decide "full match" by `value_bytes == value.len()` - value-unit vs value-unit (fixes lg F4 for both the exact gate and the priority flag);
- keep every other decision (`matching > 0`, `matching >= match_len`, replace-vs-push) EXACTLY as today, now in consistent input-byte units. Do NOT fix the declaration-order dependence (lg F5, WP-03).

Semantic note (document in a comment): per-char `to_lowercase()` equality is identical to whole-string lowercase prefix comparison for all ASCII and for every 1:1 lowercase mapping; it differs only for chars whose lowercase expands to multiple chars (`İ`), where the old code did cross-char partial matches on the expanded string - behavior that was already unindexable (panic) territory. `UniCase`-style full folding is deliberately NOT introduced (lg F17 / WP-04 owns folding convention).

**Edge cases:** empty input -> `(0, 0)` -> no match, `Err` (same as today); all-ASCII values -> byte lens == char counts, identical behavior including the existing dedup (`searched` set, still keyed on `to_lowercase()` of the value - display-only, never sliced) and ambiguity errors; multi-byte value + input stopping exactly at the multi-byte char boundary (`"café"` vs input `"caf"`) -> partial match of 3 bytes, slices cleanly; multi-byte value fully matched with trailing input (`"caféx"`) -> consumed `"café"` (5 bytes), remaining `"x"`; exact Enum with multi-byte value now matches (was: never); case-insensitive multi-byte (`"CAFÉ"` input vs `"café"` value: É 2 bytes lowercases 1:1 to é) -> matches, input bytes 5 counted from the input's own encoding; combining-char input (`"cafe\u{301}"` vs value `"café"`) -> `e` != `é` as chars, partial match `"caf"` (NFC vs NFD is not folded - same as today's behavior class, no regression); 4-byte emoji value (`"🀄"`) in an exact Enum -> matches input `"🀄"`.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (`shared_prefix` lines 578-593; `Enum::parse` lines 600-663)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the failing tests:

```rust
    #[test]
    fn enum_parser_handles_multibyte_values() {
        // lg F3: shared_prefix returned chars, Enum sliced bytes.
        let parser = Enum::partial(vec!["café", "dog"]);
        assert_eq!(
            Output {
                value: "café",
                consumed: "café",
                remaining: "x",
            },
            parser
                .parse("caféx", &[])
                .expect("expected 'caféx' to parse")
        );
        // Partial prefix stopping before the multi-byte char.
        assert_eq!(
            Output {
                value: "café",
                consumed: "caf",
                remaining: "",
            },
            parser.parse("caf", &[]).expect("expected 'caf' to parse")
        );
        // Case-insensitive multi-byte match.
        assert_eq!(
            Output {
                value: "café",
                consumed: "CAFÉ",
                remaining: "",
            },
            parser.parse("CAFÉ", &[]).expect("expected 'CAFÉ' to parse")
        );
    }

    #[test]
    fn enum_parser_multibyte_player_name() {
        // lg F3 reachability: Player builds Enum::partial from user names.
        let names = vec!["José".to_string(), "Bob".to_string()];
        let parser = Player {};
        assert_eq!(
            Output {
                value: 0,
                consumed: "josé",
                remaining: "",
            },
            parser
                .parse("josé", &names)
                .expect("expected player name 'josé' to parse")
        );
    }

    #[test]
    fn exact_enum_matches_multibyte_values() {
        // lg F4: chars-vs-bytes comparison made exact multi-byte values
        // unmatchable, and broke full-match priority.
        let parser = Enum::exact(vec!["café", "dog"]);
        assert_eq!(
            Output {
                value: "café",
                consumed: "café",
                remaining: "",
            },
            parser.parse("café", &[]).expect("expected 'café' to parse")
        );
        parser
            .parse("caf", &[])
            .expect_err("expected partial 'caf' to error under exact");
        // Full-match priority with multi-byte values: the exact-length full
        // match must beat the equal-input-length partial of a longer value.
        let parser = Enum::partial(vec!["café", "cafét"]);
        assert_eq!(
            Output {
                value: "café",
                consumed: "café",
                remaining: "",
            },
            parser
                .parse("café", &[])
                .expect("expected full match 'café' to win over partial 'cafét'")
        );
    }
```

- [ ] Run: `cargo test -p brdgme_game enum_parser_handles_multibyte_values enum_parser_multibyte_player_name exact_enum_matches_multibyte_values` (run each; filters can be given one at a time: `cargo test -p brdgme_game enum_parser_` and `cargo test -p brdgme_game exact_enum_`). Expected: `enum_parser_handles_multibyte_values` and `enum_parser_multibyte_player_name` FAIL with panic `byte index 4 is not a char boundary`; `exact_enum_matches_multibyte_values` FAILS on the first `expect` (returns `Err`, no panic).
- [ ] Implement. Replace `shared_prefix` (lines 578-593) with:

```rust
/// Case-insensitive shared prefix of `input` and `value`, compared per char
/// via char::to_lowercase. Returns byte lengths `(input_bytes, value_bytes)`
/// of the matched prefix in each ORIGINAL string; both are char boundaries
/// of their own string, so they are safe slice indices. Byte lengths are
/// tracked separately because case-insensitively equal prefixes can differ
/// in byte length between the two strings.
fn shared_prefix(input: &str, value: &str) -> (usize, usize) {
    let mut input_bytes = 0usize;
    let mut value_bytes = 0usize;
    let mut vi = value.chars();
    for ic in input.chars() {
        match vi.next() {
            Some(vc) if ic.to_lowercase().eq(vc.to_lowercase()) => {
                input_bytes += ic.len_utf8();
                value_bytes += vc.len_utf8();
            }
            _ => break,
        }
    }
    (input_bytes, value_bytes)
}
```

  Then in `Enum::parse` (lines 600-637), delete line 605 (`let input_lower = input.to_lowercase();`) and replace the loop body so the whole method reads:

```rust
    fn parse<'a>(
        &self,
        input: &'a str,
        names: &[String],
    ) -> Result<Output<'a, Self::T>, GameError> {
        let mut matched: Vec<&T> = vec![];
        // Byte length of `input` consumed by the current best match(es).
        let mut match_len: usize = 0;
        // Exact matches are prioritised, a shorter full match will happen over a longer partial
        // match.
        let mut full_match = false;
        // Track which values have been searched to avoid duplicates.
        let mut searched: HashSet<String> = HashSet::new();
        for v in &self.values {
            let v_str = v.clone().to_string();
            let v_key = v_str.to_lowercase();
            if searched.contains(&v_key) {
                // This is a duplicate, skip it.
                continue;
            }
            searched.insert(v_key);
            let (matching, v_matching) = shared_prefix(input, &v_str);
            // Whether the whole value was matched, measured in the value's
            // own bytes (comparing input bytes to value bytes would misfire
            // whenever case folding changes byte length).
            let full = v_matching == v_str.len();
            if self.exact && !full {
                // The input isn't long enough and we require exact match, skip it.
                continue;
            }
            if matching > 0 && matching >= match_len && (!full_match || full) {
                if full {
                    full_match = true
                }
                if matching > match_len {
                    matched = vec![v];
                    match_len = matching;
                } else {
                    matched.push(v);
                }
            }
        }
        match matched.len() {
            1 => Ok(Output {
                value: matched[0].to_owned(),
                consumed: &input[..match_len],
                remaining: &input[match_len..],
            }),
            0 => Err(GameError::Parse {
                message: None,
                expected: self.expected(names),
                offset: 0,
            }),
            _ => Err(GameError::Parse {
                message: Some(format!(
                    "matched {}, more input is required to uniquely match one",
                    comma_list_and(
                        &matched
                            .iter()
                            .map(|m| m.to_string())
                            .collect::<Vec<String>>()
                    ),
                )),
                expected: self.expected(names),
                offset: 0,
            }),
        }
    }
```

  (The `1`/`0`/`_` match arms are byte-identical to today; only the loop above them changes. The replace-vs-push asymmetry at `matching > match_len` is lg F5 / WP-03 - leave it.)

- [ ] Run: the three new tests - PASS. Then `cargo test -p brdgme_game` - full suite PASS (`test_enum_works` exercises ASCII prefix/ambiguity/case behavior; the parity tests exercise Enum inside real specs).
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs` ; message: `fix(parser): byte-length shared_prefix and unit-consistent Enum matching (lg F3, lg F4, WP-01)`

---

### Task 4: Int::parse - byte length instead of char count (lg F16, hardening)

**Problem (restated):** `rust/lib/game/src/command/parser/mod.rs:124-137` computes `consumed_count` with `.chars().enumerate()...count()` (a CHAR count) and slices `&input[..consumed_count]` at lines 145 and 172. Safe TODAY only because the accepted chars (`-`, ASCII digits) are all 1 byte - the identical pattern that panics in Tasks 1-3. One future edit (accepting `+`, or `char::is_numeric()`) turns it into a panic. Severity nit; fixed here because it is the same defect class in the same file.

**Fix (re-derived):** accumulate the byte length via `char_indices`: the last accepted char at byte index `i` with width `c.len_utf8()` gives prefix length `i + c.len_utf8()`. This is NOT red-green (no observable behavior change is possible today); write the guard test first and see it PASS before and after.

**Edge cases:** empty input -> no digits -> `Err` (unchanged); `"-"` alone -> `found_digit` false -> `Err` (unchanged); `"12é"` -> stops at `é`, consumed `"12"`, remaining `"é"` (works today too - the test pins it); `"١٢"` (Arabic-Indic digits, non-ASCII) -> `is_ascii_digit()` false -> `Err` before and after; leading `-` only at position 0 - `char_indices` yields byte index 0 for the first char, so the `i == 0` guard is equivalent to the old `enumerate` ordinal-0 guard.

**Files:**
- Modify: `rust/lib/game/src/command/parser/mod.rs` (`Int::parse`, lines 123-145)
- Test: same file, inline `mod tests`

**Steps:**

- [ ] Write the guard test (expected to PASS immediately - regression pin, not red-green):

```rust
    #[test]
    fn int_parser_stops_cleanly_at_multibyte_chars() {
        let parser = Int {
            min: None,
            max: None,
        };
        assert_eq!(
            Output {
                value: 12,
                consumed: "12",
                remaining: "é",
            },
            parser.parse("12é", &[]).expect("expected '12é' to parse")
        );
        parser
            .parse("é12", &[])
            .expect_err("expected 'é12' to produce an error");
        // Non-ASCII digits are rejected, not consumed.
        parser
            .parse("١٢", &[])
            .expect_err("expected Arabic-Indic digits to produce an error");
    }
```

- [ ] Run: `cargo test -p brdgme_game int_parser_stops_cleanly_at_multibyte_chars` - expected PASS (guard established against the current code).
- [ ] Implement. In `Int::parse`, replace lines 123-137:

```rust
        let mut found_digit = false;
        let consumed_count = input
            .chars()
            .enumerate()
            .take_while(|&(i, c)| {
                if i == 0 && c == '-' {
                    true
                } else if c.is_ascii_digit() {
                    found_digit = true;
                    true
                } else {
                    false
                }
            })
            .count();
```

  with:

```rust
        let mut found_digit = false;
        // Byte length of the accepted prefix. The accepted chars are all
        // 1-byte ASCII today, but a byte length keeps this slice-safe if the
        // accepted set ever grows (see the Space/Enum multi-byte panics this
        // file previously had).
        let consumed_len = input
            .char_indices()
            .take_while(|&(i, c)| {
                if i == 0 && c == '-' {
                    true
                } else if c.is_ascii_digit() {
                    found_digit = true;
                    true
                } else {
                    false
                }
            })
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
```

  and rename the two uses: line 145 `let consumed = &input[..consumed_count];` -> `let consumed = &input[..consumed_len];`, line 172 `remaining: &input[consumed_count..],` -> `remaining: &input[consumed_len..],`.

- [ ] Run: `cargo test -p brdgme_game` - full suite PASS (`int_parser_works`, `map_parser_works`, `opt_parser_works`, parity tests all exercise Int).
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/lib/game/src/command/parser/mod.rs` ; message: `refactor(parser): byte-length prefix accumulation in Int::parse (lg F16, WP-01)`

---

### Task 5: markup slice() - char-unit slicing and exact-boundary skip (ls F1)

**Problem (restated):** In `rust/lib/markup/src/transform.rs`, every offset flowing into `slice()` is a CHAR count: `TNode::len` (ast.rs:198-207) counts `text.chars().count()`, and the canvas pipeline's `bg_ranges`, x-offsets, and overlap arithmetic (transform.rs:289-408) are all built from it. But `slice`'s `Text` arm (line 274) byte-indexes:

```rust
TNode::Text(ref text) => {
    TNode::Text(text[start..cmp::min(text.len(), end)].to_string())
}
```

`text.len()` is bytes and `start`/`end` are char offsets. For any multi-byte char (box-drawing glyphs in canvas boards, accented player names via `Node::Player` -> `<name>`, emoji) this either panics (`byte index N is not a char boundary`) or - when the clamp kicks in - silently slices the wrong byte range and corrupts output. The whole `canvas()` bg-inheritance/overlap path routes through `slice` (calls at lines 337, 341, 368, 373), so any non-ASCII glyph in a `{{canvas}}` layer is a latent crash in game rendering (server-side render responses and WASM).

Secondary defect (same finding, verification-confirmed): the node-skip check at line 264 uses `n_len < start` instead of `<=`, so a node ending EXACTLY at `range.start` is processed instead of skipped - emitting a spurious empty node (and recursing into container children needlessly).

**Fix (re-derived):** This crate's unit convention is CHARS, so - unlike Tasks 1-4 - the fix is to slice by chars, keeping every producer/consumer untouched: `text.chars().skip(start).take(end.saturating_sub(start)).collect()`. This reproduces the old ASCII behavior exactly, including the old `min(text.len(), end)` clamp (`take` saturates at the iterator's end). `saturating_sub` guards the theoretical `end < start` intermediate (defense in depth; the loop arithmetic keeps `end >= start` for reachable inputs). Also change `<` to `<=` at line 264; the skip arm's `start -= n_len; end -= n_len` arithmetic already produces the identical state the process-the-empty-slice path produced, minus the empty node.

**Edge cases:** empty range (`start >= end`) -> early return at line 256 (unchanged); all-ASCII -> identical output to today except no spurious empty `Text("")` nodes at exact boundaries (verify no test depends on them - `slice_works` does not); multi-byte at slice boundary (`"é!"` sliced `1..2`) -> was panic, now `"!"`; NBSP inside canvas text -> slices whole chars; combining chars (`"e\u{301}"`) -> counted as 2 chars by `TNode::len` and sliced as 2 chars by the fix - consistent (rendering width of combining sequences is a pre-existing, out-of-scope limitation); 4-byte emoji -> 1 char in both `len` and `slice`, consistent; `end` beyond the text -> `take` saturates, same as the old clamp.

**Files:**
- Modify: `rust/lib/markup/src/transform.rs` (lines 264 and 273-275 in `slice`)
- Test: same file, inline `mod tests` (extends the existing `slice_works` neighborhood; `slice` is private, tests live in-module)

**Steps:**

- [ ] Write the failing tests:

```rust
    #[test]
    fn slice_multibyte_works() {
        // Char offsets into multi-byte text: 'é' is 2 bytes but 1 char.
        // Byte-indexing panics at byte 1 (inside 'é').
        assert_eq!(
            slice(&[TN::text("é!")], &(1..2)),
            vec![TN::text("!")],
        );
        // Box-drawing glyphs as used by canvas boards (3 bytes each).
        assert_eq!(
            slice(&[TN::text("│ab│")], &(1..3)),
            vec![TN::text("ab")],
        );
        // Multi-byte inside a nested colored node, mirroring slice_works.
        assert_eq!(
            slice(
                &[TN::Fg(LIGHT.red, vec![TN::Bold(vec![TN::text("héllo")])])],
                &(1..3),
            ),
            vec![TN::Fg(LIGHT.red, vec![TN::Bold(vec![TN::text("él")])])]
        );
    }

    #[test]
    fn slice_skips_node_ending_at_range_start() {
        // A node ending exactly at range.start must be skipped, not emitted
        // as an empty text node.
        assert_eq!(
            slice(&[TN::text("ab"), TN::text("cd")], &(2..4)),
            vec![TN::text("cd")],
        );
    }
```

  (`TN` and `LIGHT` are already imported by the test module; `slice` is in scope via `use super::*;`.)

- [ ] Run: `cargo test -p brdgme_markup slice_multibyte_works slice_skips_node_ending_at_range_start` (as two filtered runs). Expected: `slice_multibyte_works` FAILS with panic `byte index 1 is not a char boundary; it is inside 'é'`; `slice_skips_node_ending_at_range_start` FAILS on assertion (left has a leading `Text("")`).
- [ ] Implement. In `slice` (transform.rs:255-287):

  1. Line 264, change:

```rust
        if n_len < start {
```

     to:

```rust
        if n_len <= start {
```

  2. Lines 273-275, change:

```rust
            TNode::Text(ref text) => {
                TNode::Text(text[start..cmp::min(text.len(), end)].to_string())
            }
```

     to:

```rust
            TNode::Text(ref text) => {
                // start/end are char offsets (TNode::len counts chars), so
                // slice by chars; byte indexing panics on multi-byte glyphs.
                TNode::Text(
                    text.chars()
                        .skip(start)
                        .take(end.saturating_sub(start))
                        .collect(),
                )
            }
```

- [ ] Run: the two new tests - PASS. Then `cargo test -p brdgme_markup` - full suite PASS (`slice_works` covers the ASCII nested/multi-node cases; the canvas/table/align tests cover the callers).
- [ ] Clippy + fmt clean.
- [ ] Commit: `git add rust/lib/markup/src/transform.rs` ; message: `fix(markup): char-unit slicing and exact-boundary skip in slice() (ls F1, WP-01)`

---

### Task 6: red7-1 CardParser - char-boundary card split (e F29)

**Problem (restated):** `rust/game/red7-1/src/command.rs:23-35` (`CardParser::parse`): the guard counts CHARS (`chars.len() < 2`, lines 23-24) but the slices are BYTES: `Card::parse(&input[..2])` (line 31), `consumed: &input[..2]`, `remaining: &input[2..]` (lines 34-35). Input `"r€"` (`r` 1 byte + `€` 3 bytes) passes the 2-char guard, then `&input[..2]` cuts `€`: panic `byte index 2 is not a char boundary`. Reachability (verification-traced end to end): `Request::Play` -> `handle_play` -> `Game::command_parser` -> `OneOf` of `Chain2(Token("play"|"discard"), AfterSpace(CardParser))` - so `play r€` or `discard €5` from the current player panics the red7 game service request; no `catch_unwind` in `lib/`. This is a crate-LOCAL panic distinct from the lib/game ones (Tasks 1-3 do not fix it).

**Fix (re-derived):** the parser wants exactly the first TWO CHARS; compute their combined byte length (`chars[0].len_utf8() + chars[1].len_utf8()`) and slice with that. `Card::parse` (`rust/game/red7-1/src/card.rs:113-124`) is already char-safe (collects chars, requires exactly 2) and returns `None` for any non-card pair, so non-ASCII pairs become an ordinary parse error. This crate consumes lib/game's `Output` convention, so the split must be a byte length (it flows into `chain_2`'s `consumed.len()` arithmetic).

**Edge cases:** empty input / 1 char -> existing `chars.len() < 2` guard -> `Err` (unchanged); ASCII `"r6"` -> split = 2, identical to today; ASCII with trailing input `"r6x"` -> consumed `"r6"`, remaining `"x"` (unchanged); multi-byte first char `"€5"` -> split = 4, `Card::parse("€5")` -> `None` -> `Err` (was: guard passes, `&input[..2]` panics); multi-byte second char `"r€"` -> split = 4 -> `None` -> `Err` (was panic); NBSP as second char `"r\u{a0}"` -> split = 3 -> `None` -> `Err`; 4-byte emoji `"🀄6"` -> split = 5 -> `None` -> `Err`.

**Files:**
- Modify: `rust/game/red7-1/src/command.rs` (`CardParser::parse`, lines 23-42)
- Test: same file, NEW inline `#[cfg(test)] mod tests` at the end (the file has none today; inline modules are the crate convention per lib.rs)

**Steps:**

- [ ] Write the failing test. Append to `rust/game/red7-1/src/command.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Suit;

    #[test]
    fn card_parser_handles_multibyte_input() {
        let parser = CardParser;
        // '€' is 3 bytes: the old byte-index-2 slice panicked mid-char.
        parser
            .parse("r€", &[])
            .expect_err("expected 'r€' to produce an error");
        parser
            .parse("€5", &[])
            .expect_err("expected '€5' to produce an error");
        parser
            .parse("r\u{a0}", &[])
            .expect_err("expected 'r' + NBSP to produce an error");
        // ASCII behavior unchanged.
        let out = parser.parse("r6x", &[]).expect("expected 'r6x' to parse");
        assert_eq!(
            out.value,
            Card {
                suit: Suit::Red,
                rank: 6
            }
        );
        assert_eq!(out.consumed, "r6");
        assert_eq!(out.remaining, "x");
        parser
            .parse("r", &[])
            .expect_err("expected single char to produce an error");
    }
}
```

  NOTE for the implementer: confirm the `Card` struct's field names/visibility and the `Suit` variant for `r` by reading `rust/game/red7-1/src/card.rs` (`Card { suit, rank }` construction at card.rs:110, `Suit::from_abbr`). If `Card`'s fields are not directly constructible from this module, assert via `Card::parse("r6").unwrap()` instead:

```rust
        assert_eq!(out.value, Card::parse("r6").unwrap());
```

- [ ] Run: `cargo test -p red7-1 card_parser_handles_multibyte_input`. Expected: FAIL - panics with `byte index 2 is not a char boundary` on the `"r€"` case.
- [ ] Implement. In `CardParser::parse`, replace lines 23-42:

```rust
        let chars: Vec<char> = input.chars().collect();
        if chars.len() < 2 {
            return Err(brdgme_game::errors::GameError::Parse {
                message: Some("the card must be a letter followed by a number, eg. r6".to_string()),
                expected: self.expected(_names),
                offset: 0,
            });
        }
        match Card::parse(&input[..2]) {
            Some(card) => Ok(Output {
                value: card,
                consumed: &input[..2],
                remaining: &input[2..],
            }),
            None => Err(brdgme_game::errors::GameError::Parse {
                message: Some("the card must be a letter followed by a number, eg. r6".to_string()),
                expected: self.expected(_names),
                offset: 0,
            }),
        }
```

  with:

```rust
        let chars: Vec<char> = input.chars().collect();
        if chars.len() < 2 {
            return Err(brdgme_game::errors::GameError::Parse {
                message: Some("the card must be a letter followed by a number, eg. r6".to_string()),
                expected: self.expected(_names),
                offset: 0,
            });
        }
        // Byte length of the first two chars: `2` as a byte index cuts
        // multi-byte input (e.g. "r€") mid-char and panics.
        let split = chars[0].len_utf8() + chars[1].len_utf8();
        match Card::parse(&input[..split]) {
            Some(card) => Ok(Output {
                value: card,
                consumed: &input[..split],
                remaining: &input[split..],
            }),
            None => Err(brdgme_game::errors::GameError::Parse {
                message: Some("the card must be a letter followed by a number, eg. r6".to_string()),
                expected: self.expected(_names),
                offset: 0,
            }),
        }
```

- [ ] Run: `cargo test -p red7-1 card_parser_handles_multibyte_input` - PASS. Then `cargo test -p red7-1` - full crate suite PASS.
- [ ] `cargo clippy -p red7-1 --all-targets -- -D warnings` and `cargo fmt --all -- --check` clean.
- [ ] Commit: `git add rust/game/red7-1/src/command.rs` ; message: `fix(red7-1): char-boundary card split in CardParser (e F29, WP-01)`

---

### Task 7: Workspace non-ASCII input test convention (WP-01 deliverable)

**Problem:** The core libs had ZERO non-ASCII test coverage before this package - which is exactly where all five criticals lived. The work-package requires a small, documented workspace convention so future string-handling code gets hostile-input tests by default. The repo's convention vehicle is `docs/CODING.md` (it already carries the no-panic rule at lines 46-49 and a "Testing Conventions" section at line 558); a shared fixture crate would add cross-crate dev-dependency churn for a handful of string literals, so a documented canonical list is the right minimal shape.

**Files:**
- Modify: `/home/beefsack/Development/brdgme/docs/CODING.md` (append a subsection inside "## Testing Conventions", after the existing subsections)

**Steps:**

- [ ] Add the following subsection to `docs/CODING.md` under "## Testing Conventions" (place it after the existing bolded convention paragraphs; match the surrounding style):

```markdown
**Non-ASCII input coverage for string slicing.** Rust `&str` indexing is by
bytes and panics off char boundaries, while user input is arbitrary UTF-8 -
iOS autocorrect inserts U+00A0 NBSP, and player names and commands can carry
accents or emoji. Any code that slices, indexes, or measures a string derived
from user input (commands, player names, rendered markup text) must include
at least one test where a multi-byte character sits at the computed boundary.
Use these canonical hostile inputs (all appear in the lib/game, lib/markup,
and red7-1 parser tests):

- `"\u{a0}"` - NBSP: 2-byte WHITESPACE (`char::is_whitespace` is true), the
  live iOS trigger; also `"\u{3000}"` (3-byte ideographic space).
- `"é"` / `"ñ"` - 2-byte letters; `"café"` for prefix matching that stops
  mid-value.
- `"€"` / `"│"` - 3-byte symbol / box-drawing glyph (canvas boards).
- `"e\u{301}"` - combining accent: 2 chars, 1 grapheme; pins down which unit
  a function counts in.
- `"🀄"` - 4-byte emoji.

Convention for new parser/render code: every `parse`-like function over user
input gets an `..._handles_multibyte_input`-style test alongside its ASCII
tests, asserting error-not-panic for malformed multi-byte input and correct
`consumed`/`remaining` splits for valid input containing multi-byte chars.
```

- [ ] Verify the file still renders sanely (plain re-read; no build step for docs).
- [ ] Commit: `git add docs/CODING.md` ; message: `docs(coding): non-ASCII input test convention for string slicing (WP-01)`

---

### Final verification

- [ ] Re-run the three crate suites: `cargo test -p brdgme_game && cargo test -p brdgme_markup && cargo test -p red7-1` - all PASS.
- [ ] Run the full pre-commit suite: `/home/beefsack/Development/brdgme/scripts/rust-test.sh` - must pass end to end (fmt, clippy workspace-minus-web + web-ssr, sqlx prepare check, workspace tests, web ssr tests). Required by AGENTS.md before any Rust change lands.

---

## Findings disposition

| Finding | Severity | Disposition |
|---|---|---|
| lg F1 | critical | Task 1 - `Space::parse` char count replaced with `input.len() - input.trim_start().len()` byte length (identical whitespace predicate); NBSP/ideographic-space tests added |
| lg F2 | critical | Task 2 - `Token::parse` uses `input.get(..t_len)`, `None` = mismatch; finding's `chars().zip()` alternative REJECTED (cannot reproduce UniCase full folding; `get` preserves current semantics exactly) |
| lg F3 | critical | Task 3 - finding's recommendation ADJUSTED: "shared_prefix returns byte length via len_utf8" is insufficient because `to_lowercase()` changes lengths (e.g. `İ`); re-derived fix compares ORIGINAL strings per-char case-insensitively and returns separate `(input_bytes, value_bytes)` |
| lg F4 | major | Task 3 (same rewrite) - exact gate and full-match flag now compare value-bytes to value-bytes; exact multi-byte Enum values become matchable; declaration-order priority bug deliberately untouched (lg F5, WP-03) |
| lg F16 | nit | Task 4 - `Int::parse` accumulates byte length via `char_indices`; guard test only (no observable failure exists today - not red-green) |
| ls F1 | critical | Task 5 - `slice()` slices by CHARS (this crate's end-to-end unit), not bytes-fixed like lib/game; both defects fixed: Text-arm byte indexing and the `<` vs `<=` exact-boundary skip (spurious empty-node emission) |
| e F29 | critical | Task 6 - `CardParser` splits at `chars[0].len_utf8() + chars[1].len_utf8()`; `Card::parse` itself already char-safe; new inline test module (file had none) |
