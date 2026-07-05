use crate::auth::rate_limit::{ConfirmRateLimiter, LoginRateLimiter};
use crate::websocket::GameBroadcaster;
use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: PgPool,
    pub broadcaster: GameBroadcaster,
    pub http_client: reqwest::Client,
    /// `None` when `RESEND_API_KEY` is unset, in which case login emails are
    /// logged instead of sent.
    pub resend: Option<resend_rs::Resend>,
    pub login_rate_limiter: Arc<LoginRateLimiter>,
    pub confirm_rate_limiter: Arc<ConfirmRateLimiter>,
    pub jetstream: async_nats::jetstream::Context,
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(app_state: &AppState) -> LeptosOptions {
        app_state.leptos_options.clone()
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(app_state: &AppState) -> PgPool {
        app_state.pool.clone()
    }
}

impl FromRef<AppState> for GameBroadcaster {
    fn from_ref(app_state: &AppState) -> GameBroadcaster {
        app_state.broadcaster.clone()
    }
}

impl FromRef<AppState> for reqwest::Client {
    fn from_ref(app_state: &AppState) -> reqwest::Client {
        app_state.http_client.clone()
    }
}

impl FromRef<AppState> for async_nats::jetstream::Context {
    fn from_ref(app_state: &AppState) -> async_nats::jetstream::Context {
        app_state.jetstream.clone()
    }
}
