use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git error: {0}")]
    Git(#[from] lingxi_git::GitError),
    #[error("path outside workspace boundary: {0}")]
    OutsideBoundary(String),
}
