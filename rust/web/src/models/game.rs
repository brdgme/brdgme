use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct Game {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<DateTime<Utc>>,
    pub game_state: String,
    pub chat_id: Option<Uuid>,
    pub restarted_game_id: Option<Uuid>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: DateTime<Utc>,
    pub last_turn_at: DateTime<Utc>,
    pub is_eliminated: bool,
    pub is_read: bool,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub place: Option<i32>,
    pub rating_change: Option<i32>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameVersion {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub game_type_id: Uuid,
    pub name: String,
    pub uri: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameTypeUser {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub game_type_id: Uuid,
    pub user_id: Uuid,
    pub last_game_finished_at: Option<DateTime<Utc>>,
    pub rating: i32,
    pub peak_rating: i32,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameLog {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub game_id: Uuid,
    pub body: String,
    pub is_public: bool,
    pub logged_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameLogTarget {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub game_log_id: Uuid,
    pub game_player_id: Uuid,
}