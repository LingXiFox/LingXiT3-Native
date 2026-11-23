pub mod db;
pub mod migrations;

pub use db::Database;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("rollback error: {0}")]
    Rollback(String),
}
