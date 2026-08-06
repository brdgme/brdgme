#![recursion_limit = "512"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum_prometheus::PrometheusMetricLayer;
    use leptos::logging::log;
    use leptos::prelude::*;
    use web::db::create_pool;
    use web::router::build_router;
    use web::state::AppState;
    use web::websocket::GameBroadcaster;

    // Both rustls backends are enabled in this binary's graph (reqwest ->
    // aws-lc-rs, sqlx/async-nats -> ring), so any crate reading the process
    // default provider would panic without an explicit install. See
    // docs/CODING.md "rustls crypto backends" and rust/operator/src/main.rs.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    dotenvy::dotenv().ok();
    init_tracing();
    // Runs after `init_tracing` (matches sentry-rust's own tracing-demo.rs
    // example, which sets up the tracing_subscriber registry before calling
    // `sentry::init`): the `sentry_tracing::layer()` installed in
    // `init_tracing` doesn't need an initialized client to be constructed -
    // it reads `Hub::current()` at each event, so any order works, but this
    // also lets `init_sentry`'s own "disabled" debug log go through the
    // already-installed subscriber.
    let _sentry_guard = init_sentry();

    web::crypto::load_key().expect("DATABASE_ENCRYPTION_KEY missing or malformed");
    if web::crypto::using_default_key() {
        tracing::warn!(
            "DATABASE_ENCRYPTION_KEY not set - using insecure default key (ALLOW_INSECURE_DEFAULT_KEY=true), DO NOT USE IN PRODUCTION"
        );
    }

    let turnstile_secret = std::env::var("TURNSTILE_SECRET_KEY").unwrap_or_default();
    if turnstile_secret.is_empty()
        && std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() != Ok("true")
    {
        panic!("TURNSTILE_SECRET_KEY not set - refusing to start without CAPTCHA verification");
    }

    let turnstile_site_key = std::env::var("TURNSTILE_SITE_KEY").unwrap_or_default();
    if turnstile_site_key.is_empty()
        && std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() != Ok("true")
    {
        panic!("TURNSTILE_SITE_KEY not set - refusing to start without CAPTCHA verification");
    }

    let public_base_url = web::config::public_base_url();
    if !public_base_url.starts_with("https://")
        && std::env::var("ALLOW_INSECURE_DEFAULT_KEY").as_deref() != Ok("true")
    {
        panic!(
            "PUBLIC_BASE_URL not set to an HTTPS URL - refusing to start without a valid production base URL"
        );
    }

    let pool = create_pool().await.expect("Failed to create database pool");
    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://nats:4222".to_string());
    let nats_client = async_nats::connect(&nats_url)
        .await
        .expect("Failed to connect to NATS");
    let jetstream = async_nats::jetstream::new(nats_client.clone());
    web::nats::ensure_stream_and_consumers(&jetstream)
        .await
        .expect("Failed to create/get BOT stream and consumers");
    let advisory_client = nats_client.clone();
    let broadcaster = GameBroadcaster::new(nats_client);

    let resend = std::env::var("RESEND_API_KEY")
        .ok()
        .map(|key| resend_rs::Resend::new(&key));
    if resend.is_none() {
        log!("RESEND_API_KEY not set; login emails will be logged instead of sent");
    }

    // Process-level shutdown token (R-11 / F-109): cancelled alongside
    // `broadcaster.begin_shutdown()` in the graceful-shutdown future below, and
    // observed by the bot-command consumer, the max-deliveries-advisory
    // listener, and every email/bot sweep so they wind down instead of being
    // killed mid-work at process exit. `main` holds the spawned `JoinHandle`s
    // and boundedly drains them after axum finishes serving.
    let shutdown = tokio_util::sync::CancellationToken::new();
    let mut background_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    background_tasks.push(tokio::spawn({
        let pool = pool.clone();
        let http_client = http_client.clone();
        let broadcaster = broadcaster.clone();
        let jetstream = jetstream.clone();
        let resend = resend.clone();
        let shutdown = shutdown.clone();
        async move {
            web::nats::supervise_consumer("bot-command", shutdown.clone(), move || {
                web::game::run_bot_command_consumer(
                    pool.clone(),
                    http_client.clone(),
                    broadcaster.clone(),
                    jetstream.clone(),
                    resend.clone(),
                    shutdown.clone(),
                )
            })
            .await;
        }
    }));
    background_tasks.push(tokio::spawn({
        let shutdown = shutdown.clone();
        async move {
            web::nats::supervise_consumer(
                "max-deliveries-advisory",
                shutdown.clone(),
                move || {
                    web::nats::run_max_deliveries_advisory_listener(
                        advisory_client.clone(),
                        shutdown.clone(),
                    )
                },
            )
            .await;
        }
    }));
    background_tasks.extend(web::email::sweep::spawn_periodic_sweeps(
        pool.clone(),
        resend.clone(),
        http_client.clone(),
        broadcaster.clone(),
        jetstream.clone(),
        shutdown.clone(),
    ));
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;

    let state = AppState {
        leptos_options: leptos_options.clone(),
        pool: pool.clone(),
        broadcaster: broadcaster.clone(),
        http_client: http_client.clone(),
        resend: resend.clone(),
        jetstream: jetstream.clone(),
    };

    // Wrapped around the already-built router (not inside `build_router`, which is
    // shared with the in-process SSR page tests) so `metrics::set_global_recorder`
    // is only ever called once per process, not once per test.
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let app = build_router(state).await.layer(prometheus_layer);

    tokio::spawn(serve_metrics(metric_handle));

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown({
        let broadcaster = broadcaster.clone();
        let shutdown = shutdown.clone();
        async move {
            shutdown_signal().await;
            broadcaster.begin_shutdown();
            shutdown.cancel();
        }
    })
    .await
    .unwrap();

    // Boundedly drain the background consumers/sweeps now that axum has stopped
    // (R-11 / F-109). Each task observed `shutdown` and is winding down; the
    // 5s bound (matching WP-36's original drain) caps the wait before the
    // process exits. SSE streams are NOT tracked here: they are already bounded
    // by axum's graceful shutdown plus the per-connection and shutdown
    // `CancellationToken`s added in R-10 (see events.rs), which is the accepted
    // AC3 resolution - the deleted WP-36 `TaskTracker` is deliberately not
    // reintroduced.
    let drain_bound = std::time::Duration::from_secs(5);
    if tokio::time::timeout(
        drain_bound,
        futures_util::future::join_all(background_tasks),
    )
    .await
    .is_err()
    {
        tracing::warn!(
            "background tasks did not drain within {drain_bound:?}; abandoning them"
        );
    }
}

#[cfg(feature = "ssr")]
fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false);

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(sentry_tracing::layer())
        .init();
}

/// Reads `SENTRY_DSN_SERVER` and, if set, initializes the Sentry Rust SDK
/// (error capture, panic hook, breadcrumbs from the `sentry_tracing` layer
/// installed in `init_tracing`, and the `sentry_tower` router layers in
/// `router.rs`). Returns the `ClientInitGuard` so `main` can hold it alive
/// for the process lifetime - dropping it early flushes and shuts down the
/// transport prematurely (same reasoning as `init_tracing`'s
/// `_tracer_provider`). Unset (dev/Tilt/CI default): returns `None` without
/// calling `sentry::init` at all, so the process boots normally and every
/// Sentry integration point elsewhere in the codebase is a documented no-op.
#[cfg(feature = "ssr")]
fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let Ok(dsn) = std::env::var("SENTRY_DSN_SERVER") else {
        tracing::debug!("SENTRY_DSN_SERVER not set; Sentry error tracking disabled");
        return None;
    };
    let release = std::env::var("SENTRY_RELEASE")
        .ok()
        .map(std::borrow::Cow::Owned);
    Some(sentry::init((
        dsn,
        sentry::ClientOptions {
            release,
            // Sentry's own quickstart examples default this to `true`; brdgme
            // opts out so client IPs, cookies, and auth headers are never
            // sent to the hosted Sentry SaaS instance without a separate,
            // explicit future decision (WS3 plan, 2026-07-15).
            send_default_pii: false,
            traces_sample_rate: 0.1,
            ..Default::default()
        },
    )))
}

/// Serves `/metrics` in Prometheus text format on a private port, separate from
/// the main site port (which is reachable via the public Gateway). Not exposed
/// via any k8s Service or HTTPRoute - only reachable by something with direct
/// pod-network access, e.g. an in-cluster Prometheus/Alloy scrape.
#[cfg(feature = "ssr")]
async fn serve_metrics(handle: axum_prometheus::metrics_exporter_prometheus::PrometheusHandle) {
    async fn render(
        axum::extract::State(handle): axum::extract::State<
            axum_prometheus::metrics_exporter_prometheus::PrometheusHandle,
        >,
    ) -> String {
        handle.render()
    }

    let metrics_addr = std::env::var("METRICS_ADDR").unwrap_or_else(|_| "0.0.0.0:9090".to_string());
    let app = axum::Router::new()
        .route("/metrics", axum::routing::get(render))
        .with_state(handle);
    let listener = match tokio::net::TcpListener::bind(&metrics_addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("Failed to bind metrics listener on {}: {}", metrics_addr, e);
            return;
        }
    };
    tracing::info!(metrics_addr = %metrics_addr, "Metrics endpoint listening");
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Metrics server failed: {}", e);
    }
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
