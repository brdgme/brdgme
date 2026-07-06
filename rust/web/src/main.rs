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

    dotenvy::dotenv().ok();
    let _tracer_provider = init_tracing();

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
    let broadcaster = GameBroadcaster::new(nats_client);

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
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

/// Sets up the `tracing_subscriber` registry: JSON logs to stdout always, plus an
/// OTLP trace export layer when `OTEL_EXPORTER_OTLP_ENDPOINT` is set (dev needs no
/// collector running - the layer is simply not installed if the env var is unset).
/// `with_current_span(true)` is required so the `trace_id` field recorded on the
/// root span (see `router.rs`'s `TraceLayer`) is copied onto every log line's JSON
/// output while that span is active, giving Grafana a logs<->traces join key.
/// Returns the `SdkTracerProvider` so `main` can keep it alive for the process
/// lifetime (dropping it would trigger the SDK's shutdown/flush early).
#[cfg(feature = "ssr")]
fn init_tracing() -> Option<opentelemetry_sdk::trace::SdkTracerProvider> {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_otlp::WithExportConfig;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok();

    let mut bad_sampler_arg = None;
    let ratio = std::env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|v| {
            v.parse::<f64>().ok().or_else(|| {
                bad_sampler_arg = Some(v);
                None
            })
        })
        .unwrap_or(1.0);

    let mut exporter_error = None;
    let (otel_layer, provider) = match &endpoint {
        Some(endpoint) => {
            let service_name =
                std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "web".to_string());
            match opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .build()
            {
                Ok(exporter) => {
                    let resource = opentelemetry_sdk::Resource::builder()
                        .with_service_name(service_name)
                        .build();
                    let sampler = opentelemetry_sdk::trace::Sampler::ParentBased(Box::new(
                        opentelemetry_sdk::trace::Sampler::TraceIdRatioBased(ratio),
                    ));
                    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                        .with_batch_exporter(exporter)
                        .with_sampler(sampler)
                        .with_resource(resource)
                        .build();
                    let tracer = provider.tracer("web");
                    (
                        Some(tracing_opentelemetry::layer().with_tracer(tracer)),
                        Some(provider),
                    )
                }
                Err(e) => {
                    exporter_error = Some(e.to_string());
                    (None, None)
                }
            }
        }
        None => (None, None),
    };

    let fmt_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    if let Some(bad_value) = bad_sampler_arg {
        tracing::warn!(
            value = %bad_value,
            "invalid OTEL_TRACES_SAMPLER_ARG; falling back to sample ratio 1.0"
        );
    }
    if let Some(e) = exporter_error {
        tracing::warn!(
            error = %e,
            "failed to build OTLP span exporter; trace export disabled"
        );
    }

    provider
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
