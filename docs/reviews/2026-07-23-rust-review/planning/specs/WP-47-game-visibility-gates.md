# WP-47: game_visibility gates

**Findings:** wd F17 (major), wd F45 (major). **Decision:** D-6 + D-13 answered
option A - gate game details/feeds on participation-or-public; stats compute
globally but **anonymize** private users (never exclude them from aggregates).

**Landing order:** WP-41 must land first (it touches `db.rs` visibility
predicates). **WP-42 lands after this** and reuses the predicate.

> **Read the named functions before editing. If one does not match what this
> spec describes, STOP and report rather than improvising.** This code is under
> concurrent edit; no line numbers are cited on purpose.

## 1. Problem

- **wd F17** - `get_game_details` (`rust/web/src/game/server_fns.rs`) requires
  only "any authenticated user". Any logged-in user holding a game UUID gets the
  spectator render, player names, ratings, rating changes and recent form, even
  when every player is `game_visibility = 'private'`.
- **wd F45** - the three anonymous-accessible stats server fns in
  `rust/web/src/stats/mod.rs` (`get_player_profile`,
  `get_player_game_type_stats`, `get_player_history`) return opponent
  identities (`user_id` + name + place) and head-to-head rows naming every
  human opponent, with no visibility check at all.

## 2. Why it's wrong

- **wd F17 is correct as written.** Verified live: `get_game_details` uses
  `player` (the viewer's own row) only to pick a render perspective - `None`
  falls through to the full spectator render. Contrast `get_game_logs` in the
  same file, which *does* hard-reject non-players.
- **wd F45 is correct as written.** Verified live: `get_player_game_type_stats`
  and `get_player_history` do not even call `get_current_user`;
  `stats/queries.rs::opponents_by_game` and `::head_to_head` select
  `users.name`/`users.id` with no `game_visibility` clause.
- F45 offers "filter or anonymize". D-6 chose **anonymize**. Do not filter.

## 3. Required end state

### 3a. `rust/web/src/db.rs` - one predicate, no forks

`is_game_visible_to_user(pool, game_id, viewer_id) -> Result<bool>` already has
exactly the signature WP-42 needs (`&PgPool` + `Uuid`, no leptos context, no
`get_current_user`). **Do not change its SQL and do not change its signature.**

Exactly **two** encodings of the rule may exist after this WP: the canonical fn
above, and the SQL copy WP-41 Task 8 inlines inside `friend_recent_visible_game`
(cross-reference comment + drift-guard test). **WP-47 adds callers, not
copies** - leave that inlined copy alone, do not add a third. Add two things:

- `is_game_visible_to_viewer(pool, game_id, viewer: Option<Uuid>) -> Result<bool>`
  - a thin dispatcher, **no new SQL**: `None` delegates to
  `is_game_publicly_visible`, `Some(v)` to `is_game_visible_to_user`. This is
  what WP-42's per-socket filter will call.
- `visible_user_ids(pool, user_ids: &[Uuid], viewer: Option<Uuid>) -> Result<HashSet<Uuid>>`
  - the subset of `user_ids` whose *identity* may be shown. One query. Its
  `WHERE` clause is the per-player clause lifted verbatim from
  `is_game_visible_to_user` (`'public'` OR `'friends'` AND an accepted `friends`
  row either direction), plus a self case `u.id = $2`. A NULL viewer leaves only
  `'public'` passing. Doc-comment it as cross-referencing the canonical fn.

### 3b. `rust/web/src/game/server_fns.rs::get_game_details`

After the existing `player` lookup: when `player.is_none()`, require
`crate::db::is_game_visible_to_user(&pool, game_id, user.id)` and return the
existing `"Game not found"` `ServerFnError` when false. Players always pass.

### 3c. `rust/web/src/stats/mod.rs` - thread the viewer

All three server fns resolve `viewer_user_id: Option<Uuid>` via
`get_current_user().await?.map(|u| u.id)` (`get_player_profile` already does;
add it to the other two) and pass it to the query fns below.

### 3d. `rust/web/src/stats/queries.rs` - anonymize, never filter

- `opponents_by_game` takes `viewer: Option<Uuid>`. After the rows are mapped,
  call `db::visible_user_ids` on the distinct non-NULL opponent ids and mask
  every opponent not in the set: `user_id = None`, `name = "Anonymous"`. Bot
  rows (`user_id IS NULL`) untouched; counts, places and ordering unchanged.
- Its three callers - `finished_games`, `active_games`, `game_history` - gain
  the same `viewer` param and pass it through.
- `head_to_head` takes `viewer: Option<Uuid>` and masks the same way. This
  requires `stats::HeadToHead::user_id` to become `Option<Uuid>`; masked rows
  get `None` + `"Anonymous"` and are **not** merged together.
- Aggregates (`overall_totals`, `game_type_stats`, `rating_series`,
  `recent_form*`) are unchanged - they carry no identities.

### 3e. `rust/web/src/players.rs`

The head-to-head table renders `<A href="/players/{name}">`. Render plain text
instead when `user_id.is_none()`, matching `opponents_view`'s bot handling.

## 4. Non-goals

- `/ws` filtering (WP-42), rules/game-info pages (WP-49), export privacy
  (WP-48), the stats page-offset overflow (wd F46), the `db.rs` split (ws F42).
- Do not gate `get_game_logs` - already player-only. Do not hide profile pages
  themselves. Do not exclude any game from any aggregate.

## 5. Regression test cases

- `rust/web/src/db.rs` `#[cfg(test)] mod tests`, beside the existing
  `is_game_visible_to_user_*` tests: `visible_user_ids` over a matrix - public /
  friends-and-friend / friends-not-friend / private / self / `None` viewer.
  Plus a drift guard: for a viewer who is **not** a player of the game,
  `is_game_visible_to_user(game, viewer)` equals "every human player of that
  game is in `visible_user_ids`".
- `rust/web/tests/ssr_pages.rs` (working router harness - copy
  `game_page_logged_in_player_renders_game`): a logged-in **non-player**
  requesting an all-private game's page gets the clean "Game not found" render,
  not the board; a player of that game still gets the board.
- `rust/web/src/stats/queries.rs` `#[cfg(test)] mod tests`: `opponents_by_game`
  masks a private opponent (`user_id: None`, `"Anonymous"`) for an anonymous
  viewer and for a stranger, but not for themselves nor for an accepted friend
  when set to `'friends'`; the row is still present. Same cases for
  `head_to_head`.

## 6. Riders

None - both findings are major and in scope above.
