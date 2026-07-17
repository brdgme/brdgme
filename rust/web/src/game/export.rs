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
