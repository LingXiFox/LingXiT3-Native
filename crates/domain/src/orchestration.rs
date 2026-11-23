use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{
    ApprovalId, CheckpointId, CommandId, EventId, MessageId, ProjectId, ThreadId, ToolCallId,
    TurnId,
};
use crate::provider::{ModelSelection, PermissionDecision, ProviderKind, RuntimeMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadRunStatus {
    Idle,
    Running,
    WaitingApproval,
    Paused,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentRef {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub path: String,
    pub byte_len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub id: ToolCallId,
    pub name: String,
    pub input_preview: String,
    pub output_preview: Option<String>,
    pub status: ToolCallStatus,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: ApprovalId,
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
    pub tool_name: String,
    pub summary: String,
    pub raw_input: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: MessageId,
    pub thread_id: ThreadId,
    pub turn_id: Option<TurnId>,
    pub role: MessageRole,
    pub content: String,
    pub reasoning: Option<String>,
    pub is_streaming: bool,
    pub tool_calls: Vec<ToolCallRecord>,
    pub attachments: Vec<AttachmentRef>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointRecord {
    pub id: CheckpointId,
    pub thread_id: ThreadId,
    pub turn_id: TurnId,
    pub turn_ordinal: u32,
    pub git_ref: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: ProjectId,
    pub title: String,
    pub workspace_root: String,
    pub default_model: Option<ModelSelection>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadRecord {
    pub id: ThreadId,
    pub project_id: ProjectId,
    pub title: String,
    pub model_selection: ModelSelection,
    pub runtime_mode: RuntimeMode,
    pub status: ThreadRunStatus,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
    pub provider_session_id: Option<String>,
    pub pinned: bool,
    pub archived: bool,
    pub unread: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrchestrationCommand {
    CreateProject {
        command_id: CommandId,
        project_id: ProjectId,
        title: String,
        workspace_root: String,
    },
    CreateThread {
        command_id: CommandId,
        thread_id: ThreadId,
        project_id: ProjectId,
        title: String,
        model_selection: ModelSelection,
        runtime_mode: RuntimeMode,
        branch: Option<String>,
        worktree_path: Option<String>,
    },
    StartTurn {
        command_id: CommandId,
        thread_id: ThreadId,
        turn_id: TurnId,
        message_id: MessageId,
        prompt: String,
        attachments: Vec<AttachmentRef>,
    },
    AppendAssistantDelta {
        command_id: CommandId,
        thread_id: ThreadId,
        turn_id: TurnId,
        message_id: MessageId,
        text_delta: String,
        reasoning_delta: Option<String>,
    },
    RecordToolCall {
        command_id: CommandId,
        thread_id: ThreadId,
        turn_id: TurnId,
        message_id: MessageId,
        tool_call: ToolCallRecord,
    },
    RequestPermission {
        command_id: CommandId,
        approval: PendingApproval,
    },
    ResolvePermission {
        command_id: CommandId,
        approval_id: ApprovalId,
        thread_id: ThreadId,
        decision: PermissionDecision,
        reason: Option<String>,
    },
    CompleteTurn {
        command_id: CommandId,
        thread_id: ThreadId,
        turn_id: TurnId,
        message_id: MessageId,
        provider_session_id: Option<String>,
        error: Option<String>,
    },
    CancelTurn {
        command_id: CommandId,
        thread_id: ThreadId,
        turn_id: Option<TurnId>,
    },
    PauseThread {
        command_id: CommandId,
        thread_id: ThreadId,
    },
    ResumeThread {
        command_id: CommandId,
        thread_id: ThreadId,
    },
    SetModelSelection {
        command_id: CommandId,
        thread_id: ThreadId,
        selection: ModelSelection,
    },
    SetRuntimeMode {
        command_id: CommandId,
        thread_id: ThreadId,
        runtime_mode: RuntimeMode,
    },
    CompactContext {
        command_id: CommandId,
        thread_id: ThreadId,
        summary_markdown: String,
    },
    HandoffTask {
        command_id: CommandId,
        source_thread_id: ThreadId,
        target_thread_id: ThreadId,
        target_provider: ProviderKind,
        handoff_summary: String,
    },
    CaptureCheckpoint {
        command_id: CommandId,
        checkpoint: CheckpointRecord,
    },
    RevertToCheckpoint {
        command_id: CommandId,
        thread_id: ThreadId,
        checkpoint_id: CheckpointId,
    },
    SetThreadArchived {
        command_id: CommandId,
        thread_id: ThreadId,
        archived: bool,
    },
    SetThreadPinned {
        command_id: CommandId,
        thread_id: ThreadId,
        pinned: bool,
    },
    RenameThread {
        command_id: CommandId,
        thread_id: ThreadId,
        title: String,
    },
}

impl OrchestrationCommand {
    pub fn command_id(&self) -> &CommandId {
        match self {
            Self::CreateProject { command_id, .. }
            | Self::CreateThread { command_id, .. }
            | Self::StartTurn { command_id, .. }
            | Self::AppendAssistantDelta { command_id, .. }
            | Self::RecordToolCall { command_id, .. }
            | Self::RequestPermission { command_id, .. }
            | Self::ResolvePermission { command_id, .. }
            | Self::CompleteTurn { command_id, .. }
            | Self::CancelTurn { command_id, .. }
            | Self::PauseThread { command_id, .. }
            | Self::ResumeThread { command_id, .. }
            | Self::SetModelSelection { command_id, .. }
            | Self::SetRuntimeMode { command_id, .. }
            | Self::CompactContext { command_id, .. }
            | Self::HandoffTask { command_id, .. }
            | Self::CaptureCheckpoint { command_id, .. }
            | Self::RevertToCheckpoint { command_id, .. }
            | Self::SetThreadArchived { command_id, .. }
            | Self::SetThreadPinned { command_id, .. }
            | Self::RenameThread { command_id, .. } => command_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredDomainEvent {
    pub sequence: u64,
    pub event_id: EventId,
    pub command_id: CommandId,
    pub thread_id: Option<ThreadId>,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}
