//! Router construction shared between `main.rs` (production) and the 11.6a
//! in-process SSR page tests (`tests/ssr_pages.rs`), so both exercise the
//! exact same Axum/Leptos wiring (routes, session layer, fallback).
#![cfg(feature = "ssr")]

use crate::app::{App, shell};
use crate::state::AppState;
use axum::Router;
use axum::extract::MatchedPath;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Request, Response, StatusCode};
use axum::middleware::{self, Next};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use sentry_tower::{NewSentryLayer, SentryHttpLayer};
use std::sync::LazyLock;
use std::time::Duration;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

/// Server-fn payloads are small forms, so 256 KiB is generous headroom.
const MAX_REQUEST_BODY_BYTES: usize = 256 * 1024;

/// Bounds how long a request's handler future may run before the layer
/// synthesizes a response - this does NOT bound `/ws`'s connection lifetime.
/// `WebSocketUpgrade::on_upgrade` (axum) returns the 101 response and hands
/// the actual socket off to a detached `tokio::spawn`ed task immediately, so
/// the handler future this layer times completes almost instantly regardless
/// of how long the socket stays open afterwards; a slow HTTP handler (or a
/// stalled leptos server-fn) is what this actually guards against.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

fn make_root_span(request: &Request<axum::body::Body>) -> tracing::Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or_else(|| request.uri().path());
    tracing::info_span!(
        "http_request",
        method = %request.method(),
        route = %route,
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    )
}

fn record_response(response: &Response<axum::body::Body>, latency: Duration, span: &tracing::Span) {
    span.record("status", response.status().as_u16());
    span.record("latency_ms", latency.as_millis() as u64);
}

/// Sets `Cache-Control` on responses that don't already carry one: `/pkg/`
/// assets (content-hashed via `hash-files` in web/Cargo.toml) get a
/// year-long immutable cache, since a new deploy ships new filenames rather
/// than mutating existing ones; other `text/html` responses get `no-cache`
/// so deploys switch which hashed asset URLs a page references without a
/// stale cached page pinning a client to old ones. Error responses under
/// `/pkg/` (e.g. a stale/missing hashed asset) are not cached as immutable.
/// See docs/decisions/ASSET_CACHING.md.
fn sentry_transaction_name(method: &str, route: &str) -> String {
    format!("{} {}", method, route)
}

async fn set_sentry_transaction_name(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    if let Some(route) = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
    {
        let name = sentry_transaction_name(request.method().as_str(), route);
        sentry::configure_scope(|scope| scope.set_transaction(Some(&name)));
    }
    next.run(request).await
}

async fn set_cache_control(
    request: Request<axum::body::Body>,
    next: Next,
) -> Response<axum::body::Body> {
    let is_pkg = request.uri().path().starts_with("/pkg/");
    let mut response = next.run(request).await;
    let is_success = response.status().is_success();
    let headers = response.headers_mut();
    if !headers.contains_key(CACHE_CONTROL) {
        if is_pkg && is_success {
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
        } else if headers
            .get(CONTENT_TYPE)
            .is_some_and(|ct| ct.as_bytes().starts_with(b"text/html"))
        {
            headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        }
    }
    response
}

/// `generate_route_list` sets a process-global `IS_SUPPRESSING_RESOURCE_LOAD`
/// flag while it walks the component tree (leptos-rs/leptos#4773). Calling it
/// once per `build_router` invocation means parallel `#[sqlx::test]`s each
/// open that window, and any concurrent SSR render whose `Resource` first-polls
/// inside it commits to `pending().await` forever - hanging until the
/// `TimeoutLayer` fires (408). Generating the list exactly once per process
/// shrinks the window to a single, early, sub-millisecond event.
static ROUTES: LazyLock<Vec<leptos_axum::AxumRouteListing>> =
    LazyLock::new(|| generate_route_list(App));

pub async fn build_router(state: AppState) -> Router {
    let routes = ROUTES.clone();
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
                let jetstream = state.jetstream.clone();
                move || {
                    provide_context(pool.clone());
                    provide_context(broadcaster.clone());
                    provide_context(http_client.clone());
                    provide_context(resend.clone());
                    provide_context(jetstream.clone());
                }
            },
            {
                let leptos_options = state.leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .route("/ws", axum::routing::get(crate::websocket::ws_handler))
        .route(
            "/admin/games/{id}/export",
            axum::routing::get(crate::game::export::admin_export_game),
        )
        .route(
            "/api/webhooks/resend",
            axum::routing::post(crate::email::inbound::resend_webhook),
        )
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>({
            let leptos_options = state.leptos_options.clone();
            move |_| shell(leptos_options.clone())
        }))
        .layer(session_layer)
        .layer(middleware::from_fn(set_cache_control))
        // Added after `session_layer` so the DB-backed session middleware
        // never runs for this route: a Postgres outage must not fail the
        // probe or cause k8s to restart/de-endpoint the pod, since web still
        // needs to serve error pages and the WS layer independently of the
        // database being up.
        .route("/healthz", axum::routing::get(healthz))
        // Global HTTP hygiene, not abuse-proofing (kept deliberately, spec
        // W9 of the 2026-07-10 #28 WP4 design): these two layers stop a
        // stray oversized POST or a wedged handler from tying up a
        // worker/connection, and still cover direct-to-LB traffic that
        // bypasses Cloudflare. Hard abuse quotas are the WP1 DB-backed send
        // caps (`login()`'s cooldown/per-email/global caps in
        // `auth/server.rs`) - replica-safe and restart-proof because they
        // live in Postgres; per-IP rate limiting happens at the Cloudflare
        // edge, not in-app. Added after `/healthz` (like `TraceLayer`
        // below) so both apply to it too, which is harmless since the probe
        // is bodyless and returns immediately. Placed before `TraceLayer`
        // so it stays the outermost layer and still records a span (with
        // e.g. a 413/408 status) for requests these reject.
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_root_span)
                .on_response(record_response),
        )
        // Outermost: binds a fresh Sentry hub per request, then attaches
        // request metadata (method/URL/headers, PII-scrubbed unless
        // `send_default_pii` is set) to anything captured while handling it.
        // Declared Http-then-NewSentry because axum applies `.layer()` calls
        // in the opposite order to `tower::ServiceBuilder` (see sentry-tower's
        // crate docs) - NewSentryLayer ends up outermost, wrapping every
        // other layer above including `/healthz` and `TraceLayer`. Both are
        // safe to add unconditionally: with no Sentry client bound (dev/Tilt/
        // CI, and every call from `tests/ssr_pages.rs`, which never sets
        // `SENTRY_DSN_SERVER`), they only shuffle a no-op `Hub` around and
        // never send anything (sentry-tower 0.48.5 source).
        .layer(middleware::from_fn(set_sentry_transaction_name))
        .layer(SentryHttpLayer::new().enable_transaction())
        .layer(NewSentryLayer::<Request<axum::body::Body>>::new_from_top())
        .with_state(state)
}

async fn healthz() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    #[test]
    fn transaction_name_format() {
        assert_eq!(
            super::sentry_transaction_name("GET", "/admin/games/{id}/export"),
            "GET /admin/games/{id}/export"
        );
    }
}
