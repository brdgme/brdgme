# WP-04: lib-game parser design items

**Findings:** lg F7 (major), lg F13 (minor), lg F14 (minor), lg F17 (nit),
lg F19 (nit). **Decision:** D-38, all four sub-items answered - (i) implement
OneOf offset propagation; (ii) align spec `expected()` to typed behaviour and
extend the parity tests to cover `expected()`; (iii) adopt UniCase in
`suggest`; (iv) **skip** the spec depth guard.

**Landing order:** WP-03 first (already applied in live code - the progress
guards, the `Many` max-at-top-of-loop check and the suggest dedup are present).

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

**Standing constraint for this whole package: keep the parser obvious.** No new
traits, no generic offset-tracking wrapper, no builder. Every change below is a
`map_err` at a call site, a field value, or a small free function.

## 1. Problem

- **lg F7** - both `OneOf` impls (`impl Parser for OneOf` and the
  `CommandSpec::OneOf` arm of `impl Parser for CommandSpec`, both in
  `rust/lib/game/src/command/parser/mod.rs`) rank child errors by
  `GameError::Parse.offset` to keep only the furthest-progressing alternative,
  but every `Parse` error in the crate is built with `offset: 0` and no
  combinator ever adds to it. The ranking is dead: it degrades to "accumulate
  every error in declaration order".
- **lg F13** - `Doc::expected` (typed) delegates to the inner parser; the
  `CommandSpec::Doc` arm of `CommandSpec::expected` returns `vec![name.clone()]`.
- **lg F14** - `Many::expected` (typed) wraps each entry with a cardinality
  phrase; the `CommandSpec::Many` arm of `CommandSpec::expected` returns the
  bare inner `expected`.
- **lg F17** - `suggest_spec` (`rust/lib/game/src/command/suggest.rs`) filters
  the `Spec::Token` arm with `to_lowercase()` prefix matching, while
  `Token::parse` accepts with `UniCase`. Full Unicode folding vs simple
  lowercasing diverge (`ß`/`ss`).
- **lg F19** - `suggest_spec` and the spec `parse` impl recurse per nesting
  level with no depth guard.

## 2. Why it's wrong

- **lg F7 is correct as written.** Verified live: every `GameError::Parse`
  literal in `parser/mod.rs` sets `offset: 0` except the two `OneOf` sites,
  which set `offset: error_consumed` - itself derived only from child offsets,
  so provably 0 by induction. `chain_2` and the `CommandSpec::Chain` arm
  propagate child errors with bare `?`.
- **lg F13 is correct as written.** Verified live in `CommandSpec::expected`.
- **lg F14 is correct as written.** Verified live in `CommandSpec::expected`.
- **lg F17 is correct as written**, but only for the `Spec::Token` arm. The
  `Spec::Enum` and `Spec::Player` arms of `suggest_spec` must **keep**
  `to_lowercase`, because `Enum::parse` folds via `shared_prefix`, which
  compares `char::to_lowercase` per char. Switching those arms to UniCase would
  create a new divergence, not remove one.
- **lg F19 is correct as written but out of scope by decision** (see Non-goals).

## 3. Required end state

### 3a. Offset propagation (lg F7) - `parser/mod.rs` + `parser/chain.rs`

Define the invariant in a comment on the `Parser` trait's `parse` method:
*`GameError::Parse.offset` is the byte position of the failure measured from
the start of the input slice this parser was given. Leaf parsers report 0.
Only `Chain` adds.*

Add one private free function in `parser/mod.rs`, `pub(crate)`:

```rust
/// Re-base a child error's offset onto this parser's input (see the offset
/// invariant on `Parser::parse`).
pub(crate) fn add_offset(e: GameError, by: usize) -> GameError { ... }
```

It adds `by` to `offset` for `GameError::Parse` and returns any other variant
unchanged. Then apply it at exactly these four kinds of site - nowhere else:

- `parser/chain.rs::chain_2` - the `b.parse(lhs.remaining, ...)` call gains
  `.map_err(|e| add_offset(e, lhs.consumed.len()))`.
- `parser/chain.rs` - `Chain3::parse` and `Chain4::parse` each call
  `chain_2(..., head.remaining, ...)`; that call gains
  `.map_err(|e| add_offset(e, head.consumed.len()))`. The leading `head`/`a`
  parse is at offset 0 and is left alone. `Chain2` needs no change beyond
  `chain_2`. `AfterSpace::parse` goes through `chain_2` and needs no change.
- `parser/mod.rs`, the `CommandSpec::Chain` arm of `impl Parser for
  CommandSpec` - `s.parse(remaining, ...)` gains
  `.map_err(|e| add_offset(e, consumed_len))`.
- The two min-check failures (`Many::parse` typed, and the `CommandSpec::Many`
  arm) change `offset: 0` to the bytes consumed so far (`offset` / `consumed_len`).

Everything else keeps `offset: 0`. `Map`, `Opt`, `Doc` and `Player` delegate on
the same input slice, so their child offsets are already correct - do not touch
them. The `Many` item-loop errors are discarded (`break`), so nothing to
propagate. **Neither `OneOf` body changes** - they already emit
`offset: error_consumed`; this WP only makes that number non-zero.

### 3b. `expected()` alignment (lg F13, lg F14) - `parser/mod.rs`

In `CommandSpec::expected`:

- `CommandSpec::Doc { spec, .. }` returns `spec.expected(names)` (delegate,
  matching typed `Doc::expected`). Drop the `vec![name.clone()]`.
- `CommandSpec::Many { spec, min, max, .. }` applies the same cardinality
  wrapping as typed `Many::expected`. To avoid two copies of the four-arm
  match, extract it into one `pub(crate) fn many_expected(inner: Vec<String>,
  min: Option<usize>, max: Option<usize>) -> Vec<String>` in `parser/mod.rs`
  and call it from both `Many::expected` and this arm.
- `CommandSpec::Chain(specs)` returns the `expected` of the **first non-`Space`
  spec** (currently: the literal first spec). This is a third divergence, found
  during spec drafting and not named in any finding; it falls inside D-38(ii)
  ("align spec `expected()` to typed behaviour"), and the **Lead has ruled: fix
  it this way.** Rationale: `AfterSpace::to_spec()` is `Chain([Space, inner])`
  while typed `AfterSpace::expected` returns `inner.expected`, so without this
  the spec side answers `["whitespace"]` and the new parity assertion in 5
  fails. The rejected alternative - changing typed `AfterSpace::expected` to
  return `["whitespace"]` - would align the two by making the user-facing text
  worse. **Do not take it.** If a chain is entirely `Space` specs, fall back to
  the first spec's `expected` so the function is still total.

### 3c. Case folding (lg F17) - `suggest.rs::suggest_spec`

`Spec::Token` arm only: replace the `to_lowercase` prefix test with the folded
equivalent, using `UniCase::to_folded_case()` (`unicase` 2.9 is already a direct
dependency of `lib/game`):

```rust
UniCase::new(token).to_folded_case()
    .starts_with(&UniCase::new(remaining).to_folded_case())
```

Leave the `Spec::Enum`, `Spec::Player` and `Spec::Doc` arms on `to_lowercase`.
Add a one-line comment on each of the three saying which parser it mirrors
(`Enum::parse`/`shared_prefix` for Enum and Player; the Doc arm's
`at_current_pos` is a display heuristic, not an acceptance test).

## 4. Non-goals

- **lg F19: skipped by D-38(iv).** `Spec` derives `Deserialize` but crosses no
  trust boundary today - specs are constructed by trusted game crates. No depth
  guard, no recursion limit, no iterative rewrite. Revisit only if a spec is
  ever deserialized from user input.
- No change to leaf-parser offsets beyond the two min checks; do not try to
  make `Int`/`Enum` report a partial-consumption offset.
- No new abstraction for offset tracking (no wrapper type, no trait method).
- WP-03's items (`Enum` priority, progress guards, suggest max cap, doc
  rendering, `combine` dep) are out of scope and already live.
- Do not change `Parser::expected`'s signature or the `Suggestion` type.

## 5. Regression test cases

All in the existing `#[cfg(test)] mod tests` of the named file.

- `parser/mod.rs` - **extend `assert_typed_spec_parity`** with, once per
  parser (outside the per-input loop):
  `assert_eq!(parser.expected(&[]), parser.to_spec().expected(&[]))`. All
  existing callers (`splendor_take_typed_spec_parity` and the other
  `assert_typed_spec_parity` call sites) must pass unchanged; if one fails, the
  alignment in 3b is incomplete - STOP and report rather than weakening the
  assertion.
- `parser/mod.rs` - a direct test that a `Doc`-wrapped `Enum` spec's
  `expected()` yields the enum values, not the doc name (lg F13), and that a
  bounded `Many` spec's `expected()` yields `"between 1 and 2 ..."` (lg F14).
- `parser/mod.rs` - offset propagation (lg F7): for `Chain([Token("play"),
  Space, Token("card")])`, parsing `"play x"` fails with
  `GameError::Parse { offset: 5, .. }` (assert the exact byte offset, not just
  non-zero). Then a `OneOf` of two chains sharing a first token, where only the
  branch that got further contributes to `expected` - assert the losing
  branch's `expected` entries are absent.
- `parser/mod.rs` - a `OneOf` whose branches all fail at offset 0 still
  accumulates every branch's `expected` (no behaviour change for the flat case).
- `suggest.rs` - `Spec::Token("Straße")` is suggested for the fragment
  `"STRASSE"` and for `"stra"`; a `Spec::Enum` with the same value is **not**
  suggested for `"STRASSE"` (documents the deliberate Enum/Token asymmetry).
  Existing ASCII suggest tests must be unaffected.

## 6. Riders

| finding | file | one-line fix | test needed |
|---|---|---|---|
| lg F19 | `command/suggest.rs`, `command/parser/mod.rs` | none - skipped by D-38(iv); record the no-trust-boundary reason in the `suggest_spec` module header comment | n |
