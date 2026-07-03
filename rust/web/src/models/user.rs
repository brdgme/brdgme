use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
    #[serde(skip_serializing)]
    pub login_confirmation: Option<String>,
    #[serde(skip_serializing)]
    pub login_confirmation_at: Option<PrimitiveDateTime>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserEmail {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
}
