use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use lingxi_domain::{
    ModelSelection, PermissionDecision, ProviderAuthState, ProviderInstallState, ProviderKind,
    ProviderStatus, RuntimeMode,
};
use serde_json::Value;
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::ProviderError;

#[derive(Debug, Clone)]
pub struct TurnRequest {
    pub cwd: PathBuf,
    pub prompt: String,
    pub model_selection: ModelSelection,
    pub runtime_mode: RuntimeMode,
    pub provider_session_id: Option<String>,
    pub custom_acp_command: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ProviderTurnEvent {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
    ToolCallUpdate {
        id: String,
        status: String,
        output: Option<String>,
    },
    PermissionRequested {
        id: String,
        tool_name: String,
        summary: String,
        raw_input: Value,
    },
    Usage {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
    },
    Completed {
        provider_session_id: Option<String>,
    },
    Failed(String),
}

#[async_trait]
pub trait ProviderDriver: Send + Sync {
    fn kind(&self) -> ProviderKind;

    async fn detect_status(&self, custom_cmd: Option<&str>) -> ProviderStatus;

    async fn run_turn(
        &self,
        request: TurnRequest,
        events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
        permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
    ) -> Result<Option<String>, ProviderError>;
}

#[derive(Clone, Default)]
pub struct ProviderRegistry {
    drivers: HashMap<ProviderKind, Arc<dyn ProviderDriver>>,
}

impl ProviderRegistry {
    pub fn with_all_builtin() -> Self {
        let mut reg = Self::default();
        reg.register(Arc::new(crate::codex::CodexDriver));
        reg.register(Arc::new(crate::claude::ClaudeDriver));
        reg.register(Arc::new(crate::antigravity::AntigravityDriver));
        reg.register(Arc::new(crate::dsh::DshDriver));
        reg.register(Arc::new(crate::cli_adapters::CursorDriver));
        reg.register(Arc::new(crate::cli_adapters::GrokDriver));
        reg.register(Arc::new(crate::cli_adapters::OpenCodeDriver));
        reg.register(Arc::new(crate::cli_adapters::PiDriver));
        reg.register(Arc::new(crate::cli_adapters::MuseDriver));
        reg.register(Arc::new(crate::cli_adapters::KiroDriver));
        reg.register(Arc::new(crate::cli_adapters::FxDriver));
        reg.register(Arc::new(crate::cli_adapters::AcpRegistryDriver));
        reg
    }

    pub fn register(&mut self, driver: Arc<dyn ProviderDriver>) {
        self.drivers.insert(driver.kind(), driver);
    }

    pub fn get(&self, kind: ProviderKind) -> Option<&Arc<dyn ProviderDriver>> {
        self.drivers.get(&kind)
    }

    pub async fn detect_all(&self, custom_acp_cmd: Option<&str>) -> Vec<ProviderStatus> {
        let mut out = Vec::with_capacity(ProviderKind::ALL.len());
        for kind in ProviderKind::ALL {
            if let Some(driver) = self.drivers.get(&kind) {
                out.push(driver.detect_status(custom_acp_cmd).await);
            }
        }
        out
    }
}

pub async fn probe_binary_version(
    binary_name: &str,
    version_args: &[&str],
) -> (Option<String>, Option<String>) {
    let Ok(bin_path) = which::which(binary_name) else {
        return (None, None);
    };
    let bin_str = bin_path.to_string_lossy().to_string();
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        Command::new(&bin_path).args(version_args).output(),
    )
    .await;

    let version = match output {
        Ok(Ok(out)) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let text = if stdout.is_empty() { stderr } else { stdout };
            if text.is_empty() { None } else { Some(text) }
        }
        _ => None,
    };
    (Some(bin_str), version)
}

pub fn missing_provider_status(kind: ProviderKind, binary_hint: &str) -> ProviderStatus {
    ProviderStatus {
        kind,
        install_state: ProviderInstallState::Missing,
        auth_state: ProviderAuthState::Unknown,
        binary_path: None,
        version: None,
        models: Vec::new(),
        config_options: Vec::new(),
        status_message: Some(format!("Binary '{binary_hint}' not found in PATH")),
    }
}
