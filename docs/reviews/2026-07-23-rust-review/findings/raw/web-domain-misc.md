# Raw findings: web domain - game_info, models, rules, settings, index

Scope: web/src/game_info/queries.rs, web/src/game_info/mod.rs, web/src/models/{game,user,mod}.rs, web/src/rules.rs, web/src/settings.rs, web/src/index.rs (snapshot /home/beefsack/Development/brdgme-review-snapshot). Cross-referenced web/src/db.rs and web/src/auth/server.rs read-only.

### Rules-page version picked by ORDER BY name, not latest

- severity: major
- category: correctness
- location: web/src/game_info/queries.rs:18
- finding: `game_info_rules_version_id` picks the linked rules version with `ORDER BY name LIMIT 1` over public non-deprecated versions. Version names are semver-like strings ("1.0.0"), so ascending name order returns the OLDEST version, and lexicographic ordering is wrong anyway ("10.0.0" sorts before "2.0.0"). The project convention for "current version" is `ORDER BY created_at DESC` (db.rs:219-231 `find_latest_non_deprecated_game_version`). The game info page therefore links to the rules of the oldest public version once a second version exists.
- recommendation: Use `ORDER BY created_at DESC LIMIT 1` to match `find_latest_non_deprecated_game_version`, or reuse that fn and filter is_public.

### Anonymous game-info page links to auth-gated rules endpoint

- severity: minor
- category: consistency
- location: web/src/rules.rs:308-310 (and web/src/game_info/mod.rs:180-182)
- finding: `get_game_info` is intentionally anonymous and renders a "Rules & strategy" link, but `get_rendered_rules` rejects anonymous callers with "Not authenticated" (rules.rs:308-310). A logged-out visitor browsing /game-info clicks through to a bare error page. Rules content is less sensitive than the ratings the info page already exposes, so the gate is inconsistent both ways. Parallel to W4's note that stats endpoints are anonymous.
- recommendation: Decide the public-content posture once: either drop the auth requirement on get_rendered_rules (it exposes nothing user-specific) or hide the link for anonymous visitors.

### get_rendered_rules ignores is_public/is_deprecated on the version

- severity: minor
- category: correctness
- location: web/src/rules.rs:312-321 (queries at db.rs:258-279)
- finding: `find_game_version_rules` and `find_game_version_render_meta` select by id with no `is_public = true` filter, so any authenticated user who obtains or guesses a version UUID can render rules and trigger live strategy fetches for non-public (unreleased) game versions. Every other listing path filters `is_public = true AND is_deprecated = false` (db.rs:300).
- recommendation: Add `AND is_public = true` to the two lookups (or check the flag in get_rendered_rules and return "not found").

### Unterminated brdgme fence silently dropped by render_doc

- severity: minor
- category: correctness
- location: web/src/rules.rs:201-204
- finding: `render_doc` line-scans for fences; if a doc ends while `in_fence` is still true (author forgot the closing ```` ``` ````), the accumulated `fence` buffer is discarded without error - the tail of the document silently vanishes. This contradicts the module's stated "fail loudly on authoring errors" policy (rules.rs:86, 163).
- recommendation: After the loop, if `in_fence` is true return a new `RenderError::UnterminatedFence` (or render the remainder as prose) instead of dropping it.

### Two sequential live HTTP strategy fetches per rules page view, no caching

- severity: minor
- category: quality
- location: web/src/rules.rs:261-297
- finding: `fetch_strategy` makes two blocking round trips to the game service (BasicStrategy then AdvancedStrategy) on every /rules page load, sequentially, and any failure fails the whole page including the DB-sourced rules section (rules.rs:333-335). The strategy content is static `include_str!` data on the game side.
- recommendation: At minimum degrade gracefully (render rules, omit strategy on fetch error). Consider caching per (uri, name, interface_version) since content is immutable per version, and issuing the two requests concurrently.

### Email address not trimmed/normalized before add (settings path)

- severity: minor
- category: correctness
- location: web/src/settings.rs:341 (server side auth/server.rs:789-800)
- finding: The add-email form dispatches `el.value()` raw; `add_email_address` does no trim or lowercase before `find_email_owner`/`insert_unverified_email` (only the domain is lowercased for the blocklist check at auth/server.rs:800). "User@x.com " (case variant or stray whitespace) passes the `contains('@')` check and inserts a distinct row against the case-sensitive UNIQUE on user_emails, and later confirmation-code lookups are exact-match on the stored string. Same root cause as W3's login-path finding; this is the settings-page instance.
- recommendation: Trim and lowercase once at the server-fn boundary before ownership check, insert, and confirmation; optionally trim client-side too.

### Fire-and-forget settings mutations swallow server errors

- severity: minor
- category: quality
- location: web/src/settings.rs:145-148 (colors), web/src/settings.rs:210-237 (three email-pref toggles), web/src/settings.rs:496-498 (theme sync)
- finding: ColorsSection, EmailPreferencesSection, and ThemeSection dispatch ServerActions and never observe `action.value()`. The UI optimistically updates local signals first (e.g. `turn.set(val)` at settings.rs:212-213 before dispatch), so if the server call fails (session expired, transport error) the page shows a saved state that was never persisted, with no feedback and no revert. Parallel to W5's silent error swallowing in UI actions.
- recommendation: Watch each action's value and on Err revert the local signal and surface a small error message, mirroring UsernameSection's pattern (settings.rs:62-69).

### Index page issues O(friends x scan_limit) sequential queries

- severity: minor
- category: quality
- location: web/src/index.rs:51-61 (helpers db.rs:2012-2024, db.rs:2316-2342)
- finding: `get_logged_in_index` loops over `list_friends` (unbounded) and awaits `friend_recent_visible_game` per friend; that helper fetches up to 10 candidate games and calls `is_game_visible_to_user` per candidate, each its own query. The logged-in landing page can therefore run 1 + F x (1 + up to 10) sequential DB round trips and grows linearly with friend count.
- recommendation: Fold visibility into one SQL query (LATERAL join per friend with the visibility predicate inlined), or at least bound the friend list and run the per-friend lookups concurrently (join_all).

### game_info server fn runs six sequential queries

- severity: nit
- category: quality
- location: web/src/game_info/mod.rs:39-66
- finding: header, rules_version_id, total_games, active_today, distinct_players, top_ranking, and form are seven awaits in sequence; the three count queries each re-join games->game_versions independently.
- recommendation: Fine at current scale; if it shows up in latency, merge the three counts into one query with FILTER clauses or run them concurrently.

### Redundant glob re-export of ssr queries

- severity: nit
- category: simplicity
- location: web/src/game_info/mod.rs:31-32
- finding: `pub use queries::*;` re-exports the six query fns at `crate::game_info::*`, but every call site (including this module, mod.rs:39-59) uses the `queries::` path and nothing else imports them via the glob.
- recommendation: Drop the re-export, or drop the `queries::` prefixes; not both.

### Stale module doc: "email placeholder"

- severity: nit
- category: quality
- location: web/src/settings.rs:1-2
- finding: The module doc still says "email placeholder" but EmailSection is a full add/confirm/make-active/remove implementation.
- recommendation: Update the doc comment.

### GameBot lacks FromRow unlike sibling models

- severity: nit
- category: consistency
- location: web/src/models/game.rs:42-48
- finding: Every model struct in the file derives `FromRow` except `GameBot`, which is instead constructed field-by-field in db.rs:43-47. `GameBot` also omits created_at/updated_at while the other structs carry them.
- recommendation: Derive FromRow and align fields, or leave as-is with a one-line comment noting it is a projection, whichever matches actual query usage.

### Rules markdown allows raw HTML pass-through into inner_html

- severity: nit
- category: correctness
- location: web/src/rules.rs:150-157 (rendered at rules.rs:52-64)
- finding: pulldown-cmark passes raw inline HTML through by default and the result is injected via `inner_html`. Rules/strategy sources are trusted authored content (DB rules column populated at deploy, include_str! on the game side), so this is not currently exploitable, but nothing documents that trust boundary, and the same render_doc would become an XSS sink if ever fed user-supplied markdown.
- recommendation: Add a comment stating the trust assumption, or pass the parser events through a filter that escapes `Event::Html`/`Event::InlineHtml`.

### Mixed resource strategies between the two content pages

- severity: nit
- category: consistency
- location: web/src/rules.rs:32-33 (vs web/src/game_info/mod.rs:100)
- finding: GameInfoPage uses `Resource::new_blocking` (SSR-rendered), RulesPage uses `LocalResource` (client-only, spinner on first paint) for similar content-page loads. The rules page is the more content-heavy of the two and gets no SSR.
- recommendation: If the auth gate on get_rendered_rules stays, LocalResource is forced; if it goes (see finding above), switch RulesPage to a blocking Resource for parity.

## Checked and found CLEAN

- game_info/queries.rs SQL: joins, finished/user_id filters, and DISTINCT semantics all correct; count queries verified against the tests; no injection surface (all bound params); LIMITs present on ranking.
- game_info/queries.rs tests: helper setup (trigger disable for backdating, bot vs user players) is sound and covers finished-only, today-window, distinct-humans, and top-10 ordering.
- game_info/mod.rs: no unwrap/expect in the request path; header lookup is case-insensitive by design; form lookup batches user_ids in one call (no N+1); Ok(None) for unknown game type renders a proper not-found view.
- rules.rs validate_player_indices: recursion covers every Node variant including table cells, canvas layers, and Fg/Bg colour refs; children of Fg/Bg are recursed after the colour check; tests exercise both token and colour out-of-range paths.
- rules.rs synthetic_players: max/palette clamp is correct for empty and oversized counts; slice bound `max.min(palette_len)` prevents panic.
- settings.rs authz: every mutation goes through server fns that individually require get_current_user and scope writes by user.id (verified set_username, set_pref_colors, set_theme, the three email-pref toggles, and all four email-address fns in auth/server.rs); no client-trusted user id anywhere.
- settings.rs set_pref_colors server validation: exactly-3, palette-membership, and pairwise-distinct checks match the UI's swap invariant; set_theme validates the slug against known themes.
- settings.rs email actions correctly scope by owner: make-active rejects unverified and foreign addresses, remove rejects primary and foreign, confirm requires both a valid code and a pending row for THIS user before deleting the confirmation.
- settings.rs UsernameSection: distinguishes field errors (Ok(Some)) from transport errors, disables submit while pending, shows saved state.
- index.rs authz and visibility: require_user gates the endpoint; friend recent games go through is_game_visible_to_user per candidate; history/stats/form are all scoped to the caller's own user id; history LIMIT 10.
- models/*.rs: plain data structs, no logic; serialized model exposure checked - GamePlayer (with undo_game_state) is only serialized inside server-side flows and the export path, not handed raw to anonymous clients.
- No panics/unwraps in any reviewed request path (unwraps are confined to test modules).

Severity tally: 0 critical, 1 major, 7 minor, 6 nit.
