use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "ssr")]
use tower_sessions::{Session, SessionManagerLayer, MemoryStore};
#[cfg(feature = "ssr")]
use crate::models::user::User;
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use tower_sessions::cookie::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub auth_token_id: Uuid,
}

#[cfg(feature = "ssr")]
pub const SESSION_USER_KEY: &str = "user";

#[cfg(feature = "ssr")]
pub const SESSION_AUTH_TOKEN_KEY: &str = "auth_token";

#[cfg(feature = "ssr")]
pub fn create_session_layer() -> SessionManagerLayer<MemoryStore> {
    let session_store = MemoryStore::default();
    SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(tower_sessions::Expiry::OnInactivity(Duration::hours(24))) // 24 hours
}

#[cfg(feature = "ssr")]
pub async fn set_user_session(
    session: &Session,
    user: &User,
    email: &str,
    auth_token_id: Uuid,
) -> Result<(), tower_sessions::session::Error> {
    let session_user = SessionUser {
        id: user.id,
        name: user.name.clone(),
        email: email.to_string(),
        auth_token_id,
    };
    
    session.insert(SESSION_USER_KEY, session_user).await?;
    session.insert(SESSION_AUTH_TOKEN_KEY, auth_token_id).await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn get_user_from_session(session: &Session) -> Option<SessionUser> {
    session.get::<SessionUser>(SESSION_USER_KEY).await.ok().flatten()
}

#[cfg(feature = "ssr")]
pub async fn clear_user_session(session: &Session) -> Result<(), tower_sessions::session::Error> {
    session.remove::<SessionUser>(SESSION_USER_KEY).await?;
    session.remove::<Uuid>(SESSION_AUTH_TOKEN_KEY).await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn validate_session_token(
    pool: &PgPool,
    auth_token_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let token_exists = sqlx::query!(
        "SELECT id FROM user_auth_tokens WHERE id = $1",
        auth_token_id
    )
    .fetch_optional(pool)
    .await?;
    
    Ok(token_exists.is_some())
}

#[cfg(feature = "ssr")]
pub async fn invalidate_auth_token(
    pool: &PgPool,
    auth_token_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM user_auth_tokens WHERE id = $1",
        auth_token_id
    )
    .execute(pool)
    .await?;
    
    Ok(())
}



