//! #34 admin game export (spec D4): a versioned JSON bundle for pulling a
//! prod game into a local dev environment. Served from an admin-guarded
//! plain Axum route (not a leptos server fn) because it downloads as a file.
//! The bundle contains private log bodies, their target positions, and the
//! raw `game_state` blob - all hidden information. Admin-only; must not be
//! posted publicly. Email addresses are the one thing still excluded.
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

pub const BUNDLE_SCHEMA_VERSION: u32 = 2;

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
    pub end_reason: Option<String>,
    pub game_state: String,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePlayer {
    pub position: i32,
    /// Display name only - user name or bot name, never an email.
    pub name: String,
    /// `Some(game_bots.name)` - the per-game seat name - when this seat is a
    /// bot; `None` for humans. This is NOT `game_bots.bot_name` (the bot type),
    /// which is carried in `BundleBot.bot_name`.
    pub bot_name: Option<String>,
    /// True when the seat holds a human `user_id`; replacement-human seats
    /// retain their human identity (and `bot_name`) while still playing as a bot.
    pub is_human: bool,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub place: Option<i32>,
    pub ranked_placing: Option<i32>,
    pub is_eliminated: bool,
    pub departure_reason: Option<String>,
    pub departure_sequence: Option<i32>,
    pub left_at: Option<PrimitiveDateTime>,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub rating_change: Option<i32>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub is_turn_at: PrimitiveDateTime,
    pub last_turn_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleBot {
    pub name: String,
    pub bot_name: String,
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
        "SELECT name, bot_name, personality FROM game_bots WHERE game_id = $1 ORDER BY name",
        game_id
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|b| BundleBot {
        name: b.name,
        bot_name: b.bot_name,
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
            is_human: p.user.is_some(),
            color: p.game_player.color.clone(),
            has_accepted: p.game_player.has_accepted,
            is_turn: p.game_player.is_turn,
            place: p.game_player.place,
            ranked_placing: p.game_player.ranked_placing,
            is_eliminated: p.game_player.is_eliminated,
            departure_reason: p.game_player.departure_reason.clone(),
            departure_sequence: p.game_player.departure_sequence,
            left_at: p.game_player.left_at,
            points: p.game_player.points,
            undo_game_state: p.game_player.undo_game_state.clone(),
            rating_change: p.game_player.rating_change,
            created_at: p.game_player.created_at,
            updated_at: p.game_player.updated_at,
            is_turn_at: p.game_player.is_turn_at,
            last_turn_at: p.game_player.last_turn_at,
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
            end_reason: ge.game.end_reason,
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
    let Ok(Some(session_user)) = crate::auth::session::get_user_from_session(&session).await else {
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
