# Superpowers To Processed Beef Documentation Migration Plan

## Status

Approved. Completed; retained artifacts archived at
`docs/changes/archive/2026-08-02-superpowers-to-processed-beef/`.

## Evidence

| AC | Evidence |
|---|---|
| AC-1 | Temporary exhaustive source-to-destination mapping, one source and one target per row. |
| AC-2 | Directory inspection of active R-07 and R-14 artifacts. |
| AC-3 | Mapping rows and file inspection for every historical handover/context document. |
| AC-4 | Zero-reference search outside declared exceptions plus mapped-target existence checks. |
| AC-5 | Search output showing only the two immutable migration comments. |
| AC-6 | Focused diff inspection for governance files and `docs/BACKLOG.md`. |
| AC-7 | Repository status and directory inspection before completion approval. |
| AC-8 | `git diff --check` and focused `git status --short`. |

## Final Outcome And Acceptance Evidence

User approved the result and Orchestrator approved alignment. The migration is
complete; its retained artifacts are archived at
`docs/changes/archive/2026-08-02-superpowers-to-processed-beef/` and its
transient `log.md` was removed. No commit was created.

| AC | Evidence |
|---|---|
| AC-1 | 132 legacy documents have one-to-one destinations: 12 active files in 7 directories, 120 archive files in 86 directories, 8 archive research files. |
| AC-2 | R-07 and R-14 active change directories exist with the required active artifacts. |
| AC-3 | Historical handover/context documents retained under archive `research/` paths. |
| AC-4 | All changed Markdown local links resolve. |
| AC-5 | Tracked search for `docs/superpowers` returns only the two immutable migration comments plus this migration's retained archived audit records. |
| AC-6 | `docs/BACKLOG.md` changed only where a legacy path target changes; no governance files added. |
| AC-7 | Active artifacts retained until both approvals, then archived with spec/plan retained and transient log removed. |
| AC-8 | `git diff --check` and `git diff --cached --check` pass; repository status contains only intended migration work and pre-existing unrelated changes; no commit created. |

## Work Units

### unit-01: Read-Only Survey

Status: complete.

- Confirmed the legacy tree contains 132 tracked documents.
- Confirmed R-07 and R-14 are active from current documentation.
- Confirmed path references are documentation/comments only, with no functional
  script or configuration dependency.
- Confirmed the immutable migration-reference exception.

### Approval Gate

- User approves `spec.md`.
- Orchestrator approves `plan.md`.
- Lead creates the approved migration `spec.md`, `plan.md`, and empty
  append-only `log.md`.
- No relocation starts before both approvals.

### unit-02: Build Mapping And Relocate Documents

Depends on: Approval Gate.

Allowed scope:

- Legacy Superpowers tree.
- `docs/changes/`.
- Temporary files under `/tmp/opencode/`.

Actions:

1. Generate a temporary TSV mapping table using the specification rules.
2. Verify every tracked legacy source appears once and every destination is
   unique.
3. Move all source documents to their mapped Processed Beef destinations.
4. Create minimal missing active artifacts for R-14 and any active plan-only or
   spec-only legacy change.
5. Preserve historical handover/context documents under archive `research/`.
6. Append a checkpoint to this migration's `log.md`.

Evidence:

- Mapping TSV.
- Before/after source counts.
- Focused rename diff.
- Active directory listings.

### unit-03: Update References

Depends on: unit-02.

Allowed scope:

- Files containing a mapped legacy path.
- This migration's `log.md`.
- No SQL migration files.

Actions:

1. Replace every non-immutable legacy path reference with its mapped target.
2. Update only path tokens in moved source documents.
3. Update `docs/BACKLOG.md` links without renaming or otherwise restructuring
   the file.
4. Leave the two immutable migration comments unchanged.
5. Append a checkpoint to `log.md`.

Evidence:

- Search results before and after replacement.
- Focused diff showing only path-token edits outside moved files.
- Explicit list of unchanged immutable migration references.

### unit-03a: Resolve Unmapped Historical References

Depends on: unit-03 and approved clarification.

Allowed scope:

- The ten unmapped directory/convention and deleted-plan references identified
  by unit-03.
- This migration's `log.md`.
- No SQL migration files.

Actions:

1. Minimally rewrite directory/convention references to describe
   `docs/changes/` and per-active-change `handover.md` artifacts.
2. Replace the deleted Zombie Dice plan path with non-link historical prose.
3. Append a checkpoint to `log.md`.

Evidence:

- Zero stale legacy-path references outside immutable migrations and this
  migration's approved historical record.
- Focused diff of the ten corrected references.

### unit-04: Independent Structural Review

Depends on: unit-03b.

Allowed scope: read-only.

Checks:

```bash
git diff --check
git diff --find-renames=90% --summary -- docs
git status --short
rg -n 'docs/superpowers' --hidden \
  -g '!docs/changes/superpowers-to-processed-beef/**' \
  -g '!rust/web/migrations/005_login_confirmations.sql' \
  -g '!rust/web/migrations/009_username_rules.sql' .
```

Expected observations:

- The path search has no output.
- A separate unrestricted search returns only the two immutable migration
  comments, plus this migration's approved record if it intentionally names the
  legacy path.
- The mapping table source count equals target count.
- Every mapped destination exists.
- Every changed Markdown link destination derived from the mapping exists.
- No unrelated files changed.

Review output:

- Concrete findings only.
- If a finding requires correction, return it to the Lead for one bounded
  correction unit. Do not improvise a broader reorganization.

### unit-03b: Correct Deleted Historical Handover References

Depends on: unit-03a and approved clarification.

Allowed scope:

- `docs/changes/archive/2026-07-20-session-22b-24-25-orchestration-plan/plan.md`.
- This migration's `log.md`.
- No SQL migration files.

Actions:

1. Replace only the six references to the deleted #22 email, #25
   rules/strategy, and #24 invites survey handovers with concise non-link
   historical prose preserving the survey/resolution meaning.
2. Append a checkpoint to `log.md`.

Evidence:

- Focused diff showing exactly the six historical-reference replacements.
- Search showing no remaining references to the three deleted handover paths.

## Completion Gate

After AC-1 through AC-8 have evidence:

1. Lead prepares the acceptance-evidence map and residual-risk summary.
2. Orchestrator reviews alignment.
3. User approves the result.
4. One Worker performs the completion transaction:
   - archive only `docs/changes/superpowers-to-processed-beef/`;
   - retain its `spec.md`, `plan.md`, and any durable mapping research;
   - remove its transient `log.md`;
   - verify links and repository status.

## Residual Risks

- The immutable migration comments remain stale by accepted exception.
- A mapping error could mispair historical artifacts; the temporary exhaustive
  mapping table and independent review are the controls.
- Existing historical command excerpts remain historical instructions even
  after their path tokens are updated.
- Local-link validation resolved file targets but did not validate anchor
  targets or the semantic intent of the updated links.
- Git-ignored local files (for example `.superpowers/` state files and build
  logs) may retain stale legacy-path strings; the tracked-repository search is
  the authoritative reference check.
