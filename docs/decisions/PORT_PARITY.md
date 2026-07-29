# Port-parity policy - official rules are authoritative

**DECIDED 2026-07-25, PARKED 2026-07-25, PARK CONFIRMED 2026-07-26:** for
code-vs-rules divergences in ported games, the OFFICIAL game rules are
authoritative. Porting means correcting both the code AND `RULES.md`, noting
any Go divergence - but there is no gameplay change without per-game sign-off,
and the policy is parked pending a per-game rules review.

## Context

Many Rust crates faithfully reproduce their Go origin while diverging from the
official rules; `RULES.md` sometimes documents the Go behaviour and sometimes
contradicts the code. This is the global default policy for port-parity
conflicts across all games. It is the companion to the language decision in
`docs/decisions/GO_VS_RUST_PORTING.md`.

## Decision 1 - official rules win

For a port-parity conflict, the official rules are authoritative. Where a
crate's `RULES.md` documents a Go-derived deviation from the official rules,
BOTH the code and `RULES.md` get corrected, and the commit message or doc
notes the Go divergence. This rejects the "documented-in-crate wins" precedent
(an earlier verification finding that code is correct as documented where
in-crate docs claim the deviation): the official rules outrank in-crate docs.

## Decision 2 - no gameplay change without per-game sign-off

"Official rules win" is the tie-breaker applied WHEN REVIEWING a game, not a
licence to change game behaviour autonomously. No gameplay change happens
without per-game sign-off. Edition and variation choices - which ruleset
edition or variant a game implements - are a product decision, not an
adjudication the policy settles.

## Decision 3 - some RULES.md content is AI-generated and may be wrong

`RULES.md` is not a trustworthy baseline for "code vs docs" adjudication: some
of it was AI-generated and may itself be wrong. Consequently the "docs may be
corrected" half is also suspended for parked items - do not rewrite a
`RULES.md` toward the official rules under the park either, since the doc may
be the thing that is wrong.

## Decision 4 - the policy is parked, reviewed per game

The policy stands but nothing acts on it globally. The rules review is done
PER GAME, on per-game sign-off, prioritising acquire-1, seven-wonders-1 /
splendor-2, modern-art-2 and red7-1 (those four unblock the most other work).
`BLOCKED-ON-USER-RULES-REVIEW` is STRONGER than `BLOCKED-ON-DECISION` - it does
not clear when a decision is answered, only on per-game sign-off. Individually
ruled egregious candidates may be carved out of the park; everything else
stays parked.
