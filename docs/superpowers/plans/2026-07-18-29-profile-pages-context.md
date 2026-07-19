# 29: Player Profile Pages - Context Handover

> Written 2026-07-18 by a Lead session for later Lead/Worker sessions
> implementing #29 (player profile/stats pages). Investigation snapshot; verify
> line numbers before editing, files drift.
>
> Spec: `docs/superpowers/specs/2026-07-08-29-stats-reports-design.md`
> Plan: `docs/superpowers/plans/2026-07-08-29-stats-reports.md`
> Must-read before coding: `docs/CODING.md`, `docs/hydration.md`, `docs/DEV.md`

## Codebase map (rust/web)

### Routing

- Routes registered in `rust/web/src/app.rs:192-202` inside `App()`:
  `<Route path=(StaticSegment("games"), ParamSegment("id")) view=GamePage/>` is
  the param-route pattern to copy. New routes:
  `(StaticSegment("players"), ParamSegment("name"))` and
  `(StaticSegment("players"), ParamSegment("name"), ParamSegment("game_type"))`.
- Imports at `app.rs:4-8`: `leptos_router::{ParamSegment, StaticSegment,
  components::{A, Route, Router, Routes}, hooks::use_navigate}`.
- Axum-side router: `build_router` in `rust/web/src/router.rs:102-183`. Leptos
  routes need no registration there; only non-Leptos endpoints (`/ws`,
  `/admin/...`) are added manually.

### Page/component layout

- No `pages/` dir. Pages are either inline in `app.rs` (Home, Games, Dashboard,
  GamePage at `app.rs:871`) or a single top-level file bundling server fns +
  page + subcomponents (`settings.rs`, `friends.rs`). For #29 create
  `rust/web/src/players.rs` (or `players/` module) following `friends.rs`.
- Shared UI in `rust/web/src/components/`: `layout.rs` (`MainLayout`,
  `SidebarMenu`), `game.rs` (`GameMeta`, `PlayerInfo`, `PlayerName`),
  `form.rs` (`FormField`), `spinner.rs`.
- Page skeleton example: `SettingsPage` at `rust/web/src/settings.rs:11-42` -
  `<MainLayout>` wrapping `<div class="settings content-page">`, data via
  `LocalResource`, subsections as components taking the resource as a prop.

### Server fns and view types

- Pattern: `#[server(TypeName, "/api")]` on an async fn; body gets
  `let pool = expect_context::<PgPool>();` and auth via `require_user()` /
  `get_current_user()`. Example: `send_friend_request` at
  `rust/web/src/friends.rs:118-153`. Context (PgPool etc.) provided per-request
  in `router.rs:107-123`.
- WASM-safe view types are plain ungated `#[derive(Debug, Clone, Serialize,
  Deserialize)]` structs next to their server fns - see
  `rust/web/src/game/server_fns.rs:1-93` (`GameViewData`, `PlayerViewData`,
  `GameSummary`...) and `friends.rs:18-77`. Only DB-touching helper fns carry
  `#[cfg(feature = "ssr")]`. Copy this: define `PlayerProfileData`,
  `GameTypeStats`, etc. as ungated structs in the new module.
- Expected rejections are data, not errors: return
  `Ok(None)`/`Ok(Some(msg))` style rather than `ServerFnError` for
  user-visible conditions (CODING.md "Server Functions"). For profile pages,
  "no such user" should be a renderable state, not an Err.

### Data fetching / hydration (the big gotcha)

- Every existing resource in the app is `LocalResource` (sidebar active games
  at `app.rs:124-130`, current_user, settings, logs). There is currently NO
  `Resource::new_blocking` usage anywhere in `rust/web/src`.
- Spec D2 requires stats/charts to be SSR (in initial HTML). That means
  `Resource::new_blocking` + the exact pattern in `docs/CODING.md` "Leptos: SSR
  and Hydration": layout wrapper (`MainLayout`) OUTSIDE `Suspense`,
  data-dependent content INSIDE, resource read bound unconditionally first
  inside the Suspense closure. Read `docs/hydration.md` in full before writing
  the page - hydration panics only manifest on hard refresh and killed this
  team twice already (commits c7e63d1, 63fef22).
- If SSR of the charts proves painful, the sanctioned fallback is
  `LocalResource` (never mismatches) at the cost of charts not being in initial
  HTML - a deviation from D2 to flag to the Orchestrator, not decide silently.
- Structural mismatches panic; attribute/class differences do not. Never
  branch element type on async data; toggle classes/hidden instead.
- Anonymous viewers are allowed (profiles are public) - no auth redirect
  needed, unlike `SettingsPage`.

### Frontend surfaces to touch

- Player names today are plain styled text, never links:
  - `PlayerName` component: `rust/web/src/components/game.rs:403-408`
    (`<strong style="color:var(--mk-{color})">"<"{name}">"</strong>`).
  - Game meta panel rows: `PlayerInfo` at `components/game.rs:206-210`.
  - Sidebar opponents: `components/layout.rs:182-184` (whole game row is the
    `<A>`; per-name links inside would nest anchors - restructure or skip).
- Linking names to profiles: `PlayerViewData.user_id: Option<Uuid>`
  (`game/server_fns.rs:71`) and `GameViewData.viewer_user_id: Option<Uuid>`
  (`game/server_fns.rs:54`) exist. But profile URLs are name-based and
  `PlayerViewData` already has `name` - link via name, no extra plumbing.
- Internal links: `leptos_router::components::A`, e.g.
  `<A href=format!("/games/{}", id)>` at `components/layout.rs:178-186`.
- Add-friend button pattern to copy onto profile pages: `PlayerInfo` at
  `components/game.rs:217-238` - creates
  `ServerAction::<crate::friends::SendFriendRequest>::new()` and dispatches
  `SendFriendRequest { user_id: Some(uid), name: None }`; hidden for self and
  anonymous viewers via `.filter(|uid| viewer_user_id.is_some() && Some(*uid)
  != viewer_user_id)`.

### CSS

- `rust/web/style/main.scss` (~625 lines). Colors are runtime CSS vars
  (`--mk-*`) generated by `rust/web/src/theme.rs`; palette slots
  (`PLAYER_COLOR_NAMES`, `theme.rs:66-68`): Green, Red, Blue, Orange, Purple,
  Brown, Cyan, Pink, each with `-contrast` variants.
- Win/loss colors already exist but scoped: `.game-meta .rating-change-up/
  -down/-none` at `main.scss:445-459` (green/red/blue). For form strips add
  page-scoped rules referencing `var(--mk-green)`/`var(--mk-red)` directly.
- `.content-page` (`main.scss:538-542`) is the standard centered content
  wrapper - use it for both profile routes. Site is fully monospace, so
  Unicode sparklines render native (spec D2 tier 2).
- No existing `/players` route, profile page, stats module, sparkline, or SVG
  chart code anywhere in `rust/web/src` - greenfield.

## Schema facts

Migrations in `rust/web/migrations/` (base `001_initial_schema.sql`):

- `users` (001:81-89 + later): `id uuid PK`, `name text NOT NULL` (unique
  case-insensitively via `users_name_lower_key ON users (lower(name))`,
  `009_username_rules.sql:41`; format `^[a-zA-Z0-9_-]{1,16}$` app-enforced),
  `pref_colors text[] NOT NULL`, `created_at` (= "member since"), `theme`,
  `is_admin`, `invite_policy`. No bot flag - bots are not users.
- `games` (001:171-181): `game_version_id uuid NOT NULL` (NOT game_type_id),
  `is_finished boolean NOT NULL`, `finished_at timestamp` nullable (auto-set
  by trigger when is_finished flips true, 001:448-452), `restarted_game_id`.
  Reminder: `updated_at` is trigger-maintained on every UPDATE (CODING.md).
- `game_players` (001:183-201 + 003): `game_id`, `user_id uuid` NULLABLE,
  `game_bot_id uuid` nullable (FK `game_bots`, added `003_game_bots.sql`),
  CHECK exactly one of user_id/game_bot_id set (`game_players_user_or_bot`),
  `"position" integer NOT NULL` (quote it in SQL), `color text NOT NULL`,
  `is_eliminated boolean NOT NULL`, `points real` nullable, `place integer`
  nullable, `rating_change integer` nullable. Unique `(game_id, user_id)`,
  `(game_id, color)`, `(game_id, position)`.
- `game_bots` (003): `game_id, name, difficulty, personality`; unique
  `(game_id, name)`.
- `game_type_users` (001:149-158): `game_type_id, user_id` (unique pair),
  `rating integer DEFAULT 1200 NOT NULL`, `peak_rating integer DEFAULT 1200
  NOT NULL`, `last_game_finished_at` nullable.
- Game types: `game_types` (001:140-147) has the human-readable unique `name`
  (`002`), `player_counts integer[]`, `weight`. `games -> game_versions ->
  game_types`; `game_versions.uri text NOT NULL` is the URL-safe identifier
  used to reach game services (`game/mod.rs:115`, `server_fns.rs:148`). Every
  per-game-type query must join through game_versions.

### Rating logic (already implemented, do not duplicate)

- `apply_rating_changes(tx, game_id)` at `rust/web/src/db.rs:1269-~1417`;
  ELO helpers `ELO_K = 32.0`, `elo_rating_change` at `db.rs:1244-1262`.
- Bot rule: any player with `game_bot_id` set => whole game unrated, no-op
  (`db.rs:1291-1294`). Idempotency: no-op if any rating_change already set.
- Initial rating 1200 comes from the column default via upsert
  (`db.rs:1323-1329`). `peak_rating` updated as `GREATEST(peak_rating,
  rating + change)` (`db.rs:1376-1387`) - but only since #12, so historical
  peaks are wrong (still 1200 or too low); spec mandates a one-off backfill
  from the reconstructed series.
- Rating series reconstruction (spec): `1200 + cumulative sum of
  rating_change ORDER BY games.finished_at` per user per game type; sanity
  check final value == `game_type_users.rating`.

### Colour preferences (#35/#11)

- Stored as `users.pref_colors text[]` (ordered by preference). Read via
  `get_user_pref_colors` at `db.rs:2051-2059` (plain `sqlx::query_as`, maps
  through `normalize_pref_color` at `db.rs:646-652` - legacy "Amber"->
  "Orange", "BlueGrey"->"Cyan"). Always normalize when reading raw
  pref_colors in a new query.
- Per-game assigned colour is `game_players.color` (what `PlayerName` renders
  today, via `choose_colors` at `db.rs:696+`).

## db.rs and sqlx workflow

- `db.rs` is ~4000 lines, feature-grouped sections with banner comments
  (e.g. `// --- #30 friends ---` at `db.rs:1530`), tests from `db.rs:2071`.
  Plan says stats queries go in `db.rs` or a new stats module - given the
  size, prefer a new `rust/web/src/stats/` or section with a `// --- #29
  stats ---` banner; either is consistent.
- Two query styles: `query_as!` macros (compile-checked, need `.sqlx` cache
  regen - e.g. `find_game_version` `db.rs:203-211`) vs plain
  `sqlx::query`/`query_as` (no cache - e.g. `get_user_pref_colors`).
  CODING.md's plain-query convention targets "column not already covered by
  an existing macro query"; all #29 columns are macro-covered already, and
  the aggregate queries are complex, so **use macros** and regenerate the
  cache.
- Cache workflow (`docs/DEV.md:64-99`): `cd rust/web && sqlx migrate run &&
  cargo sqlx prepare -- --tests --features ssr --all-targets`. Use the
  scratch-DB recipe in DEV.md if the dev DB checksum has drifted. `--tests`
  is mandatory or integration-test queries are missed.
- CI (`.github/workflows/ci.yml`, job `test-rust`, `SQLX_OFFLINE=true`
  throughout) runs `cargo sqlx prepare --check -- --tests --features ssr
  --all-targets` - a stale `.sqlx/` fails CI (commit 508c35b). Commit the
  regenerated `.sqlx/` files with the queries.
- Canonical local verification: `cargo clippy -p web --all-targets --features
  ssr -- -D warnings`, `cargo test -p web --features ssr` (needs live
  Postgres at devenv `DATABASE_URL`). Plain `cargo check --workspace` fails
  by design for `web`.

## Testing conventions

- `#[sqlx::test]` = fresh isolated migrated DB per test; set up exactly the
  rows needed, no shared fixtures. Fixture helpers to reuse/extend in
  `db.rs` tests: `make_user` (`db.rs:2124`), rating helpers
  `find_rating_change` (`db.rs:3556`), `game_type_rating` (`db.rs:3567`).
  Existing rating tests around `db.rs:3521-3893` cover ELO math, pairwise
  zero-sum, bot games leaving rating_change NULL, concede deltas - the model
  for #29's D1/tie/reconstruction tests.
- `db.rs` changes MUST land with tests (CODING.md "Testing Conventions").
- Route/page coverage goes in `rust/web/tests/ssr_pages.rs` (in-process
  `tower::ServiceExt::oneshot` against `build_router`; helpers `make_state`,
  `make_user`, `login_cookie`, `make_game_version`, `get()` at lines 35-184).
  Add tests asserting 200 + marker for `/players/:name` (existing and
  unknown user) and `/players/:name/:game_type`. Do NOT add scenarios to the
  Playwright smoke (`end2end/tests/page-loads.spec.ts`) beyond, at most, one
  hard-load of a profile page if hydration risk warrants it.
- Never call real game services in tests; mock pattern in
  `rust/web/src/game/client.rs` (not needed for #29 - stats are pure SQL).
- To backdate `games.updated_at` in fixtures: `ALTER TABLE games DISABLE
  TRIGGER update_games_updated_at` first. `finished_at` is set by its own
  trigger on the is_finished flip - for fixtures needing specific
  finished_at values, UPDATE it explicitly after insert (it only auto-sets
  when NULL on the false->true transition; verify trigger behaviour at
  001:448-452 when writing fixtures).

## Decisions for v1 (resolving spec gaps - overridable by Orchestrator)

1. **Game-type URL segment**: use `game_types.name`, matched
   case-insensitively, percent-encoded in links. It is unique (`002`) and the
   profile URL scheme is already name-based for users. `game_versions.uri` is
   per-version (service dispatch), not a stable game-type slug - wrong tool.
   If real names turn out URL-hostile, escalate before inventing a slug
   column (spec: no schema changes).
2. **D1 human count in SQL**: a game "counts" iff
   `(SELECT count(*) FROM game_players gp2 WHERE gp2.game_id = g.id AND
   gp2.user_id IS NOT NULL) >= 2` and `g.is_finished`. The include-bot-games
   toggle relaxes this to `>= 1`. Toggle is query param `?bots=1`, default
   off, preserved in intra-profile links.
3. **Favourite colour on profile pages**: render the profile user's name with
   their first `pref_colors` entry (normalized), falling back to default
   foreground when the list is empty. Opponent names in game lists use the
   same rule (single indexed lookup by user ids), NOT per-game
   `game_players.color` - profile context has no game seat.
4. **"Recent finished games"** list: 20 most recent by `finished_at DESC` on
   `/players/:name`; full list on the per-game-type page.
5. **"Active games"** (own profile only): `is_finished = false` games where
   the profile user has a `game_players` row, same shape as sidebar
   `find_active_game_summaries` (`db.rs:~515`). Shown only when
   `viewer_user_id == profile user id`.
6. **Form strip ordering**: chronological left-to-right, most recent
   rightmost, last 10 results, labelled "recent form (oldest to newest)".
   Same ordering for sparklines. One shared component, used on profile pages
   and the game meta panel.
7. **Form strip characters**: `W`/`L` for 2-player results, placing digit for
   3+ players; green (`--mk-green`) for place=1, red (`--mk-red`) otherwise;
   ties at place 1 are wins (spec).
8. **SSR strategy**: profile page core stats + charts via
   `Resource::new_blocking` inside `Suspense` with `MainLayout` outside
   (first use of this pattern in the codebase - budget review time; the
   pattern is fully documented in CODING.md/hydration.md). Secondary lists
   (active games) may be `LocalResource`.
9. **peak_rating backfill**: implement as a one-off admin SQL run against
   prod after deploy (documented in the PR), not a numbered migration -
   migrations are schema-oriented here and the backfill depends on the
   reconstruction query being validated first. The drift sanity check ships
   as a `#[sqlx::test]` over fixtures plus an admin query documented in the
   PR.
10. **Empty states are first-class**: unknown user name renders a "no such
    player" page (200, not 500); a user with zero finished games renders the
    header with zeroed stats; a game type with no rating data renders counts
    without ELO (spec "Awareness": never hard-assume rated).

## Suggested unit-of-work split (for the Orchestrator)

1. DB queries + tests (D1 rule, aggregates, rating series, head-to-head).
2. Server fns + view types + `/players/:name` page + SSR tests.
3. Sparkline/form-strip/SVG components + `/players/:name/:game_type` page.
4. Integration: name links (game meta, lists), form strips in meta panel,
   add-friend button, peak_rating backfill + drift check.

Each fits a 150k Lead budget; 1 and 3 are independent, 2 depends on 1,
4 depends on 2+3.
