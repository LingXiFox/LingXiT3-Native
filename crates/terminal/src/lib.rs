use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error("pty error: {0}")]
    Pty(String),
    #[error("terminal not found: {0}")]
    NotFound(String),
}
