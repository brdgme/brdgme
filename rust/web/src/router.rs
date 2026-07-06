//! Router construction shared between `main.rs` (production) and the 11.6a
//! in-process SSR page tests (`tests/ssr_pages.rs`), so both exercise the
//! exact same Axum/Leptos wiring (routes, session layer, fallback).
#![cfg(feature = "ssr")]

use crate::app::{App, shell};
use crate::state::AppState;
use axum::Router;
use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use opentelemetry::trace::{TraceContextExt, TraceId};
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Root span for every HTTP request, carrying route (matched path, not raw path -
/// same low-cardinality reasoning as the `/metrics` labels), status, and latency.
/// `trace_id` is recorded from the real OTel trace id (once the OTel layer in
/// `main.rs::init_tracing` has attached a context to this span via its
/// `on_new_span` hook, which runs synchronously as part of span creation) so
/// JSON logs emitted while this span is active carry the same id Tempo uses,
/// letting Grafana link logs <-> traces. If no OTel layer is installed (dev,
/// no `OTEL_EXPORTER_OTLP_ENDPOINT`), the trace id is the noop `TraceId::INVALID`
/// and is left unrecorded.
fn make_root_span(request: &Request<axum::body::Body>) -> tracing::Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path());
    let span = tracing::info_span!(
        "http_request",
        method = %request.method(),
        route = %route,
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
        trace_id = tracing::field::Empty,
    );
    let trace_id = span.context().span().span_context().trace_id();
    if trace_id != TraceId::INVALID {
        span.record("trace_id", trace_id.to_string());
    }
    span
}

fn record_response(response: &Response<axum::body::Body>, latency: Duration, span: &tracing::Span) {
    span.record("status", response.status().as_u16());
    span.record("latency_ms", latency.as_millis() as u64);
}

pub async fn build_router(state: AppState) -> Router {
    let routes = generate_route_list(App);
    let session_layer = crate::auth::session::create_session_layer(&state.pool).await;

    Router::new()
        .leptos_routes_with_context(
            &state,
            routes,
            {
                let pool = state.pool.clone();
                let broadcaster = state.broadcaster.clone();
                let http_client = state.http_client.clone();
                let resend = state.resend.clone();
                let login_rate_limiter = state.login_rate_limiter.clone();
                let confirm_rate_limiter = state.confirm_rate_limiter.clone();
                let jetstream = state.jetstream.clone();
                move || {
                    provide_context(pool.clone());
                    provide_context(broadcaster.clone());
                    provide_context(http_client.clone());
                    provide_context(resend.clone());
                    provide_context(login_rate_limiter.clone());
                    provide_context(confirm_rate_limiter.clone());
                    provide_context(jetstream.clone());
                }
            },
            {
                let leptos_options = state.leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .route("/ws", axum::routing::get(crate::websocket::ws_handler))
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>({
            let leptos_options = state.leptos_options.clone();
            move |_| shell(leptos_options.clone())
        }))
        .layer(session_layer)
        // Added after `session_layer` so the DB-backed session middleware
        // never runs for this route: a Postgres outage must not fail the
        // probe or cause k8s to restart/de-endpoint the pod, since web still
        // needs to serve error pages and the WS layer independently of the
        // database being up.
        .route("/healthz", axum::routing::get(healthz))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_root_span)
                .on_response(record_response),
        )
        .with_state(state)
}

async fn healthz() -> &'static str {
    "OK"
}
