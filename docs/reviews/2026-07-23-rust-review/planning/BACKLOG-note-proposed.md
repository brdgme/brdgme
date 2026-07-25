# Proposed `docs/BACKLOG.md` note - NOT APPLIED

**Why this is a proposal and not an edit.** The unit brief asked for the parity
park to be noted in `docs/BACKLOG.md`, but the brief's own HARD READ-ONLY
CONSTRAINT says *"Write only inside
docs/reviews/2026-07-23-rust-review/planning/."* `docs/BACKLOG.md` is outside that
directory, so the Lead did not write it. The exact patch is below; apply it (or
tell the Lead to) once the constraint is relaxed. **`docs/BACKLOG.md` is currently
modified in the working tree** - re-read it before applying.

## Verified context

The 2026-07-23 Rust review is **not referenced in `docs/BACKLOG.md` at all**. A
grep for `2026-07-23`, `rust-review`, `reviews/`, `remediation` and
`work-package` over the current working-tree file returns nothing relevant; only
the older, unrelated `docs/REVIEW-2026-07-04.md` pass is mentioned (History,
~:128-129), and its section was already archived (~:60). So there is **no existing
place** to hang this note - it creates one.

Highest item ID in use anywhere is **#52**, so a new item is **#53**.

## Proposed change - append one Status-table row

The Status table header is at ~:57-64; rows run ~:65-79. Rows are **not** in
numeric order (the tail runs 49, 51, 50, 52), so appending is safe. Add after the
last row:

```
| 53 | Game rules review (parity park) | **PARKED - awaiting Michael's own review of game rules.** The 2026-07-23 Rust review found ~30 port-parity findings (code vs official rules vs RULES.md). D-35 and D-26..D-32, D-34 in [`docs/reviews/2026-07-23-rust-review/planning/decisions-needed.md`](reviews/2026-07-23-rust-review/planning/decisions-needed.md) are PARKED-PENDING-USER-RULES-REVIEW: some RULES.md content was AI-generated and may be wrong, and edition/variation choices are Michael's. **No gameplay change without per-game sign-off.** Packages WP-11, WP-12, WP-16, WP-20, WP-26, WP-30 are BLOCKED-ON-USER-RULES-REVIEW - implementing agents must not pick them up. Five egregious cases (a F1, b F4, b F7, e F30, d F37) are flagged separately as candidates for immediate fix; liveness fixes WP-15/WP-25 are NOT parked. | - | - |
```

The `Captured YYYY-MM-DD` / bold `**note DATE: ...**` idiom in the Status cell
follows the existing house pattern at ~:74 and ~:76.

## Alternative, if a new row is unwanted

Item **#37** (~:72) is "Rust game port verification testing", currently *"Pending -
downgraded 2026-07-17 (Michael: games seem okay, does not block go-live)"*. It is
the pre-existing, thematically nearest row, and the park could be noted there
instead of creating #53. The Lead's recommendation is a **new #53**: #37 is about
verification testing, whereas this is a rules/edition adjudication park, and
folding them would hide the park behind a downgraded item.
