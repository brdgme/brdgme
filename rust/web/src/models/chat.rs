use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ChatUser {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub last_read_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub chat_user_id: Uuid,
    pub message: String,
}