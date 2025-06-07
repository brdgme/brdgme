use dioxus::prelude::*;
use crate::db::DbPool;
use crate::models::User;

#[derive(Clone)]
pub struct AppState {
    pub current_user: Signal<Option<User>>,
    pub db_pool: DbPool,
}

impl AppState {
    pub fn new(db_pool: DbPool) -> Self {
        Self {
            current_user: Signal::new(None),
            db_pool,
        }
    }
}