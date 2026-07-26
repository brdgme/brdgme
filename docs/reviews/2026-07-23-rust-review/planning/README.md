# planning/ - implementation plan for the 2026-07-23 Rust review

This directory is the plan for fixing what the review found. The findings
themselves are in the parent directory: `../REVIEW.md` (synthesis, tallies,
decision list), `../findings/` (per-unit finding bodies, plus
`../findings/verification/` which supersedes raw findings where present),
`../inventory.md`, `../PROGRESS.md`, `../handover.md`.

> **STATUS 2026-07-26: ALL 34 OPEN DECISIONS ARE CLOSED.**
> The resolved record is **`decisions-ANSWERED.md`** - read it before any
> other planning file. It **wins** over `decisions-needed.md`,
> `work-packages.md`, any spec and any checklist wherever they disagree.
> `open-decisions-for-user.md` is now a stub.
>
> `BLOCKED-ON-DECISION` is **extinct**; 16 packages flipped to READY and one
> new package (**WP-81**, the D-40 stats deletions) was created. Three
> packages changed scope: **WP-48 shrank**, **WP-55 and WP-58 grew**. WP-59
> Task 14 is ungated but must be **rewritten**, not executed as specced.
>
> The **parity park stays** (D-35): WP-11, WP-12, WP-16, WP-20, WP-26, WP-30
> remain `BLOCKED-ON-USER-RULES-REVIEW`, to be reviewed **per game**,
> prioritising acquire-1, seven-wonders-1/splendor-2, modern-art-2, red7-1.
> Three carve-outs are released from it and need specs: **`a F1`**,
> **`b F7`** and **`e F30`'s seat-order half** are all FIX NOW. **`b F4`** was
> re-parked with a binding user correction; **`d F37`** was **rejected** as
> not a bug - do not "fix" it later.
>
> Five **standing constraints** now bind implementers beyond the decision that
> produced them: dependency work upgrades everything to latest **first**;
> macro surfaces stay small and obvious; WP-04 keeps the parser
> straightforward and obvious; no Sentry functionality may be lost; `lib/cost`
> gains automated tests. Full text at the end of `decisions-ANSWERED.md`.
>
> **Spec/checklist coverage is effectively complete** (third session,
> 2026-07-26): **59 specs** in `specs/` plus 8 Tier 3 checklists. The only
> genuine spec-writing gap left is the **unowned cluster - WP-76, WP-77,
> WP-79, WP-80** (all small; WP-80 folds into WP-09a/WP-09b). WP-72 lives
> inside `specs/WP-69-deny-toml-hardening.md` by design; WP-78 is SUPERSEDED
> by WP-82; WP-74/WP-75 are covered by `checklists/T3-B8` (WP-75 needs a live
> render, so it is not spec-writable from source alone). The remaining work is
> mostly **execution**, which is the other agent's job.
>
> **Package totals** (recounted 2026-07-26 in `work-packages.md`'s
> Coverage-check section, which is the source of truth): **84 packages = 77
> READY + 6 BLOCKED-ON-USER-RULES-REVIEW + 1 SUPERSEDED (WP-78, by WP-82)**.
>
> **WP-82 lands first.** The `db.rs` module split is a hard predecessor for the
> whole remaining web cluster. It and the migration-numbering collision (now
> **four** packages) are owned by `landing-order.md` - read it before
> sequencing.
>
> Both doc proposals under `planning/` were **APPLIED** this session and are now
> historical: `docs/CODING.md` gained `## Request-Path Invariants`, and
> `docs/BACKLOG.md` carries item #53 (parity park, corrected in place) and new
> item #54 (maximum-performance fuzzer, D-51).
>
> `ORCHESTRATOR-HANDOVER.md` is the freshest state-of-the-world document.
> Read it first if you are resuming the planning effort.

## File map

| Path | What it is | When to open |
|---|---|---|
| `ORCHESTRATOR-HANDOVER.md` | State of the world at the end of the third planning session: what is done, what remains, landing order, parked items, open items still needing Michael | **First, if you are resuming planning** |
| `decisions-ANSWERED.md` | **All 34 decisions, CLOSED**, one table, each row stating the ruling plus its constraints and rationale. **Wins over every other file on any disagreement.** | **First. Before writing any code.** |
| `decisions-session3.md` | **D-41..D-53**, the third session's rulings (game-bin consolidation, fuzz throughput, SSE topology). Extends `decisions-ANSWERED.md`; **still to be folded into it** | Alongside `decisions-ANSWERED.md` |
| `open-decisions-for-user.md` | Stub only - the old open-questions table, now superseded | Never; go to `decisions-ANSWERED.md` |
| `BACKLOG.md` | Prioritized phase ordering (Phase 0-7) of all work packages | To pick what to do next |
| `work-packages.md` | Canonical definition of every WP: findings, scope, status (READY / BLOCKED-ON-USER-RULES-REVIEW / SUPERSEDED; `BLOCKED-ON-DECISION` is extinct). Its Coverage-check section is the **source of truth for package totals** | Before starting a package |
| `landing-order.md` | Verified cross-package sequencing constraints | Before starting any package - see below |
| `critical-path.md` | The 10 criticals + security/integrity majors, with per-finding verify status | Scoping the must-ship set |
| `decisions-needed.md` | D-1..D-41 in full, with the 2026-07-25 **and 2026-07-26** answers inlined | When a spec cites a D-number |
| `tier2-tier3-plan.md` | Dispatch plan for everything off the critical path; defines the tiering | Understanding coverage and gaps |
| `specs/` | **59 implementation specs** (`WP-NN-*.md`) plus `notes-conventions.md` (build/test conventions, toolchain, per-crate cargo rules), which is not a spec | Implementing a package |
| `checklists/` | 8 Tier 3 batch checklists (`T3-B1..B8`), one row per finding | Low-risk sweep work |
| `raw/` | Triage extracts (`w1`-`w6`), the finding-ID mapping for units 10-13, lead review notes for WP-37/41/54/59, cathedral stray-edit diff | Resolving an ID or a disputed finding |
| `architecture-observations.md` | Running notes for the **deferred** architectural review (oversized functions/types/files, crate splitting, module-tree flattening). Started 2026-07-26. **Append only; never act on it during remediation.** | When a package tempts you to widen into architecture - park it here instead |
| `specs-LOG.md` | Append-only log of every spec-writing session (thousands of lines - **grep it, never read it whole**) | Crash recovery; append as you work |
| `triage-LOG.md` | Log of the triage unit that produced work-packages/decisions/BACKLOG | History only |
| `ws-to-sse-evaluation.md` | WebSocket -> SSE evaluation. **Superseded in its recommendation** by D-48/D-50 - read for evidence, not guidance | Background on the SSE design |
| `sse-topology-decision.md` | The two-stream multi-topic topology decision (D-46, resolved by D-48 on a measured `HTTP/2 200`) | Rationale behind `specs/WP-84-sse-migration.md` |
| `fuzz-throughput-evaluation.md` | Throughput evaluation behind D-43/D-51 (keep the `_fuzz` bins; new `docs/BACKLOG.md` item #54) | Background on WP-73 / the fuzzer |
| `BACKLOG-note-proposed.md` | **APPLIED, historical.** Draft of `docs/BACKLOG.md` item #53 (parity park) | History only |
| `CODING-md-amendment-proposal.md` | **APPLIED, historical.** Draft of `docs/CODING.md`'s `## Request-Path Invariants` | History only |

## The tiering

The three bullets below are the **original** tiering from `tier2-tier3-plan.md`.
Coverage has since grown to 59 specs + 8 checklists - see the corrections that
follow, and the STATUS banner.

- **Tier 1 - 25 full specs.** WP-01, 03, 06, 07, 13, 14, 15, 19, 21, 22, 23,
  25, 28, 29, 36, 37, 39, 40, 41, 44, 51, 54, 56, 59, 68.
- **Tier 2 - 15 compact specs.** WP-02, 08, 09a, 09b, 10, 34, 35, 38, 42, 45,
  47, 49, 57, 62, 63. Written whole-package: majors in detail, the package's
  own minor/nit riders as an appendix checklist in the same file.
- **Tier 3 - 8 checklist files** in `checklists/`, covering 16 zero-major
  packages. Table only: `finding id | file | one-line fix | test needed`.

Corrections to the tiering as stated in `tier2-tier3-plan.md`:

- Its Tier 2 roster is **21 packages**, not 15. WP-09 was split into WP-09a +
  WP-09b and WP-42 was promoted from Tier 3 into a compact spec. The 8 formerly
  decision-blocked Tier 2 packages **now all have specs** (written 2026-07-26):
  WP-04, 05, 46, 55, 58, 64, 66, 67.
- Its Tier 3 roster is 23 packages. `checklists/` holds exactly **8 files**
  (`T3-B1..T3-B8`); the 7 formerly decision-blocked packages were written up as
  specs rather than checklist rows - `specs/` has WP-48, 50, 69, 70, 71 and 73.
  **WP-72 has no file of its own by design**: it is section 3d of
  `specs/WP-69-deny-toml-hardening.md`.
- Also now specced: **WP-17** including its D-25 rows
  (`specs/WP-17-lib-cost.md`; the remaining rows stay in
  `checklists/T3-B3-splendor-libcost-holdem.md`), **WP-81**
  (`specs/WP-81-stats-deletions.md`), and the three parity carve-outs `a F1`,
  `b F7`, `e F30` (`specs/WP-83-parity-fixes-released.md`).
- Per the STATUS banner, the only remaining spec-writing gap is the unowned
  cluster **WP-76, WP-77, WP-79, WP-80**.
- Section 3 names the CODING proposal `planning/CODING-amendment-proposed.md`;
  the real filename is `CODING-md-amendment-proposal.md`. Section 2.2 omits
  WP-17, which was formerly D-25-blocked; D-25 is now answered
  (`decisions-ANSWERED.md`) and WP-17 has a spec.

## Execution order

1. **DONE.** All decisions are answered; read `decisions-ANSWERED.md` and
   obey its rulings and standing constraints. No package is
   BLOCKED-ON-DECISION any more. The 6 BLOCKED-ON-USER-RULES-REVIEW packages
   (WP-11, 12, 16, 20, 26, 30) are still parked - do not pick them up **except
   for the three released carve-outs**: `a F1` (WP-12), `b F7` (WP-16) and
   `e F30`'s seat-order half (WP-30).
2. Work the specs. Global priority order is `BACKLOG.md`'s Phase 0-7.
   `landing-order.md` is **not** a global order - it is a set of verified
   pairwise constraints for the clusters where order actually matters
   (section 1 WP-41 before WP-40; section 2 WP-56/WP-59 overlap; section 3
   WP-54; section 4 recommended order for that cluster; section 6 auth,
   delivery/ack and game-state-boundary chains added by later spec Leads).
   Read it in full before sequencing anything; it overrides BACKLOG.md order
   where the two disagree.
3. Then the Tier 3 checklists, grouped by crate: T3-B1..B4 are game crates,
   T3-B5..B7 web/domain/email/bot, T3-B8 workspace hygiene and red7-1 docs.

## Implementer rules

- Read the named function before editing. Specs cite files and symbols.
- Line numbers were deliberately not recorded - the tree drifts. Locate code
  by file plus symbol name.
- On any mismatch between a spec and live source: **stop and report**. Do not
  improvise a fix.
- Specs record where an original finding recommendation was overturned. The
  spec text wins over the finding text.
- Both doc proposals were **APPLIED 2026-07-26** (`docs/CODING.md`'s
  `## Request-Path Invariants`; `docs/BACKLOG.md` items #53 and #54);
  `BACKLOG-note-proposed.md` and `CODING-md-amendment-proposal.md` are history
  only - do not re-apply them.
- **`decisions-ANSWERED.md` outranks every other file here**, including specs.
  Where a spec was written under a now-superseded recommendation (D-7, D-8,
  D-15, D-16, D-37), the ruling wins and the spec must be amended.
- Append to `specs-LOG.md` as you go. It is the crash log.
