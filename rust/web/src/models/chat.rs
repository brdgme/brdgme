use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Chat {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub last_read_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub chat_user_id: Uuid,
    pub message: String,
}

// Insert structs for creating new records
#[derive(Debug)]
pub struct NewChat {
    // Chat has no additional fields beyond id and timestamps
}

#[derive(Debug)]
pub struct NewChatUser {
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub last_read_at: NaiveDateTime,
}

#[derive(Debug)]
pub struct NewChatMessage {
    pub chat_user_id: Uuid,
    pub message: String,
}