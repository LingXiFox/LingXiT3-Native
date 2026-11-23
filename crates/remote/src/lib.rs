use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemoteError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
}
