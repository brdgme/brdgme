# Raw findings: web domain - stats/

Scope: rust/web/src/stats/queries.rs, stats/mod.rs, stats/viz.rs (snapshot at /home/beefsack/Development/brdgme-review-snapshot). Cross-referenced web/src/db.rs for visibility helpers.

### Stats endpoints bypass game_visibility privacy settings
- severity: major
- category: correctness
- location: web/src/stats/mod.rs:174
- finding: All three server fns (get_player_profile mod.rs:174, get_player_game_type_stats mod.rs:231, get_player_history mod.rs:297) are anonymous-accessible and return per-game data with no visibility check: game ids, opponent identities (user_id + name + place via opponents_by_game queries.rs:206), active/unfinished games (active_games call mod.rs:206), and head_to_head aggregates naming every human opponent (queries.rs:476). The project has a game_visibility model ('public'/'friends'/private, db.rs is_game_visible_to_user at db.rs:2228) but none of the stats queries consult it. A user who sets game_visibility to friends-only or private still appears by name, user id and placement on every other participant's public profile, game-type page and history, and their shared games' ids are enumerable. Parallels W2's missing gate in get_game_details, but these are separate endpoints with their own leak surface.
- recommendation: Decide the intended semantics: either filter games whose participants' visibility excludes the viewer (extend the queries with the all-participants-visible predicate used by is_game_visible_to_user), or at minimum anonymize opponents whose game_visibility is not public (drop user_id, mask name) in opponents_by_game and head_to_head.

### Client-controlled page can overflow offset computation
- severity: minor
- category: correctness
- location: web/src/stats/mod.rs:318
- finding: `let offset = (page - 1) * page_size;` with `page: i64` taken directly from the client and only clamped by `page.max(1)`. `page = i64::MAX` overflows the multiplication: panic in debug builds, wrap to a negative offset in release, which Postgres rejects ("OFFSET must not be negative") and surfaces as a 500. Same unvalidated-client-input class as W2/W3.
- recommendation: Clamp page to a sane upper bound (e.g. `page.clamp(1, 1_000_000)`) or use checked_mul and treat overflow as page 1.

### Base rating 1200 hardcoded in rating_series reconstruction
- severity: minor
- category: correctness
- location: web/src/stats/queries.rs:183
- finding: `let mut rating = 1200;` reconstructs the rating series by summing rating_change from a hardcoded base. The same 1200 constant necessarily exists wherever ratings are initialized (game_type_users default, migrations 011/017 tested at queries.rs:1436 and queries.rs:1554). If the starting rating ever changes, or a player's rating was ever adjusted outside rating_change rows, the whole series (and its final point, which the profile implies equals current rating) silently drifts.
- recommendation: Pull the base into a shared `const INITIAL_RATING: i32` used by both rating logic and this reconstruction, or reconstruct from game_players.rating_before where available.

### get_player_game_type_stats computes stats for every game type to use one row
- severity: minor
- category: quality
- location: web/src/stats/mod.rs:256
- finding: The game-type page calls `game_type_stats(&pool, user.user_id, include_single_human)` (the full per-type aggregate over all of the user's game types, each row involving correlated player-count subqueries) and then `.find(|s| s.game_type_name == canonical)` to keep a single row, discarding the rest.
- recommendation: Add a game_type filter parameter to game_type_stats (`AND gt.name = $n`, nullable like finished_games' $3) and pass the canonical name.

### finished_games unbounded on game-type page
- severity: minor
- category: quality
- location: web/src/stats/mod.rs:274
- finding: get_player_game_type_stats calls `finished_games(..., None)` with `limit: None`, so `LIMIT $4::bigint` binds NULL (no limit) and the endpoint returns every finished game of the user for that type plus a full opponents map, serialized into one server-fn response. rating_series (mod.rs:271) and head_to_head (mod.rs:283) are likewise unbounded. For a long-lived account this is an unbounded payload on a public, anonymous endpoint.
- recommendation: Cap the game-type page list (e.g. Some(100)) and point to the paginated history page for the rest; consider capping rating_series by sampling or a LIMIT on most recent N.

### Single-human eligibility predicate duplicated across seven queries
- severity: minor
- category: simplicity
- location: web/src/stats/queries.rs:57
- finding: The correlated subquery `(SELECT count(*) FROM game_players ... WHERE ... user_id IS NOT NULL) >= CASE WHEN $n THEN 1 ELSE 2 END` is copy-pasted in overall_totals (57), game_type_stats (100), finished_games (267), game_history (398), game_history_count (463), head_to_head (493) and recent_form (571), with recent_form_for_game_type (queries.rs:648) hardcoding `>= 2` instead of taking the flag. A future change to the eligibility rule (e.g. counting replaced players) must be made in eight places, and the hardcoded variant already diverges in shape.
- recommendation: Factor into a SQL helper (inline SQL fragment constant, or a `game_human_counts` view / generated column) so the rule lives in one place; give recent_form_for_game_type the same parameterized form or a comment stating single-human is deliberately always excluded.

### game_history runs four correlated subqueries per row
- severity: minor
- category: quality
- location: web/src/stats/queries.rs:387
- finding: Each history row computes player_count plus match_min/match_max/match_avg as four separate correlated subqueries over game_players (lines 387-390), i.e. four extra scans of the same rows per game, 200 per 50-row page, on the hottest stats page.
- recommendation: Collapse into one `LEFT JOIN LATERAL (SELECT count(*), min(rating_before), max(rating_before), avg(rating_before)::int FROM game_players r WHERE r.game_id = g.id) agg ON true` (count unfiltered, min/max/avg ignore NULLs natively so the IS NOT NULL predicates are redundant anyway).

### game_type history filter is exact-match while everything else is case-insensitive
- severity: minor
- category: consistency
- location: web/src/stats/mod.rs:297
- finding: get_player_history passes the client-supplied `game_type` string straight through to `gt.name = $3` (queries.rs:397, 462) without canonicalizing via find_game_type_name, unlike get_player_game_type_stats which resolves case-insensitively (mod.rs:248). A history link built from a lowercased URL segment silently filters to zero rows instead of matching.
- recommendation: Resolve the filter through find_game_type_name first (returning None/empty or ignoring the filter when unknown), matching the game-type page behavior.

### Checked query macros and runtime query_as mixed without cause
- severity: nit
- category: consistency
- location: web/src/stats/queries.rs:211
- finding: opponents_by_game, game_history and game_history_count use runtime-checked `sqlx::query_as`/`query_as` with hand-written FromRow structs and `.bind(...)`, while every other query in the module uses the compile-time-checked `sqlx::query!` macro. The binds are all static, so these forfeit compile-time SQL checking for no benefit.
- recommendation: Convert to `sqlx::query!` (Option binds work fine with the `$n::type IS NULL OR ...` pattern already used) or note why runtime checking was needed.

### SVG viewBox literals duplicate the chart dimension constants
- severity: nit
- category: consistency
- location: web/src/stats/viz.rs:128
- finding: `viewBox="0 0 320 120"` is hardcoded in RatingChart (viz.rs:128) and Histogram (viz.rs:210) while all coordinate math uses CHART_WIDTH/CHART_HEIGHT/HIST_WIDTH/HIST_HEIGHT constants. Changing a constant silently clips or letterboxes the chart.
- recommendation: Build the viewBox string from the constants (`format!("0 0 {CHART_WIDTH} {CHART_HEIGHT}")`).

### finished_at DESC ordering puts NULLs first for legacy finished games
- severity: nit
- category: correctness
- location: web/src/stats/queries.rs:271
- finding: finished_games orders by `g.finished_at DESC, g.id`; finished_at is nullable (selected without `!`, DTO is Option). Postgres sorts NULLs first under DESC, so any legacy is_finished=true row with NULL finished_at pins to the top of "recent" lists and eats the LIMIT budget.
- recommendation: `ORDER BY g.finished_at DESC NULLS LAST, g.id` (recent_form's window ORDER BY at queries.rs:563 has the same property; harmless there but worth the same treatment).

## Checked and found CLEAN

- viz.rs numeric edge cases: sparkline empty/flat/NaN inputs cannot panic (index clamped, NaN maps to clamped 0); chart_coords guards len==1 and flat series before dividing; bar_heights guards max<=0. Unit tests cover the edges.
- Division by zero in SQL: avg_place_percentile divides by (n-1) only under `FILTER (WHERE ... n >= 2)` (queries.rs:111); win_percent guards zero games in Rust (queries.rs:68, 139).
- No unwrap/expect/panic in request paths; expects are confined to #[cfg(test)] fixtures and tests. Server fns propagate all errors through `internal(...)` context wrappers - no silent `.ok()` swallowing in this module.
- game_type_stats FULL OUTER JOIN correctly includes rating-only types and cannot produce a NULL game_type_name; test at queries.rs:1018 confirms no cross-user rating leak from game_type_users.
- rating_series ordering (`finished_at, g.id` tiebreak) and NULL rating_change exclusion are correct; reconstruction verified against game_type_users by test at queries.rs:1045.
- Tied-first-place semantics: place=1 counts as a win for all tied players consistently in overall_totals, game_type_stats and head_to_head (ties counted separately via place comparison).
- opponents_by_game: `IS DISTINCT FROM $2` correctly keeps bot seats (NULL user_id) while excluding the profile owner; single batched query per page, no N+1.
- History pagination: LIMIT/OFFSET and count query share identical predicates; page.max(1) prevents negative offset for ordinary inputs.
- SQL injection: all inputs are bound parameters; no string interpolation into SQL.
- Hand-rolled SVG/sparkline viz: appropriate here - tiny, tested, pure functions; pulling a charting crate would not pay for itself given the project's lean-dependency stance.

Severity tally: 0 critical, 1 major, 7 minor, 3 nit.
