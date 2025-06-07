#[cfg(feature = "ssr")]
use sqlx::postgres::PgPool;
#[cfg(feature = "ssr")]
use anyhow::Result;

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