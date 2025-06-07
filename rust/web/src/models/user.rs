use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub pref_colors: Vec<String>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct UserEmail {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct UserAuthToken {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_id: Uuid,
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub source_user_id: Uuid,
    pub target_user_id: Uuid,
    pub has_accepted: Option<bool>,
}