use sqlx::postgres::PgPool;
use anyhow::Result;

pub async fn create_pool() -> Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url).await?;
    
    // Run migrations (will skip existing tables/functions/triggers)
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}

pub type DbPool = PgPool;