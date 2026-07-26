# T3-B5: web domain - stats/query perf (WP-52) + domain misc server fns (WP-53)

- **Batch**: T3-B5 = WP-52 (stats and query performance pass, 13 findings) +
  WP-53 (domain misc server fns, 14 findings)
- **Crate**: `rust/web` (package `web`)
- **Sources**: `findings/web-domain.md`. There is **no** verification file for
  web-domain - it was lead-verified in place, so the raw file is authoritative.
- **Numbering**: web-domain (`wd Fnn`), taken as the ordinal position of the
  `###` finding headings in `findings/web-domain.md` (the file carries no
  inline ids). No offset hazard - there is only one numbering.
- **Rows**: WP-52: 13 (9 minor / 4 nit). WP-53: 12 (3 minor / 9 nit) - from a
  14-finding scope: `wd F56` is dropped as WP-41-owned and `wd F18` is
  escalated, both minors. **Batch total: 25 rows** (12 minor / 13 nit).
- No findings in WP-52 were rejected (review-wide rejections were `d F13` and
  `ws F30`, neither in scope). No WP-52 row is decision-blocked.
- No line numbers are cited anywhere in this checklist by design: verification
  found 33-46% of line citations in earlier review work were wrong.

> **Read the named function before editing; if it does not match the
> description, skip the row and report it - do not improvise.**

Rows are grouped by source file so one session sweeps a file at a time.

## WP-52 - `web/src/stats/queries.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F50` | `rust/web/src/stats/queries.rs` fns `overall_totals`, `game_type_stats`, `finished_games`, `game_history`, `game_history_count`, `head_to_head`, `recent_form`, `recent_form_for_game_type` | Extract the single-human eligibility correlated subquery into one `const` SQL fragment used by all eight sites, giving `recent_form_for_game_type` the same parameterized form (it currently hardcodes `>= 2`) or a comment saying the exclusion is deliberate | y |
| `wd F51` | `rust/web/src/stats/queries.rs` fn `game_history` | Collapse the four per-row correlated subqueries (player_count, match min/max/avg) into one `LEFT JOIN LATERAL (...) agg ON true` | y |
| `wd F47` | `rust/web/src/stats/queries.rs` fn `rating_series` | Replace the hardcoded `let mut rating = 1200;` base with a shared `const INITIAL_RATING: i32` also used wherever `game_type_users` ratings are initialized | n |
| `wd F55` | `rust/web/src/stats/queries.rs` fns `finished_games` and `recent_form` | Add `NULLS LAST` to the `finished_at DESC` ordering in both (`ORDER BY g.finished_at DESC NULLS LAST, g.id`) so legacy NULL-`finished_at` finished games stop pinning to the top | y |
| `wd F53` | `rust/web/src/stats/queries.rs` fns `opponents_by_game`, `game_history`, `game_history_count` | Convert the three runtime `sqlx::query_as` calls to the compile-time-checked `sqlx::query!` macro used by the rest of the module (all binds are static), or add a comment stating why runtime checking is needed | n |

## WP-52 - `web/src/stats/mod.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F48` | `rust/web/src/stats/mod.rs` fn `get_player_game_type_stats` + `rust/web/src/stats/queries.rs` fn `game_type_stats` | Add a nullable game-type-name filter parameter to `game_type_stats` (`AND gt.name = $n`, same shape as `finished_games`' nullable bind) and pass the canonical name instead of computing every type and `.find`ing one row | y |
| `wd F49` | `rust/web/src/stats/mod.rs` fn `get_player_game_type_stats` | Cap the game-type page payload - pass `Some(100)` to `finished_games` instead of `None` and bound `rating_series` / `head_to_head` too - since this is an anonymous endpoint | n |
| `wd F52` | `rust/web/src/stats/mod.rs` fn `get_player_history` | Resolve the client-supplied `game_type` through `find_game_type_name` before passing it down, matching `get_player_game_type_stats`' case-insensitive behaviour | y |

## WP-52 - `web/src/stats/mod.rs` + `web/src/players.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F46` | `rust/web/src/stats/mod.rs` fn `get_player_history` + `rust/web/src/players.rs` component fn `PlayerHistoryPage` | Clamp `page` with `page.clamp(1, 1_000_000)` (or `checked_mul` treated as page 1) before computing `offset`, and apply the same ceiling to the client-side page parse and the `page + 1` next-page link | y |

## WP-52 - `web/src/game/server_fns.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F21` | `rust/web/src/game/server_fns.rs` fn `get_game_details` (helper lives in `rust/web/src/db.rs` fn `should_hide_add_friend`) | Add a batched `should_hide_add_friend_many(pool, viewer, &[Uuid]) -> HashSet<Uuid>` using `= ANY($1)` and call it once instead of awaiting `should_hide_add_friend` per human opponent - a new fn, so it does not collide with WP-41's edits to the existing one | y |

## WP-52 - `web/src/friends.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F62` | `rust/web/src/friends.rs` fn `get_friends_overview` | `tokio::try_join!` the six independent queries (friends, incoming, outgoing, blocked, invite_policy, game_visibility) instead of awaiting them in sequence | n |

## WP-52 - `web/src/index.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F74` | `rust/web/src/index.rs` fn `get_logged_in_index` (helper `rust/web/src/db.rs` fn `friend_recent_visible_game`) | Bound the `list_friends` result and run the per-friend `friend_recent_visible_game` calls concurrently (e.g. `futures::future::try_join_all`) - the inner per-candidate N+1 is WP-41 Task 8's, so land WP-41 first and fix only the caller loop here | n |

## WP-52 - `web/src/game_info/mod.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F75` | `rust/web/src/game_info/mod.rs` fn `get_game_info` | Run the seven sequential awaits concurrently via `tokio::try_join!`, or merge the three count queries into one with `FILTER` clauses | n |

## WP-53 - `web/src/game/mod.rs` + `web/src/db.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F6` | `rust/web/src/game/mod.rs` fn `status_fields` (`Status::Finished` arm) + `rust/web/src/db.rs` fn `update_game_command_success` | Stop the finish path wiping elimination history - in `update_game_command_success`' `game_players` UPDATE, leave `is_eliminated` unchanged when the status is finished (`is_eliminated = CASE WHEN $is_finished THEN is_eliminated ELSE ... END`) rather than trying to carry the Active-arm list through `StatusUpdate` | y |

## WP-53 - `web/src/game/server_fns.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F19` | `rust/web/src/game/server_fns.rs` fn `restart_game_with_roster` (the `find_proposal_players` invite-mailer block) | Replace `if let Ok(players) = ...` with a `match` / `inspect_err` that `tracing::warn!`s the error, keeping the fire-and-forget send | n |
| `wd F20` | `rust/web/src/game/server_fns.rs` fns `get_game_logs` and `render_game_public` | `tracing::warn!` the game id and log id in the `brdgme_markup::from_string(...).unwrap_or_else(\|_\| (vec![], ""))` fallback in both sites, keeping the blank-line degradation | n |
| `wd F22` | `rust/web/src/game/server_fns.rs` fn `get_game_logs` | Compute `is_new` from `log.logged_at >= last_turn_at` to match the field the list sorts and displays on, or add a comment saying `created_at` is deliberate | n |
| `wd F23` | `rust/web/src/game/server_fns.rs` fn `generate_bot_name` | Add the standard `get_current_user` guard, or a one-line comment stating the endpoint is intentionally anonymous like `get_public_index` | n |
| `wd F24` | `rust/web/src/game/server_fns.rs` fn `restart_game_with_roster` | Replace the `filter(...is_some())` + `p.user_id.unwrap()` pair with a single `filter_map` yielding the unwrapped id | n |
| `wd F25` | `rust/web/src/game/server_fns.rs` fn `restart_core` | Duplicate the "You are not a player in this game" membership check inside `restart_core` (it already re-reads the game row under `FOR UPDATE`) instead of trusting its two callers | y |

## WP-53 - `web/src/stats/viz.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F54` | `rust/web/src/stats/viz.rs` components `RatingChart` and `Histogram` | Build both `viewBox` strings from `CHART_WIDTH`/`CHART_HEIGHT` and `HIST_WIDTH`/`HIST_HEIGHT` instead of the hardcoded `"0 0 320 120"` literals | n |

## WP-53 - `web/src/friends.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F61` | `rust/web/src/friends.rs` server fn `block_user` (DB writer `rust/web/src/db.rs` fn `block_user`) | Resolve the target user first and return "User not found" (the shape `send_friend_request` already uses) instead of letting the `blocks` FK violation surface as a generic internal error | y |

## WP-53 - `web/src/players.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F65` | `rust/web/src/players.rs` fn `encode_path_segment` (callers in `players.rs`, `friends.rs`, `new_game.rs`) | Keep the helper and its tests but delegate the body to `percent_encoding::utf8_percent_encode` - note the crate is only a transitive dependency today, so this needs an explicit `percent-encoding` entry in `rust/web/Cargo.toml`, otherwise skip the row | n |

## WP-53 - `web/src/settings.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F77` | `rust/web/src/settings.rs` module doc comment | Replace the stale "email placeholder" wording with a line describing the real add/confirm/make-active/remove `EmailSection` | n |

## WP-53 - `web/src/models/game.rs`

| Finding | File | Fix (one line) | Test? |
|---|---|---|---|
| `wd F78` | `rust/web/src/models/game.rs` struct `GameBot` (constructed field-by-field in `rust/web/src/db.rs`) | Add a one-line comment noting `GameBot` is a deliberate projection (no `created_at`/`updated_at`); deriving `FromRow` as the finding first suggests would require adding those columns and changing the db.rs construction site, which is more than a nit | n |

## Decision-blocked rows

None for WP-52. No entry in `planning/decisions-needed.md` blocks it -
`work-packages.md` marks WP-52 READY, and the only WP-52-adjacent decisions
(D-6 `game_visibility` scope, D-9 email canonicalization) gate WP-47/WP-49 and
WP-50 respectively, none of which own a WP-52 row. The parked rules-review
packages (WP-11, WP-12, WP-16, WP-20, WP-26, WP-30) own no findings in this
batch.

None for WP-53 either. D-41 ("Friends-page select revert after a rejected
change") is tagged *informs WP-54, WP-53* only because both packages touch
`friends.rs`: D-41 is entirely about the `FriendsPage` component's
`<select>` binding for invite-policy / game-visibility, and WP-53's only
`friends.rs` row (`wd F61`) is in the `#[server]` region. No WP-53 row waits
on D-41 or on any other open decision.

## Not in this checklist (owned elsewhere)

- `wd F56` (concurrent opposite-direction friend requests hit
  `friends_pair_key` instead of auto-accepting, `db.rs::send_friend_request`) -
  owned by `specs/WP-41-db-quality-pass.md`, whose stated goal is to
  "serialize `send_friend_request`'s read-then-insert so opposite-direction
  requests auto-accept instead of surfacing a raw 23505" under its own id
  `ws F39`. Same defect, same function; **dropped from WP-53's table.**
- `specs/WP-54-frontend-ux-error-handling.md`'s coordination section mislabels
  two of WP-53's ids: it calls `wd F56` the `block_user` target-existence nit
  (that is `wd F61`, kept above) and `wd F65` the `get_friends_overview`
  sequential-query nit (that is `wd F62`, WP-52's). The line ranges it fences
  off are still correct - WP-54 owns `friends.rs`' `FriendsPage` and the
  `settings.rs` component bodies, WP-53 owns only `friends.rs`' `#[server]`
  region and `settings.rs`' module doc, so `wd F61` and `wd F77` do not
  collide with it.
- `wd F45` (stats endpoints bypass `game_visibility`) - owned by
  `specs/WP-47-game-visibility-gates.md`, which rewrites `finished_games`,
  `active_games` and `game_history` to take `visible_user_ids`. `wd F49`,
  `wd F50`, `wd F51` and `wd F55` above touch those same queries, so land
  WP-47 first and rebase these rows onto its SQL.
- `wd F46` is **not** WP-47's despite being mentioned in its Non-goals section -
  that section explicitly excludes it, so it stays here.
- `ws F40` (the per-candidate visibility N+1 *inside*
  `db.rs::friend_recent_visible_game`) - owned by
  `specs/WP-41-db-quality-pass.md` Task 8, which also states that the
  caller-side amplification in `index.rs` (`wd F74`) belongs to WP-52.
- `wd F54` (SVG `viewBox` literals duplicate the chart dimension constants,
  `stats/viz.rs`) - in WP-53's scope, not WP-52's, despite living under
  `stats/`.
- `specs/WP-54-frontend-ux-error-handling.md` cites "wd F62" as WP-50-owned
  email-canonicalization work; that is a mislabel (WP-50's scope is
  `ws F9`, `wd F37`, `wd F60`, `wd F72`). `wd F62` is the `get_friends_overview`
  sequential-query nit above and is correctly WP-52's - but WP-54 Task 9 still
  must not touch the `new_game.rs` email arm.

## Escalate

**WP-53: `wd F18`** (minor) - "Game-service HTTP call made inside an open
transaction holding a `FOR UPDATE` lock", `game/server_fns.rs::restart_core`.
The finding's fix ("call the game service before `pool.begin()`") cannot be
done in one line: `create_game_from_service` takes `&mut tx` and performs the
HTTP request *and* all of the new game's inserts in one body, so pulling the
request out means splitting that helper into a fetch half and an insert half
and updating every caller (`restart_core`, the proposal path, `new_game`, the
email command path). It also lands in the same function WP-40 (lock/guard
discipline) and WP-45 (bot-slot validation before `create_game_from_service`)
both edit. Tier 2 work - sequence it after those two.

Kept as a row despite touching contested code: **`wd F6`** edits
`db.rs::update_game_command_success`, which WP-40 (sticky `is_finished`) and
WP-41 (`updated_at` cleanup, `left_at` CASE) also modify. The one-line
`is_eliminated` CASE is independent of both, but land WP-40 and WP-41 first
and re-read the UPDATE before editing. The finding's first suggestion (carry
the Active-arm eliminated list through `StatusUpdate`) is the worse option -
it changes a shared struct for a data-preservation bug that the SQL guard
fixes locally, so the row takes the second suggestion.

Two of WP-53's recommendations were also narrowed rather than taken as
written: `wd F78`'s "derive `FromRow`" would force new columns onto `GameBot`
and a db.rs rewrite (row takes the comment half), and `wd F65`'s "the crate is
already in the dependency tree" is true only transitively - `percent-encoding`
appears in no `Cargo.toml` in `rust/`, so the row calls out the needed direct
dependency.

WP-52: none. All 13 WP-52 fixes compress to one line. Two carry sequencing debt rather
than scope creep: `wd F74` depends on WP-41 Task 8, and the four
`stats/queries.rs` rows touching `finished_games` / `game_history` depend on
WP-47's signature change. The findings' own recommendations were usable as
written for all 13; `wd F74`'s "fold visibility into one SQL query" suggestion
is the Tier 2 version and is superseded by WP-41 Task 8, so only the
concurrency half is taken here.
