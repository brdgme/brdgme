use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use sqlx::PgPool;
use crate::websocket::GameBroadcaster;

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: PgPool,
    pub broadcaster: GameBroadcaster,
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
