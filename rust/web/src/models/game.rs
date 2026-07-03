use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameVersion {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
    pub uri: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Game {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<PrimitiveDateTime>,
    pub game_state: String,
    pub chat_id: Option<Uuid>,
    pub restarted_game_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameBot {
    pub id: Uuid,
    pub game_id: Uuid,
    pub name: String,
    pub difficulty: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_id: Uuid,
    pub user_id: Option<Uuid>,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: PrimitiveDateTime,
    pub place: Option<i32>,
    pub last_turn_at: PrimitiveDateTime,
    pub is_eliminated: bool,
    pub is_read: bool,
    pub points: Option<f32>,
    pub undo_game_state: Option<String>,
    pub rating_change: Option<i32>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameLog {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_id: Uuid,
    pub body: String,
    pub is_public: bool,
    pub logged_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameTypeUser {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub game_type_id: Uuid,
    pub user_id: Uuid,
    pub last_game_finished_at: Option<PrimitiveDateTime>,
    pub rating: i32,
    pub peak_rating: i32,
}
