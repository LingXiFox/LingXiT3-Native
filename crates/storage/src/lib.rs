pub mod db;
pub mod legacy_importer;
pub mod migrations;

pub use db::Database;
pub use legacy_importer::{LegacyImportReport, import_legacy_readonly, rollback_import};
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
