use thiserror::Error;

#[derive(Debug, Error)]
pub enum AcpError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("protocol error ({code}): {message}")]
    Protocol { code: i64, message: String },
    #[error("timeout: {0}")]
    Timeout(String),
}
