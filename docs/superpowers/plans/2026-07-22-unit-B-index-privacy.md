# Unit B - Index redesign + privacy control (Implementation Plan)

Research + planning doc only. No source was modified. All paths relative to
repo root. Crate is `rust/web` (Leptos SSR + WASM, Axum backend). This doc is
self-contained for the index/dashboard/privacy work.

Unit B replaces the stub homepage and the dashboard with a single rich `/`
index that shows different content logged-out vs logged-in, adds a live
public game render driven by the existing websocket, and introduces a
per-user privacy control that gates which games may appear on the public
(logged-out) index. The four requirement groups are quoted inline as R1..R4.

Requirements (verbatim intent):
- R1 Remove dashboard - the current `/dashboard` page goes away.
- R2 Logged-out index (`/`):
  - Live game render with websocket (an active game being played in real time).
  - 3 recent log lines from that game.
  - Title above the render.
  - "Lo-fi board games by email and web" subheading.
  - "Start a game" CTA -> login page.
  - Game selection: the game with the most players active in the last 10
    minutes (uses the presence system from unit I: `users.last_active_at`,
    `RECENTLY_ACTIVE_WINDOW` = 10 min).
- R3 Logged-in index (`/`):
  - Friends list with recent play info.
  - Game-type list with ELO + trend.
  - Game-history table (last 10 games): yellow = your turn, grey = finished.
- R4 Privacy control (public/friends/private):
  - Build on existing `friends`/`blocks`/`users.invite_policy` tables.
  - Game shows on the public index only if ALL players allow (privacy = public).
  - Per-user privacy setting.

---

## 1. Current behaviour

### Routing + the two pages today (`app.rs`)

- Route table (`app.rs:216-234`): `/` -> `HomePage` (`:217`), `/dashboard` ->
  `DashboardPage` (`:221`). Login is `/login` (`:218`).
- **`HomePage` (`app.rs:285-295`)** is a stub: `<MainLayout>` wrapping
  `<h1>"Welcome to brdg.me"</h1>`, `<p>"Lo-fi board games by email and
  web."</p>`, and `<A href="/dashboard">"Go to Dashboard"</A>`. No resources,
  no data. This is the component R2/R3 rebuild.
- **`DashboardPage` (`app.rs:469-620`)** is the logged-in landing page today.
  Five `<section>`s under `.dashboard-sections`:
  - "Friend requests" - `get_friends_overview().incoming`, Accept | Decline |
    Decline-and-block (native confirm) (`:501-543`).
  - "Pending invites" - `get_pending_invites()`, Accept | Decline (`:545-576`).
  - "Friends' active games" - `get_friend_activity().active` (`:578-591`).
  - "Friends' recent results" - `get_friend_activity().results` (`:593-610`).
  - "Active Games" - static "Use the sidebar to navigate your games." (`:612-615`).
  All data via `LocalResource`s (`overview`, `activity`, `invites`) with a
  shared `refresh` counter bumped by the respond actions. R1 deletes this whole
  component; R3's logged-in index reuses some of these data sources (friends,
  activity) but NOT the friend-request / pending-invite sections by default -
  see decision D-dashboard-sections.

### Where `/dashboard` is referenced (must be rewired on removal, R1)

- `app.rs:221` - the `<Route path=StaticSegment("dashboard") ...>`.
- `app.rs:291` - HomePage "Go to Dashboard" link (goes away with the stub).
- `app.rs:341` - `LoginPage` navigates to `/dashboard` on successful login
  (the post-login landing). Must point at `/`.
- `proposals.rs:1851` - `InvitePage` navigates to `/dashboard` after cancel
  (`nav3("/dashboard", ...)`). Must point at `/`.
- `tests/ssr_pages.rs:253-257` - `dashboard_page_anonymous` asserts `/dashboard`
  returns 200 + "Dashboard" marker. Must be removed or flipped to a 404/redirect
  assertion (D-redirect).
- The sidebar (`components/layout.rs:116-265`) has NO dashboard link - it links
  `/`, `/games/new`, `/settings`, `/friends`, and the game lists. No change there.

### Game rendering (the live board) - reusable for R2

- **`GameBoard(html, player_style)`** (`components/game.rs:19-23`):
  `<div class="game-render" style=player_style><pre inner_html=html></pre></div>`.
  Pure presentational component, no resources, no browser API - safe to render
  inside a `LocalResource` callback or `Suspense`. `html` is already-rendered
  markup-converted HTML; `player_style` is the `--mk-player-{n}` var block.
- **How a board is produced server-side** (`get_game_details`,
  `server_fns.rs:220-345`):
  1. `db::find_game_extended(pool, game_id)` (`db.rs:400`) -> `GameExtended`.
  2. `client::render(http, &ge.game_version.uri, &ge.game_version.name,
     ge.game.game_state.clone(), player.map(|p| p.game_player.position as usize))`
     - `client` is `brdgme_game_client` re-exported at `game/mod.rs:2`.
     `render(...)` (`lib/game_client/src/lib.rs:149-160`) dispatches on the
     `Option<usize>`: `Some(p)` -> `player_render` (private, per-seat), **`None`
     -> `pub_render` (public spectator render)**. The public index uses `None`.
     Returns `RenderResponse { render, state, command_spec }`
     (`lib.rs:122-127`); for `pub_render`, `command_spec` is always `None`.
  3. Markup -> HTML, semantically (colours stay symbolic CSS classes, follow the
     viewer's theme): `brdgme_markup::from_string(&render_resp.render)` then
     `brdgme_markup::html_class(&brdgme_markup::transform_semantic(&nodes,
     &ge.semantic_players()))` (`server_fns.rs:254-260`).
  4. `player_style = ge.player_style()` (`db.rs:392-395`, via
     `theme::player_style_vars(slots)`); `semantic_players()` at `db.rs:382-389`.
- **`get_game_details` requires auth** (`server_fns.rs:227-229`,
  `Not authenticated`) and renders from the caller's seat. The public index
  needs a NEW anonymous server fn that renders from `None` (spectator) and gates
  on privacy (R4) instead of on being a player.
- The mock game service in `tests/ssr_pages.rs:104-120` (`spawn_mock_game_service`)
  answers ONLY `GameRequest::PlayerRender`. A public render uses
  `GameRequest::PubRender` - the mock must be extended to answer `PubRender` for
  any SSR test that exercises the live board (gotcha, carry forward).

### Game log lines - reusable for R2's "3 recent log lines"

- Schema: `game_logs` (`001:203-211`) - `body` (brdgme markup), `is_public bool
  NOT NULL`, `logged_at`, `created_at`. Private per-player lines are targeted via
  `game_log_targets` (`001:213`). Model `GameLog` at `models/game.rs:71-80`.
- `db::get_game_logs(pool, game_id, game_player_id)` (`db.rs:1482-1504`) returns
  `is_public = true OR targeted-at-this-player` lines, `ORDER BY logged_at ASC`.
  For a spectator there is no player, so the public index queries **`is_public =
  true` only** (a new helper), last 3 by `logged_at DESC`.
- Log markup -> HTML uses the SAME pipeline as the board
  (`from_string` + `transform_semantic(nodes, semantic_players)` + `html_class`),
  see `get_game_logs` server fn `server_fns.rs:582-597`. `GameLogEntry {
  body_html, logged_at, is_new }` (`server_fns.rs:164-169`).
- `render_log_entries(entries, show_timestamp)` (`components/game.rs:306-345`)
  groups entries into 10-minute windows and renders each `body_html` in a
  `.game-log-entry`. It calls `format_log_time` (`:291-304`, uses `js_sys::Date`,
  client-only) - so `render_log_entries` is only reachable via a `LocalResource`
  value (None on SSR). For 3 static recent lines on the public index we can reuse
  `render_log_entries` inside the mounted-gate pattern, or render the 3 `body_html`
  strings directly without timestamps (simpler; no `js_sys`). See D-log-render.
- `GameLogs`/`RecentGameLogs` (`components/game.rs:347-439`) demonstrate the
  **mounted-gate** (`let mounted = RwSignal::new(false); Effect::new(move |_|
  mounted.set(true));` then `mounted.get().then(|| logs.get())...`) that keeps SSR
  and hydration identical for client-only log content. The public index's log/WS
  region must use the same idiom (hydration, carry forward).

### WebSocket infrastructure (drives R2's "real time")

- **Server** (`websocket.rs`): `GameBroadcaster::broadcast_game_update(game_id)`
  (`:39-58`) publishes a skinny `GameUpdateSignal { game_id }` to NATS subject
  `game.{id}` (+ `flush`). `ws_handler`/`handle_socket` (`:82-175`) subscribes to
  `game.>` and `proposal.>` and forwards every payload to the browser WS, with a
  30s ping keepalive. Broadcasts happen after every command/undo/concede/restart
  (e.g. `broadcast_and_trigger`, `game/mod.rs:50-58`).
- **Client** (`websocket_client.rs`): `use_websocket()` (`:38-85`, hydrate-only)
  connects to `/ws`; on a `GameUpdateSignal` it bumps the app-wide
  `WebSocketTrigger.last_update` counter AND `bump_game_update(game_update,
  game_id)` (`:23-28`) on the `RwSignal<Option<(Uuid, u64)>>` game-update context
  (provided in `App`, `app.rs:113`). `WebSocketTrigger` (`:5-8`) is the general
  refetch counter the sidebar's `active_games` resource keys on (`app.rs:128-133`).
- **Reuse for the index:** the public index resource can key on
  `WebSocketTrigger.last_update` (refetch on ANY game update - simplest, mirrors
  the sidebar) so the live board + 3 logs refresh whenever the displayed game (or
  any game) changes. Keying on the per-game `game_update` signal is possible but
  the index doesn't know the game id until the first fetch resolves, so the
  general counter is the clean choice (D-ws-key). No new WS protocol work needed -
  the broadcaster already fires for every game the index might display.

### Presence system (unit I) - drives R2 game selection

- `users.last_active_at timestamptz` (migration `019_user_presence.sql`), NULL =
  never pinged. Stamped by `db::set_user_last_active` (`db.rs:1809-1815`) via the
  `ping_active` server fn (`auth/server.rs:525`), called by the client every 5 min
  while any page is open (`app.rs:178-192`, `PRESENCE_PING_INTERVAL_MS` `app.rs:31`).
- `db::RECENTLY_ACTIVE_WINDOW = 600s` (10 min) (`db.rs:1806`) - 2x the ping cadence.
- `db::active_within_window(last_active_at, now, window)` pure predicate
  (`db.rs:1820-1830`); `db::is_user_recently_active(pool, user_id)` (`db.rs:1835-1854`)
  reads the column and applies the window (fails closed => false).
- **R2 selection** = among active (non-finished) games whose human players are ALL
  `game_visibility = 'public'` (R4), pick the one with the most human players whose
  `last_active_at` is within `RECENTLY_ACTIVE_WINDOW`. No existing query does this -
  B2 adds it. SQL can compare `last_active_at > now() - interval '10 minutes'`
  directly (the column is `timestamptz`), or reuse the window constant via a bind.

### Friends / blocks / invite_policy (the seam for R4)

- `friends` table (migration `010_friends.sql`): one row per direction + a pair
  unique index; `has_accepted` (NULL = pending, TRUE = accepted, FALSE = declined).
  `blocks` table (`010:18-30`): directed `blocker_user_id -> blocked_user_id`.
- `users.invite_policy text NOT NULL DEFAULT 'open' CHECK IN ('open','friends','none')`
  (`010:13-15`). Deliberately NOT a field on `models::user::User` - read/written via
  `db::get_invite_policy` / `db::set_invite_policy` (`db.rs:2139-2155`) only.
  Labels `INVITE_POLICIES` at `friends.rs:12-16`. Enforced by
  `db::check_invite_policy_tx` (`db.rs:2166-2215`) which also consults
  `has_block_conn`/`are_friends_conn`. **This is invite-gating only** - it governs
  who may add you to a game, not whether your games are visible publicly. R4 is a
  separate visibility axis (D-privacy-model).
- Friend queries (all plain sqlx, `db.rs`): `list_friends` (`:2008-2020`, accepted
  friends), `are_friends_conn` (`:1979`), `should_hide_add_friend` (`:1994`),
  `has_block` (`:2098`), `friends_active_games` (`:2321-2350`, active games with >=1
  friend, excluding the caller's own), `friends_recent_results` (`:2356-2393`,
  finished games with >=1 friend, names ordered by place). Server fns
  `get_friends_overview` (`friends.rs:86-120`) and `get_friend_activity`
  (`friends.rs:277-309`) wrap these. The invite-policy `<select>` UI is on the
  Friends page (`friends.rs:505-515`) via `SetInvitePolicy` (`friends.rs:228-239`).

### ELO / ratings + trend (drives R3 game-type list)

- `game_type_users` (`001:149`): `rating` (default 1200), `peak_rating`,
  `last_game_finished_at`, per `(game_type_id, user_id)`. Per-game change in
  `game_players.rating_change` (+ `rating_before`, migration 017). ELO math in
  `db.rs:1506+` (`ELO_K=32`, `apply_rating_changes`); bots excluded from ratings.
- `stats::queries::game_type_stats` (`stats/queries.rs:~80-155`) returns per-game-type
  `GameTypeStats { game_type_name, games, wins, win_percent, avg_place_percentile,
  rating, peak_rating }` (`stats/mod.rs:28-36`) - rating + peak already there.
- Trend: `stats::queries::rating_series` (`stats/queries.rs:157-194`) reconstructs
  the full rating series from `rating_change`s; `stats::recent_form_for_game_type`
  (`:619-683`) gives per-user recent `FormResult`s; `players::rating_trend(current,
  results)` (`players.rs:16-32`) reconstructs the recent trend (needs >=2 changes).
  SVG viz is zero-dep SSR: `stats::viz::Sparkline` (trend line) and `FormStrip`
  (W/L dots) - the profile page's by-game-type table already uses both
  (`players.rs:306-395`). R3's "ELO + trend" reuses `GameTypeStats.rating` +
  `rating_trend`/`Sparkline` (or `FormStrip`).
- `get_player_profile` (`stats/mod.rs:174-229`) already computes `game_types`,
  `recent_form`, `active_games`, `recent_finished` for a named user and is
  **anonymous-allowed** (`viewer_user_id = get_current_user().await?.map(|u| u.id)`).
  R3 could reuse it for the current user, or B4 adds a leaner dedicated fn
  (D-index-data-fn).

### Game history (drives R3 history table)

- `stats::queries::game_history` (`stats/queries.rs:373-444`) returns paginated
  `HistoryRow { game_id, game_type_name, is_finished, started_at, finished_at,
  my_place, player_count, my_rating_change, opponents, match_elo }` - filters by
  status/game_type, ordered `created_at DESC`. It has `is_finished` but NOT
  `is_turn`. `stats::queries::active_games` (`:311-356`) returns `ActiveGameRow {
  ..., is_turn, ... }` for non-finished games. The sidebar's
  `db::find_active_game_summaries` (`db.rs:583-641`) carries `is_turn` (active only);
  `db::find_finished_game_summaries` (`db.rs:718-766`) carries the last 3 finished.
- R3's "last 10 games, yellow = your turn, grey = finished" needs the user's last 10
  games across active+finished WITH `is_turn` and `is_finished`. No single existing
  query returns exactly that - B4 adds one (or composes `active_games` + a finished
  query and merges in Rust). Yellow = `!is_finished && is_turn`; grey = `is_finished`.

### Auth pattern for anonymous-or-logged-in server fns

- `get_current_user()` (`auth/server.rs:437-462`) returns `Option<AuthUser>` (None =
  anonymous). Logged-in-only fns start with `.ok_or_else(|| ServerFnError::new("Not
  authenticated"))?`. Anonymous-allowed fns (e.g. `get_player_profile`) read
  `get_current_user().await?.map(|u| u.id)` and branch. The public index render fn
  (B3) is anonymous-allowed; the logged-in index fn (B4) requires auth.
- `App` provides a `current_user: LocalResource<Result<Option<AuthUser>, _>>`
  (`app.rs:137-142`), None until resolved and treated as logged-out - the sidebar
  uses `logged_in = move || matches!(current_user.get(), Some(Ok(Some(_))))`
  (`layout.rs:133`). The new `HomePage` branches the same way; because the resource
  is None on SSR AND on the client's first hydration pass, both render the
  logged-out branch initially (no structural mismatch), then the client reactively
  switches to the logged-in view once it resolves (hydration-safe, see D-index-branch).

---

## 2. Shared building blocks

### 2a. Public game render fn - extract the render pipeline from `get_game_details`

`get_game_details` (`server_fns.rs:220-345`) bundles auth + seat-specific render +
per-player form/friend/admin enrichment. The public index needs only the spectator
render + player_style + a title. Factor a small shared SSR helper rather than
duplicating the markup pipeline:

```rust
// game/server_fns.rs (#[cfg(feature = "ssr")])
pub(crate) async fn render_game_public(
    pool: &PgPool, http: &reqwest::Client, game_id: Uuid,
) -> Result<Option<PublicGameRender>, ServerFnError>
// -> loads GameExtended, calls client::render(..., None) (pub_render),
//    transforms markup with ge.semantic_players(), builds player_style,
//    returns { game_id, type_name, version_name, html, player_style, player_names }
```

`get_game_details` keeps its seat-specific path (it needs `command_spec` + per-seat
render, which `pub_render` does not provide), so this is an additive helper, not a
refactor of the existing fn. Both call `find_game_extended` + the markup transform.

### 2b. Anonymous-or-logged-in `HomePage` branch (hydration-safe)

The single `HomePage` (`app.rs:285`) renders one of two views from the shared
`current_user` `LocalResource`:
- `Some(Ok(Some(_)))` => logged-in index (R3).
- everything else (None / Ok(None) / Err) => logged-out index (R2).

Because `current_user` is None during SSR and the client's first hydration pass,
both passes initially build the LOGGED-OUT view; the switch to logged-in happens as
a normal reactive update after the resource resolves. This is the same safe pattern
the sidebar uses (`layout.rs:133,151-175`, `hidden=` toggles on a `LocalResource`
that is None at hydration). Do NOT gate the two views with a structural `if` on a
value that differs between SSR and client run 0 - branch on the resource value
inside the render closure (a reactive re-render after mount is fine; the hydration
MATCH is run 0, where both sides agree). See D-index-branch.

### 2c. Privacy gate - "all human players public" predicate (R4)

A game is eligible for the public (logged-out) index iff EVERY human player
(`game_players.user_id IS NOT NULL`) has `users.game_visibility = 'public'`. Bots
(`user_id IS NULL`) have no visibility setting and never block. This predicate is
used in two places that must agree: the selection query (B2, picks the game) and the
render fn (B3, refuses to render a game that no longer qualifies). Implement it once
as a SQL `EXISTS`/`NOT EXISTS` condition reused by both (a CTE or a `db` helper
returning bool), so selection and rendering cannot drift. Blocks are NOT part of this
gate (blocks govern invites/visibility-between-two-users, not public broadcast) -
confirm in D-privacy-blocks.

### 2d. Privacy setting storage (R4) - new column vs overload `invite_policy`

Two viable storage models (full trade-off in D-privacy-model):
- **(recommended) New column** `users.game_visibility text NOT NULL DEFAULT 'public'
  CHECK IN ('public','friends','private')`, mirroring the `invite_policy` pattern
  exactly: NOT on the `User` struct, read/written via `db::get_game_visibility` /
  `db::set_game_visibility` plain queries (avoid `.sqlx` churn, CODING.md "Plain
  (non-macro) sqlx queries"). Keeps invite-gating and visibility independent.
- (alt) Overload `invite_policy` (`open|friends|none`) to ALSO mean visibility
  (open~public, friends~friends, none~private). Zero new columns, but couples two
  distinct user choices and changes the meaning of an existing setting.

Both "build on" the existing friends/blocks tables for the `friends` tier (evaluated
via `are_friends_conn`). The logged-out index only ever uses the `public` tier (no
viewer); `friends`/`private` both exclude a game from the logged-out index. The
`friends` tier's viewer-scoped meaning is future scope (D-friends-tier-meaning).

---

## 3. Implementation units (dependency order)

Each unit is one layer/concern, sized to stay well under the 150k budget. Backend
units (B1-B4) land first and independently of the frontend (they add fns/queries the
existing UI ignores until B5/B6 wire them). Every unit ends with fmt + clippy green
and its own commit; DB-touching units add `#[sqlx::test]`s. Suggested commit order:
B1, B2, B3, B4, B5, B6, B7. Push deferred to a final cleanup unit.

### B1. Privacy setting: schema + db helpers + set fn + UI (backend + small frontend)
- Goal: R4 foundation - a per-user `game_visibility` (public/friends/private) with
  read/write helpers, a server fn, and a settings UI.
- Files: new migration `rust/web/migrations/021_user_game_visibility.sql`;
  `rust/web/src/db.rs` (helpers); `rust/web/src/friends.rs` (server fn + labels +
  UI) OR `rust/web/src/settings.rs` (D-privacy-ui-location).
- Change:
  - Migration 021: `ALTER TABLE public.users ADD COLUMN game_visibility text NOT
    NULL DEFAULT 'public' CHECK (game_visibility IN ('public','friends','private'));`
    (follow migration 010's `invite_policy` style; comment referencing this plan).
  - `db::get_game_visibility(pool, user_id) -> Result<String>` and
    `db::set_game_visibility(pool, user_id, &str) -> Result<()>` - plain queries,
    mirror `get_invite_policy`/`set_invite_policy` (`db.rs:2139-2155`),
    `set` bumps `updated_at = NOW()`.
  - `GAME_VISIBILITIES: [(&str,&str); 3]` labels constant (mirror `INVITE_POLICIES`,
    `friends.rs:12-16`), e.g. public "Anyone can see my games on the home page",
    friends "Only friends can see my games", private "Hide my games from the home page".
  - `set_game_visibility` server fn (auth-guarded, validates against the labels -
    mirror `SetInvitePolicy`, `friends.rs:228-239`).
  - UI: a `<select>` next to the invite-policy control on the Friends page
    (`friends.rs:505-515`) OR a section on Settings - fire-and-forget on change
    (CODING.md "Save model"). Surface the current value via `get_friends_overview`
    (add `game_visibility` to `FriendsOverview`, `friends.rs:32-40`) or a dedicated
    read.
- Acceptance: column exists default 'public'; get/set round-trip; server fn rejects
  unknown values; UI shows + changes the setting; fmt/clippy green.
- Tests (`#[sqlx::test]`): default is 'public'; set/get round-trip for each value;
  (server-fn validation is thin - cover the db helpers).
- Depends on: nothing. **Next migration number is 021** (020 is current max).

### B2. Public game selection query (backend)
- Goal: R2 selection - pick the active, all-public game with the most recently-active
  human players.
- Files: `rust/web/src/db.rs` (new query + helper).
- Change:
  - `db::find_public_index_game_id(pool) -> Result<Option<Uuid>>`:
    - Candidate games: `games.is_finished = false` AND no human player with
      `game_visibility <> 'public'` (the 2c predicate, as a `NOT EXISTS (SELECT 1
      FROM game_players gp JOIN users u ON u.id = gp.user_id WHERE gp.game_id = g.id
      AND u.game_visibility <> 'public')` - bots have NULL user_id so the JOIN drops
      them).
    - Score = count of human players with `u.last_active_at > now() - interval '10
      minutes'` (reuse `RECENTLY_ACTIVE_WINDOW` as a bind, or inline the interval -
      D-window-bind). 
    - `ORDER BY active_count DESC, g.updated_at DESC, g.id LIMIT 1` (tie-break:
      most recently updated, then id - D-tiebreak). Return the game id (or None if no
      candidate).
  - Keep the all-public predicate in a form B3 can reuse (shared CTE text or a
    `db::game_is_publicly_visible(pool, game_id) -> Result<bool>` helper that B3 calls
    and B2's query embeds - keep them textually consistent).
- Acceptance: returns the game with the most active players; excludes finished games;
  excludes any game with a non-public human player; returns None when nothing qualifies.
- Tests (`#[sqlx::test]`): picks max-active game; a non-public player disqualifies a
  game; finished games excluded; all-stale-players still selects (score 0) vs no
  candidate at all; tie-break deterministic. Backdate `last_active_at` directly (the
  column has no trigger; unlike `games.updated_at` which DOES - disable
  `update_games_updated_at` if backdating `updated_at`, CODING.md "Database").
- Depends on: B1 (reads `game_visibility`).

### B3. Public index render server fn (backend, anonymous)
- Goal: R2 data - one anonymous server fn returning the selected game's spectator
  render + title + 3 recent public log lines, privacy-gated.
- Files: `rust/web/src/game/server_fns.rs` (new fn + DTO + the 2a helper); possibly
  `rust/web/src/db.rs` (public-logs helper).
- Change:
  - DTO `PublicIndexGame { game_id: Uuid, type_name: String, version_name: String,
    html: String, player_style: String, player_names: Vec<String>, logs:
    Vec<GameLogEntry> }` (reuse `GameLogEntry`, `server_fns.rs:164-169`).
  - `render_game_public(pool, http, game_id)` helper (section 2a): `find_game_extended`
    -> `client::render(..., None)` -> markup transform with `ge.semantic_players()` ->
    `html` + `ge.player_style()` + player names from `ge.game_players`.
  - `db::get_public_game_logs(pool, game_id, limit) -> Result<Vec<GameLog>>`:
    `SELECT ... FROM game_logs WHERE game_id=$1 AND is_public = true ORDER BY
    logged_at DESC LIMIT $2` then reverse to chronological (plain query; the existing
    `get_game_logs` is player-scoped and won't do).
  - `#[server(GetPublicIndex, "/api")] get_public_index() ->
    Result<Option<PublicIndexGame>, ServerFnError>`: NO auth guard; `pool`/`http` from
    context; `find_public_index_game_id` (B2) -> if None, `Ok(None)`; else
    `game_is_publicly_visible` re-check (race-safe) -> `render_game_public` ->
    transform the 3 log bodies with the same `semantic_players` -> `Ok(Some(...))`.
    Map the 3 logs to `GameLogEntry { body_html, logged_at, is_new: false }`.
- Acceptance: anonymous caller gets the selected public game's board HTML + 3 logs +
  title; a game that stops being public between selection and render is skipped (returns
  None or the next candidate - D-render-race); no auth required; fmt/clippy green.
- Tests (`#[sqlx::test]`, mock game service): returns render+logs for a qualifying game;
  returns None when no public game; refuses a game with a non-public player. **Requires
  extending `spawn_mock_game_service` (`ssr_pages.rs:104`) to answer
  `GameRequest::PubRender`** (gotcha).
- Depends on: B1, B2.

### B4. Logged-in index data server fns (backend)
- Goal: R3 data - friends w/ recent play, game-type ELO + trend, last-10 game history
  (with is_turn/is_finished) for the logged-in user.
- Files: `rust/web/src/game/server_fns.rs` OR a new `rust/web/src/index.rs` (D-index-fn-home);
  `rust/web/src/db.rs` and/or `rust/web/src/stats/queries.rs` (new queries).
- Change (one auth-guarded `#[server(GetLoggedInIndex, "/api")] get_logged_in_index()
  -> Result<LoggedInIndex, ServerFnError>` returning a single DTO, OR three thin fns -
  D-index-data-fn):
  - **Friends + recent play:** `db::list_friends` for the friend list; per friend a
    "recent play" line. Define recent-play as the friend's most recent finished game
    (type + result/place + when) OR the most recent game you played WITH that friend
    (D-friend-recent-play). New query `friends_with_recent_play(pool, user_id, limit)`.
  - **Game-type ELO + trend:** reuse `stats::queries::game_type_stats` (rating/peak) +
    `stats::recent_form` + `players::rating_trend` to produce per-type `{ name, rating,
    trend: Vec<f64> }` (the profile page already assembles this; lift the assembly into
    the index fn or reuse `get_player_profile` for self).
  - **Game history (last 10):** new query `recent_games_for_index(pool, user_id, 10)`
    returning `{ game_id, type_name, is_finished, is_turn, opponents, finished_at/
    updated_at }` across active+finished, `ORDER BY g.updated_at DESC LIMIT 10`
    (D-history-order). Yellow = `!is_finished && is_turn`; grey = `is_finished`.
    (`game_history` lacks `is_turn`; `active_games` lacks finished - so a new combined
    query is cleanest.)
- Acceptance: auth required; returns the three sections for the caller; empty sections
  render empty (not error); fmt/clippy green.
- Tests (`#[sqlx::test]`): friends list + recent play populated; game-type rating +
  trend present after a rated game; history returns last 10 with correct is_turn/
  is_finished flags and ordering.
- Depends on: nothing (independent of B1-B3; serial anyway).

### B5. Logged-out index frontend (R2)
- Goal: rebuild `HomePage` (`app.rs:285`) - logged-out branch shows the live game
  render (websocket-refreshed), 3 log lines, title above the render, the subheading,
  and the "Start a game" CTA -> `/login`. Establishes the logged-in/logged-out branch
  shell (section 2b) with a logged-in placeholder filled by B6.
- Files: `rust/web/src/app.rs` (HomePage); maybe `rust/web/src/main.scss` (index
  layout styles); reuse `components/game.rs::GameBoard`.
- Change:
  - `HomePage` reads the shared `current_user` `LocalResource` and branches (2b).
    Logged-out branch:
    - A `LocalResource` on `get_public_index()` (B3), keyed on
      `WebSocketTrigger.last_update` (from context) so it refetches on every game
      update (D-ws-key) - the live board + logs track the game in real time.
    - Layout (always present, outside the resource-dependent content): `<h1>` title
      (site name or the game's type name - D-title-content), `<p>"Lo-fi board games
      by email and web."</p>` subheading, and a "Start a game" CTA `<A href="/login">`.
    - Board + logs: render `GameBoard(html, player_style)` and the 3 log lines from the
      resource value. Because the resource is a `LocalResource` (None on SSR), wrap the
      board/log region so SSR and hydration match: either a mounted-gate (like
      `GameLogs`, `components/game.rs:365-366`) or a `Suspense fallback` whose fallback
      is the SAME structure as the loaded state (a `.game-render` placeholder). Keep the
      title/subheading/CTA OUTSIDE the gated region so they SSR normally.
    - No public game (`Ok(None)`): show title + subheading + CTA only (no board) - keep
      the structure stable (toggle a `hidden` attr or render an empty `.game-render`,
      NOT a structural swap, per CODING.md "Structural vs attribute hydration mismatches").
  - Logged-in branch: a placeholder for B6 (e.g. the title + a loading/empty container),
    so B5 ships a compiling, hydration-clean page and B6 fills it in.
- Acceptance: logged-out `/` shows the live board + 3 logs + title + subheading +
  "Start a game" -> `/login`; board refreshes on WS updates; no-game state shows the
  marketing content; hydration clean (Playwright hard-load zero console errors);
  fmt/clippy green.
- Tests: SSR page test `/` anonymous stays 200/no-panic - UPDATE
  `home_page_anonymous` (`ssr_pages.rs:234-238`) marker (the stub's "Welcome to
  brdg.me" changes); add a logged-out SSR test that seeds a public game + mock
  `PubRender` and asserts the board/log markup appears. Playwright smoke
  (`end2end/tests/page-loads.spec.ts`).
- Depends on: B3.

### B6. Logged-in index frontend (R3)
- Goal: fill the logged-in branch of `HomePage` - friends list w/ recent play,
  game-type list w/ ELO + trend, game-history table (last 10, yellow=your-turn /
  grey=finished).
- Files: `rust/web/src/app.rs` (HomePage logged-in branch); `rust/web/src/main.scss`;
  reuse `stats::viz::Sparkline`/`FormStrip`, `components/game.rs::PlayerName`.
- Change:
  - Logged-in branch reads `get_logged_in_index()` (B4) via a `LocalResource` (None on
    SSR => the logged-out branch renders at hydration run 0 per 2b; this content appears
    as a reactive update after `current_user` + the index resource resolve - safe).
  - **Friends list:** friend name (link to `/players/{name}`) + recent-play line.
  - **Game-type list:** type name (link to `/games/type/{name}`), rating (ELO), trend
    (`Sparkline` from `rating_trend`, or `FormStrip` for W/L form) - mirror the profile
    by-game-type table styling (`players.rs:306-395`).
  - **Game-history table:** last 10 rows; row class `my-turn` (yellow, reuse the
    sidebar's `.my-turn` styling, `layout.rs:192`) when `!is_finished && is_turn`, and a
    `finished` (grey) class when `is_finished`; link to `/games/{id}`. Use always-present
    rows with class toggles, not structural swaps.
  - Decide whether friend-requests / pending-invites (lost from the dashboard, R1) appear
    here or move to the Friends page (D-dashboard-sections). Default: move pending-invites
    + friend-requests affordances to the Friends page; the index stays a status overview.
- Acceptance: logged-in `/` shows all three sections; yellow/grey highlighting correct;
  hydration clean; fmt/clippy green.
- Tests: SSR page test `/` logged-in (seed a session via `login_cookie`,
  `ssr_pages.rs:72-99`) returns 200/no-panic with a section marker; Playwright smoke.
- Depends on: B4, B5 (B5 owns the HomePage shell + branch).

### B7. Remove dashboard + rewire links + tests (cleanup)
- Goal: R1 - delete `/dashboard` and point everything that targeted it at `/`.
- Files: `rust/web/src/app.rs` (delete `DashboardPage` `:469-620`, remove route `:221`,
  fix LoginPage navigate `:341`); `rust/web/src/proposals.rs` (fix cancel navigate
  `:1851`); `rust/web/tests/ssr_pages.rs` (remove/replace `dashboard_page_anonymous`
  `:253-257`); optionally `rust/web/src/router.rs` if a redirect is added (D-redirect).
- Change:
  - Delete the `DashboardPage` component and its `<Route>`.
  - `LoginPage` post-login navigate `/dashboard` -> `/` (`app.rs:341`).
  - `InvitePage` post-cancel navigate `/dashboard` -> `/` (`proposals.rs:1851`).
  - Remove the HomePage "Go to Dashboard" link (already gone after B5 rebuilds HomePage).
  - `/dashboard` handling: either let it fall through to the `Routes` fallback
    ("Page not found.", 404) or add a permanent redirect to `/` for bookmarks
    (D-redirect). Update `dashboard_page_anonymous` accordingly (assert 404 marker or
    3xx + Location: /).
  - Verify no other `/dashboard` references remain (grep). The sidebar has none.
- Acceptance: `/dashboard` no longer renders the dashboard (404 or redirect); login and
  invite-cancel land on `/`; no dangling references; the deleted component's server fns
  (`get_friends_overview`, `get_friend_activity`, `get_pending_invites`) remain (still
  used by the Friends page / index) - do NOT delete them; fmt/clippy green; full web test
  suite green.
- Tests: replace `dashboard_page_anonymous`; keep `home_page_anonymous` (updated in B5);
  add a logged-in `/` test if not already (B6). Run `scripts/rust-test.sh`.
- Depends on: B5, B6 (the index must exist before the dashboard is removed, since login
  navigates there).

---

## 4. Decisions for the user

1. **D-privacy-model - storage for the privacy control (R4):** new column
   `users.game_visibility` (`public|friends|private`, default 'public') mirroring
   `invite_policy` (recommended - keeps invite-gating and visibility independent), OR
   overload the existing `users.invite_policy` (`open|friends|none`) to also govern
   visibility (zero new columns, but couples two distinct choices and redefines an
   existing setting). Backlog handover open question #3 / D3 flag this exact choice.
2. **D-privacy-default - default visibility:** 'public' (existing games show on the
   public index immediately, matches today's implicitly-public active games) vs
   'private' (opt-in; the public index stays empty until users opt in). Proposed:
   'public'.
3. **D-friends-tier-meaning - what 'friends'/'private' do for the index:** the
   logged-out index has no viewer, so only 'public' games can appear there; both
   'friends' and 'private' exclude a game from the logged-out index. Is the 'friends'
   tier meaningful anywhere in THIS unit (e.g. a logged-in "public games" browser that
   shows friends' games), or is it reserved for future scope? Proposed: 'friends' is
   stored + honoured by the gate (a 'friends' player blocks public-index eligibility
   exactly like 'private'); its viewer-scoped positive use is future scope.
4. **D-privacy-blocks - do blocks affect public-index eligibility?** Proposed: NO -
   blocks govern invites/visibility between two specific users, not anonymous public
   broadcast. The gate is purely "all human players game_visibility='public'". Confirm.
5. **D-privacy-ui-location - where the user sets visibility:** on the Friends page next
   to the existing invite-policy `<select>` (`friends.rs:505-515`, recommended - groups
   the two social controls), or on the Settings page. Confirm.
6. **D-title-content - the logged-out title above the render (R2):** the site name
   ("brdg.me"), or the live game's identity (e.g. "{type_name}: {player_names}")? R2
   says "Title above the render" without specifying. Proposed: the game type + player
   names as the title (describes the live game), with the "Lo-fi board games by email
   and web" subheading beneath it. Confirm exact wording.
7. **D-ws-key - websocket refetch key for the live index:** key the public-index
   resource on the app-wide `WebSocketTrigger.last_update` (refetch on ANY game update;
   simplest, mirrors the sidebar; one extra refetch per unrelated game update - low
   traffic) vs the per-game `game_update` signal (needs the game id from the first fetch
   to filter). Proposed: `WebSocketTrigger.last_update`.
8. **D-log-render - how the 3 log lines render:** reuse `render_log_entries`
   (`components/game.rs:306`) inside a mounted-gate (gives 10-min timestamp grouping, but
   pulls in client-only `js_sys::Date`), or render the 3 `body_html` strings directly as
   plain `.game-log-entry` rows with no timestamps (simpler, no `js_sys`, hydration-safe
   inside the LocalResource). Proposed: plain rows, no timestamps (they are "recent
   lines", not a full log view).
9. **D-index-branch - logged-in/logged-out structural branch:** confirm the safe approach
   - branch on the shared `current_user` `LocalResource` value inside the render closure
   (None at SSR + hydration run 0 => logged-out view on both; client switches to logged-in
   reactively after resolve). No structural `if` on a value that differs between SSR and
   client run 0. (Documented here so the implementer doesn't reach for a riskier pattern.)
10. **D-index-data-fn - one index fn or several:** a single auth-guarded
    `get_logged_in_index()` returning all three R3 sections in one DTO (one round-trip,
    one resource) vs three thin fns (friends / game-types / history) each with its own
    resource. Proposed: single fn + single `LocalResource`.
11. **D-index-fn-home - where the new server fns live:** add to
    `rust/web/src/game/server_fns.rs` (the public render fn naturally lives there) and/or
    a new `rust/web/src/index.rs` module for the logged-in index DTO/fn. Proposed: public
    render fn in `game/server_fns.rs`; logged-in index fn + DTO in a new `index.rs`
    (keeps `app.rs` lean). Confirm.
12. **D-friend-recent-play - "recent play info" per friend (R3):** the friend's most
    recent finished game (any opponents; type + place + when), OR the most recent game
    you played WITH that friend, OR the friend's `last_active_at` ("active 5m ago").
    Proposed: the friend's most recent finished game (type + their place + when), falling
    back to "no recent games". Confirm.
13. **D-history-order - game-history table ordering (R3):** last 10 by `updated_at DESC`
    (most recently active first - a game you're playing stays at the top) vs `created_at
    DESC` (newest first). Proposed: `updated_at DESC` so an in-progress your-turn game
    surfaces at the top (matches the yellow=your-turn intent).
14. **D-dashboard-sections - where dashboard-only features go (R1):** the dashboard's
    "Friend requests" and "Pending invites" sections are not in R3's index spec. Move them
    to the Friends page (proposed), fold a compact version into the logged-in index, or
    drop them (the sidebar's Pending section + `/invites/:id` already cover invites;
    `/friends` already covers friend requests). Proposed: rely on the existing Friends
    page + sidebar Pending section; do NOT rebuild these on the index. Confirm nothing is
    lost that the user expects on `/`.
15. **D-redirect - `/dashboard` after removal:** fall through to the 404 fallback
    ("Page not found.") vs a permanent redirect (301/302) to `/` for existing bookmarks
    and the (rewired) login flow. Proposed: redirect to `/` (cheap, preserves bookmarks);
    alternatively a plain 404 is acceptable since all in-app links are rewired.
16. **D-window-bind / D-tiebreak - selection query details (B2):** bind
    `RECENTLY_ACTIVE_WINDOW` into the SQL (`now() - $1`) vs inline `interval '10 minutes'`
    (proposed: bind the constant so the window stays single-sourced); tie-break after
    active-count is `updated_at DESC, id` (proposed). Confirm.
17. **D-render-race - selection/render race (B3):** if a game becomes non-public between
    `find_public_index_game_id` (B2) and the render, return `Ok(None)` (index briefly
    empty) vs fall back to the next-best candidate (re-query). Proposed: re-check
    visibility and, if it fails, return `Ok(None)` for this fetch (the next WS-driven
    refetch re-selects) - simplest, self-heals in seconds.

---

## 5. Known issues / gotchas (carry forward to every Lead)

- **Migrations are immutable.** Never edit an applied `.sql` file (sqlx checksum break -
  happened with 005 on 2026-07-11). New schema work => a NEW numbered file; next number is
  **021** (`020_drop_user_last_seen_at.sql` is current max). B1 needs migration 021; no
  other unit should need a migration (B2-B4 are queries against existing columns + the 021
  column).
- **SQLX_OFFLINE=true for clippy/check.** Canonical gates (DEV.md):
  `cargo fmt --all -- --check`;
  `SQLX_OFFLINE=true cargo clippy -p web --all-targets --features ssr -- -D warnings`;
  `cargo check -p web --features hydrate`;
  `cargo test -p web --features ssr` (needs live Postgres). New queries MUST be plain
  (`sqlx::query`/`query_as`/`query_scalar`), NOT macros, to avoid `.sqlx` regeneration
  (CODING.md "Plain (non-macro) sqlx queries") - this matters for B1's new column and
  B2/B4's new queries.
- **clippy `--all-targets` gate is mandatory** - never commit with outstanding fmt/clippy.
- **DB tests need real Postgres.** Plain local runs fail DB tests (pre-existing, backlog
  #40 - not a regression). Use `scripts/rust-test.sh` (temp Postgres+NATS, ports
  15432/14222/18222) or the long-lived `brdgme-test-{pg,nats}-47116` containers.
  `#[sqlx::test]` gives each test an isolated migrated DB.
- **Pre-existing flake:** `email::sweep::tests::invite_expiry_threshold_defaults_to_14_days`
  (env race) - do not chase it as a regression.
- **Mock game service only answers `PlayerRender`** (`tests/ssr_pages.rs:104-120`). The
  public render uses `GameRequest::PubRender` (`lib/game_client/src/lib.rs:162-172`), so
  B3/B5 SSR tests that exercise the board MUST extend the mock to answer `PubRender`
  (return a `PubRender { pub_state, render }`). There is a second mock
  `spawn_ok_new_game_service` in `server_fns.rs` tests that already returns a `PubRender`
  in a `Response::New` - copy that shape.
- **Hydration safety** (`docs/hydration.md`, CODING.md "Leptos: SSR and Hydration"):
  - The public-index board/logs come from a `LocalResource` (None on SSR) - use the
    mounted-gate idiom (`components/game.rs:365-366`) or a `Suspense` whose fallback is
    structurally identical to the loaded state. Never swap element STRUCTURE on the
    async value (no-game vs game) - toggle `hidden`/classes.
  - The logged-in/logged-out branch keys off the shared `current_user` `LocalResource`
    (None at SSR + hydration run 0 => both render logged-out; the client switches
    reactively after resolve). Do NOT introduce a structural `if` on a value that differs
    between SSR and client run 0 (D-index-branch).
  - Keep `MainLayout` and the title/subheading/CTA OUTSIDE any `Suspense`/gated region so
    they are always in the initial SSR HTML.
  - `format_log_time` uses `js_sys::Date` and is client-only - if B5 reuses
    `render_log_entries`, it stays behind the mounted-gate; the proposed plain-row render
    (D-log-render) avoids `js_sys` entirely.
- **No panics** in handlers/components (CODING.md): `NodeRef::get()` is `Option`;
  `web_sys::window()` is `Option`. The render fn returns `Result<Option<...>>`; the
  component matches all arms.
- **`games.updated_at` is trigger-maintained** (CODING.md "Database"): any `UPDATE games`
  bumps it; relevant to B2's tie-break ordering and to tests that backdate it (disable
  `update_games_updated_at` first). `users.last_active_at` has NO trigger - backdate freely.
- **`get_game_logs` is player-scoped** (`db.rs:1482`, filters `is_public OR targeted`) -
  the spectator index needs the new `is_public`-only helper (B3), not this fn.
- **`get_game_details`/`get_game_logs` require auth** - the public index fns (B3) must be
  anonymous-allowed (follow `get_player_profile`'s `get_current_user().await?.map(...)`
  pattern, NOT the `.ok_or_else(Not authenticated)?` pattern).
- **Privacy gate must be consistent** between selection (B2) and render (B3) - implement
  the "all human players public" predicate once (section 2c) so they cannot drift; B3
  re-checks at render time (race, D-render-race).
- **Do not delete the dashboard's backing server fns** in B7 - `get_friends_overview`,
  `get_friend_activity`, `get_pending_invites` are still used by the Friends page and/or
  the new index. Only the `DashboardPage` component + its route go away.
- **Org is `brdgme`** (not `beefsack`) for any image/URL references.
