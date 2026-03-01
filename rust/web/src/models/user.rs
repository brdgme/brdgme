use time::PrimitiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<PrimitiveDateTime>,
}

impl User {
    pub fn into_public(self) -> PublicUser {
        PublicUser {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            name: self.name,
            pref_colors: self.pref_colors,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PublicUser {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
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

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserAuthToken {
    pub id: Uuid,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
    pub user_id: Uuid,
}

// Insert structs for creating new records
#[derive(Debug)]
pub struct NewUser {
    pub name: String,
    pub pref_colors: Vec<String>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<PrimitiveDateTime>,
}

#[derive(Debug)]
pub struct NewUserEmail {
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
}

#[derive(Debug)]
pub struct NewUserAuthToken {
    pub user_id: Uuid,
}
