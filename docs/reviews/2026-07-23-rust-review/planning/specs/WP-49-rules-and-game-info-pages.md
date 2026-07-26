# WP-49: rules and game-info pages

**Findings:** wd F67 (major), F68-F71 (minor), F76/F79/F80 (nit), plus a
routed-in error-surfacing item on `rules.rs`.
**Decision:** D-6 answered option A - **rules pages stay public**.
**Landing order: WP-41 must land before WP-49** (both touch `rust/web/src/db.rs`).
Do NOT attempt the `db.rs` module split (ws F42) or touch any game's `RULES.md`
(parked under BLOCKED-ON-USER-RULES-REVIEW).

> **Read the named function before editing. If it does not match what this spec
> describes, STOP and report rather than improvising.** This code is under
> concurrent edit; line numbers are deliberately omitted.

## 1. Problem

- **wd F67 (major)** - `game_info_rules_version_id`
  (`rust/web/src/game_info/queries.rs`) picks the rules version with
  `ORDER BY name LIMIT 1`.
- **Routed-in** - `RulesPage` (`rust/web/src/rules.rs`) renders the server error
  verbatim: the `Some(Err(e))` arm interpolates `{e.to_string()}`.
- Riders F68-F80: see the table in section 6.

## 2. Why it's wrong

- **wd F67 is correct as written.** Verified live. Version names are semver-like
  strings, so ascending name order returns the OLDEST version, and lexicographic
  order is wrong anyway ("10.0.0" < "2.0.0"). The project convention is
  `find_latest_non_deprecated_game_version` (`db.rs`): `ORDER BY created_at DESC`.
- **Routed-in item, as it actually exists in live code:** this is *not* an
  error-swallow. `get_rendered_rules` already routes DB/service failures through
  `crate::error::internal` (logs, returns opaque "Internal server error"). What
  leaks is everything that does *not*: `RenderError` strings (authoring detail),
  "Game version not found", "Not authenticated" - painted raw into the page. The
  right helper already exists (`crate::error::user_facing_server_error`, used by
  `rust/web/src/new_game.rs`); `rules.rs` does not use it.
- **wd F69 is correct as written.** Verified live: `find_game_version_rules` and
  `find_game_version_render_meta` (`db.rs`) both select by id with no
  `is_public` / `is_deprecated` filter.
- **wd F76 is correct as written.** Verified: no `use crate::game_info::*`
  anywhere in `rust/web/src/`; every call site uses the `queries::` path.
- No finding in this package is rejected.

## 3. Required end state

1. `rust/web/src/game_info/queries.rs` -> `game_info_rules_version_id`: replace
   `ORDER BY name` with `ORDER BY created_at DESC`. Keep the existing
   `is_public = true AND is_deprecated = false` predicate and `LIMIT 1`.
2. `rust/web/src/rules.rs` -> `get_rendered_rules`: **remove the auth gate** -
   delete the `get_current_user()` call, its `ok_or_else(... "Not authenticated")`
   and the now-unused import. D-6 makes rules public; nothing in the fn is
   user-specific.
3. `rust/web/src/db.rs` -> `find_game_version_rules` **and**
   `find_game_version_render_meta`: add `AND is_public = true AND
   is_deprecated = false` to each `WHERE`. Both are plain (non-macro) queries -
   no `.sqlx` regeneration. **Then re-read WP-41's `game_and_version_lookups`
   db test and fix any fixture now expecting a non-public/deprecated version to
   return `Some`.** If that test is ambiguous, STOP and report.
4. `rust/web/src/rules.rs` -> `render_doc`: after the line loop, if `in_fence`
   is still true, return a new `RenderError::UnterminatedFence` variant (message
   `"unterminated brdgme fence"`, added to the enum in the same file) instead of
   dropping the buffer.
5. `rust/web/src/rules.rs` -> `get_rendered_rules`: replace
   `fetch_strategy(...).await.map_err(internal(...))?` with a match that on
   `Err` logs via `tracing::error!` and continues with `(None, None)`, so the
   DB-sourced rules section still renders.
6. `rust/web/src/rules.rs` -> `RulesPage`: switch `docs` from `LocalResource` to
   `Resource::new_blocking` and wrap in `<Suspense>`, mirroring
   `rust/web/src/game_info/mod.rs` -> `GameInfoPage`. Error arm renders
   `{crate::error::user_facing_server_error(&e)}`, not `{e.to_string()}`.
7. `rust/web/src/game_info/mod.rs`: delete `pub use queries::*;` and its
   `#[cfg(feature = "ssr")]` attribute. Keep `mod queries;`.
8. `rust/web/src/rules.rs` -> `render_markdown`: extend the doc comment to state
   the trust boundary - raw HTML passes straight through into `inner_html`;
   sources are trusted authored content (the `rules` DB column populated at
   deploy, and game-side `include_str!`) and must never be user-supplied
   markdown. Comment only; no behaviour change.

## 4. Non-goals

- No caching or concurrency for `fetch_strategy` (F71 mentions both; only the
  graceful-degrade half is in scope). No HTML sanitizer / `Event::Html`
  escaping filter (F79 is a comment only).
- No change to `GameInfoPage`'s own raw `{e.to_string()}` arm; routed-in item is
  `rules.rs` only.
- No `db.rs` module split, no other `db.rs` functions, no game `RULES.md` edits,
  no visibility gating of game-info stats (WP-47 owns that).

## 5. Regression test cases

In-file `mod tests` in `rust/web/src/rules.rs` (exists, ~14 tests) - add
`unterminated_fence_errors_loudly`: `render_doc("```brdgme\n{{player 0}}")`
returns `Err(RenderError::UnterminatedFence)`; existing fence tests keep passing.

In-file `mod tests` in `rust/web/src/game_info/queries.rs` (exists, `#[sqlx::test]`
plus a `make_game_type` helper) - add:
- `rules_version_id_picks_newest_created_at`: two public non-deprecated versions,
  `'10.0.0'` with the older `created_at` and `'2.0.0'` newer; the fn must return
  `'2.0.0'`. Fails under `ORDER BY name`, passes after the fix.
- `rules_version_id_skips_non_public_and_deprecated`: `is_public = false` is
  never returned.

In `rust/web/tests/ssr_pages.rs` (helpers `make_game_version_for_type`,
`assert_clean_html_body`, `get` already exist) - add:
- `rules_page_anonymous_renders_rules`: GET `/rules/{version_id}` with no cookie
  returns 200 with a clean body containing the rendered rules heading - proving
  the auth gate is gone and SSR (not a spinner) produced it.
- `rules_page_renders_when_strategy_fetch_fails`: game version `uri` points at a
  dead address; page still returns 200 with the rules section present.

## 6. Riders

| finding | file | one-line fix | test needed |
|---|---|---|---|
| wd F68 | `rust/web/src/rules.rs` (`get_rendered_rules`) | Delete the `get_current_user` auth gate and its import - D-6 makes rules public. | Y (ssr_pages anonymous GET) |
| wd F69 | `rust/web/src/db.rs` (`find_game_version_rules`, `find_game_version_render_meta`) | Add `AND is_public = true AND is_deprecated = false` to both `WHERE` clauses. | Y (WP-41 db test update) |
| wd F70 | `rust/web/src/rules.rs` (`render_doc`, `RenderError`) | Add `UnterminatedFence` variant; return it when `in_fence` is true after the loop. | Y (in-file unit test) |
| wd F71 | `rust/web/src/rules.rs` (`get_rendered_rules`) | On `fetch_strategy` error, log and continue with `(None, None)` instead of `?`. | Y (ssr_pages dead-uri test) |
| wd F76 | `rust/web/src/game_info/mod.rs` | Delete the unused `pub use queries::*;` re-export. | N (compile-only) |
| wd F79 | `rust/web/src/rules.rs` (`render_markdown`) | Doc-comment the raw-HTML trust boundary; no behaviour change. | N |
| wd F80 | `rust/web/src/rules.rs` (`RulesPage`) | `LocalResource` -> `Resource::new_blocking` + `<Suspense>`, mirroring `GameInfoPage`. | Y (covered by the anonymous SSR test) |
| routed-in | `rust/web/src/rules.rs` (`RulesPage`) | Error arm renders `crate::error::user_facing_server_error(&e)`, not `e.to_string()`. | N |
