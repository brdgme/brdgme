use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use time::{OffsetDateTime, PrimitiveDateTime};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
    pub theme: Option<String>,
    pub is_admin: bool,
}

/// A pending login code, keyed by email. See D2 in
/// `docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md`: no
/// `users` row exists until the code here is confirmed.
#[derive(Debug, Clone, FromRow)]
pub struct LoginConfirmation {
    pub email: String,
    pub code: String,
    pub created_at: OffsetDateTime,
    pub attempts: i32,
    pub sent_count: i32,
    pub last_sent_at: Option<OffsetDateTime>,
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
