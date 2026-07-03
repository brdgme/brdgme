use crate::websocket::GameBroadcaster;
use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: PgPool,
    pub broadcaster: GameBroadcaster,
    pub http_client: reqwest::Client,
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
