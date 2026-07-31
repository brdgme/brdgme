# Unit 01 - Core libraries (lib/game, lib/markup, lib/color, lib/cmd, lib/game_client, lib/rand_bot)

Review of the 2026-07-25..2026-07-30 remediation work for WP-01 .. WP-09b.
Adversarial quality/correctness review of the remediation itself.

## Commits in scope

| hash | WP | subject |
|---|---|---|
| `9abe8b4a` | WP-01 | char/byte index panics |
| `91f26820` | WP-02 | markup robustness |
| `c39786f9` | WP-03 | parser mechanical fixes |
| `82157548` | WP-04 | parser design items |
| `4a978cbe` | WP-05 | lib/color dead-API delete |
| `a543120f` | WP-06 | lib/cmd HTTP + CLI hardening |
| `63063a4b` | WP-07 | game_client / rand_bot |
| `f13450a1` | WP-08 | epilogue dedup |
| `c14bc655` | WP-08b | epilogue dedup follow-up |
| `ff8f83ba` | WP-09a | requester-boundary trust |
| `c078c3ee` | WP-09b | per-crate `validate()` |

## Findings

### F-01 (Low) `markup::from_string` returns a value that is now always empty

`rust/lib/markup/src/lib.rs:43-56`

WP-02 made `from_string` reject any unconsumed input, but kept the
`Result<(Vec<Node>, &str), MarkupError>` signature. The `&str` is
unconditionally `""` on the success path, so every caller destructures and
discards a dead value, and the tuple invites a reader to believe partial
parses are still returned. The unit's own test
(`from_string_well_formed_returns_empty_rest`) asserts exactly that it is
always empty.

Remediation: change the signature to `Result<Vec<Node>, MarkupError>` and
update the (few) callers; drop the tuple from the doc comment.

### F-02 (Low) Overflow tests are written as disjunctions and pin no behaviour

`rust/lib/markup/src/parser.rs:737-757`
(`overflowing_u8_returns_err_not_panic`,
`overflowing_usize_returns_err_not_panic`)

Both tests assert `result.is_err() || <no such node produced>`. That passes
under either outcome, so they only prove "did not panic" and would keep
passing if `parse_u8`'s `and_then` were reverted to a lossy fallback that
silently produced a wrong colour. The actual behaviour is deterministic
(`and_then` yields a committed parse error, so `markup()` leaves the tag
unconsumed and `from_string` errors) and should be asserted directly.

Remediation: assert `markup().parse("{{fg rgb(999,1,1)}}x{{/fg}}").is_err()`
(or the exact leftover `rest`) rather than an `||`.

### F-03 (Low) `Token::parse` byte-length prefix vs `Spec::Token` suggest full case folding

`rust/lib/game/src/command/parser/mod.rs:45-62` and
`rust/lib/game/src/command/suggest.rs:39-52`

`Token::parse` takes `input.get(..self.token.len())` - a *byte* count from the
token - and then compares with `UniCase`. `Spec::Token`'s suggester (added in
WP-04) instead does `UniCase::to_folded_case(token).starts_with(folded
remaining)`. These agree only while case folding preserves byte length. The
unit's own test picks `"Straße"`/`"STRASSE"`, where both sides happen to be 7
bytes; for any token whose folded form changes length (e.g. `"İ"` U+0130, 2
bytes, folding to a 3-byte sequence) the suggester will offer a completion
that `Token::parse` then rejects. The committed test
`enum_does_not_unicode_fold_like_token` documents the *Enum* divergence but
nothing pins the Token parse/suggest pair.

Remediation: make `Token::parse` compare `UniCase` over the folded prefix
rather than a token-byte-length slice (walk `input.char_indices()` until the
folded prefixes diverge), or make the suggester use the same byte-prefix
`UniCase` test as `parse`. Either way add a parity test over a token whose
fold changes length.

### F-04 (Low) `word_wrap` recounts chars per word

`rust/lib/markup/src/wrap.rs:14-24`

The ls F8 fix replaced `current.len()` with `current.chars().count()` inside
the per-word loop, making `wrap_segment` O(n^2) in the segment length. Game
logs and rules text are the inputs, so it is not a live problem, but the
straightforward fix is to carry a running char count alongside `current`.

### F-05 (High) `for-sale-2` converts a panic into a silent no-op that can wedge the game

`rust/game/for-sale-2/src/lib.rs:130-135` and `144-149` (WP-09a)

```rust
pub fn start_buying_round(&mut self) -> Vec<Log> {
    let n = self.players;
    if self.building_deck.len() < n {
        return vec![];
    }
    self.open_cards = self.building_deck.split_off(self.building_deck.len() - n);
```

The pre-existing bug was a `usize` underflow in `len() - n`. The remediation
guards it by returning an empty log vector and doing nothing else. Nothing is
logged, no error is returned, and `open_cards` is left as it was (empty at
that point in the round). `Gamer::status` for this crate is
`open_cards.is_empty() && building_deck.is_empty() && cheque_deck.is_empty()`,
so a state with a short-but-non-empty deck reports `Active` with no legal
move for anyone - a permanent wedge, which is strictly worse to operate than
a panic that the requester now converts into a `SystemError`.

This is also the one crate in WP-09a/b whose `validate()` (added in WP-09b,
same file, `:389-414`) checks *only* the per-player vector lengths and
`bidding_player`, not `building_deck`/`cheque_deck` against `players`. So the
guard is the sole defence and it is a silent one.

Remediation: add `building_deck.len() % players == 0` and
`cheque_deck.len() % players == 0` (or `>= players` where a round is pending)
to `for-sale-2`'s `validate()`, and make the two round-start functions return
`Result<Vec<Log>, GameError>` so the impossible case surfaces as
`GameError::internal` rather than an empty log. Both callers are inside
`command`/`start`, which already return `Result`.

### F-06 (High) `Gamer::validate` is a fail-open default, and only 15 of 27 game crates override it

`rust/lib/game/src/game.rs:106-108`

```rust
fn validate(&self) -> Result<(), GameError> {
    Ok(())
}
```

D-36 ("deserialized state is not trusted") is implemented as an opt-in hook
with a permissive default. Confirmed by grep over `rust/game/*/src/`: **15 of 28 crates define
`validate`, 13 do not.**

- With `validate`: age-of-war-2, battleship-2, category-5-2, farkle-2,
  for-sale-2, greed-2, hanamikoji-1, liars-dice-2, lost-cities-2,
  love-letter-2, modern-art-2, no-thanks-2, red7-1, tic-tac-toe-2,
  zombie-dice-2.
- **Without `validate`: acquire-1, alhambra-1, cathedral-2, jaipur-2,
  lords-of-vegas-1, lost-cities-1, roll-through-the-ages-2, seven-wonders-1,
  splendor-2, starship-catan-1, sushi-go-2, sushizock-2, texas-holdem-2.**

That list is not a set of index-free crates. It includes the two crates whose
*critical* findings were duplicate-card minting (alhambra-1) and the
`Box::leak` leak (cathedral-2), plus `sushizock-2` and `lords-of-vegas-1`,
which WP-09a/09b touched for exactly this class of bug (`steal_blue`/
`steal_red` target bounds, `Loc::parse_str` lot bounds) *without* adding the
`validate` hook that would catch the same violation arriving via deserialised
state rather than via a command. `lost-cities-2` got a `validate`; its sibling
`lost-cities-1`, which WP-28 describes as sharing fixes, did not.

A reviewer or a new game port cannot tell from the trait whether a crate has
been audited or has simply never implemented the hook - the two are
indistinguishable at the call site in
`rust/lib/cmd/src/requester/gamer.rs:44-51`.

Remediation: remove the default body so `validate` is a required trait method
and every crate must state its invariants explicitly (an empty
`Ok(())` with a comment is a fine answer for genuinely index-free crates).
That converts "not yet audited" from a silent pass into a compile error. See
the coverage-gap note below for the crates currently relying on the default.

### F-07 (Low) `no-thanks-2` render drops a line instead of surfacing bad state

`rust/game/no-thanks-2/src/render.rs:74-77` (WP-09b)

`pub_state.current_card.unwrap()` became `if let Some(c) = ...`, which is the
right call for a `Renderer` that cannot return an error - but an unfinished
game with no current card is an invariant violation and the render now hides
it entirely (no card line, no marker). `no-thanks-2`'s new `validate()` does
not check `!finished => current_card.is_some()` either, so nothing catches
it.

Remediation: add `if !self.remaining_cards.is_empty() &&
self.current_card.is_none()` (or the equivalent field check) to
`no-thanks-2::validate`, so the requester rejects the state before it reaches
the renderer.

### F-08 (Medium) The typed/spec `expected()` drift was fixed on one side only

`rust/lib/game/src/command/parser/mod.rs:1095-1100` vs
`rust/lib/game/src/command/parser/chain.rs:63-65`, `:118-120`, `:177-179`

WP-04 added `assert_eq!(parser.expected(&[]), spec.expected(&[]))` to
`assert_typed_spec_parity` and, to make it pass, taught
`CommandSpec::Chain::expected` to skip a leading `Space`:

```rust
CommandSpec::Chain(specs) => specs
    .iter()
    .find(|s| !matches!(s, CommandSpec::Space))
    .or_else(|| specs.first())
    ...
```

The typed side was not changed. `Chain2::expected`, `Chain3::expected` and
`Chain4::expected` all unconditionally return `self.a.expected(names)`. The
two sides therefore still disagree for any hand-built `Chain2<Space, _>`:
typed reports `["whitespace"]`, spec reports the second element's expectations.
The parity assertion does not catch it because no committed test constructs
such a chain - `AfterSpace` is the only real producer of a `Space`-leading
chain and its `expected()` (`mod.rs:869-871`) sidesteps `a` entirely by
delegating to `self.parser`.

So the fix targets the symptom (make the existing tests' `AfterSpace` shape
agree) rather than the asymmetry (two independent `expected()` implementations
of the same grammar). It also puts a `Space`-specific special case into the
generic `Chain` arm, where it will silently do the wrong thing for a chain
that legitimately begins with whitespace.

Remediation: make `ChainN::expected` apply the same skip (or, better, give
`AfterSpace` a `to_spec()` that does not synthesise a `Space`-leading `Chain`,
and revert the special case in `CommandSpec::Chain::expected`). Add a parity
test over an explicit `Chain2<Space, Token>` so the assertion actually
exercises the case it was added for.

## Verified good

Read in final form and traced by hand, not just diff-read:

- **`Space::parse`** (`rust/lib/game/src/command/parser/mod.rs:439-457`).
  `input.len() - input.trim_start().len()` is correct: `str::trim_start`
  strips exactly `char::is_whitespace`, so the result is always a char
  boundary and matches the old char-count semantics on ASCII. NBSP/U+3000
  covered by test.
- **`Token::parse`** (`:45-62`). `input.get(..t_len)` returns `None` both for
  short input and for a non-boundary index, and both are mismatches - no
  panic path remains. See F-03 for the residual fold-length caveat.
- **`Int::parse`** (`:125-182`). `char_indices().take_while(..).last().map(|(i,
  c)| i + c.len_utf8())` yields the byte length of the accepted prefix.
  Verified against `""`, `"-"`, `"-5"`, `"5-3"`, `"12é"`, `"é12"`: leading `-`
  is still only accepted at index 0 (byte index 0 == char index 0), and
  `found_digit` gating is unchanged.
- **`shared_prefix` + `Enum::parse`** (`:628-728`). Returning `(input_bytes,
  value_bytes)` separately is the right shape, and `full = v_matching ==
  v_str.len()` correctly measures fullness in the value's own bytes.
  `match_len` is comparable across candidates because the input-byte count of
  the first N input chars is candidate-independent. Hand-traced order
  independence for `["abc","ab"]`/`["ab","abc"]` on input `"ab"` and
  `["ab","abcd"]`/`["abcd","ab"]` on `"abcx"` - both orderings agree, so the
  lg F5 fix is real, not just test-shaped.
- **`markup::slice`** (`rust/lib/markup/src/transform.rs:252-291`). Char-based
  slicing plus the `n_len <= start` skip. Traced `["ab","cd"]` over ranges
  `1..4`, `0..3`, `2..4` and the nested `Fg(Bold(..))` case; `end -=
  min(start + n_s_len, end)` correctly consumes the per-node prefix. The
  `canvas` caller cannot underflow the `ex_n_line_bgr.start - x` subtraction
  because `bg_ranges_slice` clamps `start` to `>= bgr.start + x`.
- **Both `Many` progress guards** (`parser/mod.rs:359-399` typed,
  `:995-1027` spec). The two loops are now structurally identical: max checked
  at loop top, item pushed then `break` on zero progress, min checked after
  the loop. The degenerate `max == 0` / `max < min` configs now fall through
  to the min check in both, which is what lg F8 asked for. Non-zero-width
  delimiters still terminate because `offset` strictly increases.
- **`add_offset` propagation** (`parser/chain.rs:15-27`, `:105-112`,
  `:164-171`; `parser/mod.rs:564-577`). Chain3 = `a` then `chain_2(b, c)`; the
  inner `chain_2` already adds `b`'s consumed length and the outer adds `a`'s,
  so offsets compose without double counting. Verified the documented
  `"play x"` -> offset 5 result by hand.
- **`suggest.rs` bounded-`Many` cap** (`rust/lib/game/src/command/suggest.rs:135-190`).
  `consumed_items` counts only fully-consumed item+delimiter pairs, so a word
  still being typed at the cap is correctly still suggested (`"1 2"` ->
  `["2"]`) while a completed cap suggests nothing (`"1 2 "` -> `[]`). The
  zero-progress escape (`return suggest_spec(spec, rem, names)`) recurses into
  the *item* spec, not the `Many`, so it cannot loop.
- **`Spec::Int` suggestion saturation** (`suggest.rs:105-118`).
  `start.saturating_add(4)` then `max.map(|m| m.min(capped))`;
  `i32::MAX..=i32::MAX` yields one entry.
- **`markup::to_string` / `from_string` brace round trip**. `replace('{',
  "{{lbrace}}")` is single-pass so the replacement is not re-scanned, and the
  `text()` parser tries `none_of("{")` before `attempt(string("{{lbrace}}"))`.
  Hand-traced the hostile cases `"{{lbrace}}"` and `"{{/b}}"` as literal text
  - both round trip. `}` needs no escaping because no tag opens with it.
- **`check_player` at the requester boundary**
  (`rust/lib/cmd/src/requester/gamer.rs:25-37`). Applied to both request
  variants that carry a player index (`Play`, `PlayerRender`), after
  `validate()` and before any `Gamer` call, and returns `UserError` (correct:
  the index is caller-supplied) while `validate()` failure returns
  `SystemError` (correct: the stored state is ours). `Status`/`PubRender`
  carry no player index.
- **`sushizock-2::steal_blue`/`steal_red` and `love-letter-2::assert_target`**
  bounds checks are placed before every indexing use of `target` and return
  `InvalidInput`, which is the right kind for a user-supplied target.
- **`lords-of-vegas-1::Loc::parse_str`** (`src/board.rs:87-92`) now
  range-checks `lot` against `block.max_lot()` before constructing the `Loc`,
  closing the parse-side hole rather than bounds-checking at each use site.
- **`modern-art-2::validate`'s `round >= ROUNDS` rejection is safe.** I
  suspected it would reject legitimately-finished games. It does not:
  `ROUNDS = 4` (`src/lib.rs:24`) and the sole increment site (`:368`) is in the
  `else` of `if self.round == ROUNDS - 1` (`:350`), which sets
  `finished = true` instead. Max persisted `round` is 3.
- **`love-letter-2::validate`'s "non-eliminated player has a non-empty hand"
  invariant is safe.** `start_round` (`src/lib.rs:86-110`) resets `eliminated`
  and deals `hands` inside one synchronous call, and `eliminate()` (`:156-173`)
  sets the flag and drains the hand in the same call, so the transient
  violation is never persisted. `end_round` already indexes `hands[p][0]` for
  every non-eliminated `p` (`:184`), so the invariant was load-bearing before
  `validate` asserted it.
- **`acquire-1` `panic!` -> `GameError::internal`** conversions in
  `end_sell_trade_phase` / the merger-conversion path: both are in functions
  that already return `Result`, so no signature contortion, and the wrong-phase
  case is genuinely internal (not user input).

## Coverage gaps

Unit 01 is ~11,400 diff lines across the 11 commits, which does not fit a
150k review budget. This report covers the correctness-critical half:

**Reviewed (diff + final code):** `9abe8b4a` WP-01, `91f26820` WP-02,
`c39786f9` WP-03, `82157548` WP-04, `ff8f83ba` WP-09a, `c078c3ee` WP-09b.

**Not reviewed - needs a follow-up sub-unit (01b):**

| commit | WP | diff lines | why it matters |
|---|---|---|---|
| `4a978cbe` | WP-05 | 2,749 (mostly deletions) | dead-API deletion; needs a check that nothing deleted was still referenced and that `IN_USE_MIXES`/`IN_USE_SOFTENS` still cover every live call site |
| `a543120f` | WP-06 | 914 | lib/cmd HTTP handler panic removal + CLI/REPL error paths; the WP-02 commit message explicitly defers "CLI REPL will panic" to this commit, so it must be verified that it actually landed |
| `63063a4b` | WP-07 | 1,234 | game_client timeout ceiling / retry widening / version validation, rand_bot degenerate specs |
| `f13450a1` + `c14bc655` | WP-08/08b | 2,915 | epilogue dedup across 13 crates - highest behaviour-change risk in the unit; each crate's `placings()`/`points()` must be shown to produce identical output to the code it replaced |

Also not covered within the reviewed commits:

- WP-05's decision D-39 (regex/lazy_static drop) was not cross-checked against
  `docs/CODING.md`'s dependency-strategy section.
- Beyond `ChainN` (see F-08) I did not audit every typed combinator's
  `expected()` against its `CommandSpec` counterpart; `OneOf`, `Opt`, `Map` and
  `Doc` were only spot-checked.
- The 13 crates without a `validate()` override (F-06) were identified but
  their invariants were not audited - that belongs to Units 02-04, which own
  those crates. F-06 should be carried into those units' briefs.
- Non-ASCII coverage was confirmed added for `Space`/`Token`/`Int`/`Enum`/
  `Player`/`slice`/`word_wrap`/red7-1 `CardParser`. I did not audit whether any
  *other* byte-slicing site in `lib/game` or `lib/markup` was missed by WP-01
  (e.g. `AfterSpace`, `command/doc.rs`).
