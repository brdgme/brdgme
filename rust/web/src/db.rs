
#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use anyhow::Result;
#[cfg(feature = "ssr")]
use crate::models::user::{User, NewUser, NewUserEmail};
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[cfg(feature = "ssr")]
pub async fn create_pool() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url).await?;
    
    // Run migrations (will skip existing tables)
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
}

#[cfg(feature = "ssr")]
impl AppState {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
}

#[cfg(feature = "ssr")]
pub async fn create_user(pool: &PgPool, new_user: NewUser) -> Result<User> {
    sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (name, pref_colors, login_confirmation, login_confirmation_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id, created_at, updated_at, name, pref_colors, login_confirmation, login_confirmation_at
        "#,
        new_user.name,
        &new_user.pref_colors,
        new_user.login_confirmation,
        new_user.login_confirmation_at
    )
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn create_user_email(pool: &PgPool, new_email: NewUserEmail) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO user_emails (user_id, email, is_primary)
        VALUES ($1, $2, $3)
        "#,
        new_email.user_id,
        new_email.email,
        new_email.is_primary
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "ssr")]
pub async fn get_user_by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT u.id, u.created_at, u.updated_at, u.name, u.pref_colors, u.login_confirmation, u.login_confirmation_at
        FROM users u
        JOIN user_emails ue ON u.id = ue.user_id
        WHERE ue.email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

#[cfg(feature = "ssr")]
pub async fn get_user(pool: &PgPool, id: Uuid) -> Result<Option<User>> {
    sqlx::query_as!(
        User,
        r#"
        SELECT id, created_at, updated_at, name, pref_colors, login_confirmation, login_confirmation_at
        FROM users
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}