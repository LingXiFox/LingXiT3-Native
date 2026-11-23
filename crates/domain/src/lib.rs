use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod ids;
pub mod orchestration;
pub mod provider;
pub mod settings;
pub mod telemetry;

pub use ids::*;
pub use orchestration::*;
pub use provider::*;
pub use settings::*;
pub use telemetry::*;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("invalid state transition: {0}")]
    InvalidState(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitFileDiff {
    pub path: String,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
    pub patch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusSummary {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub staged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub untracked_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestSummary {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub url: String,
    pub head_ref: String,
    pub base_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateReleaseInfo {
    pub version: String,
    pub channel: UpdateChannel,
    pub notes: String,
    pub download_url: String,
    pub sha256: Option<String>,
    pub published_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_breakdown_ratios_are_bounded() {
        let cb = ContextBreakdown {
            system_tokens: 1_000,
            tools_tokens: 2_000,
            messages_tokens: 7_000,
            cache_read_tokens: 3_000,
            cache_write_tokens: 500,
            context_window_limit: 20_000,
        };
        assert_eq!(cb.total_used(), 10_000);
        assert!((cb.utilization_ratio() - 0.5).abs() < f64::EPSILON);
        assert!((cb.cache_hit_ratio() - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn settings_default_is_consistent() {
        let settings = AppSettings::default();
        assert_eq!(settings.remote_server_port, 3777);
        assert!(!settings.keybindings.is_empty());
    }
}
