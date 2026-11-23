use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdaterError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
