use lingxi_domain::ProviderKind;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider {0:?} is not installed")]
    NotInstalled(ProviderKind),
    #[error("provider {0:?} execution failed: {1}")]
    Execution(ProviderKind, String),
    #[error("acp error: {0}")]
    Acp(#[from] lingxi_acp::AcpError),
}
