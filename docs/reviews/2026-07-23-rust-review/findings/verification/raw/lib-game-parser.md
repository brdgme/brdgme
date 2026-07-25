# Verification: lib-game parser findings (F1-F8, F13, F14, F16)

Snapshot: /home/beefsack/Development/brdgme-review-snapshot commit f8763a5.
Verifier read parser/mod.rs and suggest.rs in full; all Parse construction
sites located by grep.

### F1: Space::parse panics on multi-byte whitespace
- original severity: critical
- verdict: CONFIRMED
- evidence: parser/mod.rs:431 `let consumed = input.chars().take_while(|c| c.is_whitespace()).count();`
  then lines 440-442 `input[..consumed].to_owned()` / `consumed: &input[..consumed]` /
  `remaining: &input[consumed..]`. `consumed` is a char count used as a byte
  index. U+00A0 NBSP is 2 bytes and `char::is_whitespace()` returns true for
  it. Trace for input `"\u{a0}x"`: chars iterator yields NBSP (whitespace,
  counted) then 'x' (stop) -> consumed = 1; `&input[..1]` falls inside the
  2-byte NBSP encoding (0xC2 0xA0) -> byte index 1 is not a char boundary ->
  panic. Space is used in every `*_spaced` Many and `AfterSpace`, so it runs
  on raw user command input server-side and in the WASM suggest path
  (suggest.rs Chain/Many arms call `spec.parse`). Critical is appropriate
  under the stated severity guide (panic reachable in runtime paths).

### F2: Token::parse splits a multi-byte char at the token byte length
- original severity: critical
- verdict: CONFIRMED
- evidence: parser/mod.rs:49-51:
  `let t_len = self.token.len(); if input.len() < self.token.len() || UniCase::new(&input[..t_len]) != UniCase::new(&self.token)`.
  Both length checks are in bytes. Token "no" (t_len 2), input "nñ" (bytes:
  'n'=0x6E, 'ñ'=0xC3 0xB1, total 3): `input.len() < 2` is false, so
  `&input[..2]` is evaluated and byte 2 is mid-'ñ' -> panic before the
  UniCase comparison can return false. Same server/WASM reachability as F1
  (Token is the first element of nearly every command grammar). Critical
  stands.

### F3: Enum::parse uses shared_prefix char count as byte index
- original severity: critical
- verdict: CONFIRMED
- evidence: `shared_prefix` (parser/mod.rs:578-593) iterates `.chars()` and
  increments `len += 1` per matching char -> returns a char count. In the
  single-match arm (parser/mod.rs:639-643):
  `consumed: &input[..match_len], remaining: &input[match_len..]` -- byte
  slices of the original input. Trace for value "café", input "caféx":
  lowercased both sides, shared_prefix = 4 chars; input bytes are
  c(0) a(1) f(2) é(3-4) x(5), so `&input[..4]` splits 'é' -> panic.
  Reachability via player names confirmed: Player::parse at parser/mod.rs:766
  is `Map::new(Enum::partial(self.player_nums(names)), |pn| pn.num)` where
  names are user-chosen display names. The additional to_lowercase hazard is
  real: line 605 `let input_lower = input.to_lowercase();` -- lowercasing can
  change char count (e.g. 'İ' -> "i\u{307}"), so match_len can exceed even
  the char count of the original input. Critical stands.

### F4: Exact Enum compares char count against byte length
- original severity: major
- verdict: CONFIRMED
- evidence: parser/mod.rs:620-622:
  `let v_len = v_str.len(); let matching = shared_prefix(&input_lower, &v_str); if self.exact && matching < v_len { continue; }`.
  `v_len` is bytes of the lowercased value, `matching` is chars (see F3).
  For value "café" (5 bytes, 4 chars) the maximum possible `matching` is 4,
  so `matching < v_len` always holds and an exact Enum silently never matches
  that value (no panic -- unreachable slice -- just silent non-match). The
  same unit mismatch makes `matching == v_len` at 626-627 never true for
  multi-byte values, so `full_match` is never set and the full-match-priority
  ambiguity resolution is disabled for them. Major (clear defect, silent
  wrong behavior rather than panic) is right.

### F5: Enum full-match priority is declaration-order dependent
- original severity: major
- verdict: CONFIRMED
- evidence: parser/mod.rs:626-636. Trace with values ["abc","ab"], input "ab"
  (all ASCII, so char/byte units agree):
  - v="abc": matching=2, v_len=3. Line 626: `2 > 0 && 2 >= 0 && (!false || ...)`
    -> enter; `matching == v_len` false; `matching > match_len` (2>0) ->
    `matched = [abc]; match_len = 2`.
  - v="ab": matching=2, v_len=2. Line 626: `2 > 0 && 2 >= 2 && (!false || 2==2)`
    -> enter; `matching == v_len` -> `full_match = true`; but
    `matching > match_len` is 2>2 false -> `matched.push(ab)` ->
    matched = [abc, ab] -> the `_` arm at 649 returns the "matched abc and ab,
    more input is required" ambiguity error.
  Reversed order ["ab","abc"]: "ab" full-matches first (full_match=true,
  matched=[ab]); then "abc" hits `(!full_match || matching==v_len)` =
  `(false || 2==3)` = false -> skipped -> unique match succeeds. So the same
  grammar behaves differently by declaration order, and a value that is a
  prefix of an earlier-declared value cannot be selected at its exact length.
  The comment at 608-609 ("a shorter full match will happen over a longer
  partial match") promises the opposite. CONFIRMED, major.

### F6: No zero-progress guard in the three Many loops
- original severity: major
- verdict: CONFIRMED
- evidence:
  - Typed Many (parser/mod.rs:353-381): loop advances only via
    `offset = inner_offset + consumed.len()`; if the item parser returns
    `Ok` with `consumed.len() == 0` and `delim` is `None` (or zero-width),
    offset never changes and `parsed.push(value)` repeats forever when
    `max` is `None`.
  - Spec Many (parser/mod.rs:928-953): identical structure -- breaks only on
    `values.len() >= max` or `Err`; `remaining = out.remaining` does not
    change on a zero-consumption Ok.
  - Suggest Many (suggest.rs:111-144): with `delim: None` the else branch at
    135-138 sets `rem = after_item; continue;` -- a zero-consumption Ok on
    nonempty `rem` gives `after_item == rem`, infinite loop; with a
    zero-width delim the Ok arm at 122-125 does the same.
  Zero-width successes exist: `Opt` always succeeds with `consumed: &input[..0]`
  (parser/mod.rs:265-269), `Token::new("")` succeeds consuming "" (t_len=0
  passes both checks at 50-51), and `CommandSpec::Chain(vec![])` returns Ok
  with consumed_len 0 (parser/mod.rs:902-916). No in-tree spec combines these
  without a Space delim, so latent -- but Spec's fields are public and Spec
  derives Deserialize, so the hang is one bad spec away in both the
  game-service thread and the WASM main thread. Major (latent DoS,
  quality/robustness) is defensible.

### F7: OneOf furthest-error ranking is dead code
- original severity: major
- verdict: CONFIRMED
- evidence: Grep over the crate finds exactly 12 `GameError::Parse`
  construction sites in parser/mod.rs (none in chain.rs, suggest.rs, or
  elsewhere in src/). Ten use literal `offset: 0` (lines 56, 142, 149, 157,
  166, 392, 436, 647, 660, 965 -- Token, Int x4, typed Many min, Space,
  Enum x2, spec Many min). The remaining two are the OneOf impls themselves
  (lines 518 and 899), which use `offset: error_consumed` -- and
  `error_consumed` starts at 0 and only grows if a child error has a nonzero
  offset, so by induction it is always 0 too. Chain propagation never
  adjusts offsets: chain.rs uses bare `?` (chain.rs:17-18, 105-106, 163-164)
  and spec Chain at parser/mod.rs:907 is `let out = s.parse(remaining, names)?;`.
  Therefore `e_consumed.cmp(&error_consumed)` at 483/865 is always `Equal`
  and the ranking degrades to accumulate-in-declaration-order. The finding's
  "all 10 construction sites use offset: 0" matches the 10 literal-zero
  sites; the two error_consumed sites are provably always-zero, so the claim
  is substantively exact. Major (misleading dead machinery, degraded error
  quality) is reasonable.

### F8: Typed Many early-return bypasses the min check; spec impl diverges
- original severity: minor
- verdict: CONFIRMED
- evidence: Typed impl, parser/mod.rs:342-350:
  `if let Some(max) = self.max && (max == 0 || max < self.min.unwrap_or(0)) { return Ok(Output { value: parsed, ... }) }`
  -- returns Ok with an empty vec, never reaching the min check at 382-384.
  So `Many { min: Some(2), max: Some(1) }` succeeds empty. The spec impl has
  no early return: with `max: Some(1)` the loop (929-932) breaks after one
  item (or zero on item failure), then 955-956
  `if let Some(min_val) = min && values.len() < *min_val` fails with
  "expected at least 2 items". Concrete divergence: min=2, max=1, input with
  one parseable item -> typed Ok(vec![]) consuming nothing, spec Err. For
  max=0 with min>0: typed Ok(empty), spec breaks immediately at 929 then
  fails min. Divergence between the two impls the parity tests are meant to
  guard confirmed; degenerate configs only, so minor is right.

### F13: Doc::expected diverges between typed and spec impls
- original severity: minor
- verdict: CONFIRMED
- evidence: Typed (parser/mod.rs:718-720):
  `fn expected(&self, names: &[String]) -> Vec<String> { self.parser.expected(names) }`
  -- delegates to the inner parser. Spec (parser/mod.rs:1031):
  `CommandSpec::Doc { name, .. } => vec![name.clone()],` -- returns the doc
  name. Same grammar yields different expected-lists depending on impl; the
  parity tests (1319-1350) compare only parse results (`remaining` and
  success), never `expected`. Divergence as described; minor/consistency is
  right.

### F14: Many::expected diverges between typed and spec impls
- original severity: minor
- verdict: CONFIRMED
- evidence: Typed (parser/mod.rs:402-413) wraps each inner entry:
  `(None, None) => format!("any number of {}", e)`, `(Some(min), None) =>
  format!("{} or more {}", min, e)`, `(None, Some(max)) => format!("up to {} {}", max, e)`,
  `(Some(min), Some(max)) => format!("between {} and {} {}", min, max, e)`.
  Spec (parser/mod.rs:1025): `CommandSpec::Many { spec, .. } => spec.expected(names),`
  -- bare inner expected, min/max destructured away. Divergence as described;
  minor is right.

### F16: Int::parse char-count-as-byte-index is safe today but fragile
- original severity: nit
- verdict: CONFIRMED
- evidence: parser/mod.rs:124-137 computes `consumed_count` via
  `.chars().enumerate().take_while(...).count()`, accepting only `-` at
  position 0 and `c.is_ascii_digit()` -- every accepted char is exactly 1
  byte, so char count equals byte length and the slices at 145
  (`&input[..consumed_count]`) and 172 (`&input[consumed_count..]`) are
  currently always on char boundaries (a leading non-ASCII char stops the
  take_while at count 0, so no mid-char slice is possible). The fragility
  claim is accurate: it is the identical pattern that is broken in F1/F3,
  and widening the accepted set (e.g. `is_numeric()`) would introduce the
  same panic. Nit is the right severity.
