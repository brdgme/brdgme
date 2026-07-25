# TRIAGE Lead log

## Plan (2026-07-25)

Role: Lead for the TRIAGE unit of the rust-review follow-up. Goal: produce
planning layer (work-packages.md, decisions-needed.md, BACKLOG.md) from
the 563 surviving findings across 13 units.

Approach:
- Serial Workers (model fable per user override), each extracts structured
  triage rows from 2-4 unit findings files into planning/raw/.
- Workers for units 1-9 read verification reports alongside and apply
  corrected severities; skip rejected findings (games-batch-d F13,
  web-server F30).
- Lead synthesizes the three deliverables from raw notes.

Worker breakdown (finding counts from REVIEW.md tallies):
- W1: lib-game (20) + lib-support (45) = 65
- W2: games-batch-a (20) + games-batch-b (35) + games-batch-c (34) = 89
- W3: games-batch-d (45) + games-batch-e (46) = 91
- W4: games-batch-f (58) + web-server (66) = 124
- W5: web-domain (78) + web-frontend-email (60) = 138
- W6: bot-operator-tools (30) + dependencies (26) = 56
Total = 563.

Raw row format per finding:
`<unit> F<n> | <sev> | <one-line summary> | <paths> | M|D | <group hint>`
(M = mechanical fix, D = needs design decision; D rows note which of the
34 decision items they map to, or NEW if not covered.)

Coverage check at synthesis time: per-package counts must sum to 563.

## W1 return (a282584c9a6516d74)

raw/w1-libs.md written: 65 rows. lib-game 20 (3c/4M/8m/5n), lib-support 45
(1c/5M/23m/16n). Matches expected tallies; no rejections.

## W2 return (adcc78c9e6a91eddb)

raw/w2-games-abc.md written: 89 rows. a 20 (0c/6M/6m/8n), b 35
(1c/5M/13m/16n, F9 nit->minor applied), c 34 (0c/4M/14m/16n, F23
minor->nit applied). Matches expected.

## W3 return (ad95dcdec3c82e853)

raw/w3-games-de.md written: 91 rows. d 45 (1c/6M/16m/22n, F13 excluded,
F4/F15/F26/F36 adjustments applied), e 46 (1c/5M/18m/22n, F37 applied).
Matches expected.

## W4 return (a9f95ff1da4aff7c1)

raw/w4-gamesf-webserver.md written: 124 rows. f 58 (0c/2M/22m/34n),
web-server 66 (0c/8M/36m/22n, F30 excluded, F1/F18/F27/F58 adjustments).
Matches expected. Grouping notes carry the unsound-recommendation cluster.

## W5 return (a0536db785e26c9f4)

raw/w5-webdomain-email.md written: 143 rows. DISCREPANCY: web-domain body
holds 80 finding headings vs stated tally 78 (+2 minor); web-frontend-email
63 vs stated 60 (+1 major, +2 minor). Follow-up ID audit (appended to the
raw file) confirms: no gaps/duplicates in extraction; the findings docs'
own "Severity tally" sections undercount their bodies; the curation-merge
notes do not reconcile the gap. Note: units 10-13 findings docs do not
number findings - W5/W6 assigned sequential F-numbers in document order;
these IDs are canonical for planning and recorded in planning/raw/.

## W6 return (ad18449cade67a21f)

raw/w6-botops-deps.md written: 58 rows. DISCREPANCY: bot-operator-tools 31
headings vs stated 30 (+1 minor); dependencies 27 vs stated 26 (+1 minor,
stale after chrono merge note).

## Synthesis decisions (Lead)

- Authoritative finding count = actual body headings = 570, not the 563 in
  REVIEW.md (units 10-13 doc tallies stale by +7: wd +2m, wfe +1M/+2m,
  botops +1m, deps +1m). Corrected grand tally: 10c / 78M / 257m / 225n.
  Coverage check in work-packages.md reconciles both numbers.
- Rejected findings (games-batch-d F13, web-server F30) excluded per
  verification; all other verification severity adjustments applied.
- 73 work packages defined; every one of the 570 findings assigned to
  exactly one. Cross-cutting packages: char/byte panics, epilogue-dup,
  state-trust hardening, pubstate redaction, port parity, undo/concede+
  ratings, sweeps, bot pipeline (wedge vs supervision split).
- Decisions doc carries the 34 REVIEW items plus 6 additional items
  surfaced during grouping (D-35..D-40).

## Completion (2026-07-25)

Deliverables written:
- planning/work-packages.md - 73 packages (39 READY / 34
  BLOCKED-ON-DECISION), every one of the 570 surviving findings assigned
  to exactly one package; coverage table sums to 570 with per-unit checks
  and the 563-vs-570 reconciliation (stale tallies in units 10-13 docs).
- planning/decisions-needed.md - 40 decisions (D-1..D-34 from REVIEW.md
  section 6, D-35..D-40 new), grouped: A security/integrity, B platform,
  C dependencies/build, D port-parity (D-35 global policy first),
  E additional. Each with context/question/options/recommendation.
- planning/BACKLOG.md - 6 phases + decision-batch schedule; Phase 0 is
  the security decisions batch, Phase 1 the 8 security/corruption
  packages, workspace-deps pulled early into Phase 2 for unblocking
  value, rules adjudication batched in Phase 4, deny.toml flip last.
- planning/raw/ - w1..w6 worker extraction notes (the canonical
  finding-ID mapping for the unnumbered units 10-13 docs).

Workers: 6 serial (all model fable) + 1 follow-up audit of W5. All
returns logged above. No code changed. Unit complete.
