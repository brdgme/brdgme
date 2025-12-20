use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub pool: PgPool,
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
