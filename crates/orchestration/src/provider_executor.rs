use std::sync::Arc;

use lingxi_domain::{
    CommandId, MessageId, OrchestrationCommand, PermissionDecision, ThreadId, ToolCallId,
    ToolCallRecord, ToolCallStatus, TurnId,
};
use lingxi_providers::{ProviderRegistry, ProviderTurnEvent, TurnRequest};
use tokio::sync::mpsc;

use crate::effect_worker::{BoxFuture, TurnExecutor};
use crate::orchestrator::Orchestrator;

pub struct ProviderTurnExecutor {
    pub registry: Arc<ProviderRegistry>,
    pub custom_acp_command: Option<String>,
}

impl TurnExecutor for ProviderTurnExecutor {
    fn execute_turn(
        &self,
        thread_id: ThreadId,
        turn_id: TurnId,
        assistant_message_id: MessageId,
        prompt: String,
        orchestrator: Arc<Orchestrator>,
    ) -> BoxFuture<'_, Result<Option<String>, String>> {
        Box::pin(async move {
            let db = orchestrator.db();
            let thread = db
                .list_threads()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|t| t.id == thread_id)
                .ok_or_else(|| format!("thread {thread_id} not found"))?;
            let project = db
                .list_projects()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.id == thread.project_id)
                .ok_or_else(|| format!("project {} not found", thread.project_id))?;

            let cwd = thread
                .worktree_path
                .clone()
                .unwrap_or(project.workspace_root);

            let driver = self
                .registry
                .get(thread.model_selection.provider)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "provider {} is not registered",
                        thread.model_selection.provider.display_name()
                    )
                })?;

            let (events_tx, mut events_rx) = mpsc::unbounded_channel();
            let (_perm_tx, perm_rx) = mpsc::unbounded_channel::<(String, PermissionDecision)>();

            let req = TurnRequest {
                cwd: cwd.into(),
                prompt,
                model_selection: thread.model_selection.clone(),
                runtime_mode: thread.runtime_mode,
                provider_session_id: thread.provider_session_id.clone(),
                custom_acp_command: self.custom_acp_command.clone(),
            };

            let orch_stream = Arc::clone(&orchestrator);
            let tid_stream = thread_id.clone();
            let turn_stream = turn_id.clone();
            let msg_stream = assistant_message_id.clone();
            let stream_task = tokio::spawn(async move {
                while let Some(ev) = events_rx.recv().await {
                    match ev {
                        ProviderTurnEvent::TextDelta(t) => {
                            let _ = orch_stream
                                .dispatch(OrchestrationCommand::AppendAssistantDelta {
                                    command_id: CommandId::new(),
                                    thread_id: tid_stream.clone(),
                                    turn_id: turn_stream.clone(),
                                    message_id: msg_stream.clone(),
                                    text_delta: t,
                                    reasoning_delta: None,
                                })
                                .await;
                        }
                        ProviderTurnEvent::ReasoningDelta(r) => {
                            let _ = orch_stream
                                .dispatch(OrchestrationCommand::AppendAssistantDelta {
                                    command_id: CommandId::new(),
                                    thread_id: tid_stream.clone(),
                                    turn_id: turn_stream.clone(),
                                    message_id: msg_stream.clone(),
                                    text_delta: String::new(),
                                    reasoning_delta: Some(r),
                                })
                                .await;
                        }
                        ProviderTurnEvent::ToolCall { id, name, input } => {
                            let _ = orch_stream
                                .dispatch(OrchestrationCommand::RecordToolCall {
                                    command_id: CommandId::new(),
                                    thread_id: tid_stream.clone(),
                                    turn_id: turn_stream.clone(),
                                    message_id: msg_stream.clone(),
                                    tool_call: ToolCallRecord {
                                        id: ToolCallId::from_string(id),
                                        name,
                                        input_preview: input.to_string(),
                                        output_preview: None,
                                        status: ToolCallStatus::Running,
                                        duration_ms: None,
                                    },
                                })
                                .await;
                        }
                        _ => {}
                    }
                }
            });

            let res = driver.run_turn(req, events_tx, perm_rx).await;
            let _ = stream_task.await;
            res.map_err(|e| e.to_string())
        })
    }
}
