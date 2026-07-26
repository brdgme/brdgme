# planning/ - index for the 2026-07-23 Rust review remediation

This directory holds the remediation plan for what the review found. The
findings themselves are in the parent directory: `../REVIEW.md` (synthesis,
tallies), `../findings/` (per-unit finding bodies, with
`../findings/verification/` superseding raw findings where present),
`../inventory.md`, `../PROGRESS.md`, `../handover.md`. **This README is an index
only - it is not the execution entry point and carries no rulings.**

## Start here

- To EXECUTE the remediation: `EXECUTION-PROMPT.md` (the verbatim boot prompt
  for the executing orchestrator), then `EXECUTION-README.md` (the operating
  manual - read FIRST and in FULL).
- For any ruling: **`DECISIONS.md` is the single decision authority.** It holds
  `D-01`..`D-56` as 56 contiguous zero-padded sections with no gaps and no
  duplicates, plus 6 finding-level sections (`a F1`, `b F4`, `b F7`, `e F30`,
  `d F37`, `bo F25`) and `N-1`..`N-6`. Where a ruling contradicts a spec, a
  checklist or `work-packages.md`, the ruling wins.
- `DECISIONS.md` supersedes four retired decision sources: `decisions-needed.md`,
  `open-decisions-for-user.md` and `decisions-session3.md` (all retired to
  `archive/`), and `decisions-ANSWERED.md` (**DELETED** - its "D-01..D-34"
  banner was FALSE; `34` was a row count, not a range; recoverable from git
  history).
- `tier2-tier3-plan.md` is also **DELETED** (stale at source). The old tiering it
  defined is superseded by `specs-CLASSIFICATION.md` and `EXECUTION-README.md`.

## File map

| File | What it is |
|---|---|
| `EXECUTION-PROMPT.md` (195) | Verbatim copy-paste boot prompt for the executing orchestrator: path shorthand, reference documents, do-not-obey list, roughly 77 ready packages |
| `EXECUTION-README.md` (500) | The executor's operating manual, 11 numbered sections: authority precedence, the WP-82-first hard rule, phase order plus overriding constraints, the twelve packages `planning/BACKLOG.md` does not know about (section 2.3), the `023` migration collision, the parked list, the WP-85 deferral, executor-binding rulings, citation risk, coverage gaps, the uncommitted-work rule |
| `DECISIONS.md` (1274) | The single decision authority. D-01..D-56 plus 6 finding-level rulings plus N-1..N-6 |
| `work-packages.md` (1455) | Canonical per-package definition of all 85 WPs: constituent finding IDs, scope, status. Totals: **85 = 77 READY + 6 BLOCKED-ON-USER-RULES-REVIEW + 1 SUPERSEDED (WP-78, by WP-82) + 1 DEFERRED-BLOCKED-ON-MICHAEL (WP-85)**. `BLOCKED-ON-DECISION` is extinct. Its per-package status lines have proved repeatedly stale - verify against live source |
| `BACKLOG.md` (173) | The global Phase 0-7 remediation order. See the traps section |
| `landing-order.md` (596) | Verified per-cluster pairwise sequencing constraints, 10 numbered sections. Overrides `BACKLOG.md` order where they disagree. See the traps section |
| `specs-CLASSIFICATION.md` (820) | Read-only KEEP/ARCHIVE verdict over 60 spec files: 47 KEEP, 13 ARCHIVE, 0 UNCERTAIN, each read in full with git-log and live-source evidence. Includes the 2026-07-27 re-verification of the 13 ARCHIVE specs against clean committed master. See the traps section |
| `critical-path.md` (352) | The 10 criticals plus security/integrity majors with per-finding verify status. Self-declared **"STALE - HISTORICAL SNAPSHOT"**, citations against worktree `f8763a5`. `EXECUTION-README.md` lists it as a NON-authority - do not obey it |
| `architecture-observations.md` (318) | Append-only parking lot for structural observations (oversized functions/types/files, crate splitting, module-tree flattening) deferred to a post-remediation session. **Never act on it during remediation**; park temptations here instead |
| `sse-topology-decision.md` (268) | The two-stream vs one-stream SSE topology analysis for D-46. Banner: "RESOLVED - 2026-07-26. Retained for RATIONALE only." D-48 ruled two streams; D-50 replaced `?game=<uuid>` with repeatable `?topic=`. Background to `specs/WP-84-sse-migration.md` |
| `ws-to-sse-evaluation.md` (328) | The WebSocket to SSE migration evaluation. Banner: "SUPERSEDED IN ITS RECOMMENDATION - 2026-07-26. Read for evidence, not for guidance." It recommended one stream; D-48/D-50 ruled two |
| `fuzz-throughput-evaluation.md` (221) | Fuzzer throughput analysis behind D-43/D-51 and `docs/BACKLOG.md` item #54. No measurements were taken; UNKNOWNs and settling commands are in its section 6 |
| `triage-LOG.md` (107) | Log of the triage unit that produced `work-packages.md`, `decisions-needed.md` and `BACKLOG.md` from the 563 surviving findings. History only |
| `CODING-md-amendment-proposal.md` | Draft of the `## Request-Path Invariants` section for `docs/CODING.md`. **APPLIED** - live at `docs/CODING.md` (section `## Request-Path Invariants`). Historical only; do not re-apply. Its own title still reads "NOT APPLIED" - that title is stale |
| `BACKLOG-note-proposed.md` | Draft of `docs/BACKLOG.md` item #53 (the game-rules parity park). **APPLIED** - live as item #53 in `docs/BACKLOG.md` (item #54, the maximum-performance fuzzer from D-51, is also live there). Historical only; do not re-apply. Its own title also still reads "NOT APPLIED" - stale |
| `README.md` | This index |

## Directories

| Directory | What it is |
|---|---|
| `specs/` | **47 live WP implementation specs** (`WP-NN-*.md`) plus `notes-conventions.md` (build/test conventions, toolchain, per-crate cargo rules - not a spec) |
| `checklists/` | 8 Tier 3 batch checklists `T3-B1`..`T3-B8`, one table row per finding; worked after the WP suite. B1-B4 game crates, B5-B7 web/domain/email/bot, B8 workspace hygiene plus red7-1 docs |
| `raw/` | 16 triage and provenance extracts: the `w1`-`w6` per-unit triage notes (including the finding-ID mapping for units 10-13), lead review notes for WP-37/41/54/59, the websocket, db-split and wp73 inventories, and the cathedral stray-edit diff. Use to resolve an ID or a disputed finding |
| `archive/` | **PROVENANCE ONLY.** 6 retired documents plus an indexing README: `ORCHESTRATOR-HANDOVER.md`, the three superseded decision docs (`decisions-needed.md`, `open-decisions-for-user.md`, `decisions-session3.md`), `wp85-deferral-finding.md`, and `specs-LOG.md` (430KB+ of process history, not instructions) |
| `specs/archive/` | **PROVENANCE ONLY.** The 13 specs whose work has fully landed on `master` (WP-01, 03, 06, 13, 14, 15, 21, 25, 36, 37, 39, 41, 44), re-verified 2026-07-27 as "13 CONFIRMED-LANDED, 0 NOT-LANDED", plus a "DO NOT EXECUTE" README |

**An executor must NOT act on anything inside `archive/` or `specs/archive/`.**
Their line-addressed instructions describe already-shipped code and would
corrupt it. They are kept for provenance and crash-recovery reading only.

## Traps

1. **Two files are named `BACKLOG.md`.** `planning/BACKLOG.md` is the
   remediation phase order and is the one the execution order means.
   `docs/BACKLOG.md` is the product backlog - a different file, 513 lines,
   touched only by D-51/N-5. Older docs here write the bare basename
   ambiguously - always resolve the full path.
2. **`planning/BACKLOG.md` is stale.** It covers WP-01..WP-75 only, with no
   knowledge of WP-76..WP-85 and no knowledge of the WP-09a/WP-09b split.
   `EXECUTION-README.md` section 2.3 places all twelve. Its `D-nn` blocker tags
   are stale too - use its ordering, ignore its tags; Phase 0 is finished.
3. **`landing-order.md` contains no global order.** It is per-cluster pairwise
   constraints only; a reader expecting a single ordered list will not find one.
   Its sections 4, 8.1 and 10.1 are per-cluster orders, not global ones. The
   global order is `planning/BACKLOG.md` Phase 0-7, as amended by
   `EXECUTION-README.md` sections 2.1-2.3.
4. **`specs-CLASSIFICATION.md` does not cover every live spec.** Its 60 verdict
   rows are 59 WP specs plus `notes-conventions.md`.
   `specs/WP-85-email-parser-first-dispatch.md` postdates it and is
   live-but-unclassified (WP-85 is `DEFERRED-BLOCKED-ON-MICHAEL` in any case).
   The 13 rows it marked ARCHIVE now live in `specs/archive/`, so its file list
   no longer matches `specs/` as it stands.
5. **Line numbers across this corpus are unreliable** - measured 33-46% wrong;
   two delete ranges would have destroyed live code. Navigate by named function,
   type or symbol. On any mismatch between a document and live source, STOP AND
   REPORT - do not improvise.
6. **Both doc proposals are applied but still titled "NOT APPLIED".**
   `CODING-md-amendment-proposal.md` and `BACKLOG-note-proposed.md` are
   historical - do not re-apply them.

## Reader rules

- Read the named function before editing; specs cite files and symbols, never
  line numbers.
- On any mismatch between a document and live source: stop and report.
- A ruling in `DECISIONS.md` outranks every spec, checklist and status line here.
- Package status lines in `work-packages.md` and verdicts in
  `specs-CLASSIFICATION.md` are only as current as the worktree they were read
  against - verify against live source.
