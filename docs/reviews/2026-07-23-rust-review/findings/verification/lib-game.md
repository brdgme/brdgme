# Verification: lib-game (unit 1)

Independent verification of `findings/lib-game.md` (originally reviewed by
Kimi K3), performed 2026-07-24 against the snapshot at
`/home/beefsack/Development/brdgme-review-snapshot/rust`, commit `f8763a5`.
Raw verdict dumps: `raw/lib-game-parser.md`, `raw/lib-game-suggest-doc.md`.
Process log: `lib-game-LOG.md`.

## Per-finding verdicts

| # | Finding | Orig. severity | Verdict | Rationale |
|---|---|---|---|---|
| F1 | Space::parse panics on multi-byte whitespace | critical | CONFIRMED | Char count (mod.rs:431) byte-sliced at 440-442; NBSP input panics; lead re-traced |
| F2 | Token::parse panics splitting multi-byte char | critical | CONFIRMED | Byte-length check passes while `&input[..t_len]` splits a char |
| F3 | Enum::parse panics on multi-byte values | critical | CONFIRMED | shared_prefix returns chars, sliced as bytes at 641-642; reachable via Player names |
| F4 | Exact Enum multi-byte values never match | major | CONFIRMED | `matching` (chars) vs `v_len` (bytes) comparison at 622 |
| F5 | Enum full-match priority order-dependent | major | CONFIRMED | Lead re-traced both orderings; ["abc","ab"] + "ab" gives spurious ambiguity |
| F6 | Many loops lack zero-progress guard | major | CONFIRMED | All three loops (typed, spec, suggest) verified guard-free; zero-width constructible |
| F7 | OneOf furthest-error ranking is dead code | major | CONFIRMED | 12 Parse sites (not 10), but the 2 extras are inductively offset 0 — substance holds |
| F8 | Typed Many early-return bypasses min check | minor | CONFIRMED | Typed returns Ok(empty), spec fails min check — real divergence |
| F9 | suggest Many ignores min/max | minor | CONFIRMED | min/max destructured away; reachable via sushi-go-2/sushizock-2 bounded specs |
| F10 | Int suggestion range overflow | minor | CONFIRMED | `start + 4` overflows near i32::MAX; debug panic / release empty range |
| F11 | doc_int renders open min as 0 | minor | CONFIRMED | `min.unwrap_or(0)` contradicts parser and expected_output; for-sale-2 reachable |
| F12 | doc_many drops bounded max | minor | CONFIRMED | `*`/`+` arms shadow the range arm; reachability stronger than stated (see corrections) |
| F13 | Doc::expected diverges typed vs spec | minor | CONFIRMED | Typed delegates; spec returns `vec![name.clone()]` at 1031 |
| F14 | Many::expected diverges typed vs spec | minor | CONFIRMED | Typed wraps cardinality phrases; spec returns bare inner |
| F15 | `combine` declared but unused | minor | CONFIRMED | Zero usage in lib/game/src; unicase/log/serde_json genuinely used |
| F16 | Int::parse char-count-as-byte-index fragile | nit | CONFIRMED | Safe today (ASCII-only accepted chars); same pattern as F1/F3 |
| F17 | Case-folding differs suggest vs Token::parse | nit | CONFIRMED | to_lowercase vs UniCase full folding; theoretical for ASCII |
| F18 | Suggestions not deduplicated | nit | CONFIRMED | Enum::parse dedupes via HashSet; suggest Enum/OneOf arms do not |
| F19 | Unbounded recursion over spec nesting | nit | CONFIRMED | No depth guard; Spec derives Deserialize |
| F20 | Token("") shadows later chain elements | nit | CONFIRMED | Empty-string suggestion short-circuits Chain at suggest.rs:74-75; limited to empty fragment |

## Summary

- Findings verified: 20
- CONFIRMED: 20, ADJUSTED: 0, REJECTED: 0, UNVERIFIABLE: 0
- Corrected tallies for the unit (unchanged from original): 3 critical /
  4 major / 8 minor / 5 nit
- Lead spot-checked F1 and F5 directly (the two hardest traces); both
  reproduced exactly.

## Notable corrections

None changed a verdict; two factual refinements:

- F7: the crate has 12 `GameError::Parse` construction sites, not the
  claimed 10. The 2 extras are the OneOf impls' `offset: error_consumed`,
  which is provably always 0 by induction — the "ranking is dead" claim
  holds exactly.
- F12: the "latent" framing understates reachability. `Many::to_spec`
  (parser/mod.rs:415-422) propagates min/max from `Many::bounded_spaced`,
  and the sushi-go-2 / sushizock-2 specs feed `spec.doc()` via
  repl.rs:95 and notify.rs:94, so the bounded-max misrendering ("+"
  instead of "(1-2)") is user-visible today. Severity (minor) still
  appropriate.

Overall assessment: the original lib-game review is highly accurate — every
claim, severity, and location checked out, including the subtle char/byte
arithmetic traces and the cross-crate reachability claims.
