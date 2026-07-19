# Board Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a cached, deterministic initial-board render in the new game page's detail panel when a game is selected, so users can see what a game looks like before starting one.

**Architecture:** A new server fn `get_game_preview(game_version_id)` renders a fresh game via the existing stateless `Request::New` microservice path (fixed seed, placeholder players, lowest supported player count), converts the markup to themed HTML via the existing `brdgme_markup` pipeline, and caches the HTML in an in-memory `HashMap` keyed by game version id (the render is deterministic per version, so one cached copy serves every user). Failures are never cached. The detail panel (in `rust/web/src/new_game.rs`, added by the companion plan) fetches the preview lazily via a reactive `LocalResource` keyed on the selected version id, shows "Loading preview..." while pending, and renders nothing on error - the page is fully functional without this feature.

**Tech Stack:** Rust, Leptos 0.8 (`LocalResource`, `#[server]` fns), `std::sync::{LazyLock, RwLock}` for the in-memory cache, `brdgme_markup`/`brdgme_cmd::api`/`brdgme_game_client` (the existing game-render pipeline), SCSS (`rust/web/style/main.scss`).

**Spec:** `docs/superpowers/specs/2026-07-19-new-game-preview-design.md`. Companion plan (assumed fully implemented; this plan is strictly additive on top of it and touches none of its interfaces): `docs/superpowers/plans/2026-07-19-new-game-page.md`, which adds `rust/web/src/new_game.rs` (the `NewGamePage`/`GameBrowser` components), `GameTypeInfo`/`GameVersionInfo` with `weight`/`blurb` fields, and `db::find_game_type_player_counts`.

## Global Constraints

- Strictly severable: the new game page must be complete and fully functional if this feature is absent, disabled, or failing (spec).
- No DB schema changes; no new `game_types`/`game_versions` columns (spec: "No DB changes").
- Player names in the preview are generic placeholders; player count for the preview is the game's lowest supported count (spec).
- Any failure (microservice unreachable, render error, markup transform error) means the preview area simply does not render - no error surfaced to the user. Failures are never cached, so a later selection retries (spec, "Failure behavior").
- The preview container has a fixed max-height AND max-width with `overflow: auto` in both axes, and renders at ~50% scale via a font-size reduction (spec, "Size handling") - it must never widen the panel or the page.
- Colors only via `--mk-*` custom properties or `mk-fg-*`/`mk-bg-*` classes; must work under all themes; never hard-code a color (repo-wide constraint, inherited from the companion plan).
- All shell commands run from `/home/beefsack/Development/brdgme/rust` unless a step says otherwise.
- DB-backed tests need Postgres running with `DATABASE_URL` set, migrations applied. CI uses `DATABASE_URL=postgres://postgres:postgres@localhost/brdgme`. `#[sqlx::test]` provisions an isolated database per test.
- NEVER run `cargo test -p web` or `cargo check -p web` without `--features ssr` — the crate has no default features and fails to compile by design without them.
- Test commands (mirroring `.github/workflows/ci.yml`):
  - `cargo test --workspace --exclude web`
  - `cargo test -p web --features ssr`
  - `cargo clippy --workspace --exclude web --all-targets -- -D warnings`
  - `cargo clippy -p web --all-targets --features ssr -- -D warnings`
  - `cargo fmt --all -- --check`
- This plan introduces no new `sqlx::query!`/`query_as!`/`query_scalar!` macro calls (it reuses `db::find_game_version` and `db::find_game_type_player_counts`, both already implemented by the companion plan; test fixtures use the untyped `sqlx::query_scalar`/`sqlx::query_as` runtime API, same as the companion plan's Task 6 tests), so no `.sqlx/` offline-cache regeneration is required anywhere in this plan.
- Commit frequently, one commit per task minimum, conventional-commit style messages (`feat(web): ...`).

---

### Task 1: Preview cache and render core

**Files:**
- Create: `rust/web/src/game/preview.rs`
- Modify: `rust/web/src/game/mod.rs` (module declarations, lines 1-7)

**Interfaces:**
- Consumes: `crate::db::find_game_version(pool: &PgPool, id: Uuid) -> Result<Option<crate::models::game::GameVersion>>` (existing), `crate::db::find_game_type_player_counts(pool: &PgPool, game_version_id: Uuid) -> Result<Option<Vec<i32>>>` (existing, added by the companion plan's Task 5), `crate::game::client::request(client: &reqwest::Client, uri: &str, version_name: &str, request: &brdgme_cmd::api::Request) -> anyhow::Result<brdgme_cmd::api::Response>` (existing), `crate::error::internal` (existing), `brdgme_markup::{from_string, transform_semantic, html_class, SemanticPlayer}` (existing crate).
- Produces: `pub async fn get_or_render_preview(pool: &sqlx::PgPool, http_client: &reqwest::Client, game_version_id: Uuid) -> Result<String, ServerFnError>`. Task 2's server fn calls this exact function.

- [ ] **Step 1: Register the module**

In `rust/web/src/game/mod.rs`, change the top of the file from:

```rust
#[cfg(feature = "ssr")]
pub use brdgme_game_client as client;
#[cfg(feature = "ssr")]
pub mod export;
#[cfg(feature = "ssr")]
pub mod import;
pub mod server_fns;
```

to:

```rust
#[cfg(feature = "ssr")]
pub use brdgme_game_client as client;
#[cfg(feature = "ssr")]
pub mod export;
#[cfg(feature = "ssr")]
pub mod import;
#[cfg(feature = "ssr")]
pub mod preview;
pub mod server_fns;
```

- [ ] **Step 2: Write the failing tests**

Create `rust/web/src/game/preview.rs`:

```rust
//! #44 board preview (spec 2026-07-19-new-game-preview-design.md): renders a
//! fresh initial board for a game version using placeholder players and a
//! fixed seed, and caches the resulting HTML in memory keyed by game version
//! id. The render is deterministic per version, so the cache is valid across
//! all users and repopulates lazily after a restart. Failures are never
//! cached, so the next request retries from scratch. This whole module is
//! only ever reached via `server_fns::get_game_preview`; any failure here
//! must never affect game creation or the rest of the new game page - the
//! caller (new_game.rs) simply omits the preview on error.

use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use leptos::prelude::ServerFnError;
use uuid::Uuid;

use crate::error::internal;

/// Arbitrary but fixed: every render of a given game version must be
/// identical, which is what makes the in-memory cache valid indefinitely.
const PREVIEW_SEED: u64 = 42;

static PREVIEW_CACHE: LazyLock<RwLock<HashMap<Uuid, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Generic placeholder names for a preview render: "Player 1".."Player N".
fn placeholder_player_names(count: usize) -> Vec<String> {
    (1..=count).map(|n| format!("Player {n}")).collect()
}

/// Returns the cached preview HTML for `game_version_id`, if present.
fn cached(game_version_id: Uuid) -> Option<String> {
    PREVIEW_CACHE
        .read()
        .expect("preview cache lock poisoned")
        .get(&game_version_id)
        .cloned()
}

/// Stores `html` in the cache for `game_version_id`.
fn store(game_version_id: Uuid, html: String) {
    PREVIEW_CACHE
        .write()
        .expect("preview cache lock poisoned")
        .insert(game_version_id, html);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;

    #[test]
    fn placeholder_player_names_formats_sequential_names() {
        assert_eq!(placeholder_player_names(1), vec!["Player 1"]);
        assert_eq!(
            placeholder_player_names(3),
            vec!["Player 1", "Player 2", "Player 3"]
        );
    }

    #[sqlx::test]
    async fn get_or_render_preview_returns_cached_html_without_rendering(pool: PgPool) {
        let game_version_id = Uuid::new_v4();
        store(game_version_id, "<div>cached</div>".to_string());

        // No DB row exists for this id and the pool is never queried on a
        // cache hit - if this ever tried to render, it would fail (no such
        // game_version), proving the short-circuit works.
        let html = get_or_render_preview(&pool, &reqwest::Client::new(), game_version_id)
            .await
            .unwrap();

        assert_eq!(html, "<div>cached</div>");
    }

    #[sqlx::test]
    async fn get_or_render_preview_failure_is_not_cached(pool: PgPool) {
        let game_type_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_types (name, player_counts) VALUES ($1, $2) RETURNING id",
        )
        .bind("Preview Failure Test")
        .bind(vec![2i32, 3])
        .fetch_one(&pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, 'preview-fail-1', 'http://localhost:0/mock', true, false) RETURNING id",
        )
        .bind(game_type_id)
        .fetch_one(&pool)
        .await
        .unwrap();

        // Port 0 is not a reachable address: the request fails fast.
        let result = get_or_render_preview(&pool, &reqwest::Client::new(), game_version_id).await;

        assert!(result.is_err());
        assert_eq!(cached(game_version_id), None);
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr game::preview::
```
Expected: FAIL to compile — `cannot find function 'get_or_render_preview' in this scope`.

- [ ] **Step 4: Implement `render_preview` and `get_or_render_preview`**

In `rust/web/src/game/preview.rs`, insert the following between the `store` function and the `#[cfg(test)]` module:

```rust
/// Requests a fresh game from the game service using the game type's lowest
/// supported player count and placeholder player names, and converts the
/// resulting public markup to themed HTML. Does not touch the cache; callers
/// decide whether to store the result, so a failure here is never cached.
async fn render_preview(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_version_id: Uuid,
) -> Result<String, ServerFnError> {
    use brdgme_cmd::api::{Request, Response};

    let game_version = crate::db::find_game_version(pool, game_version_id)
        .await
        .map_err(internal("render_preview: find game version"))?
        .ok_or_else(|| ServerFnError::new("Game version not found"))?;

    let player_counts = crate::db::find_game_type_player_counts(pool, game_version_id)
        .await
        .map_err(internal("render_preview: find player counts"))?
        .ok_or_else(|| ServerFnError::new("Game type not found"))?;

    let lowest_count = player_counts.into_iter().min().unwrap_or(2).max(1) as usize;

    let resp = crate::game::client::request(
        http_client,
        &game_version.uri,
        &game_version.name,
        &Request::New {
            players: lowest_count,
            seed: Some(PREVIEW_SEED),
        },
    )
    .await
    .map_err(internal("render_preview: request new game"))?;

    let render = match resp {
        Response::New { public_render, .. } => public_render.render,
        _ => return Err(ServerFnError::new("Unexpected response from game service")),
    };

    let players: Vec<brdgme_markup::SemanticPlayer> = placeholder_player_names(lowest_count)
        .into_iter()
        .map(|name| brdgme_markup::SemanticPlayer { name })
        .collect();

    let (nodes, _) =
        brdgme_markup::from_string(&render).map_err(internal("render_preview: parse markup"))?;

    Ok(brdgme_markup::html_class(&brdgme_markup::transform_semantic(
        &nodes, &players,
    )))
}

/// Returns the cached preview HTML for `game_version_id`, rendering and
/// caching it on first request. Failures are never cached (spec: "Failure
/// behavior") - the next call for the same id retries from scratch.
pub async fn get_or_render_preview(
    pool: &sqlx::PgPool,
    http_client: &reqwest::Client,
    game_version_id: Uuid,
) -> Result<String, ServerFnError> {
    if let Some(html) = cached(game_version_id) {
        return Ok(html);
    }
    let html = render_preview(pool, http_client, game_version_id).await?;
    store(game_version_id, html.clone());
    Ok(html)
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo test -p web --features ssr game::preview::
```
Expected: PASS (3 passed).

- [ ] **Step 6: Clippy check**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings
```
Expected: clean.

- [ ] **Step 7: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/game/preview.rs rust/web/src/game/mod.rs
git commit -m "feat(web): in-memory board preview cache and render core (#44)"
```

---

### Task 2: `get_game_preview` server fn

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (add `get_game_preview` immediately after `get_available_game_types`, added by the companion plan's Task 4)

**Interfaces:**
- Consumes: `crate::game::preview::get_or_render_preview(pool: &PgPool, http_client: &reqwest::Client, game_version_id: Uuid) -> Result<String, ServerFnError>` (Task 1).
- Produces: `#[server(GetGamePreview, "/api")] pub async fn get_game_preview(game_version_id: Uuid) -> Result<String, ServerFnError>`. Task 4's UI calls `crate::game::server_fns::get_game_preview`.

- [ ] **Step 1: Add the server fn**

In `rust/web/src/game/server_fns.rs`, immediately after the closing brace of `get_available_game_types`, add:

```rust
/// #44 board preview (spec 2026-07-19-new-game-preview-design.md): returns
/// cached preview HTML for a game version, rendering it on first request.
/// Login required, matching every other page server fn here. Any failure is
/// returned as an error; the UI (new_game.rs) simply omits the preview on
/// error rather than surfacing it - this endpoint failing must never affect
/// the rest of the new game page.
#[server(GetGamePreview, "/api")]
pub async fn get_game_preview(game_version_id: Uuid) -> Result<String, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let http_client = expect_context::<reqwest::Client>();
    let _ = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    crate::game::preview::get_or_render_preview(&pool, &http_client, game_version_id).await
}
```

- [ ] **Step 2: Verify compile, clippy, and existing tests**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings && cargo test -p web --features ssr game::preview::
```
Expected: clippy clean; 3 tests still PASS. (No new test here: `get_game_preview` is a straight pass-through to `get_or_render_preview`, checked at compile time — same rationale as the companion plan's Task 4 for `get_available_game_types`'s field-copying body.)

- [ ] **Step 3: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/game/server_fns.rs
git commit -m "feat(web): get_game_preview server fn (#44)"
```

---

### Task 3: SCSS for the preview container

**Files:**
- Modify: `rust/web/style/main.scss` (append after the block added by the companion plan's Task 7)

**Interfaces:**
- Consumes: `--mk-foreground`, `--mk-grey` (existing `--mk-*` vars).
- Produces: classes `.new-game-preview`, `.new-game-preview-loading`, consumed by Task 4.

- [ ] **Step 1: Append the styles**

Add to the end of `rust/web/style/main.scss`:

```scss
/* #44 board preview (spec 2026-07-19-new-game-preview-design.md). Renders
   vary wildly in both dimensions (e.g. Cathedral is large both ways), so the
   container is constrained on BOTH axes with independent scrollbars - it
   must never widen the panel or the page. Scaled via font-size (renders are
   text-based, so this scales cleanly without transforms); the exact
   max-height/scale below is a starting point, tuned against Cathedral
   (largest) and the smallest render during manual verification (Task 5). */
.new-game-preview {
  margin-top: 0.75em;
  max-height: 20em;
  max-width: 100%;
  overflow: auto;
  font-size: 50%;
  border: 1px solid color-mix(in srgb, var(--mk-foreground) 25%, transparent);
  border-radius: 4px;
  padding: 0.5em;
}

.new-game-preview-loading {
  margin-top: 0.75em;
  font-size: 0.85em;
  color: var(--mk-grey);
}
```

- [ ] **Step 2: Verify the stylesheet still builds**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings
```
Expected: clean (cargo-leptos compiles the SCSS at build time in dev/CI; clippy at minimum confirms nothing else broke — if a `just`/`cargo leptos` dev build is running, confirm no SCSS compile error in its output).

- [ ] **Step 3: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/style/main.scss
git commit -m "feat(web): styles for the board preview container (#44)"
```

---

### Task 4: UI integration in the detail panel

**Files:**
- Modify: `rust/web/src/new_game.rs` (component `GameBrowser`, added by the companion plan's Task 8)

**Interfaces:**
- Consumes: `crate::game::server_fns::get_game_preview(game_version_id: Uuid) -> Result<String, ServerFnError>` (Task 2); `.new-game-preview`/`.new-game-preview-loading` (Task 3); the existing `selected_version_id: ReadSignal<Option<Uuid>>` signal and the panel's `gt`-guarded view branch (both from the companion plan's Task 8).
- Produces: no new public interface — this is the leaf feature; the preview only ever renders inside the existing panel.

- [ ] **Step 1: Add the reactive preview resource**

In `rust/web/src/new_game.rs`, inside `GameBrowser`, immediately after the line:

```rust
    let suggestions = LocalResource::new(crate::friends::get_opponent_suggestions);
```

add:

```rust
    // #44 board preview: refetches whenever the selected version changes
    // (the signal read happens synchronously before the async block, the
    // same pattern used for `friends.rs`'s `refresh`-tracked resource).
    // Loading = None; no selection or a failed fetch = Some(None), which
    // renders nothing (spec: failures are silent, never surfaced as an
    // error); a successful fetch = Some(Some(html)).
    let preview_html = LocalResource::new(move || {
        let version_id = selected_version_id.get();
        async move {
            match version_id {
                Some(id) => crate::game::server_fns::get_game_preview(id).await.ok(),
                None => None,
            }
        }
    });
```

- [ ] **Step 2: Render the preview in the panel**

In `rust/web/src/new_game.rs`, inside the panel's `view!` block, change:

```rust
                        {(!gt.blurb.is_empty())
                            .then(|| view! { <p class="new-game-blurb">{gt.blurb.clone()}</p> })}
                        <form on:submit=on_submit>
```

to:

```rust
                        {(!gt.blurb.is_empty())
                            .then(|| view! { <p class="new-game-blurb">{gt.blurb.clone()}</p> })}
                        {move || match preview_html.get() {
                            None => view! {
                                <p class="new-game-preview-loading">"Loading preview..."</p>
                            }
                            .into_any(),
                            Some(None) => ().into_any(),
                            Some(Some(html)) => {
                                view! { <div class="new-game-preview" inner_html=html></div> }
                                    .into_any()
                            }
                        }}
                        <form on:submit=on_submit>
```

- [ ] **Step 3: Verify compile, clippy, and existing tests**

Run:
```bash
cd /home/beefsack/Development/brdgme/rust && cargo clippy -p web --all-targets --features ssr -- -D warnings && cargo test -p web --features ssr new_game::
```
Expected: clippy clean; existing `new_game::` unit tests still PASS (this task adds no new Rust unit tests — it's Leptos view wiring, verified manually next, matching the companion plan's convention for UI-only steps in its Tasks 8-9).

- [ ] **Step 4: Manual verification in the browser**

On `/games`:
- Select a game: "Loading preview..." appears briefly, then a bordered, scrollable board render replaces it, scaled down and themed.
- Select a different game: the preview updates to the new game's board (proves the resource refetches on reselection).
- With the game microservice for one version stopped (or pointed at an unreachable URL in dev config), select that game: the panel, blurb, player-count radios and form all still work normally, and the preview area shows nothing (no loading text stuck, no error text).
- Re-select a working game afterward: its preview loads normally (proves a prior failure did not poison anything for other versions, and that failures retry on the next selection).

- [ ] **Step 5: Commit**

```bash
cd /home/beefsack/Development/brdgme
git add rust/web/src/new_game.rs
git commit -m "feat(web): lazy board preview in the new game detail panel (#44)"
```

---

### Task 5: Full verification pass

**Files:** none created; fixes only if checks fail.

**Interfaces:**
- Consumes: everything above.
- Produces: a green CI-equivalent run and a manual sign-off covering the spec's size-handling and failure-behavior requirements.

- [ ] **Step 1: Run the full CI-equivalent check suite**

Run, from `/home/beefsack/Development/brdgme/rust` (Postgres + `DATABASE_URL` required):
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude web --all-targets -- -D warnings
cargo clippy -p web --all-targets --features ssr -- -D warnings
cd web && cargo sqlx prepare --check -- --tests --features ssr --all-targets && cd ..
cargo test --workspace --exclude web
cargo test -p web --features ssr
```
Expected: every command exits 0. `sqlx prepare --check` should pass unchanged since this plan added no compile-checked queries; if it fails, re-run `cargo sqlx prepare -- --tests --features ssr --all-targets` in `rust/web` and commit the `.sqlx/` diff.

- [ ] **Step 2: Manual verification — size handling and failure behavior**

- Cathedral (largest render in both width and height): select it, confirm the preview container shows both a vertical and a horizontal scrollbar rather than growing past its `max-height`/`max-width`, and that the surrounding panel/page never widen.
- A small render (e.g. Tic-Tac-Toe or Battleship): confirm the container isn't left with excessive empty space and the ~50% scale still reads clearly.
- Themes: spot-check brdgme-light, brdgme-dark, and one tritanopia theme — the preview's border and any text inside the render must stay legible.
- Breakpoints: confirm the preview behaves the same in the >= 60em two-pane layout and the < 60em single-column layout (companion plan's Task 8/9 breakpoint at 60em).
- Re-confirm the failure path from Task 4 Step 4 one more time end-to-end after all changes: an unreachable game version's preview stays silently absent while the rest of the panel and the "Create game" flow work normally.

- [ ] **Step 3: Tune container sizing if needed**

If Step 2 shows the `max-height`/`font-size` values from Task 3 are wrong (too much scroll on small renders, or Cathedral still overflows), adjust the two values in `.new-game-preview` (`rust/web/style/main.scss`) and re-check Step 2's Cathedral/small-render cases.

- [ ] **Step 4: Commit any fixes**

```bash
cd /home/beefsack/Development/brdgme
git add -A
git commit -m "fix(web): verification and size tuning for board preview (#44)"
```
(Skip if nothing changed.)

---

## Self-Review Notes (kept for the executor)

- Spec coverage: server-cached render off the stateless `Request::New` path with fixed seed and placeholder players/lowest count (Task 1), server fn delivery (Task 2), size handling with both-axis constraints and font-size scaling (Task 3, tuned in Task 5), lazy fetch / "Loading preview..." / silent-absence-on-error / retry-on-reselect (Task 4), severability (additive-only design across all tasks, explicitly manually re-verified in Task 4 Step 4 and Task 5 Step 2), testing section (cache hit path, placeholder-player construction, failure path not poisoning the cache - all three in Task 1; manual multi-game/theme pass in Task 5). Out-of-scope items (mid-game snapshots, real finished games, rules display) are correctly absent.
- Placeholder scan: no TBD/TODO markers; every step shows complete code; DB test fixtures use the same untyped `sqlx::query_scalar`/`sqlx::query_as` style already established by the companion plan rather than inventing new macro-checked queries.
- Type consistency: `get_or_render_preview(pool: &sqlx::PgPool, http_client: &reqwest::Client, game_version_id: Uuid) -> Result<String, ServerFnError>` (Task 1) is the exact signature called by `get_game_preview` (Task 2) and matches `crate::game::server_fns::get_game_preview(game_version_id: Uuid) -> Result<String, ServerFnError>` called from `new_game.rs` (Task 4). `PREVIEW_CACHE`/`cached`/`store` are private to `preview.rs`, used consistently by both `get_or_render_preview` and its tests. `Response::New`'s field shape (`game`, `logs`, `public_render: PubRender { pub_state, render }`, `player_renders`, `seed`) was verified directly against `rust/lib/cmd/src/api.rs:84-90`, not assumed.
