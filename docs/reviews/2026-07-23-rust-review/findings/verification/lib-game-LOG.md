# Verification LOG: lib-game (2026-07-24)

Independent verification of `findings/lib-game.md` (unit 1, originally
reviewed by Kimi K3). Read-only; verdicts per finding:
CONFIRMED / ADJUSTED / REJECTED / UNVERIFIABLE, evidence required.
Snapshot: `/home/beefsack/Development/brdgme-review-snapshot/rust`,
commit `f8763a5`.

## Plan

20 findings total in lib-game.md. Two serial Workers (model fable):

| Worker | Scope | Findings | Dump |
|---|---|---|---|
| W1 | parser/mod.rs cluster | F1 Space panic (crit), F2 Token panic (crit), F3 Enum panic (crit), F4 exact Enum bytes/chars (major), F5 full-match order (major), F6 Many zero-progress (major), F7 OneOf dead offset (major), F8 typed Many early-return (minor), F13 Doc::expected divergence (minor), F14 Many::expected divergence (minor), F16 Int char count (nit) | raw/lib-game-parser.md |
| W2 | suggest.rs / doc.rs / deps cluster | F9 suggest Many min/max (minor), F10 Int suggest overflow (minor), F11 doc_int min=0 (minor), F12 doc_many drops max (minor), F15 combine unused (minor), F17 case-folding (nit), F18 suggest dedupe (nit), F19 recursion depth (nit), F20 Token("") (nit) | raw/lib-game-suggest-doc.md |

Lead spot-checks all REJECTED/ADJUSTED verdicts, then curates
`verification/lib-game.md`.

### W1 dispatched — parser/mod.rs cluster (11 findings)

### W1 returned

All 11 CONFIRMED (F1-F8, F13, F14, F16). Dump: raw/lib-game-parser.md.
- VERIFIED: F7 site count is 12 not 10, but the 2 extras (`offset:
  error_consumed` in the OneOf impls) are inductively always 0 — claim's
  substance holds exactly.
- VERIFIED: F5 trace reproduced including reversed-order success path;
  comment at 608-609 promises the opposite behavior.
- No REJECTED/ADJUSTED verdicts to spot-check in this batch.

### W2 dispatched — suggest.rs / doc.rs / deps cluster (9 findings)

### W2 returned

All 9 CONFIRMED (F9-F12, F15, F17-F20). Dump: raw/lib-game-suggest-doc.md.
- VERIFIED: F12 reachability is stronger than the finding's "latent"
  framing — bounded_spaced specs from sushi-go-2/sushizock-2 reach
  spec.doc() via repl.rs:95 and notify.rs:94; verdict stays CONFIRMED.
- VERIFIED: F20 shadowing confirmed but limited to an empty trailing
  fragment, matching the finding's "on empty input" claim.
- No REJECTED/ADJUSTED verdicts to spot-check.

## Lead spot-checks

Since both batches were all-CONFIRMED (nothing to spot-check per the
rejection rule), the Lead instead sanity-checked the two hardest
confirmations against rubber-stamping:

- CONFIRMED: F1 — read parser/mod.rs:425-449; char count at 431,
  byte slices at 440-442. Panic claim correct.
- CONFIRMED: F5 — read parser/mod.rs:595-649 and traced both orderings:
  ["abc","ab"] + input "ab" yields matched=["abc","ab"] (ambiguity error);
  ["ab","abc"] succeeds. Order dependence exactly as claimed.

## Curation complete (2026-07-24)

20/20 CONFIRMED, 0 ADJUSTED, 0 REJECTED, 0 UNVERIFIABLE.
Corrected unit tally (unchanged): 3 critical / 4 major / 8 minor / 5 nit.
Report: verification/lib-game.md. LOG closed.
