use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpError {
    #[error("mcp server error: {0}")]
    Server(String),
}
