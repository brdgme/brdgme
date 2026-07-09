mod controller;
mod crd;

use axum::{Router, http::StatusCode, routing::get};

async fn healthz() -> StatusCode {
    StatusCode::OK
}

/// Serves `/healthz` on `LISTEN_ADDR`. Spawned only after the `kube::Client`
/// and DB pool are constructed in `main`, so a 200 means startup
/// dependencies are established (same pattern as the bot's `/healthz`).
async fn serve_health(listen_addr: String) -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/healthz", get(healthz));
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    tracing::info!(listen_addr = %listen_addr, "Operator health endpoint listening");
    axum::serve(listener, app).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = sqlx::PgPool::connect(&database_url).await?;

    let client = kube::Client::try_default().await?;
    tracing::info!("brdgme operator starting");

    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:4000".to_string());
    tokio::spawn(async move {
        if let Err(e) = serve_health(listen_addr).await {
            tracing::error!("Operator health endpoint failed: {}", e);
        }
    });

    controller::run(client, pool).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn healthz_returns_ok() {
        assert_eq!(healthz().await, StatusCode::OK);
    }
}
