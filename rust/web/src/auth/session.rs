#[cfg(feature = "ssr")]
use crate::models::user::User;
#[cfg(feature = "ssr")]
use brdgme_session_store::PostgresStore;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use tower_sessions::cookie::time::Duration;
#[cfg(feature = "ssr")]
use tower_sessions::{Session, SessionManagerLayer};
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

/// `SECURE_COOKIE` env semantics: the session cookie carries the `Secure`
/// attribute unless the value is the exact literal "false". Unset (prod
/// k8s, which sets nothing) means Secure. The opt-out exists for dev
/// environments served over plain HTTP on non-localhost hostnames
/// (http://web.brdgme.lvh.me:8080 in-cluster Tilt), where browsers refuse
/// Secure cookies entirely; see k8s/dev/web-patch.yaml and the Tiltfile
/// local web resource.
#[cfg(feature = "ssr")]
fn secure_cookie(env_value: Option<&str>) -> bool {
    env_value != Some("false")
}

#[cfg(feature = "ssr")]
pub async fn create_session_layer(pool: &PgPool) -> SessionManagerLayer<PostgresStore> {
    let store = PostgresStore::new(pool.clone());
    store
        .migrate()
        .await
        .expect("Failed to run session store migration");
    let secure = secure_cookie(std::env::var("SECURE_COOKIE").ok().as_deref());
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

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::secure_cookie;

    #[test]
    fn secure_cookie_defaults_to_secure_when_unset() {
        assert!(secure_cookie(None));
    }

    #[test]
    fn secure_cookie_explicit_false_opts_out() {
        assert!(!secure_cookie(Some("false")));
    }

    #[test]
    fn secure_cookie_any_other_value_stays_secure() {
        assert!(secure_cookie(Some("true")));
        assert!(secure_cookie(Some("0")));
        assert!(secure_cookie(Some("")));
        assert!(secure_cookie(Some("FALSE"))); // opt-out is the exact literal only
    }
}
