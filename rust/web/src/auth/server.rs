use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "ssr")]
use crate::models::user::{User, UserEmail};
#[cfg(feature = "ssr")]
use sqlx::PgPool;
#[cfg(feature = "ssr")]
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[server(Login, "/api")]
pub async fn login(email: String) -> Result<LoginResponse, ServerFnError> {
    // Validate email format
    if email.is_empty() || !email.contains('@') {
        return Ok(LoginResponse {
            success: false,
            message: "Invalid email address".to_string(),
            user_id: None,
        });
    }
    
    // Get database pool from Leptos context
    let pool = expect_context::<PgPool>();
    
    // Check if user exists with this email
    let user_email = sqlx::query_as!(
        UserEmail,
        "SELECT * FROM user_emails WHERE email = $1 AND is_primary = true",
        email
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Database error: {}", e)))?;
    
    let user_id = if let Some(user_email) = user_email {
        user_email.user_id
    } else {
        // Create new user if doesn't exist
        let new_user_id = Uuid::new_v4();
        let username = email.split('@').next().unwrap_or("user").to_string();
        
        // Insert new user
        sqlx::query!(
            "INSERT INTO users (id, name, pref_colors) VALUES ($1, $2, $3)",
            new_user_id,
            username,
            &Vec::<String>::new()
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create user: {}", e)))?;
        
        // Insert user email
        sqlx::query!(
            "INSERT INTO user_emails (user_id, email, is_primary) VALUES ($1, $2, true)",
            new_user_id,
            email
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to create user email: {}", e)))?;
        
        new_user_id
    };
    
    // Generate login confirmation token
    let confirmation_token = Uuid::new_v4().to_string();
    let confirmation_time = Utc::now().naive_utc();
    
    // Update user with login confirmation
    sqlx::query!(
        "UPDATE users SET login_confirmation = $1, login_confirmation_at = $2 WHERE id = $3",
        confirmation_token,
        confirmation_time,
        user_id
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(format!("Failed to update user: {}", e)))?;
    
    // TODO: Send email with confirmation link
    // For now, we'll just return success with the token for testing
    
    Ok(LoginResponse {
        success: true,
        message: format!("Login email sent! For testing, your confirmation token is: {}", confirmation_token),
        user_id: Some(user_id),
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
    
    // TODO: Set session with user data when session context is available
    
    Ok(AuthUser {
        id: user.id,
        name: user.name,
        email: user_email.email,
    })
}

#[server(GetCurrentUser, "/api")]
pub async fn get_current_user() -> Result<Option<AuthUser>, ServerFnError> {
    // TODO: Implement session-based user authentication
    // For now, return None (not logged in)
    Ok(None)
}

#[server(Logout, "/api")]
pub async fn logout() -> Result<bool, ServerFnError> {
    // TODO: Implement logout by clearing session and invalidating tokens
    Ok(true)
}