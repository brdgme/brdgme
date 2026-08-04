# F-140 Deprecated Version Rules Access

## Intent

Keep authored rules publicly readable for a public game version after it is
deprecated, while continuing to hide rules for non-public versions.

## Scope

- The shared `find_game_version_rules` and `find_game_version_render_meta`
  predicates in `rust/web/src/db/game_types.rs`.
- Database regressions for both shared lookups.
- An anonymous SSR rules-page regression.
- A static audit of callers and new-game predicates.

## Non-goals

- Migrations.
- Operator or CR changes.
- Game creation selectors.
- URL or email rendering changes.
- Authorization routes.
- Service-availability or strategy-fetch behavior.

## Constraints

- Public deprecated versions remain publicly rules-readable.
- Non-public versions remain inaccessible regardless of deprecation.
- New-game selection remains `is_public = true AND is_deprecated = false`.
- Existing `/rules/<version_id>` and email rules flows remain unchanged; no
  special route, authorization, or link is added.
- Preserve unrelated uncommitted remediation work, including R-51 F-196.

## Acceptance Criteria

- Public-deprecated rules and render metadata lookups succeed.
- Non-public versions return neither rules nor render metadata.
- An anonymous rules page renders authored rules for a public-deprecated
  version.
- New-game predicates retain the non-deprecated filter.
- Email callers continue through the shared lookup behavior.
- Static audit and permitted compile check pass.
- Database and SSR runtime tests are CI-only.

## Decisions

- Deprecation excludes a version from new-game creation, not from access to
  authored public rules.
- Visibility remains the access control boundary for shared rules lookups.

## Open Questions

- None.
