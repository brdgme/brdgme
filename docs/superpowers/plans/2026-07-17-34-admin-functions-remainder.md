# #34 Admin Functions Remainder (Force-Delete, Export, Import CLI) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete #34 - admin force-delete of a game, admin JSON export of a game, and a dev-only CLI that imports an exported bundle into local Postgres.

**Architecture:** Force-delete is a leptos server fn guarded by `is_user_admin` calling a transactional `db::delete_game`, with a confirm-dialog link in `GameMeta`. Export is a plain admin-guarded Axum route (`GET /admin/games/{id}/export`) serving a versioned JSON bundle built by a new `game/export.rs` module. Import is a `[[bin]]` in `rust/web` (`import-game`) wrapping a testable `game/import.rs::import_bundle` that maps the bundle onto the local game version by name and creates placeholder users.

**Tech Stack:** Rust, Leptos 0.8 (server fns + components), Axum 0.8, sqlx (query! macros against live `DATABASE_URL`), tower-sessions, serde_json, time.

**Spec:** `docs/superpowers/specs/2026-07-11-34-admin-functions-design.md` (D3, D4, D5; D1/D2 patterns already exist - follow `bump_bot_turns` in `rust/web/src/game/server_fns.rs:679` and the admin-gated `<Show>` in `rust/web/src/components/game.rs:124`).

## Global Constraints

- All work in `rust/web` (crate `web`). No new migrations - the schema already supports everything.
- sqlx `query!`/`query_as!` macros compile against a live `DATABASE_URL` (dev/CI convention; `.env` in `rust/` workspace). Use the macros, not runtime-checked queries, matching db.rs style (single exception documented at `db.rs:504` does not apply here).
- CI gates (from `.github/workflows/ci.yml`): `cargo clippy -p web --all-targets --features ssr -- -D warnings` and `cargo test -p web --features ssr`. Run both (plus `cargo fmt`) before every commit. Run from `/home/beefsack/Development/brdgme/rust`.
- Admin enforcement is always server-side (`crate::db::is_user_admin`); UI gating on `viewer_is_admin` is cosmetic only.
- Export bundles must never contain email addresses (spec D4).
- No rating rewind on delete (spec D3).
- Do not touch existing unpushed commits on master; one commit per task.

---

### Task 1: `db::delete_game` (transactional hard delete)

**Files:**
- Modify: `rust/web/src/db.rs` (new function near `concede_game` at line ~1039; tests appended to the existing `mod tests`)

**Interfaces:**
- Produces: `pub async fn delete_game(pool: &PgPool, game_id: Uuid) -> Result<bool>` - `Ok(true)` when a game row was deleted, `Ok(false)` when no such game. Used by Task 2.
- Consumes: existing test fixtures `make_user`, `make_game_type_and_version`, `make_game_with_players` in db.rs `mod tests`.

Deletion order matters for FKs: `game_log_targets` references `game_logs` AND `game_players`; `game_players.game_bot_id` references `game_bots`; `games.restarted_game_id` self-references `games`.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `rust/web/src/db.rs`:

```rust
    // --- delete_game (#34 force delete, spec D3) ---

    #[sqlx::test]
    async fn delete_game_removes_all_dependent_rows(pool: PgPool) {
        let user = make_user(&pool, "deleter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let game = make_game_with_players(&pool, game_version_id, user.id, &[], 1, &[0]).await;

        // A log targeted at the human player, so game_log_targets is exercised.
        let log_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at)
             VALUES ($1, 'hello', false, timezone('utc', now())) RETURNING id",
            game.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let player_id: Uuid = sqlx::query_scalar!(
            "SELECT id FROM game_players WHERE game_id = $1 AND user_id = $2",
            game.id,
            user.id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
            log_id,
            player_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_game(&pool, game.id).await.unwrap();
        assert!(deleted);

        for (table, count) in [
            ("games", count_rows(&pool, "games").await),
            ("game_players", count_rows(&pool, "game_players").await),
            ("game_bots", count_rows(&pool, "game_bots").await),
            ("game_logs", count_rows(&pool, "game_logs").await),
            ("game_log_targets", count_rows(&pool, "game_log_targets").await),
        ] {
            assert_eq!(count, 0, "expected no rows left in {}", table);
        }
        // The user survives the delete.
        assert_eq!(count_rows(&pool, "users").await, 1);
    }

    #[sqlx::test]
    async fn delete_game_nulls_restarted_game_id_references(pool: PgPool) {
        let user = make_user(&pool, "restarter").await;
        let (_, game_version_id) = make_game_type_and_version(&pool).await;
        let old_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[]).await;
        let new_game = make_game_with_players(&pool, game_version_id, user.id, &[], 0, &[0]).await;
        sqlx::query!(
            "UPDATE games SET restarted_game_id = $1 WHERE id = $2",
            new_game.id,
            old_game.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_game(&pool, new_game.id).await.unwrap();
        assert!(deleted);

        let restarted: Option<Uuid> =
            sqlx::query_scalar!("SELECT restarted_game_id FROM games WHERE id = $1", old_game.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(restarted, None);
    }

    #[sqlx::test]
    async fn delete_game_returns_false_for_missing_game(pool: PgPool) {
        let deleted = delete_game(&pool, Uuid::new_v4()).await.unwrap();
        assert!(!deleted);
    }

    async fn count_rows(pool: &PgPool, table: &str) -> i64 {
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table))
            .fetch_one(pool)
            .await
            .unwrap()
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p web --features ssr delete_game`
Expected: compile error - `delete_game` not found.

- [ ] **Step 3: Implement `delete_game`**

Add to `rust/web/src/db.rs` (after `concede_game`):

```rust
/// #34 admin force delete (spec D3): hard-deletes a game and all dependent
/// rows in one transaction. Any game referencing the deleted one via
/// `restarted_game_id` has that link nulled (making it restartable again).
/// Ratings are deliberately NOT rewound. Returns false if the game did not
/// exist.
pub async fn delete_game(pool: &PgPool, game_id: Uuid) -> Result<bool> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "UPDATE games SET restarted_game_id = NULL, updated_at = NOW() WHERE restarted_game_id = $1",
        game_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM game_log_targets WHERE game_log_id IN (SELECT id FROM game_logs WHERE game_id = $1)",
        game_id
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!("DELETE FROM game_logs WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    // game_players before game_bots: game_players.game_bot_id FK.
    sqlx::query!("DELETE FROM game_players WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM game_bots WHERE game_id = $1", game_id)
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query!("DELETE FROM games WHERE id = $1", game_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p web --features ssr delete_game`
Expected: 3 passed.

- [ ] **Step 5: Gate and commit**

```bash
cargo fmt
cargo clippy -p web --all-targets --features ssr -- -D warnings
git add rust/web/src/db.rs
git commit -m "feat #34: transactional hard delete of a game in db layer"
```

---

### Task 2: `force_delete_game` server fn + GameMeta UI

**Files:**
- Modify: `rust/web/src/game/server_fns.rs` (new impl fn + server fn after `bump_bot_turns`; tests in existing `mod tests`)
- Modify: `rust/web/src/components/game.rs` (admin action in `GameMeta`, lines ~124-131 area)

**Interfaces:**
- Consumes: `crate::db::delete_game(pool, game_id) -> Result<bool>` (Task 1), `crate::db::is_user_admin`, `GameBroadcaster::broadcast_game_update`.
- Produces: `#[server] pub async fn force_delete_game(game_id: Uuid) -> Result<(), ServerFnError>` and its generated `ForceDeleteGame` action type, used by the UI.

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `rust/web/src/game/server_fns.rs`:

```rust
    #[sqlx::test]
    async fn force_delete_game_rejects_non_admin(pool: PgPool) {
        let user_id = make_user(&pool, "notadmin").await;
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: user_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
            },
        )
        .await
        .unwrap();

        let result = force_delete_game_impl(&pool, user_id, game.id).await;
        assert!(result.is_err());
        // Game must still exist.
        assert!(crate::db::find_game(&pool, game.id).await.unwrap().is_some());
    }

    #[sqlx::test]
    async fn force_delete_game_deletes_for_admin(pool: PgPool) {
        let admin_id = make_user(&pool, "admin").await;
        sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin_id)
            .execute(&pool)
            .await
            .unwrap();
        let game_version_id = make_game_version(&pool).await;
        let game = crate::db::create_game_with_users(
            &pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: admin_id,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[],
                chat_id: None,
                game_state: "state",
            },
        )
        .await
        .unwrap();

        force_delete_game_impl(&pool, admin_id, game.id).await.unwrap();
        assert!(crate::db::find_game(&pool, game.id).await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn force_delete_game_missing_game_errors(pool: PgPool) {
        let admin_id = make_user(&pool, "admin2").await;
        sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin_id)
            .execute(&pool)
            .await
            .unwrap();
        let result = force_delete_game_impl(&pool, admin_id, Uuid::new_v4()).await;
        assert!(result.is_err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p web --features ssr force_delete`
Expected: compile error - `force_delete_game_impl` not found.

- [ ] **Step 3: Implement the server fn**

Add to `rust/web/src/game/server_fns.rs` after `bump_bot_turns` (~line 707):

```rust
/// Admin-only hard delete, minus leptos context plumbing so tests can drive
/// it. Admins need not be players in the game.
#[cfg(feature = "ssr")]
async fn force_delete_game_impl(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    game_id: Uuid,
) -> Result<(), ServerFnError> {
    let is_admin = crate::db::is_user_admin(pool, user_id)
        .await
        .map_err(internal("force_delete_game: check admin"))?;
    if !is_admin {
        return Err(ServerFnError::new("Admin access required"));
    }

    let deleted = crate::db::delete_game(pool, game_id)
        .await
        .map_err(internal("force_delete_game: delete game"))?;
    if !deleted {
        return Err(ServerFnError::new("Game not found"));
    }
    Ok(())
}

#[server(ForceDeleteGame, "/api")]
pub async fn force_delete_game(game_id: Uuid) -> Result<(), ServerFnError> {
    use crate::auth::server::get_current_user;
    use crate::websocket::GameBroadcaster;
    use sqlx::PgPool;

    let pool = expect_context::<PgPool>();
    let broadcaster = expect_context::<GameBroadcaster>();
    let user = get_current_user()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    force_delete_game_impl(&pool, user.id, game_id).await?;

    // Spec D3: broadcast the usual game-update signal so open clients
    // refresh (their refetch will surface "Game not found"). No bot trigger.
    broadcaster.broadcast_game_update(game_id).await;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p web --features ssr force_delete`
Expected: 3 passed.

- [ ] **Step 5: Add the GameMeta action**

In `rust/web/src/components/game.rs`:

1. Extend the import at line 1:

```rust
use crate::game::server_fns::{
    BumpBotTurns, ConcedeGame, ForceDeleteGame, GameViewData, PlayerViewData, RestartGame,
    SubmitCommand, UndoGame,
};
```

2. In `GameMeta`, alongside the other `ServerAction`s (~line 41):

```rust
    let force_delete_action = ServerAction::<ForceDeleteGame>::new();
```

3. After the restart-navigation `Effect` (~line 65) - note `use_navigate` is called a second time because the restart effect consumes the first:

```rust
    // Navigate away after force delete (spec D3); bump the sidebar trigger so
    // the deleted game drops out of the active-games list.
    let navigate_after_delete = use_navigate();
    Effect::new(move |_| {
        if let Some(Ok(())) = force_delete_action.value().get() {
            trigger.set_last_update.update(|n| *n += 1);
            navigate_after_delete("/", NavigateOptions::default());
        }
    });
```

4. In the `game-actions` view block, after the bump-bot `<Show>` (~line 131):

```rust
                        <Show when=move || viewer_is_admin>
                            <div>
                                <a href="#" on:click=move |ev| {
                                    ev.prevent_default();
                                    let confirmed = web_sys::window()
                                        .and_then(|w| w.confirm_with_message("Permanently delete this game for all players? This cannot be undone.").ok())
                                        .unwrap_or(false);
                                    if confirmed {
                                        force_delete_action.dispatch(ForceDeleteGame { game_id });
                                    }
                                }>"Delete game (admin)"</a>
                            </div>
                        </Show>
```

- [ ] **Step 6: Gate and commit**

```bash
cargo fmt
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
git add rust/web/src/game/server_fns.rs rust/web/src/components/game.rs
git commit -m "feat #34: admin force-delete game (server fn + GameMeta action)"
```

---

### Task 3: JSON export bundle + admin Axum route + UI link

**Files:**
- Create: `rust/web/src/game/export.rs`
- Modify: `rust/web/src/game/mod.rs` (register module)
- Modify: `rust/web/src/router.rs` (add route next to `/ws`, line ~129)
- Modify: `rust/web/Cargo.toml` (time features for rfc3339 serde)
- Modify: `rust/web/src/components/game.rs` (export link in the admin `<Show>` from Task 2)
- Test: `rust/web/tests/ssr_pages.rs` (route auth + content tests)

**Interfaces:**
- Consumes: `crate::db::find_game_extended`, `crate::auth::session::{get_user_from_session, validate_session_token}`, `crate::db::is_user_admin`, `AppState`.
- Produces (Task 4 depends on these exact types):
  - `pub const BUNDLE_SCHEMA_VERSION: u32 = 1;`
  - `pub struct ExportBundle { schema_version: u32, exported_at: OffsetDateTime, game_type_name: String, game_version_name: String, game_version_uri: String, game: BundleGame, players: Vec<BundlePlayer>, bots: Vec<BundleBot>, logs: Vec<BundleLog> }` (all fields `pub`, all structs `Serialize + Deserialize + Debug + Clone`)
  - `pub async fn build_export_bundle(pool: &sqlx::PgPool, game_id: Uuid) -> anyhow::Result<Option<ExportBundle>>`
  - `pub async fn admin_export_game(...) -> axum::response::Response` (route handler)

Linking is by `position` (players) and `name` (bots), not raw UUIDs, so import needs no ID remapping. `BundleGame.id` keeps the original prod id for reference in bug reports only.

- [ ] **Step 1: Enable rfc3339 serde for `time`**

In `rust/web/Cargo.toml` change:

```toml
time = { version = "0.3", features = ["serde", "formatting", "parsing"] }
```

(`time::serde::rfc3339` requires formatting + parsing; additive, compiles for both wasm and ssr targets.)

- [ ] **Step 2: Write the failing route tests**

Append to `rust/web/tests/ssr_pages.rs` (reuse existing helpers `make_state`, `make_user`, `login_cookie`, `make_game_version`, `get`; create the game via `web::db::create_game_with_users` as other tests in this file do):

```rust
#[sqlx::test]
async fn admin_export_route_requires_login(pool: PgPool) {
    let app = build_router(make_state(pool).await).await;
    let (status, _, _) = get(
        app,
        &format!("/admin/games/{}/export", uuid::Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn admin_export_route_rejects_non_admin(pool: PgPool) {
    let user = make_user(&pool, "pleb").await;
    let cookie = login_cookie(&pool, &user, "pleb@example.com").await;
    let app = build_router(make_state(pool).await).await;
    let (status, _, _) = get(
        app,
        &format!("/admin/games/{}/export", uuid::Uuid::new_v4()),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn admin_export_route_returns_bundle_without_emails(pool: PgPool) {
    let admin = make_user(&pool, "boss").await;
    sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin.id)
        .execute(&pool)
        .await
        .unwrap();
    let cookie = login_cookie(&pool, &admin, "boss@example.com").await;

    let game_version_id = make_game_version(&pool, "http://localhost:0/mock").await;
    let game = web::db::create_game_with_users(
        &pool,
        web::db::CreateGameOpts {
            game_version_id,
            whose_turn: &[0],
            eliminated: &[],
            placings: &[],
            points: &[],
            creator_id: admin.id,
            opponent_ids: &[],
            opponent_emails: &[],
            bot_slots: &[web::game::server_fns::BotSlot {
                name: "Botty".to_string(),
                difficulty: "easy".to_string(),
            }],
            chat_id: None,
            game_state: "opaque_state_blob",
        },
    )
    .await
    .unwrap();

    let app = build_router(make_state(pool).await).await;
    let (status, body, content_type) =
        get(app, &format!("/admin/games/{}/export", game.id), Some(&cookie)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(content_type.starts_with("application/json"));
    let bundle: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(bundle["schema_version"], 1);
    assert_eq!(bundle["game"]["game_state"], "opaque_state_blob");
    assert_eq!(bundle["players"].as_array().unwrap().len(), 2);
    assert_eq!(bundle["bots"][0]["name"], "Botty");
    // Spec D4: no email addresses in the bundle, ever.
    assert!(!body.contains("boss@example.com"));
    assert!(!body.contains('@'));
}

#[sqlx::test]
async fn admin_export_route_missing_game_404s(pool: PgPool) {
    let admin = make_user(&pool, "boss2").await;
    sqlx::query!("UPDATE users SET is_admin = true WHERE id = $1", admin.id)
        .execute(&pool)
        .await
        .unwrap();
    let cookie = login_cookie(&pool, &admin, "boss2@example.com").await;
    let app = build_router(make_state(pool).await).await;
    let (status, _, _) = get(
        app,
        &format!("/admin/games/{}/export", uuid::Uuid::new_v4()),
        Some(&cookie),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
```

Adjust helper call shapes to the file's actual signatures if they differ (e.g. `get`'s return tuple order) - the file's existing tests are the source of truth.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p web --features ssr --test ssr_pages admin_export`
Expected: 404s / compile errors - route and module don't exist yet.

- [ ] **Step 4: Implement `game/export.rs`**

Create `rust/web/src/game/export.rs`:

```rust
//! #34 admin game export (spec D4): a versioned JSON bundle for pulling a
//! prod game into a local dev environment. Served from an admin-guarded
//! plain Axum route (not a leptos server fn) because it downloads as a file.
//! Never includes email addresses - the bundle may get pasted into issues.
#![cfg(feature = "ssr")]

use crate::state::AppState;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use time::{OffsetDateTime, PrimitiveDateTime};
use tower_sessions::Session;
use uuid::Uuid;

pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBundle {
    pub schema_version: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub exported_at: OffsetDateTime,
    pub game_type_name: String,
    pub game_version_name: String,
    /// The exporting environment's game service URI - will not resolve
    /// elsewhere; the import CLI maps to the local URI by game type name.
    pub game_version_uri: String,
    pub game: BundleGame,
    pub players: Vec<BundlePlayer>,
    pub bots: Vec<BundleBot>,
    pub logs: Vec<BundleLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleGame {
    /// Original id in the exporting environment - reference only, import
    /// assigns fresh ids.
    pub id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<PrimitiveDateTime>,
    pub game_state: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePlayer {
    pub position: i32,
    /// Display name only - user name or bot name, never an email.
    pub name: String,
    /// `Some(game_bots.name)` when this seat is a bot; `None` for humans.
    pub bot_name: Option<String>,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub place: Option<i32>,
    pub is_eliminated: bool,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub rating_change: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleBot {
    pub name: String,
    pub difficulty: String,
    pub personality: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleLog {
    pub body: String,
    pub is_public: bool,
    pub logged_at: PrimitiveDateTime,
    pub created_at: PrimitiveDateTime,
    /// Positions of the players this (private) log targets.
    pub target_positions: Vec<i32>,
}

pub async fn build_export_bundle(
    pool: &PgPool,
    game_id: Uuid,
) -> anyhow::Result<Option<ExportBundle>> {
    let Some(ge) = crate::db::find_game_extended(pool, game_id).await? else {
        return Ok(None);
    };

    // game_bots.personality is not on the GameBot model; fetch directly.
    let bots = sqlx::query!(
        "SELECT name, difficulty, personality FROM game_bots WHERE game_id = $1 ORDER BY name",
        game_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|b| BundleBot {
        name: b.name,
        difficulty: b.difficulty,
        personality: b.personality,
    })
    .collect();

    let log_rows = sqlx::query!(
        "SELECT id, body, is_public, logged_at, created_at
         FROM game_logs WHERE game_id = $1 ORDER BY logged_at, id",
        game_id
    )
    .fetch_all(pool)
    .await?;
    let target_rows = sqlx::query!(
        "SELECT glt.game_log_id, gp.position
         FROM game_log_targets glt
         JOIN game_players gp ON gp.id = glt.game_player_id
         WHERE gp.game_id = $1",
        game_id
    )
    .fetch_all(pool)
    .await?;
    let logs = log_rows
        .into_iter()
        .map(|l| BundleLog {
            target_positions: target_rows
                .iter()
                .filter(|t| t.game_log_id == l.id)
                .map(|t| t.position)
                .collect(),
            body: l.body,
            is_public: l.is_public,
            logged_at: l.logged_at,
            created_at: l.created_at,
        })
        .collect();

    let players = ge
        .game_players
        .iter()
        .map(|p| BundlePlayer {
            position: p.game_player.position,
            name: p.name().to_string(),
            bot_name: p.game_bot.as_ref().map(|b| b.name.clone()),
            color: p.game_player.color.clone(),
            has_accepted: p.game_player.has_accepted,
            is_turn: p.game_player.is_turn,
            place: p.game_player.place,
            is_eliminated: p.game_player.is_eliminated,
            points: p.game_player.points,
            undo_game_state: p.game_player.undo_game_state.clone(),
            rating_change: p.game_player.rating_change,
        })
        .collect();

    Ok(Some(ExportBundle {
        schema_version: BUNDLE_SCHEMA_VERSION,
        exported_at: OffsetDateTime::now_utc(),
        game_type_name: ge.game_type.name,
        game_version_name: ge.game_version.name,
        game_version_uri: ge.game_version.uri,
        game: BundleGame {
            id: ge.game.id,
            is_finished: ge.game.is_finished,
            finished_at: ge.game.finished_at,
            game_state: ge.game.game_state,
            created_at: ge.game.created_at,
            updated_at: ge.game.updated_at,
        },
        players,
        bots,
        logs,
    }))
}

/// `GET /admin/games/{id}/export`. Session + is_admin checked server-side
/// (spec D1/D4); registered before the session layer wrap in router.rs so
/// the tower-sessions extractor works.
pub async fn admin_export_game(
    State(state): State<AppState>,
    session: Session,
    Path(game_id): Path<Uuid>,
) -> Response {
    let Some(session_user) = crate::auth::session::get_user_from_session(&session).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match crate::auth::session::validate_session_token(&state.pool, session_user.auth_token_id)
        .await
    {
        Ok(true) => {}
        Ok(false) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::error!("admin_export_game: validate token: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
    match crate::db::is_user_admin(&state.pool, session_user.id).await {
        Ok(true) => {}
        Ok(false) => return StatusCode::FORBIDDEN.into_response(),
        Err(e) => {
            tracing::error!("admin_export_game: check admin: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    match build_export_bundle(&state.pool, game_id).await {
        Ok(Some(bundle)) => {
            let disposition = format!("attachment; filename=\"brdgme-game-{}.json\"", game_id);
            (
                [(
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_str(&disposition)
                        .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
                )],
                Json(bundle),
            )
                .into_response()
        }
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("admin_export_game: build bundle: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
```

Register the module in `rust/web/src/game/mod.rs` (top of file, next to the other mods):

```rust
#[cfg(feature = "ssr")]
pub mod export;
```

(If `game/mod.rs` gates ssr-only modules differently, follow its existing pattern.)

- [ ] **Step 5: Register the route**

In `rust/web/src/router.rs`, directly below the `/ws` route (line ~129) - it must sit before `.layer(session_layer)` so the session extractor has a session to read, same as `/ws` and unlike `/healthz`:

```rust
        .route(
            "/admin/games/{id}/export",
            axum::routing::get(crate::game::export::admin_export_game),
        )
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p web --features ssr --test ssr_pages admin_export`
Expected: 4 passed.

- [ ] **Step 7: Add the export link to GameMeta**

In `rust/web/src/components/game.rs`, inside the admin `<Show>` added in Task 2, above the delete link:

```rust
                            <div>
                                <a href=format!("/admin/games/{}/export", game_id)>
                                    "Export JSON (admin)"
                                </a>
                            </div>
```

(Plain anchor - the browser follows the Content-Disposition header and downloads; no leptos action involved.)

- [ ] **Step 8: Gate and commit**

```bash
cargo fmt
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
git add rust/web/src/game/export.rs rust/web/src/game/mod.rs rust/web/src/router.rs rust/web/src/components/game.rs rust/web/Cargo.toml rust/Cargo.lock
git commit -m "feat #34: admin JSON export route for games"
```

---

### Task 4: `import_bundle` (testable ingest logic)

**Files:**
- Create: `rust/web/src/game/import.rs` (logic + tests in-file)
- Modify: `rust/web/src/game/mod.rs` (register module)

**Interfaces:**
- Consumes: `crate::game::export::{ExportBundle, BUNDLE_SCHEMA_VERSION, build_export_bundle}` (Task 3), `crate::db::{find_latest_non_deprecated_game_version, validate_username, generate_unique_username}`.
- Produces (Task 5 depends on these):
  - `pub struct ImportOutcome { pub game_id: Uuid, pub warnings: Vec<String> }`
  - `pub async fn import_bundle(pool: &sqlx::PgPool, bundle: &ExportBundle) -> anyhow::Result<ImportOutcome>`

Mapping rules (spec D5): local game type is matched by name (error if absent - the dev operator registers game types); the latest non-deprecated local game version is used, with a warning when its name differs from the bundle's (state-blob fidelity caveat). Placeholder users are created for human players - bundle name if valid and free, else `generate_unique_username`. Everything inserts under fresh IDs in one transaction.

- [ ] **Step 1: Write the failing tests**

Create `rust/web/src/game/import.rs` starting with only the tests module (and a stub-free file - tests first):

```rust
//! #34 dev-side game import (spec D5): ingests an `ExportBundle` into local
//! Postgres. Dev-only tooling - consumed by the `import-game` binary, never
//! deployed or reachable in prod.
#![cfg(feature = "ssr")]

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::export::build_export_bundle;
    use crate::game::server_fns::BotSlot;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn make_exported_game(pool: &PgPool) -> crate::game::export::ExportBundle {
        let creator = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, 'alice', $2)",
            creator,
            &Vec::<String>::new()
        )
        .execute(pool)
        .await
        .unwrap();
        let game_type_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_types (name, player_counts) VALUES ('Lost Cities', $1) RETURNING id",
            &vec![2i32]
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let game_version_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_versions (game_type_id, name, uri, is_public, is_deprecated)
             VALUES ($1, 'v1', 'http://localhost:0/mock', true, false) RETURNING id",
            game_type_id
        )
        .fetch_one(pool)
        .await
        .unwrap();
        let game = crate::db::create_game_with_users(
            pool,
            crate::db::CreateGameOpts {
                game_version_id,
                whose_turn: &[0],
                eliminated: &[],
                placings: &[],
                points: &[],
                creator_id: creator,
                opponent_ids: &[],
                opponent_emails: &[],
                bot_slots: &[BotSlot {
                    name: "Botty".to_string(),
                    difficulty: "easy".to_string(),
                }],
                chat_id: None,
                game_state: "prod_state_blob",
            },
        )
        .await
        .unwrap();
        // One public and one private (creator-targeted) log.
        crate::db::insert_game_logs_tx(
            &mut *pool.acquire().await.unwrap(),
            game.id,
            vec![
                brdgme_cmd::api::CliLog {
                    content: "public entry".to_string(),
                    at: None,
                    public: true,
                    to: vec![],
                },
                brdgme_cmd::api::CliLog {
                    content: "private entry".to_string(),
                    at: None,
                    public: false,
                    to: vec![0],
                },
            ],
        )
        .await
        .unwrap();
        build_export_bundle(pool, game.id).await.unwrap().unwrap()
    }

    #[sqlx::test]
    async fn import_bundle_round_trips_a_game(pool: PgPool) {
        let bundle = make_exported_game(&pool).await;

        let outcome = import_bundle(&pool, &bundle).await.unwrap();
        assert_ne!(outcome.game_id, bundle.game.id);
        // Same local version name as the bundle - no fidelity warning.
        assert!(outcome.warnings.is_empty(), "warnings: {:?}", outcome.warnings);

        let ge = crate::db::find_game_extended(&pool, outcome.game_id)
            .await
            .unwrap()
            .expect("imported game exists");
        assert_eq!(ge.game.game_state, "prod_state_blob");
        assert_eq!(ge.game_type.name, "Lost Cities");
        assert_eq!(ge.game_players.len(), 2);
        // Placeholder human user created: "alice" is taken by the original
        // user in this same database, so the import generated a fresh name.
        let human = ge
            .game_players
            .iter()
            .find(|p| p.user.is_some())
            .expect("human seat imported");
        let bot = ge
            .game_players
            .iter()
            .find(|p| p.game_bot.is_some())
            .expect("bot seat imported");
        assert_eq!(bot.game_bot.as_ref().unwrap().name, "Botty");
        assert!(human.user.as_ref().unwrap().id != bundle_original_user_id(&pool).await);

        // Logs and targets came across, remapped to the new player ids.
        let log_count: i64 =
            sqlx::query_scalar!("SELECT COUNT(*) AS \"c!\" FROM game_logs WHERE game_id = $1", outcome.game_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(log_count, 2);
        let target_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) AS \"c!\" FROM game_log_targets glt
             JOIN game_players gp ON gp.id = glt.game_player_id
             WHERE gp.game_id = $1",
            outcome.game_id
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(target_count, 1);
    }

    async fn bundle_original_user_id(pool: &PgPool) -> Uuid {
        sqlx::query_scalar!("SELECT id FROM users WHERE name = 'alice'")
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn import_bundle_warns_on_version_mismatch(pool: PgPool) {
        let mut bundle = make_exported_game(&pool).await;
        bundle.game_version_name = "v0-ancient".to_string();

        let outcome = import_bundle(&pool, &bundle).await.unwrap();
        assert!(
            outcome.warnings.iter().any(|w| w.contains("v0-ancient")),
            "expected version-mismatch warning, got {:?}",
            outcome.warnings
        );
    }

    #[sqlx::test]
    async fn import_bundle_errors_when_game_type_missing(pool: PgPool) {
        let mut bundle = make_exported_game(&pool).await;
        bundle.game_type_name = "No Such Game".to_string();

        let result = import_bundle(&pool, &bundle).await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn import_bundle_rejects_unknown_schema_version(pool: PgPool) {
        let mut bundle = make_exported_game(&pool).await;
        bundle.schema_version = 999;

        let result = import_bundle(&pool, &bundle).await;
        assert!(result.is_err());
    }
}
```

Check `insert_game_logs_tx`'s actual log-item type at `rust/web/src/db.rs:976` and adjust the fixture (field names above assume `brdgme_cmd::api::CliLog`; use whatever type/fields it really takes - if constructing it is awkward, insert the two log rows plus one target row with raw `sqlx::query!` instead, as in Task 1's test).

- [ ] **Step 2: Register module and run tests to verify they fail**

Add to `rust/web/src/game/mod.rs`:

```rust
#[cfg(feature = "ssr")]
pub mod import;
```

Run: `cargo test -p web --features ssr import_bundle`
Expected: compile error - `import_bundle` not found.

- [ ] **Step 3: Implement `import_bundle`**

Add above the tests module in `rust/web/src/game/import.rs`:

```rust
use crate::game::export::{BUNDLE_SCHEMA_VERSION, ExportBundle};
use anyhow::{Context, anyhow};
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

pub struct ImportOutcome {
    pub game_id: Uuid,
    pub warnings: Vec<String>,
}

pub async fn import_bundle(pool: &PgPool, bundle: &ExportBundle) -> anyhow::Result<ImportOutcome> {
    if bundle.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported bundle schema_version {} (this build supports {})",
            bundle.schema_version,
            BUNDLE_SCHEMA_VERSION
        ));
    }

    let mut warnings = Vec::new();

    // Map the bundle's game type to the local registration by name; the
    // bundle's URI is the exporting environment's and will not resolve here.
    let game_type_id: Uuid = sqlx::query_scalar!(
        "SELECT id FROM game_types WHERE name = $1",
        bundle.game_type_name
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| {
        anyhow!(
            "game type {:?} is not registered locally - start the dev stack so the operator registers it first",
            bundle.game_type_name
        )
    })?;
    let local_version = crate::db::find_latest_non_deprecated_game_version(pool, game_type_id)
        .await?
        .ok_or_else(|| anyhow!("no non-deprecated local game version for {:?}", bundle.game_type_name))?;
    if local_version.name != bundle.game_version_name {
        warnings.push(format!(
            "bundle was exported from game version {:?} but the local service runs {:?} - the state blob may not load or may behave differently",
            bundle.game_version_name, local_version.name
        ));
    }

    let mut tx = pool.begin().await?;

    let game_id: Uuid = sqlx::query_scalar!(
        "INSERT INTO games (game_version_id, is_finished, finished_at, game_state)
         VALUES ($1, $2, $3, $4) RETURNING id",
        local_version.id,
        bundle.game.is_finished,
        bundle.game.finished_at,
        bundle.game.game_state
    )
    .fetch_one(&mut *tx)
    .await?;

    // Bots: fresh rows keyed by name for the player mapping below.
    let mut bot_ids: HashMap<String, Uuid> = HashMap::new();
    for bot in &bundle.bots {
        let id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_bots (game_id, name, difficulty, personality)
             VALUES ($1, $2, $3, $4) RETURNING id",
            game_id,
            bot.name,
            bot.difficulty,
            bot.personality
        )
        .fetch_one(&mut *tx)
        .await?;
        bot_ids.insert(bot.name.clone(), id);
    }

    // Players: placeholder local users for humans (spec D5 - named players,
    // no emails exist in the bundle), bots linked by name.
    let mut player_ids_by_position: HashMap<i32, Uuid> = HashMap::new();
    for player in &bundle.players {
        let (user_id, game_bot_id) = match &player.bot_name {
            Some(bot_name) => (
                None,
                Some(*bot_ids.get(bot_name).ok_or_else(|| {
                    anyhow!("bundle player {:?} references unknown bot {:?}", player.name, bot_name)
                })?),
            ),
            None => (
                Some(placeholder_user(&mut tx, &player.name).await?),
                None,
            ),
        };
        let gp_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_players
               (game_id, user_id, game_bot_id, position, color, has_accepted,
                is_turn, place, is_eliminated, points, undo_game_state, rating_change)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING id",
            game_id,
            user_id,
            game_bot_id,
            player.position,
            player.color,
            player.has_accepted,
            player.is_turn,
            player.place,
            player.is_eliminated,
            player.points,
            player.undo_game_state,
            player.rating_change
        )
        .fetch_one(&mut *tx)
        .await?;
        player_ids_by_position.insert(player.position, gp_id);

        // Rating rows so game rendering has real game_type_users joins.
        if let Some(user_id) = user_id {
            sqlx::query!(
                "INSERT INTO game_type_users (game_type_id, user_id) VALUES ($1, $2)
                 ON CONFLICT DO NOTHING",
                game_type_id,
                user_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    for log in &bundle.logs {
        let log_id: Uuid = sqlx::query_scalar!(
            "INSERT INTO game_logs (game_id, body, is_public, logged_at)
             VALUES ($1, $2, $3, $4) RETURNING id",
            game_id,
            log.body,
            log.is_public,
            log.logged_at
        )
        .fetch_one(&mut *tx)
        .await?;
        for position in &log.target_positions {
            let gp_id = player_ids_by_position.get(position).ok_or_else(|| {
                anyhow!("bundle log targets unknown player position {}", position)
            })?;
            sqlx::query!(
                "INSERT INTO game_log_targets (game_log_id, game_player_id) VALUES ($1, $2)",
                log_id,
                gp_id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(ImportOutcome { game_id, warnings })
}

/// Uses the bundle's display name when it is a valid, unclaimed username;
/// otherwise generates a fresh one (username rules: migration 009).
async fn placeholder_user(tx: &mut sqlx::PgConnection, name: &str) -> anyhow::Result<Uuid> {
    let taken = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM users WHERE lower(name) = lower($1)) AS "taken!""#,
        name
    )
    .fetch_one(&mut *tx)
    .await?;
    let final_name = if crate::db::validate_username(name) && !taken {
        name.to_string()
    } else {
        crate::db::generate_unique_username(&mut *tx)
            .await
            .context("generate placeholder username")?
    };
    let id = sqlx::query_scalar!(
        "INSERT INTO users (name, pref_colors) VALUES ($1, $2) RETURNING id",
        final_name,
        &Vec::<String>::new()
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok(id)
}
```

Adjust to the real signature of `generate_unique_username` (`rust/web/src/db.rs:620` takes `&mut sqlx::PgConnection`) and the `games` insert columns to the actual NOT NULL set (defaults cover `created_at`/`updated_at`/`chat_id`/`restarted_game_id`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p web --features ssr import_bundle`
Expected: 4 passed.

- [ ] **Step 5: Gate and commit**

```bash
cargo fmt
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
git add rust/web/src/game/import.rs rust/web/src/game/mod.rs
git commit -m "feat #34: import_bundle ingest logic for exported games"
```

---

### Task 5: `import-game` binary + wiring + backlog update

**Files:**
- Create: `rust/web/src/bin/import_game.rs`
- Modify: `rust/web/Cargo.toml` (`[[bin]]` section + `bin-target` leptos metadata)
- Modify: `docs/BACKLOG.md` (#34 row status)

**Interfaces:**
- Consumes: `web::game::export::ExportBundle` (serde Deserialize), `web::game::import::import_bundle`, `web::db::create_pool` (reads `DATABASE_URL`, `rust/web/src/db.rs:157`).

- [ ] **Step 1: Declare the binary**

In `rust/web/Cargo.toml` add after `[lib]`:

```toml
# #34 dev-only import CLI (spec D5): never deployed; requires ssr because it
# links the server-side db/game modules.
[[bin]]
name = "import-game"
path = "src/bin/import_game.rs"
required-features = ["ssr"]
```

Edition 2024 keeps auto-discovery of `src/main.rs` (the `web` bin) alongside an explicit `[[bin]]`. cargo-leptos must keep building the `web` bin - add to `[package.metadata.leptos]` (next to `output-name`):

```toml
bin-target = "web"
```

- [ ] **Step 2: Write the binary**

Create `rust/web/src/bin/import_game.rs`:

```rust
//! #34 dev-side game import CLI (spec D5).
//!
//! Usage: cargo run -p web --features ssr --bin import-game -- bundle.json
//!
//! Reads DATABASE_URL (via .env / environment), ingests the bundle into
//! local Postgres under fresh IDs. Dev-only; never deployed.

fn usage() -> ! {
    eprintln!("usage: import-game <bundle.json>");
    std::process::exit(2);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let Some(path) = std::env::args().nth(1) else { usage() };

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
    let bundle: web::game::export::ExportBundle = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("parsing {path}: {e}"))?;

    let pool = web::db::create_pool().await?;
    let outcome = web::game::import::import_bundle(&pool, &bundle).await?;

    for warning in &outcome.warnings {
        eprintln!("warning: {warning}");
    }
    println!(
        "imported {} game {} as local game {}",
        bundle.game_type_name, bundle.game.id, outcome.game_id
    );
    println!("open: /games/{}", outcome.game_id);
    Ok(())
}
```

(`create_pool` runs from `web::db` - if it also runs migrations that is fine for dev. If `create_pool`'s error type is not anyhow-compatible, map it with `.map_err(anyhow::Error::from)`.)

- [ ] **Step 3: Verify it builds and the usage path works**

```bash
cargo build -p web --features ssr --bin import-game
cargo run -p web --features ssr --bin import-game 2>&1 | head -2
```

Expected: build succeeds; running with no args prints `usage: import-game <bundle.json>` and exits 2.

- [ ] **Step 4: End-to-end smoke test (dev stack running)**

With the local dev stack up (game services registered by the operator):

```bash
# Export any local game while logged in as an admin, or via curl with a
# session cookie, then:
cargo run -p web --features ssr --bin import-game -- /tmp/brdgme-game-<id>.json
```

Expected: prints the new local game id (plus a version warning if applicable); the game page renders at `/games/<new-id>`. If no dev stack is available at execution time, note this as pending manual verification rather than skipping the gates.

- [ ] **Step 5: Full gate, backlog update, commit**

```bash
cargo fmt
cargo clippy -p web --all-targets --features ssr -- -D warnings
cargo test -p web --features ssr
```

Update `docs/BACKLOG.md`: the #34 table row (line ~104) status becomes complete (force-delete, export route, and import CLI implemented; note the date), and adjust the "Remaining pre-go-live" paragraph (line ~34) accordingly.

```bash
git add rust/web/Cargo.toml rust/Cargo.lock rust/web/src/bin/import_game.rs docs/BACKLOG.md
git commit -m "feat #34: dev import-game CLI for exported game bundles"
```

---

## Self-Review Notes

- **Spec coverage:** D3 -> Tasks 1-2 (hard delete, cascade order, restarted_game_id nulling, no rating rewind, confirm dialog, navigate away + broadcast). D4 -> Task 3 (admin-guarded Axum route, schema_version/exported_at/game row/type+version name+uri/players/bots incl. personality/logs+targets/display names, no emails, file download). D5 -> Tasks 4-5 (CLI in rust/web, DATABASE_URL, placeholder users, type/version mapping by name, fresh IDs, version-mismatch warning, dev-only). D1/D2 already shipped - reused, not re-planned.
- **Known judgment calls (flag to user):** (1) bundle links players/logs by position and bots by name instead of raw UUID tables - same information, no ID remapping needed on import; (2) `chat_id` and `restarted_game_id` are not exported (chats are out of scope, restart links are meaningless cross-environment); (3) export keeps `undo_game_state` for debug fidelity; (4) `time` crate gains `formatting`+`parsing` features for an RFC3339 `exported_at`.
- **Type consistency:** `ExportBundle`/`ImportOutcome` names and signatures match across Tasks 3-5; `delete_game` bool contract matches Task 2's usage.
