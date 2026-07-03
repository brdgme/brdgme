#[cfg(feature = "ssr")]
use crate::models::user::User;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use tower_sessions::cookie::time::Duration;
#[cfg(feature = "ssr")]
use tower_sessions::{Session, SessionManagerLayer};
#[cfg(feature = "ssr")]
use tower_sessions_sqlx_store::PostgresStore;
use uuid::Uuid;

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
pub async fn create_session_layer(pool: &PgPool) -> SessionManagerLayer<PostgresStore> {
    let store = PostgresStore::new(pool.clone());
    store
        .migrate()
        .await
        .expect("Failed to run session store migration");
    let secure = std::env::var("SECURE_COOKIE")
        .map(|v| v == "true")
        .unwrap_or(false);
    SessionManagerLayer::new(store)
        .with_secure(secure)
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(tower_sessions::Expiry::OnInactivity(Duration::days(30)))
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
    session.insert(SESSION_USER_KEY, session_user).await
}

#[cfg(feature = "ssr")]
pub async fn get_user_from_session(session: &Session) -> Option<SessionUser> {
    session
        .get::<SessionUser>(SESSION_USER_KEY)
        .await
        .ok()
        .flatten()
}

#[cfg(feature = "ssr")]
pub async fn clear_user_session(session: &Session) -> Result<(), tower_sessions::session::Error> {
    session.remove::<SessionUser>(SESSION_USER_KEY).await?;
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
pub async fn invalidate_auth_token(pool: &PgPool, auth_token_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM user_auth_tokens WHERE id = $1", auth_token_id)
        .execute(pool)
        .await?;

    Ok(())
}
