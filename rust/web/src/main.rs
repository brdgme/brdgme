#![recursion_limit = "512"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use leptos::logging::log;
    use leptos::prelude::*;
    use web::db::create_pool;
    use web::router::build_router;
    use web::state::AppState;
    use web::websocket::GameBroadcaster;

    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let pool = create_pool().await.expect("Failed to create database pool");
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");
    let redis_conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis");
    let broadcaster = GameBroadcaster::new(redis_conn, redis_client);
    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
    let jetstream = web::nats::connect(&nats_url)
        .await
        .expect("Failed to connect to NATS");
    web::nats::ensure_stream_and_consumers(&jetstream)
        .await
        .expect("Failed to create/get BOT stream and consumers");

    tokio::spawn({
        let pool = pool.clone();
        let http_client = http_client.clone();
        let broadcaster = broadcaster.clone();
        let jetstream = jetstream.clone();
        async move {
            if let Err(e) =
                web::game::run_bot_command_consumer(pool, http_client, broadcaster, jetstream).await
            {
                tracing::error!("bot.command consumer exited: {}", e);
            }
        }
    });
    let resend = std::env::var("RESEND_API_KEY")
        .ok()
        .map(|key| resend_rs::Resend::new(&key));
    if resend.is_none() {
        log!("RESEND_API_KEY not set; login emails will be logged instead of sent");
    }
    let login_rate_limiter = web::auth::rate_limit::build_login_rate_limiter();
    let confirm_rate_limiter = web::auth::rate_limit::build_confirm_rate_limiter();

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let state = AppState {
        leptos_options: leptos_options.clone(),
        pool: pool.clone(),
        broadcaster: broadcaster.clone(),
        http_client: http_client.clone(),
        resend: resend.clone(),
        login_rate_limiter: login_rate_limiter.clone(),
        confirm_rate_limiter: confirm_rate_limiter.clone(),
        jetstream: jetstream.clone(),
    };

    let app = build_router(state).await;

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

#[cfg(feature = "ssr")]
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for pure client-side testing
    // see lib.rs for hydration function instead
}
