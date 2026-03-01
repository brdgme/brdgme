use time::PrimitiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Friend {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub source_user_id: Uuid,
    pub target_user_id: Uuid,
    pub has_accepted: Option<bool>,
}

// Insert struct for creating new records
#[derive(Debug)]
pub struct NewFriend {
    pub source_user_id: Uuid,
    pub target_user_id: Uuid,
    pub has_accepted: Option<bool>,
}
