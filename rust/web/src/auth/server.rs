use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "ssr")]
use crate::models::user::{User, UserEmail};
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use time::OffsetDateTime;
#[cfg(feature = "ssr")]
use crate::auth::session::{set_user_session, get_user_from_session, clear_user_session, validate_session_token, invalidate_auth_token};
#[cfg(feature = "ssr")]
use tower_sessions::Session;
#[cfg(feature = "ssr")]
use leptos_axum::extract;
#[cfg(feature = "ssr")]
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

#[cfg(feature = "ssr")]
async fn send_login_email(to_email: &str, token: &str) {
    let smtp_host = match std::env::var("SMTP_HOST") {
        Ok(h) => h,
        Err(_) => {
            println!("\n==> LOGIN CODE for {}: {}\n", to_email, token);
            return;
        }
    };
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(25);
    let from_addr = std::env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@brdgme.com".to_string());

    let from_mailbox = match from_addr.parse() {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Invalid SMTP_FROM address '{}': {}", from_addr, e);
            return;
        }
    };

    let email = match Message::builder()
        .from(from_mailbox)
        .to(match to_email.parse() {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("Invalid to address {}: {}", to_email, e);
                return;
            }
        })
        .subject("Your brdgme login code")
        .body(format!("Your login code is: {}\n\nThis code expires in 1 hour.", token))
    {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to build email: {}", e);
            return;
        }
    };

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&smtp_host)
        .port(smtp_port)
        .build();

    if let Err(e) = mailer.send(email).await {
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

#[server(Login, "/api")]
pub async fn login(email: String) -> Result<LoginResponse, ServerFnError> {
    if email.is_empty() || !email.contains('@') {
        return Ok(LoginResponse {
            success: false,
            message: "Invalid email address".to_string(),
        });
    }

    let pool = expect_context::<PgPool>();
    let mut tx = pool.begin().await
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

    tx.commit().await
        .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;

    send_login_email(&email, &confirmation_token).await;

    Ok(LoginResponse {
        success: true,
        message: "Login email sent".to_string(),
    })
}

#[server(ConfirmLogin, "/api")]
pub async fn confirm_login(token: String) -> Result<AuthUser, ServerFnError> {
    let pool = expect_context::<PgPool>();
    
    // Find user with this confirmation token
    let user = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE login_confirmation = $1 AND login_confirmation_at > NOW() - INTERVAL '1 hour'",
        token
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let user = user.ok_or_else(|| ServerFnError::new("Invalid or expired token".to_string()))?;
    
    // Get user's primary email
    let user_email = sqlx::query_as!(
        UserEmail,
        "SELECT * FROM user_emails WHERE user_id = $1 AND is_primary = true",
        user.id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
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
    let session: Session = extract().await.map_err(|e| ServerFnError::new(format!("Failed to extract session: {}", e)))?;
    set_user_session(&session, &user, &user_email.email, auth_token_id).await
        .map_err(|e| ServerFnError::new(format!("Failed to set session: {}", e)))?;
    
    Ok(AuthUser {
        id: user.id,
        name: user.name,
        email: user_email.email,
    })
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    let session: Session = extract().await.map_err(|e| ServerFnError::new(format!("Failed to extract session: {}", e)))?;
    let session_user = get_user_from_session(&session).await;
    
    if let Some(user) = session_user {
        let pool = expect_context::<PgPool>();
        // Validate token matches database
        if validate_session_token(&pool, user.auth_token_id).await.unwrap_or(false) {
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
    let session: Session = extract().await.map_err(|e| ServerFnError::new(format!("Failed to extract session: {}", e)))?;
    
    // Get user to check for auth token to invalidate
    if let Some(user) = get_user_from_session(&session).await {
        let pool = expect_context::<PgPool>();
        let _ = invalidate_auth_token(&pool, user.auth_token_id).await;
    }
    
    clear_user_session(&session).await
        .map_err(|e| ServerFnError::new(format!("Failed to clear session: {}", e)))?;
        
    Ok(true)
}