use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

pub type PublicGameType = GameType;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameVersion {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
    pub uri: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

impl GameVersion {
    pub fn into_public(self) -> PublicGameVersion {
        PublicGameVersion {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            game_type_id: self.game_type_id,
            name: self.name,
            is_public: self.is_public,
            is_deprecated: self.is_deprecated,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicGameVersion {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Game {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<NaiveDateTime>,
    pub game_state: String,
    pub chat_id: Option<Uuid>,
    pub restarted_game_id: Option<Uuid>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: NaiveDateTime,
    pub place: Option<i32>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameLog {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub body: String,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameLogTarget {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_log_id: Uuid,
    pub game_player_id: Uuid,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GameTypeUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub user_id: Uuid,
    pub last_game_finished_at: Option<NaiveDateTime>,
    pub rating: i32,
    pub peak_rating: i32,
}

// Insert structs for creating new records
#[derive(Debug)]
pub struct NewGameType {
    pub name: String,
    pub player_counts: Vec<i32>,
    pub weight: f32,
}

#[derive(Debug)]
pub struct NewGameVersion {
    pub game_type_id: Uuid,
    pub name: String,
    pub uri: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Debug)]
pub struct NewGame {
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub finished_at: Option<NaiveDateTime>,
    pub game_state: String,
    pub chat_id: Option<Uuid>,
    pub restarted_game_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct NewGamePlayer {
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_turn_at: NaiveDateTime,
    pub place: Option<i32>,
}