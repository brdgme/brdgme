//! #29 player stats: DTOs, queries and server fns for /players pages.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::error::internal;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileUser {
    pub user_id: Uuid,
    pub name: String,
    pub pref_color: Option<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverallTotals {
    pub finished_games: i64,
    pub wins: i64,
    pub win_percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeStats {
    pub game_type_name: String,
    pub games: i64,
    pub wins: i64,
    pub win_percent: f64,
    pub avg_place_percentile: Option<f64>,
    pub rating: Option<i32>,
    pub peak_rating: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatingPoint {
    pub finished_at: PrimitiveDateTime,
    pub rating: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opponent {
    pub user_id: Option<Uuid>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinishedGameRow {
    pub game_id: Uuid,
    pub game_type_name: String,
    pub finished_at: Option<PrimitiveDateTime>,
    pub place: Option<i32>,
    pub player_count: i64,
    pub rating_change: Option<i32>,
    pub opponents: Vec<Opponent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveGameRow {
    pub game_id: Uuid,
    pub game_type_name: String,
    pub is_turn: bool,
    pub opponents: Vec<Opponent>,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadToHead {
    pub user_id: Uuid,
    pub name: String,
    pub games: i64,
    pub wins: i64,
    pub losses: i64,
    pub ties: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormResult {
    pub game_id: Uuid,
    pub finished_at: Option<PrimitiveDateTime>,
    pub place: Option<i32>,
    pub player_count: i64,
    pub rating_change: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameTypeForm {
    pub game_type_name: String,
    pub results: Vec<FormResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerProfileData {
    pub user: ProfileUser,
    pub totals: OverallTotals,
    pub game_types: Vec<GameTypeStats>,
    pub recent_form: Vec<GameTypeForm>,
    pub recent_finished: Vec<FinishedGameRow>,
    pub active_games: Vec<ActiveGameRow>,
    /// None when the viewer is anonymous (profiles are public).
    pub viewer_user_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerGameTypeData {
    pub user: ProfileUser,
    /// Canonical game type name (URL segment is matched case-insensitively).
    pub game_type_name: String,
    /// Aggregate row for this game type (games/wins/win_percent/avg_place_percentile/rating/peak_rating).
    pub stats: GameTypeStats,
    pub rating_series: Vec<RatingPoint>,
    pub finished_games: Vec<FinishedGameRow>,
    pub head_to_head: Vec<HeadToHead>,
}

#[cfg(feature = "ssr")]
mod queries;

#[cfg(feature = "ssr")]
pub use queries::*;

pub mod viz;

#[server(GetPlayerProfile, "/api")]
pub async fn get_player_profile(
    name: String,
    include_single_human: bool,
) -> Result<Option<PlayerProfileData>, ServerFnError> {
    use crate::auth::server::get_current_user;
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();

    let viewer_user_id = get_current_user().await?.map(|u| u.id);

    let user = match get_profile_user(&pool, &name)
        .await
        .map_err(internal("get_player_profile: find user"))?
    {
        Some(user) => user,
        None => return Ok(None),
    };

    let totals = overall_totals(&pool, user.user_id, include_single_human)
        .await
        .map_err(internal("get_player_profile: totals"))?;
    let game_types = game_type_stats(&pool, user.user_id, include_single_human)
        .await
        .map_err(internal("get_player_profile: game_types"))?;
    let recent_form = recent_form(&pool, user.user_id, 10, include_single_human)
        .await
        .map_err(internal("get_player_profile: recent_form"))?;
    let recent_finished = finished_games(&pool, user.user_id, None, include_single_human, Some(20))
        .await
        .map_err(internal("get_player_profile: recent_finished"))?;

    let active_games = active_games(&pool, user.user_id)
        .await
        .map_err(internal("get_player_profile: active_games"))?;

    Ok(Some(PlayerProfileData {
        user,
        totals,
        game_types,
        recent_form,
        recent_finished,
        active_games,
        viewer_user_id,
    }))
}

#[server(GetPlayerGameTypeStats, "/api")]
pub async fn get_player_game_type_stats(
    name: String,
    game_type: String,
    include_single_human: bool,
) -> Result<Option<PlayerGameTypeData>, ServerFnError> {
    use sqlx::PgPool;
    let pool = expect_context::<PgPool>();

    let user = match get_profile_user(&pool, &name)
        .await
        .map_err(internal("get_player_game_type_stats: find user"))?
    {
        Some(user) => user,
        None => return Ok(None),
    };

    let canonical = match find_game_type_name(&pool, &game_type)
        .await
        .map_err(internal("get_player_game_type_stats: find game type"))?
    {
        Some(name) => name,
        None => return Ok(None),
    };

    let stats = game_type_stats(&pool, user.user_id, include_single_human)
        .await
        .map_err(internal("get_player_game_type_stats: stats"))?
        .into_iter()
        .find(|s| s.game_type_name == canonical)
        .unwrap_or_else(|| GameTypeStats {
            game_type_name: canonical.clone(),
            games: 0,
            wins: 0,
            win_percent: 0.0,
            avg_place_percentile: None,
            rating: None,
            peak_rating: None,
        });

    let rating_series = rating_series(&pool, user.user_id, &canonical)
        .await
        .map_err(internal("get_player_game_type_stats: rating_series"))?;
    let finished_games = finished_games(
        &pool,
        user.user_id,
        Some(&canonical),
        include_single_human,
        None,
    )
    .await
    .map_err(internal("get_player_game_type_stats: finished_games"))?;
    let head_to_head = head_to_head(&pool, user.user_id, &canonical, include_single_human)
        .await
        .map_err(internal("get_player_game_type_stats: head_to_head"))?;

    Ok(Some(PlayerGameTypeData {
        user,
        game_type_name: canonical,
        stats,
        rating_series,
        finished_games,
        head_to_head,
    }))
}
