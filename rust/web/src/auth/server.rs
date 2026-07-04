#[cfg(feature = "ssr")]
use crate::auth::session::{
    clear_user_session, get_user_from_session, invalidate_auth_token, set_user_session,
    validate_session_token,
};
#[cfg(feature = "ssr")]
use crate::models::user::{User, UserEmail};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use leptos_axum::extract;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use time::OffsetDateTime;
#[cfg(feature = "ssr")]
use tower_sessions::Session;
use uuid::Uuid;

#[cfg(feature = "ssr")]
async fn send_login_email(resend: Option<&resend_rs::Resend>, to_email: &str, token: &str) {
    let Some(resend) = resend else {
        // No RESEND_API_KEY configured (dev default): log instead of sending.
        println!("\n==> LOGIN CODE for {}: {}\n", to_email, token);
        return;
    };

    let from_addr = std::env::var("EMAIL_FROM").unwrap_or_else(|_| "login@brdg.me".to_string());
    let email = resend_rs::types::CreateEmailBaseOptions::new(
        from_addr,
        [to_email.to_string()],
        "Your brdgme login code",
    )
    .with_text(&format!(
        "Your login code is: {}\n\nThis code expires in 1 hour.",
        token
    ));

    if let Err(e) = resend.emails.send(email).await {
        tracing::error!("Failed to send login email to {}: {}", to_email, e);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[cfg(feature = "ssr")]
async fn login_client_ip() -> Option<std::net::IpAddr> {
    let headers: axum::http::HeaderMap = extract().await.ok()?;
    let peer_addr = extract::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .await
        .ok()
        .map(|ci| ci.0);
    crate::auth::rate_limit::extract_client_ip(&headers, peer_addr)
}

#[server(Login, "/api")]
pub async fn login(email: String) -> Result<LoginResponse, ServerFnError> {
    if email.is_empty() || !email.contains('@') {
        return Ok(LoginResponse {
            success: false,
            message: "Invalid email address".to_string(),
        });
    }

    let login_rate_limiter =
        expect_context::<std::sync::Arc<crate::auth::rate_limit::LoginRateLimiter>>();
    // Fail open if the client IP can't be determined (e.g. missing ConnectInfo
    // in a test harness) rather than blocking logins outright.
    if let Some(ip) = login_client_ip().await
        && let Err(wait_secs) =
            crate::auth::rate_limit::check_login_rate_limit(&login_rate_limiter, ip)
    {
        return Ok(LoginResponse {
            success: false,
            message: format!("Too many login attempts. Try again in {}s.", wait_secs),
        });
    }

    let pool = expect_context::<PgPool>();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let user_email = sqlx::query_as!(
        UserEmail,
        "SELECT * FROM user_emails WHERE email = $1 AND is_primary = true",
        email
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let user_id = if let Some(user_email) = user_email {
        user_email.user_id
    } else {
        let new_user_id = Uuid::new_v4();
        let username = email.split('@').next().unwrap_or("user").to_string();

        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3)",
            new_user_id,
            username,
            &Vec::<String>::new()
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create user: {}", e)))?;

        sqlx::query!(
            "INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)",
            new_user_id,
            email
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create user email: {}", e)))?;

        new_user_id
    };

    let confirmation_token = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let now = OffsetDateTime::now_utc();
    let confirmation_time = time::PrimitiveDateTime::new(now.date(), now.time());

    sqlx::query!(
        "UPDATE users SET login_confirmation = $1, login_confirmation_at = $2 WHERE id = $3",
        confirmation_token,
        confirmation_time,
        user_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to update user: {}", e)))?;

    tx.commit()
        .await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let resend = expect_context::<Option<resend_rs::Resend>>();
    send_login_email(resend.as_ref(), &email, &confirmation_token).await;

    Ok(LoginResponse {
        success: true,
        message: "Login email sent".to_string(),
    })
}

#[server(ConfirmLogin, "/api")]
pub async fn confirm_login(email: String, token: String) -> Result<AuthUser, ServerFnError> {
    let confirm_rate_limiter =
        expect_context::<std::sync::Arc<crate::auth::rate_limit::ConfirmRateLimiter>>();
    // Fail open if the client IP can't be determined (e.g. missing ConnectInfo
    // in a test harness) rather than blocking confirmation outright.
    if let Some(ip) = login_client_ip().await
        && let Err(wait_secs) =
            crate::auth::rate_limit::check_confirm_rate_limit(&confirm_rate_limiter, ip)
    {
        return Err(ServerFnError::new(format!(
            "Too many attempts. Try again in {}s.",
            wait_secs
        )));
    }

    let pool = expect_context::<PgPool>();

    // Find the user with this email and confirmation token, scoped to that
    // user's row so a code collision or brute-force attempt against another
    // pending login can't succeed here.
    let user_email = sqlx::query_as!(
        UserEmail,
        "SELECT * FROM user_emails WHERE email = $1 AND is_primary = true",
        email
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let user_email =
        user_email.ok_or_else(|| ServerFnError::new("Invalid or expired token".to_string()))?;

    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE id = $1 AND login_confirmation = $2 AND login_confirmation_at > NOW() - INTERVAL '1 hour'",
        user_email.user_id,
        token
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    let user = user.ok_or_else(|| ServerFnError::new("Invalid or expired token".to_string()))?;

    // Create auth token
    let auth_token_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)",
        auth_token_id,
        user.id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to create auth token: {}", e)))?;

    // Clear login confirmation
    sqlx::query!(
        "UPDATE users SET login_confirmation = NULL, login_confirmation_at = NULL WHERE id = $1",
        user.id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to clear confirmation: {}", e)))?;

    // Set session
    let session: Session = extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract session: {}", e)))?;
    set_user_session(&session, &user, &user_email.email, auth_token_id)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to set session: {}", e)))?;

    Ok(AuthUser {
        id: user.id,
        name: user.name,
        email: user_email.email,
    })
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    let session: Session = extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract session: {}", e)))?;
    let session_user = get_user_from_session(&session).await;

    if let Some(user) = session_user {
        let pool = expect_context::<PgPool>();
        // Validate token matches database
        if validate_session_token(&pool, user.auth_token_id)
            .await
            .unwrap_or(false)
        {
            return Ok(Some(AuthUser {
                id: user.id,
                name: user.name,
                email: user.email,
            }));
        } else {
            // Token invalid, clear session
            let _ = clear_user_session(&session).await;
        }
    }

    Ok(None)
}

#[server(Logout, "/api")]
pub async fn logout() -> Result<bool, ServerFnError> {
    let session: Session = extract()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract session: {}", e)))?;

    // Get user to check for auth token to invalidate
    if let Some(user) = get_user_from_session(&session).await {
        let pool = expect_context::<PgPool>();
        let _ = invalidate_auth_token(&pool, user.auth_token_id).await;
    }

    clear_user_session(&session)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to clear session: {}", e)))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use leptos::reactive::owner::Owner;

    async fn with_pool_context<F, Fut, T>(pool: &PgPool, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let owner = Owner::new();
        owner.with(|| {
            provide_context(pool.clone());
            provide_context(None::<resend_rs::Resend>);
            provide_context(crate::auth::rate_limit::build_login_rate_limiter());
            provide_context(crate::auth::rate_limit::build_confirm_rate_limiter());
        });
        owner
            .with(|| leptos::reactive::computed::ScopedFuture::new(f()))
            .await
    }

    #[sqlx::test]
    async fn login_rejects_invalid_email(pool: PgPool) {
        let resp = with_pool_context(&pool, || login("not-an-email".to_string()))
            .await
            .unwrap();
        assert!(!resp.success);
    }

    #[sqlx::test]
    async fn login_creates_user_and_sets_confirmation_token(pool: PgPool) {
        let email = "new-user@example.com";
        let resp = with_pool_context(&pool, || login(email.to_string()))
            .await
            .unwrap();
        assert!(resp.success);

        let row = sqlx::query!(
            "SELECT u.login_confirmation FROM users u
             JOIN user_emails ue ON ue.user_id = u.id
             WHERE ue.email = $1",
            email
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let token = row.login_confirmation.expect("confirmation token set");
        assert_eq!(token.len(), 6);
    }

    // `with_pool_context` always provides `None::<resend_rs::Resend>`, so
    // every successful `login()` test above already exercises the
    // RESEND_API_KEY-unset log-fallback path in `send_login_email` (it
    // would panic/hang trying to reach the real Resend API otherwise).
    // This test exercises `send_login_email` directly for that path.
    #[tokio::test]
    async fn send_login_email_logs_when_resend_unset() {
        // Must not panic or attempt any network I/O when `resend` is `None`.
        send_login_email(None, "someone@example.com", "123456").await;
    }

    #[sqlx::test]
    async fn confirm_login_rejects_wrong_token(pool: PgPool) {
        let result = with_pool_context(&pool, || {
            confirm_login("nobody@example.com".to_string(), "000000".to_string())
        })
        .await;
        assert!(result.is_err());
    }

    async fn insert_user_with_confirmation(
        pool: &PgPool,
        name: &str,
        email: &str,
        token: &str,
        confirmation_age: time::Duration,
    ) -> Uuid {
        let user_id = Uuid::new_v4();
        let confirmation_at = {
            let now = OffsetDateTime::now_utc();
            time::PrimitiveDateTime::new(now.date(), now.time()) - confirmation_age
        };
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors, login_confirmation, login_confirmation_at)
             VALUES ($1, $2, $3, $4, $5)",
            user_id,
            name,
            &Vec::<String>::new(),
            token,
            confirmation_at
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)",
            user_id,
            email
        )
        .execute(pool)
        .await
        .unwrap();
        user_id
    }

    #[sqlx::test]
    async fn confirm_login_rejects_expired_token(pool: PgPool) {
        insert_user_with_confirmation(
            &pool,
            "expired-user",
            "expired-user@example.com",
            "123456",
            time::Duration::hours(2),
        )
        .await;

        let result = with_pool_context(&pool, || {
            confirm_login("expired-user@example.com".to_string(), "123456".to_string())
        })
        .await;
        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn confirm_login_rejects_right_code_wrong_email(pool: PgPool) {
        insert_user_with_confirmation(
            &pool,
            "scoped-user",
            "scoped-user@example.com",
            "654321",
            time::Duration::minutes(1),
        )
        .await;

        // Right code, but for a different email than the one the code was
        // issued to: must not log in as the other user.
        let result = with_pool_context(&pool, || {
            confirm_login("someone-else@example.com".to_string(), "654321".to_string())
        })
        .await;
        assert!(result.is_err());
    }
}
