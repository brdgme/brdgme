# F-140 Plan

## Approach

Change only the shared rules and render-metadata visibility predicates so they
require `is_public = true` without excluding deprecated versions. Retain every
new-game predicate requiring both public and non-deprecated. Cover the shared
database lookups and the anonymous production SSR route, then audit callers and
predicates.

## Work Units

| Unit | Scope | Status |
|---|---|---|
| `f140-u01` | Change only `find_game_version_rules` and `find_game_version_render_meta` visibility predicates; preserve new-game predicates. | complete |
| `f140-u02` | Add DB regressions in `db/game_types.rs` and an SSR regression in `rust/web/tests/ssr_pages.rs`. | complete |
| `f140-u03` | Audit call sites and predicates; run `git diff --check` and `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`; record acceptance map. | complete |

## Verification And Acceptance Map

| Acceptance criterion | Evidence |
|---|---|
| Public-deprecated rules and render metadata succeed | `f140-u02` `#[sqlx::test]` exercises both production lookup functions. |
| Non-public versions return neither | `f140-u02` `#[sqlx::test]` exercises both production lookup functions. |
| Anonymous page renders public-deprecated authored rules | `f140-u02` `#[sqlx::test]` drives the real `/rules/<version_id>` SSR route. |
| New-game predicates retain non-deprecated filter | `f140-u01` Lead inspection and `f140-u03` static predicate audit. |
| Email callers continue shared behavior | `f140-u03` static caller audit confirms shared lookup use with no caller change. |
| Static audit and permitted compile pass | `f140-u03`: `git diff --check`; `SQLX_OFFLINE=true cargo check -p web --all-targets --features ssr`. |
| DB and SSR runtime tests | CI-only; not run locally under the approved constraints. |

## Constraints

- Do not modify migrations, creation selectors, routes, authorization, email
  rendering, operator/CR behavior, or strategy-fetch behavior.
- Do not stage, commit, or push.
- Preserve all unrelated worktree changes and isolate F-140 hunks from R-51 in
  `rust/web/src/db/game_types.rs`.

## Pending User Decisions

- None.

## Residual Risks

- Runtime database and SSR coverage requires CI and is not executed locally.

## Current Evidence

- `find_game_version_rules` and `find_game_version_render_meta` now require
  only `is_public = true`; their DB regression covers public-deprecated success
  and non-public failure in both deprecated states.
- The anonymous `/rules/<version_id>` SSR regression uses a public deprecated
  V1 version and asserts authored rules render through the production route.
- The static audit confirms the rules page and email flows continue to use both
  shared lookups, with no caller changes.
- New-game selectors in `find_latest_non_deprecated_game_version`,
  `find_available_game_types`, and `game_info_rules_version_id` retain
  `is_public = true AND is_deprecated = false`.
- `git diff --check` passed. `SQLX_OFFLINE=true cargo check -p web --all-targets
  --features ssr` passed with the known unrelated `NotifyKind::Reminder`
  dead-code and `import_game` unused-`mut` warnings.
- Independent access-control/test-quality review found no actionable F-140
  issue. Its two scope observations were rejected as R-51 F-196 hunks that
  predated F-140 and were explicitly excluded from this change.
