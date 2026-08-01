# Superpowers To Processed Beef Documentation Migration

## Status

Approved. Completed; retained artifacts archived at
`docs/changes/archive/2026-08-02-superpowers-to-processed-beef/`.

## Intent

Replace the legacy Superpowers documentation layout with the Processed Beef
change layout while preserving every source document and every non-immutable
reference to it.

## Scope

- Relocate every document under `docs/superpowers/`.
- Create required Processed Beef active/archive change directories and artifacts.
- Update every non-immutable repository reference to the relocated documents.
- Preserve complete source document content, except exact path-token replacements
  needed to keep references valid.
- Retain historical handovers and context documents as `research/` artifacts.
- Keep this migration's own Standard artifacts active at
  `docs/changes/superpowers-to-processed-beef/` until user result approval.

## Non-Goals

- Do not rename or structurally rewrite `docs/BACKLOG.md`.
- Do not create `docs/agent-process.md`, `docs/principles.md`,
  `docs/decisions.md`, or `docs/backlog.md`.
- Do not reorganize documentation outside the legacy Superpowers tree.
- Do not change code, runtime behavior, CI behavior, Kubernetes behavior, or
  migration content.
- Do not archive this migration's own artifacts before user result approval.

## Classification And Mapping Rules

1. Build an exhaustive temporary mapping table from every tracked legacy source
   file before any move. Each row records source path, destination path,
   artifact role, and active/archive classification.
2. A spec ending in `-design.md` pairs with a plan whose basename is exactly the
   same after removing `-design`. The pair becomes one change directory with
   `spec.md` and `plan.md`.
3. Any unpaired spec or plan receives its own uniquely named change directory.
   Never merge multiple legacy specs or plans into one generic artifact.
4. Source files whose basename includes `handover`, `context`, `retrospective`,
   or `lead-notes` have no spec/plan role and are retained as
   `research/<original-filename>` in their own destination change directory.
5. The source root R-14 handover is active and becomes
   `docs/changes/r-14-nats-wire-protocol/handover.md`. Create the minimal
   Processed Beef `spec.md`, `plan.md`, and `log.md` required for its active
   change directory without altering the retained handover content.
6. The R-07 production-email-repair spec and plan are active and become
   `docs/changes/r-07-production-email-repair/{spec,plan}.md`; create its
   required `log.md`.
7. Legacy documents directly linked by an open row in `docs/BACKLOG.md` are
   active. Their matching counterparts use the same active change directory.
   Where an active legacy change lacks a spec or plan, add a minimal artifact
   that records the absent legacy counterpart and points to the preserved source
   artifact.
8. All remaining documents are historical. Place them under
   `docs/changes/archive/YYYY-MM-DD-<unique-change>/`.
9. For archive names, use the leading ISO date from the source filename. For an
   undated source, use the source file's latest Git commit date. Include the
   source role and normalized basename in an unpaired directory name to prevent
   collisions.
10. Preserve source content exactly except for path-token replacements from the
    mapping table. Do not edit substantive prose, code blocks, decisions,
    commands, or operational instructions.
11. Update non-immutable references using the mapping table. The two applied
    migration comments remain unchanged:
    - `rust/web/migrations/005_login_confirmations.sql:2`
    - `rust/web/migrations/009_username_rules.sql:1`

## Approved Clarification

- Directory and convention references that have no file-level mapping may be
  minimally rewritten to accurately describe `docs/changes/` and the
  per-active-change `handover.md` location.
- Replace references to the deleted Zombie Dice deterministic-test plan with
  non-link historical prose identifying it as a deleted superseded plan.
- Replace the six references to deleted #22 email, #25 rules/strategy, and #24
  invites survey handovers with concise non-link historical prose identifying
  the deleted handover material.
- These edits clarify the approved path-reference work only. They do not alter
  scope, acceptance criteria, architecture, behavior, or governance.

## Acceptance Criteria

| ID | Criterion |
|---|---|
| AC-1 | Every tracked legacy source document has exactly one mapped destination and no source document is discarded. |
| AC-2 | R-07 and R-14 are active Processed Beef change directories with required active artifacts. |
| AC-3 | Historical handover and context material is retained under an appropriate archive `research/` path. |
| AC-4 | Every non-immutable repository reference resolves to its mapped destination. |
| AC-5 | The only remaining legacy-layout references are the two immutable migration comments, recorded as an accepted exception. |
| AC-6 | No project governance file is added and `docs/BACKLOG.md` is modified only where a legacy path target changes. |
| AC-7 | This migration's `spec.md`, `plan.md`, and `log.md` remain in its active directory until Orchestrator alignment approval and user result approval. |
| AC-8 | `git diff --check` passes and repository status contains only intended documentation changes. |

## Verification Exception

Applied SQL migrations are immutable because sqlx validates their checksums.
Their two historical path comments will become stale after the relocation and
are intentionally excluded from link-resolution and zero-reference checks.

## Residual Risks

- Generic artifact filenames require a complete mapping table to prevent a
  wrong spec/plan pairing.
- Historical Markdown and command excerpts contain many path references; only
  exact mapped path replacements are permitted.
- The immutable migration comments retain dead historical paths by design.

## Pending User Decisions

None. The requested classification and preservation rules resolve the remaining
migration choices.
