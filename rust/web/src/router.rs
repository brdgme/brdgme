//! Router construction shared between `main.rs` (production) and the 11.6a
//! in-process SSR page tests (`tests/ssr_pages.rs`), so both exercise the
//! exact same Axum/Leptos wiring (routes, session layer, fallback).
#![cfg(feature = "ssr")]

use crate::app::{App, shell};
use crate::state::AppState;
use axum::Router;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};

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
        .with_state(state)
}
