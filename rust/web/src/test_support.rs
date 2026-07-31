use std::future::Future;
use std::sync::Arc;

use leptos::prelude::provide_context;
use leptos::reactive::owner::Owner;
use sqlx::PgPool;
use tower_sessions::{MemoryStore, Session};
use uuid::Uuid;

use crate::auth::session::{SessionUser, SESSION_USER_KEY};

async fn run_with_session<F, Fut, T>(pool: &PgPool, session: Session, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let (mut parts, _) = axum::http::Request::new(()).into_parts();
    parts.extensions.insert(session);

    // Contexts must be provided in a separate `Owner::with` before the scoped
    // future runs, so `extract`/`expect_context` inside `f` see them.
    let owner = Owner::new();
    owner.with(|| {
        provide_context(pool.clone());
        provide_context(parts);
    });
    owner
        .with(|| leptos::reactive::computed::ScopedFuture::new(f()))
        .await
}

async fn seed_user(pool: &PgPool, is_admin: bool) -> SessionUser {
    let name = format!("test_support_{}", Uuid::new_v4());
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (name, pref_colors, is_admin) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&name)
    .bind(Vec::<String>::new())
    .bind(is_admin)
    .fetch_one(pool)
    .await
    .unwrap();

    let auth_token_id = Uuid::new_v4();
    sqlx::query("INSERT INTO user_auth_tokens (id, user_id) VALUES ($1, $2)")
        .bind(auth_token_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();

    SessionUser {
        id: user_id,
        name,
        email: format!("{}@example.com", Uuid::new_v4()),
        auth_token_id,
    }
}

async fn authenticated<F, Fut, T>(pool: &PgPool, is_admin: bool, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    session
        .insert(SESSION_USER_KEY, seed_user(pool, is_admin).await)
        .await
        .unwrap();
    run_with_session(pool, session, f).await
}

pub async fn anonymous<F, Fut, T>(pool: &PgPool, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let session = Session::new(None, Arc::new(MemoryStore::default()), None);
    run_with_session(pool, session, f).await
}

pub async fn non_admin<F, Fut, T>(pool: &PgPool, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    authenticated(pool, false, f).await
}

pub async fn admin<F, Fut, T>(pool: &PgPool, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    authenticated(pool, true, f).await
}
