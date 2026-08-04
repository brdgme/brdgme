//! Regression tests for `PostgresStore::migrate()` (F-200 / R-44).
//!
//! `migrate()` is the sole creator of `tower_sessions.session`, but its
//! duplicate-key branch returns `Ok(())` before `create table` and without
//! committing. These tests encode the R-44 acceptance criteria against a fresh
//! `#[sqlx::test]` database and are expected to fail on the current vendored
//! code.

use std::time::{Duration, Instant};

use brdgme_session_store::sqlx::postgres::{PgPool, PgPoolOptions};
use brdgme_session_store::{PostgresStore, sqlx};

/// The store's default schema/table names (see `PostgresStore::new`).
const SESSION_SCHEMA: &str = "tower_sessions";
const SESSION_TABLE: &str = "session";

async fn session_table_exists(pool: &PgPool) -> bool {
    let count: i64 = sqlx::query_scalar(
        "select count(*) from information_schema.tables \
         where table_schema = $1 and table_name = $2",
    )
    .bind(SESSION_SCHEMA)
    .bind(SESSION_TABLE)
    .fetch_one(pool)
    .await
    .expect("check session table existence");
    count > 0
}

/// Waits until the store's concurrent `create schema` has hit the duplicate-key
/// race and is blocked behind the control connection's uncommitted schema row.
async fn wait_for_blocked_create_schema(pool: &PgPool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let blocked: i64 = sqlx::query_scalar(
            "select count(*) from pg_stat_activity \
             where datname = current_database() and state = 'active' \
               and query ilike '%create schema%' \
               and pid <> pg_backend_pid()",
        )
        .fetch_one(pool)
        .await
        .expect("observe pg_stat_activity");
        if blocked > 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the store's concurrent create schema was never observed"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// R-44 AC1: two connections cold-start `migrate()` against a fresh database
/// and the session table must exist afterwards.
///
/// On the current code one of the two races loses `create schema`, hits the
/// duplicate-key branch, and returns `Ok(())` early. Whether the table exists
/// afterwards depends on which side won, so this test also exercises the
/// F-201 risk that the 0.9 error text no longer matches the swallow substring
/// (in which case the loser propagates `Err` instead).
#[sqlx::test(migrations = false)]
async fn concurrent_cold_start_migrate_creates_session_table(pool: PgPool) {
    let second_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(pool.connect_options().as_ref().clone())
        .await
        .expect("connect second pool");
    let store = PostgresStore::new(pool.clone());
    let second_store = PostgresStore::new(second_pool);

    let (first, second) = tokio::join!(store.migrate(), second_store.migrate());
    first.expect("first concurrent migrate() failed");
    second.expect("second concurrent migrate() failed");

    assert!(
        session_table_exists(&pool).await,
        "concurrent cold-start migrate() did not leave {SESSION_SCHEMA}.{SESSION_TABLE} existing"
    );
}

/// R-44 AC2: `migrate()` must not report success when its table was not
/// created.
///
/// Holds an uncommitted `tower_sessions` schema row on a control connection so
/// the store's `create schema if not exists` deterministically collides with it
/// and takes the duplicate-key branch. On the current code that branch returns
/// `Ok(())` before creating the table, so the assertion fails (RED).
#[sqlx::test(migrations = false)]
async fn migrate_does_not_report_success_when_session_table_missing(pool: PgPool) {
    let mut ctrl = pool.acquire().await.expect("acquire control connection");
    sqlx::query("begin")
        .execute(&mut *ctrl)
        .await
        .expect("begin control tx");
    sqlx::query("create schema if not exists tower_sessions")
        .execute(&mut *ctrl)
        .await
        .expect("hold uncommitted schema row");

    let store = PostgresStore::new(pool.clone());
    let migrate = store.migrate();
    let commit = async {
        wait_for_blocked_create_schema(&pool).await;
        sqlx::query("commit")
            .execute(&mut *ctrl)
            .await
            .expect("commit control tx");
    };

    let (result, ()) = tokio::join!(migrate, commit);
    drop(ctrl);

    if let Ok(()) = result {
        assert!(
            session_table_exists(&pool).await,
            "migrate() returned Ok but the session table was not created"
        )
    }
}
