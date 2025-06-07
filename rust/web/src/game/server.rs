// Game server module - enhanced with real database operations

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;
#[cfg(feature = "ssr")]
use crate::models::game::GameType;
#[cfg(feature = "ssr")]
use sqlx::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameListResponse {
    pub games: Vec<GameSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub id: Uuid,
    pub name: String,
    pub game_type: String,
    pub player_count: i32,
    pub max_players: i32,
    pub is_finished: bool,
    pub created_at: NaiveDateTime,
    pub is_user_turn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDetail {
    pub id: Uuid,
    pub game_type: String,
    pub is_finished: bool,
    pub players: Vec<GamePlayerInfo>,
    pub created_at: NaiveDateTime,
    pub game_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayerInfo {
    pub id: Uuid,
    pub user_name: String,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameTypeInfo {
    pub id: Uuid,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

#[server(GetGames, "/api")]
pub async fn get_games() -> Result<GameListResponse, ServerFnError> {
    let pool = expect_context::<PgPool>();
    
    // Get active games with basic info
    let games = sqlx::query!(
        r#"
        SELECT 
            g.id,
            gt.name as game_type_name,
            g.is_finished,
            g.created_at,
            COUNT(gp.id) as player_count,
            MAX(gt.player_counts[1]) as max_players
        FROM games g
        JOIN game_versions gv ON g.game_version_id = gv.id
        JOIN game_types gt ON gv.game_type_id = gt.id
        LEFT JOIN game_players gp ON g.id = gp.game_id
        WHERE NOT g.is_finished
        GROUP BY g.id, gt.name, g.is_finished, g.created_at
        ORDER BY g.created_at DESC
        LIMIT 20
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let game_summaries: Vec<GameSummary> = games
        .into_iter()
        .map(|row| GameSummary {
            id: row.id,
            name: format!("Game #{}", &row.id.to_string()[..8]),
            game_type: row.game_type_name,
            player_count: row.player_count.unwrap_or(0) as i32,
            max_players: row.max_players.unwrap_or(2) as i32,
            is_finished: row.is_finished,
            created_at: row.created_at,
            is_user_turn: false, // TODO: Calculate based on current user
        })
        .collect();
    
    Ok(GameListResponse {
        games: game_summaries,
    })
}

#[server(GetGame, "/api")]
pub async fn get_game(id: String) -> Result<Option<GameDetail>, ServerFnError> {
    let pool = expect_context::<PgPool>();
    
    let game_id = Uuid::parse_str(&id)
        .map_err(|_| ServerFnError::new("Invalid game ID format".to_string()))?;
    
    // Get game with type info
    let game_info = sqlx::query!(
        r#"
        SELECT 
            g.id,
            g.is_finished,
            g.created_at,
            g.game_state,
            gt.name as game_type_name
        FROM games g
        JOIN game_versions gv ON g.game_version_id = gv.id
        JOIN game_types gt ON gv.game_type_id = gt.id
        WHERE g.id = $1
        "#,
        game_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let Some(game) = game_info else {
        return Ok(None);
    };
    
    // Get players for this game
    let players = sqlx::query!(
        r#"
        SELECT 
            gp.id,
            gp.position,
            gp.color,
            gp.has_accepted,
            gp.is_turn,
            u.name as user_name
        FROM game_players gp
        JOIN users u ON gp.user_id = u.id
        WHERE gp.game_id = $1
        ORDER BY gp.position
        "#,
        game_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let player_infos: Vec<GamePlayerInfo> = players
        .into_iter()
        .map(|row| GamePlayerInfo {
            id: row.id,
            user_name: row.user_name,
            position: row.position,
            color: row.color,
            has_accepted: row.has_accepted,
            is_turn: row.is_turn,
        })
        .collect();
    
    Ok(Some(GameDetail {
        id: game.id,
        game_type: game.game_type_name,
        is_finished: game.is_finished,
        players: player_infos,
        created_at: game.created_at,
        game_state: game.game_state,
    }))
}

#[server(GetGameTypes, "/api")]
pub async fn get_game_types() -> Result<Vec<GameTypeInfo>, ServerFnError> {
    let pool = expect_context::<PgPool>();
    
    let game_types = sqlx::query_as!(
        GameType,
        "SELECT * FROM game_types ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let game_type_infos: Vec<GameTypeInfo> = game_types
        .into_iter()
        .map(|gt| GameTypeInfo {
            id: gt.id,
            name: gt.name,
            player_counts: gt.player_counts,
            weight: gt.weight,
        })
        .collect();
    
    Ok(game_type_infos)
}

#[server(CreateGame, "/api")]
pub async fn create_game(game_type_id: String, _player_count: i32) -> Result<Uuid, ServerFnError> {
    let pool = expect_context::<PgPool>();
    
    let game_type_uuid = Uuid::parse_str(&game_type_id)
        .map_err(|_| ServerFnError::new("Invalid game type ID".to_string()))?;
    
    // Get the latest version of this game type
    let game_version = sqlx::query!(
        "SELECT id FROM game_versions WHERE game_type_id = $1 AND is_public = true AND NOT is_deprecated ORDER BY created_at DESC LIMIT 1",
        game_type_uuid
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let Some(version) = game_version else {
        return Err(ServerFnError::new("No available version for this game type".to_string()));
    };
    
    // Create new game
    let game_id = Uuid::new_v4();
    let initial_state = "{}"; // TODO: Generate proper initial game state
    
    sqlx::query!(
        "INSERT INTO games (id, game_version_id, is_finished, game_state) VALUES ($1, $2, false, $3)",
        game_id,
        version.id,
        initial_state
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to create game: {}", e)))?;
    
    Ok(game_id)
}