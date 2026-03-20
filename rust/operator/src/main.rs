mod controller;
mod crd;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&database_url).await?;

    let client = kube::Client::try_default().await?;
    tracing::info!("brdgme operator starting");

    controller::run(client, pool).await;
    Ok(())
}
