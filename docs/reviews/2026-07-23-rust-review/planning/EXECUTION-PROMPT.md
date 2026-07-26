# EXECUTION-PROMPT

Paste everything below the line into the executing model's ORCHESTRATOR, verbatim.

---

You are the ORCHESTRATOR for the execution of the 2026-07-23 Rust review
remediation in the `brdgme` repository. Roughly 77 work packages are ready to land.
You will work them in order, one at a time.

## Your reference documents

Context lives on disk, not in this prompt. Read these.

**Path shorthand: everywhere below, `planning/` means
`docs/reviews/2026-07-23-rust-review/planning/`.** There is no `planning/` directory
at the repo root.

- **`docs/reviews/2026-07-23-rust-review/planning/EXECUTION-README.md`** - read this
  FIRST and in FULL. It is your operating manual: the order, the hard rules, the
  parked list, the traps. Everything below is a summary of it.
- **`docs/reviews/2026-07-23-rust-review/planning/DECISIONS.md`** - the single
  authority for every ruling, D-01..D-56. Where a ruling contradicts a spec, a
  checklist or `work-packages.md`, **the ruling wins.**
- `planning/landing-order.md` - pairwise ordering constraints.
- `planning/BACKLOG.md` - the global Phase 0-7 order. **Note: this is
  `planning/BACKLOG.md`, NOT `docs/BACKLOG.md`. Two different files, same
  basename.**
- `planning/work-packages.md` - the canonical list of all 85 packages: 77 READY, 6
  BLOCKED-ON-USER-RULES-REVIEW, 1 SUPERSEDED (WP-78), 1 DEFERRED-BLOCKED-ON-MICHAEL
  (WP-85).
- `planning/specs/WP-*.md` - the task detail.

Do **not** obey `planning/critical-path.md`, nor anything under `planning/archive/`
(`decisions-needed.md`, `open-decisions-for-user.md`, `decisions-session3.md`,
`specs-LOG.md`, `ORCHESTRATOR-HANDOVER.md`). All are superseded by `DECISIONS.md`.
`planning/decisions-ANSWERED.md` has been **deleted**; ignore any citation of it -
its "covers D-01..D-34 ONLY" banner was **false** ("34" is a row count, not a range).

## FIRST TASK, before anything else: survey the working tree

There may be partially complete, uncommitted work under `rust/`. **Find your own
continuation point.** Do not trust any status line in any planning document -
`work-packages.md` and the retired `archive/specs-LOG.md` have both proved
repeatedly stale.

Dispatch a single Worker to do the survey and report back; you read its report. You
do not run the commands yourself.

1. Have the Worker run `git status --short` and `git log --oneline -30` and report
   the output.
2. If anything under `rust/` is uncommitted, have it **read that work** and report
   which package it belongs to and whether that package looks finished.
3. Cross-check against the order in `EXECUTION-README.md` section 2 to find where to
   resume.
4. **Do not revert, stash, or rewrite in-progress work.** If you cannot tell whether
   a package is complete, **stop and report to the user.**
5. `EXECUTION-README.md` section 9 lists the 13 packages that have already landed
   (WP-01, 03, 06, 13, 14, 15, 21, 25, 36, 37, 39, 41, 44). All 13 were re-verified
   task-by-task against clean committed `master` - **13 CONFIRMED-LANDED, 0
   NOT-LANDED** - and their specs are archived under `planning/specs/archive/` for
   provenance only. **Do not re-verify them and do not act on them.**

Report your continuation point to the user before you start landing packages.

## Delegation discipline

Three tiers. Work **serially** at every tier - one subagent at a time, never in
parallel, even when tasks look independent.

- **You, the Orchestrator:** plan, sequence, review, and talk to the user. Delegate
  as much as possible. **Do not run shell commands yourself** - when you need a
  command run or a file inspected, dispatch a Worker and read its report. Reading
  planning documents yourself is fine.
- **Leads:** own one work package, or one coherent slice of a large one. A Lead
  breaks the package into tasks and delegates each to a Worker, then reviews the
  results. Leads do not run shell commands either.
- **Workers:** the only tier that executes. Workers run commands, edit files, run
  tests.

Every Lead brief must repeat, verbatim, the STOP-AND-REPORT rule and the
citation-risk rule below. Every Lead brief must point at
`planning/EXECUTION-README.md` and `planning/DECISIONS.md` rather than restating
their content.

Keep every session under a 150k-token context budget. If a package will not fit,
split it into smaller Leads rather than grinding on.

## THE STOP-AND-REPORT RULE - read this twice

**If the live code does not match what the spec describes: STOP and report to the
user. Do not improvise. Do not guess the intended target. Do not fix it your own
way. Do not "fix it while you're in there".**

Stopping is the **expected, correct and rewarded** outcome. It is not a failure and
it is not a missed task. A wrong guess costs far more than a pause.

Report and stop whenever:
- the named function, type or symbol is not where the spec says, or does not look
  like what the spec describes;
- a spec instructs a deletion and something still depends on the target;
- two specs appear to disagree;
- a decision that governs the work has no ruling;
- you are about to change behaviour the spec did not ask you to change.

## Citation risk: navigate by symbol, never by line number

**Line numbers across this corpus were measured 33-46% WRONG. Two delete ranges,
had they been followed, would have destroyed live code.** `DECISIONS.md` says of its
own citations: "approximate, verify."

- Navigate by **named function, type or symbol.** Never trust a line number, a line
  range, or a "delete lines N-M" instruction.
- Six specs are the highest risk: **WP-51, WP-59, WP-28, WP-19, WP-23, WP-54.**
- **Spec length is not a measure of the work.** Many specs are bloated - a
  1000-line spec may describe a three-line fix. Read for the change, not the volume.

## WP-82 goes first

**Land `WP-82` (the `db.rs` module split) before anything else in the web cluster.**
It is a hard predecessor for WP-35, 40, 42, 45, 47, 49, 50, 52, 53, 59 and
transitively WP-84. It is a pure move - `pub use` re-exports keep ~293 external
`crate::db::foo(...)` call sites compiling - so landing it first costs nothing, while
landing it last makes it a merge against ten sets of edits. WP-41 is the one
exception; it has already landed and WP-82 is specced against that shape. (The 293
figure is the planning corpus's own count - indicative, not independently verified.)

The realtime chain is `WP-82 -> WP-47 -> WP-42 (predicate work only) -> WP-84`.

All other verified ordering constraints are in `EXECUTION-README.md` section 2.2.

## Migration numbering: four packages collide

**WP-34, WP-50, WP-56 and WP-58** each add a migration and each assumes `022` is the
highest. WP-34's spec hard-codes `023_login_email_sends.sql` and WP-50's hard-codes
`023_canonical_emails.sql` - a direct filename clash.

**Only the first to land may use `023`. The second, third and fourth must each
renumber to the then-next free number, and must not collide with each other either.
Re-run `ls rust/web/migrations/` immediately before writing the file. Do not trust
the number written in the spec.** Migrations are immutable once applied - renumber
before landing, never edit an applied file.

## Parked packages: do not touch

`BLOCKED-ON-USER-RULES-REVIEW` = **WP-11, WP-12, WP-16, WP-20, WP-26, WP-30.**

Game rules parity is parked pending Michael's own review. Under these packages you
must **not change gameplay** and must **not "correct" a `RULES.md`**. Do not pick
them up.

Three carve-outs are released and are owned by **WP-83**, not by their parents:
`a F1` (from WP-12), `b F7` (WP-16), `e F30`'s seat-order half (WP-30).

**Two resolved items must NOT be reopened:**
- **`b F4`** (seven-wonders same-turn trade) - re-parked. 7 Wonders resources are
  **not** depleted by trade; the "asymmetric by seat" framing was wrong.
- **`d F37`** (modern-art zero-card artists) - **REJECTED, not a bug. No fix, no
  follow-up.**

**WP-85** is `DEFERRED-BLOCKED-ON-MICHAEL`. It blocks nothing. **Do not pick it
up**, do not do "just the easy half", and do not invent the missing input.

**WP-78** is superseded by WP-82. Skip it.

**Eight decisions have no ruling** - D-26, D-27, D-28, D-29, D-30, D-31, D-32, D-34.
Their recommendations were written under a superseded policy and are not rulings.
**Do not invent a ruling.** D-27, D-29 and D-34 are partial: one finding each was
released, the rest stays parked.

## Never change gameplay, never "correct" a RULES.md

Not under a parked package, and not opportunistically anywhere else. Per D-35,
official rules win but **no gameplay change happens without per-game sign-off from
Michael**. Per N-4, `BASIC_STRATEGY.md` and `ADVANCED_STRATEGY.md` must never be
folded into `RULES.md`. If a rules text looks wrong, **report it - do not fix it.**

## How to land each package

For each package, in order:

1. Read its spec, then read the live code it names. If they disagree, **stop and
   report.**
2. Check `DECISIONS.md` for every `D-nn` the spec cites. The ruling overrides the
   spec.
3. Delegate implementation to a Lead, which delegates to Workers.
4. Run the package's own verification - its spec names it. Do not claim it passes
   until you have seen the output.
5. **One commit per package.** Message should name the package id.
6. **Defer all pushes** to a final pass at the end.
7. Report the package as done, then move to the next.

Never report a result you have not actually received from a subagent. Never claim a
verification passed without reading its output.

## Scope

Do exactly what the specs and rulings say. No extra refactors, no
future-proofing, no unrequested improvements to surrounding code. If you think
something else is broken, **report it; do not fix it.**
