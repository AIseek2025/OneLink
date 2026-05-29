pub mod postgres;

use std::fmt;

use postgres::PostgresStore;

#[derive(Debug)]
pub enum StoreError {
    Postgres(tokio_postgres::Error),
    Pool(deadpool_postgres::PoolError),
    InvalidId(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Postgres(e) => write!(f, "postgres: {e}"),
            StoreError::Pool(e) => write!(f, "pool: {e}"),
            StoreError::InvalidId(s) => write!(f, "invalid id: {s}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Postgres(e) => Some(e),
            StoreError::Pool(e) => Some(e),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for StoreError {
    fn from(e: tokio_postgres::Error) -> Self {
        StoreError::Postgres(e)
    }
}

impl From<deadpool_postgres::PoolError> for StoreError {
    fn from(e: deadpool_postgres::PoolError) -> Self {
        StoreError::Pool(e)
    }
}

#[derive(Debug)]
pub enum PersistenceBackend {
    InMemory,
    Postgres(PostgresStore),
}

impl PersistenceBackend {
    pub async fn from_config(database_url: Option<&str>) -> Self {
        match database_url {
            Some(url) if !url.trim().is_empty() => match PostgresStore::connect(url).await {
                Ok(pg) => {
                    tracing::info!("safety-service: connected to Postgres");
                    PersistenceBackend::Postgres(pg)
                }
                Err(e) => {
                    tracing::error!(error = %e, "safety-service: Postgres connect failed — refusing silent in-memory fallback");
                    panic!(
                        "safety-service: FATAL: Postgres connect failed (DATABASE_URL was set). \
                         Silent in-memory fallback is forbidden for shared environments. \
                         Fix the database connection or unset DATABASE_URL to explicitly use dev-only in-memory mode. \
                         Error: {e}"
                    );
                }
            },
            _ => {
                tracing::info!(
                    "safety-service: no DATABASE_URL, using in-memory store — dev/smoke only"
                );
                PersistenceBackend::InMemory
            }
        }
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, PersistenceBackend::Postgres(_))
    }

    pub fn postgres_store(&self) -> Option<&PostgresStore> {
        match self {
            PersistenceBackend::Postgres(pg) => Some(pg),
            _ => None,
        }
    }
}

impl fmt::Display for PersistenceBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceBackend::InMemory => write!(f, "in-memory"),
            PersistenceBackend::Postgres(_) => write!(f, "postgres"),
        }
    }
}
