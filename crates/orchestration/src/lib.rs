use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestrationError {
    #[error("domain error: {0}")]
    Domain(#[from] lingxi_domain::DomainError),
    #[error("storage error: {0}")]
    Storage(#[from] lingxi_storage::StorageError),
}
