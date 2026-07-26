# RETIRED planning documents - provenance only

**These documents are RETIRED. Do not act on them. They are kept for provenance
only.** They record how decisions were reached, not what is currently true. Where
they disagree with the live documents, the live documents win.

Live entry points for an executor:

- `../EXECUTION-README.md` - what to do and how.
- `../EXECUTION-PROMPT.md` - the prompt to run a unit with.
- `../DECISIONS.md` - the **single decision authority**. D-01..D-56 plus the
  finding-level rulings and N-items. Nothing in this directory overrides it.

## Retired files

| file | superseded by | why retired |
|---|---|---|
| `ORCHESTRATOR-HANDOVER.md` | `../EXECUTION-README.md` + `../EXECUTION-PROMPT.md` | The old handover/process description. Kept as a genuine historical record of how the orchestration was run; its instructions are no longer current. |
| `decisions-needed.md` | `../DECISIONS.md` | The original question-and-options framing of D-1..D-41 (its D-41 is now `DECISIONS.md` D-56). Kept as decision provenance - it holds the option lists and recommendations as originally posed. |
| `open-decisions-for-user.md` | `../DECISIONS.md` | A 6-line CLOSED pointer stub with no decision content of its own. Its full text is reproduced in `DECISIONS.md`. |
| `decisions-session3.md` | `../DECISIONS.md` | Provenance for D-41..D-55 (the SSE pivot, fuzz/repl binaries, WP-85 carve-out and deferral). Not in numeric order, and its header inherits `decisions-ANSWERED.md`'s wrong "D-01..D-34" range. |
| `wp85-deferral-finding.md` | partly summarised in `specs-LOG.md` (2026-07-26 decision-record consolidation unit); the deferral itself is now recorded in `../DECISIONS.md` D-54/D-55 | The WP-85 / D-55 deferral safety check. Retired because the deferral decision is authoritative in `DECISIONS.md`, but kept because it holds four things no summary elsewhere carries: the live read of `docs/authoring/COMMANDS.md` (**zero matches for `reserved` or `email`**, so the deferral leaves no stale documentation behind), a per-downstream impact table for the WP-59 chain, WP-54's verbatim self-disclaimer, and the exact stale wording in `work-packages.md`'s WP-59 entry. Originally archived as the leading-dot file `.wp85-deferral-finding.md`; renamed on 2026-07-27 so a human browsing this directory can see it. Contents unchanged. |
| `specs-LOG.md` | **nothing** - it is not superseded | The session-by-session crash/durability log of the whole planning effort. Retired as the final action of the 2026-07-27 cleanup pass because it is 430KB+ of **process history, not instructions** - an executor must not read it. Kept for provenance: it is the only record of what each unit did and why. Its final entry summarises the cleanup pass end to end. |

## `DECISIONS.md` coverage verification (2026-07-27)

The four decision sources above were retired only after a read-only coverage
check against `../DECISIONS.md`. Recorded here so the evidence survives the
deletion of the scratch file it was written in
(`.decisions-coverage-check.md`).

**VERDICT: COVERAGE PASS.**

- `DECISIONS.md` carries `D-01`..`D-56` zero-padded, **56 sections, contiguous,
  no gaps, no duplicates**, plus 6 finding-level sections (`a F1`, `b F4`,
  `b F7`, `e F30`, `d F37`, `bo F25`) and `N-1`..`N-6`. It is a **strict
  superset** of the union of all four sources.
- Per-source ID counts:
  - `decisions-needed.md` - 41 decision IDs (`D-1`..`D-41`, contiguous), plus 25
    finding IDs and the `N-1`..`N-6` group.
  - `open-decisions-for-user.md` - **0** decision IDs (pointer stub).
  - `decisions-ANSWERED.md` - 34 table rows: 23 D-keyed rows + 11 non-D rows
    (`bo F25`, `a F1`, `b F4`, `b F7`, `e F30`, `d F37`, `N-1`..`N-6`).
  - `decisions-session3.md` - 15 decision IDs (`D-41`..`D-55`), plus one
    unnumbered "Terminology correction on the record" section.
  - Union of the sources: `D-1`..`D-55` + 25 finding IDs + `N-1`..`N-6`.
- **Set difference (sources minus `DECISIONS.md`) is EMPTY.** Zero decision IDs
  would be lost by retiring the sources.
- **12 sampled rulings all matched in substance** - D-07, D-08, D-12, D-14,
  D-15, D-35, D-37, D-41, D-43, D-48, D-55, D-56 - deliberately covering the
  REFINED, CORRECTED, REVERSED, REOPENED and PARKED cases and the `D-41`
  ID collision. The three places `DECISIONS.md` differs from a source are all
  correct precedence applications and are labelled as such in the file (D-15
  superseded by D-54/D-55; D-13's `/ws` design marked historical by D-44's SSE
  pivot; the D-41 collision resolved by renumbering the friends-page decision to
  D-56).
- **`decisions-ANSWERED.md` false-banner finding (confirmed).** Its banner
  claimed to cover "D-01..D-34 ONLY". Its main table has **exactly 34 rows** -
  "34" was a ROW COUNT, not a range. The table has **no** rows for `D-1`..`D-6`,
  `D-12`, `D-13`, `D-26`..`D-34`, `D-36`, yet **does** have rows for `D-35` and
  `D-37`..`D-40`. Its banner also said session 3 added "D-41 through D-53" when
  D-54 and D-55 also exist. This false banner is the entire origin of the phantom
  "D-35..D-40 gap"; both errors are recorded verbatim as source defects in
  `../DECISIONS.md`.

## Deleted, not archived

Two files were **DELETED** in the 2026-07-27 cleanup rather than archived. Both
were tracked in git and are recoverable from history.

- **`decisions-ANSWERED.md`** - actively dangerous to leave readable. Its
  false "D-01..D-34" banner (see above) misleads any reader about what it covers,
  and `README.md` pointed executors at it. Its content is fully carried by
  `../DECISIONS.md`, whose source-defects section records the false banner
  verbatim.
- **`tier2-tier3-plan.md`** - stale at source. Only `README.md`'s *description*
  of it was ever corrected; the file itself never was, so a cheap executor model
  could follow it into wrong work.
