use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Codex,
    Claude,
    Antigravity,
    Dsh,
    Cursor,
    Grok,
    OpenCode,
    Pi,
    Muse,
    Kiro,
    Fx,
    AcpRegistry,
}

impl ProviderKind {
    pub const ALL: [Self; 12] = [
        Self::Codex,
        Self::Claude,
        Self::Antigravity,
        Self::Dsh,
        Self::Cursor,
        Self::Grok,
        Self::OpenCode,
        Self::Pi,
        Self::Muse,
        Self::Kiro,
        Self::Fx,
        Self::AcpRegistry,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Antigravity => "antigravity",
            Self::Dsh => "dsh",
            Self::Cursor => "cursor",
            Self::Grok => "grok",
            Self::OpenCode => "opencode",
            Self::Pi => "pi",
            Self::Muse => "muse",
            Self::Kiro => "kiro",
            Self::Fx => "fx",
            Self::AcpRegistry => "acp-registry",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude Code",
            Self::Antigravity => "Antigravity",
            Self::Dsh => "DeepSeek Harness",
            Self::Cursor => "Cursor",
            Self::Grok => "Grok Build",
            Self::OpenCode => "OpenCode",
            Self::Pi => "Pi",
            Self::Muse => "Muse Code",
            Self::Kiro => "Kiro",
            Self::Fx => "fx",
            Self::AcpRegistry => "ACP Registry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    #[default]
    ApprovalRequired,
    AutoAcceptEdits,
    Auto,
    FullAccess,
}

impl RuntimeMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::ApprovalRequired => "Approval Required",
            Self::AutoAcceptEdits => "Auto-Accept Edits",
            Self::Auto => "Auto",
            Self::FullAccess => "Full Access",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelOptionValue {
    pub value: String,
    pub label: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigOptionDescriptor {
    pub id: String,
    pub name: String,
    pub category: Option<String>,
    pub current_value: Option<String>,
    pub options: Vec<ModelOptionValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    pub supports_thinking: bool,
    pub supports_images: bool,
    pub supports_resume: bool,
    pub supports_orchestrator: bool,
    pub supports_planner_executor_split: bool,
    pub option_descriptors: Vec<ConfigOptionDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderModel {
    pub slug: String,
    pub name: String,
    pub is_default: bool,
    pub is_custom: bool,
    pub capabilities: ModelCapabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderInstallState {
    Installed,
    Missing,
    IncompatibleVersion,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAuthState {
    Authenticated,
    Unauthenticated,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub kind: ProviderKind,
    pub install_state: ProviderInstallState,
    pub auth_state: ProviderAuthState,
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub models: Vec<ProviderModel>,
    pub config_options: Vec<ConfigOptionDescriptor>,
    pub status_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSelection {
    pub provider: ProviderKind,
    pub model_slug: String,
    pub thinking_level: Option<String>,
    pub planner_model: Option<String>,
    pub executor_model: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    AllowOnce,
    AllowSession,
    Deny,
}
