// Vendored from tower-sessions-sqlx-store 0.15.0 (MIT licence).
// Upstream: https://github.com/maxcountryman/tower-sessions-stores
// Only the Postgres store is included; MySQL and SQLite stores are dropped.

pub use sqlx;
use tower_sessions::session_store;

pub use self::postgres_store::PostgresStore;

mod postgres_store;

#[derive(thiserror::Error, Debug)]
pub enum SqlxStoreError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),

    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),
}

impl From<SqlxStoreError> for session_store::Error {
    fn from(err: SqlxStoreError) -> Self {
        match err {
            SqlxStoreError::Sqlx(inner) => session_store::Error::Backend(inner.to_string()),
            SqlxStoreError::Decode(inner) => session_store::Error::Decode(inner.to_string()),
            SqlxStoreError::Encode(inner) => session_store::Error::Encode(inner.to_string()),
        }
    }
}
