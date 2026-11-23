use chrono::Utc;
use lingxi_domain::{
    CommandId, DomainError, EventId, MessageId, OrchestrationCommand, PermissionDecision,
    StoredDomainEvent, ThreadId, ThreadRunStatus, ToolCallStatus,
};
use lingxi_storage::Database;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

use crate::OrchestrationError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxEffect {
    pub effect_id: String,
    pub command_id: CommandId,
    pub thread_id: ThreadId,
    pub effect_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct DispatchOutcome {
    pub deduplicated: bool,
    pub events: Vec<StoredDomainEvent>,
    pub outbox_effects: Vec<OutboxEffect>,
}

pub struct Orchestrator {
    db: Database,
    serial_lock: Mutex<()>,
    event_tx: broadcast::Sender<StoredDomainEvent>,
}

impl Orchestrator {
    pub fn new(db: Database) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            db,
            serial_lock: Mutex::new(()),
            event_tx,
        }
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StoredDomainEvent> {
        self.event_tx.subscribe()
    }

    pub async fn dispatch(
        &self,
        command: OrchestrationCommand,
    ) -> Result<DispatchOutcome, OrchestrationError> {
        let _guard = self.serial_lock.lock().await;
        let db = self.db.clone();
        let outcome = tokio::task::spawn_blocking(move || Self::dispatch_sync(&db, command))
            .await
            .map_err(|e| {
                OrchestrationError::Domain(DomainError::InvalidState(format!(
                    "orchestrator join error: {e}"
                )))
            })??;

        if !outcome.deduplicated {
            for ev in &outcome.events {
                let _ = self.event_tx.send(ev.clone());
            }
        }
        Ok(outcome)
    }

    fn dispatch_sync(
        db: &Database,
        command: OrchestrationCommand,
    ) -> Result<DispatchOutcome, OrchestrationError> {
        let cmd_id = command.command_id().clone();
        let mut conn = db.lock();

        // Idempotency check via command receipt
        let existing_seq: Option<i64> = conn
            .query_row(
                "SELECT event_sequence FROM orchestration_command_receipts WHERE command_id = ?1",
                [cmd_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(lingxi_storage::StorageError::from)?;

        if let Some(_seq) = existing_seq {
            let mut stmt = conn
                .prepare(
                    "SELECT sequence, event_id, command_id, thread_id, event_type, payload_json, occurred_at
                     FROM orchestration_events WHERE command_id = ?1 ORDER BY sequence ASC",
                )
                .map_err(lingxi_storage::StorageError::from)?;
            let events = stmt
                .query_map([cmd_id.as_str()], |row| {
                    let payload_str: String = row.get(5)?;
                    let occurred_str: String = row.get(6)?;
                    Ok(StoredDomainEvent {
                        sequence: row.get::<_, i64>(0)? as u64,
                        event_id: EventId::from_string(row.get::<_, String>(1)?),
                        command_id: CommandId::from_string(row.get::<_, String>(2)?),
                        thread_id: row.get::<_, Option<String>>(3)?.map(ThreadId::from_string),
                        event_type: row.get(4)?,
                        payload: serde_json::from_str(&payload_str)
                            .unwrap_or(serde_json::Value::Null),
                        occurred_at: chrono::DateTime::parse_from_rfc3339(&occurred_str)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    })
                })
                .map_err(lingxi_storage::StorageError::from)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(lingxi_storage::StorageError::from)?;

            return Ok(DispatchOutcome {
                deduplicated: true,
                events,
                outbox_effects: Vec::new(),
            });
        }

        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let tx = conn
            .transaction()
            .map_err(lingxi_storage::StorageError::from)?;

        let mut pending_events: Vec<(Option<ThreadId>, String, serde_json::Value)> = Vec::new();
        let mut outbox_effects: Vec<OutboxEffect> = Vec::new();

        match &command {
            OrchestrationCommand::CreateProject {
                project_id,
                title,
                workspace_root,
                ..
            } => {
                tx.execute(
                    "INSERT INTO projects (project_id, title, workspace_root, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![project_id.as_str(), title, workspace_root, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    None,
                    "project.created".to_string(),
                    serde_json::json!({
                        "project_id": project_id,
                        "title": title,
                        "workspace_root": workspace_root,
                    }),
                ));
            }

            OrchestrationCommand::CreateThread {
                thread_id,
                project_id,
                title,
                model_selection,
                runtime_mode,
                branch,
                worktree_path,
                ..
            } => {
                let model_json = serde_json::to_string(model_selection)
                    .map_err(lingxi_storage::StorageError::from)?;
                let mode_str = serde_json::to_string(runtime_mode)
                    .map_err(lingxi_storage::StorageError::from)?
                    .trim_matches('"')
                    .to_string();
                tx.execute(
                    "INSERT INTO threads (
                        thread_id, project_id, title, model_selection_json, runtime_mode,
                        status, branch, worktree_path, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, 'idle', ?6, ?7, ?8, ?8)",
                    params![
                        thread_id.as_str(),
                        project_id.as_str(),
                        title,
                        model_json,
                        mode_str,
                        branch,
                        worktree_path,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.created".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "project_id": project_id,
                        "title": title,
                    }),
                ));
            }

            OrchestrationCommand::StartTurn {
                thread_id,
                turn_id,
                message_id,
                prompt,
                attachments,
                ..
            } => {
                let current_status = fetch_thread_status(&tx, thread_id)?;
                if matches!(
                    current_status,
                    ThreadRunStatus::Running | ThreadRunStatus::WaitingApproval
                ) {
                    return Err(OrchestrationError::Domain(DomainError::InvalidState(
                        format!(
                            "thread {} is already active ({current_status:?})",
                            thread_id
                        ),
                    )));
                }
                let next_ordinal: i64 = tx
                    .query_row(
                        "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM turns WHERE thread_id = ?1",
                        [thread_id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(lingxi_storage::StorageError::from)?;
                let att_json = serde_json::to_string(attachments)
                    .map_err(lingxi_storage::StorageError::from)?;

                tx.execute(
                    "INSERT INTO turns (turn_id, thread_id, ordinal, status, prompt, started_at)
                     VALUES (?1, ?2, ?3, 'running', ?4, ?5)",
                    params![
                        turn_id.as_str(),
                        thread_id.as_str(),
                        next_ordinal,
                        prompt,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;

                tx.execute(
                    "INSERT INTO messages (
                        message_id, thread_id, turn_id, role, content, is_streaming,
                        attachments_json, created_at, updated_at
                    ) VALUES (?1, ?2, ?3, 'user', ?4, 0, ?5, ?6, ?6)",
                    params![
                        message_id.as_str(),
                        thread_id.as_str(),
                        turn_id.as_str(),
                        prompt,
                        att_json,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;

                let assistant_msg_id = MessageId::new();
                tx.execute(
                    "INSERT INTO messages (
                        message_id, thread_id, turn_id, role, content, is_streaming,
                        created_at, updated_at
                    ) VALUES (?1, ?2, ?3, 'assistant', '', 1, ?4, ?4)",
                    params![
                        assistant_msg_id.as_str(),
                        thread_id.as_str(),
                        turn_id.as_str(),
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;

                tx.execute(
                    "UPDATE threads SET status = 'running', updated_at = ?2 WHERE thread_id = ?1",
                    params![thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;

                pending_events.push((
                    Some(thread_id.clone()),
                    "turn.started".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "turn_id": turn_id,
                        "ordinal": next_ordinal,
                        "assistant_message_id": assistant_msg_id,
                    }),
                ));

                outbox_effects.push(OutboxEffect {
                    effect_id: Uuid::new_v4().to_string(),
                    command_id: cmd_id.clone(),
                    thread_id: thread_id.clone(),
                    effect_type: "run_provider_turn".to_string(),
                    payload: serde_json::json!({
                        "thread_id": thread_id,
                        "turn_id": turn_id,
                        "turn_ordinal": next_ordinal,
                        "assistant_message_id": assistant_msg_id,
                        "prompt": prompt,
                        "attachments": attachments,
                    }),
                });
            }

            OrchestrationCommand::AppendAssistantDelta {
                thread_id,
                turn_id,
                message_id,
                text_delta,
                reasoning_delta,
                ..
            } => {
                tx.execute(
                    "UPDATE messages
                     SET content = content || ?2,
                         reasoning = CASE
                             WHEN ?3 IS NULL THEN reasoning
                             ELSE COALESCE(reasoning, '') || ?3
                         END,
                         updated_at = ?4
                     WHERE message_id = ?1",
                    params![message_id.as_str(), text_delta, reasoning_delta, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "turn.delta".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "turn_id": turn_id,
                        "message_id": message_id,
                        "text_delta": text_delta,
                        "reasoning_delta": reasoning_delta,
                    }),
                ));
            }

            OrchestrationCommand::RecordToolCall {
                thread_id,
                turn_id,
                message_id,
                tool_call,
                ..
            } => {
                let existing_json: String = tx
                    .query_row(
                        "SELECT tool_calls_json FROM messages WHERE message_id = ?1",
                        [message_id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(lingxi_storage::StorageError::from)?;
                let mut calls: Vec<lingxi_domain::ToolCallRecord> =
                    serde_json::from_str(&existing_json).unwrap_or_default();
                if let Some(pos) = calls.iter().position(|c| c.id == tool_call.id) {
                    calls[pos] = tool_call.clone();
                } else {
                    calls.push(tool_call.clone());
                }
                let updated_json =
                    serde_json::to_string(&calls).map_err(lingxi_storage::StorageError::from)?;
                tx.execute(
                    "UPDATE messages SET tool_calls_json = ?2, updated_at = ?3 WHERE message_id = ?1",
                    params![message_id.as_str(), updated_json, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "turn.tool_call".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "turn_id": turn_id,
                        "message_id": message_id,
                        "tool_call": tool_call,
                    }),
                ));
            }

            OrchestrationCommand::RequestPermission { approval, .. } => {
                let raw_json = serde_json::to_string(&approval.raw_input)
                    .map_err(lingxi_storage::StorageError::from)?;
                tx.execute(
                    "INSERT INTO pending_approvals (
                        approval_id, thread_id, turn_id, tool_name, summary,
                        raw_input_json, status, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)",
                    params![
                        approval.id.as_str(),
                        approval.thread_id.as_str(),
                        approval.turn_id.as_str(),
                        approval.tool_name,
                        approval.summary,
                        raw_json,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                tx.execute(
                    "UPDATE threads SET status = 'waiting_approval', updated_at = ?2 WHERE thread_id = ?1",
                    params![approval.thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(approval.thread_id.clone()),
                    "approval.requested".to_string(),
                    serde_json::json!({ "approval": approval }),
                ));
            }

            OrchestrationCommand::ResolvePermission {
                approval_id,
                thread_id,
                decision,
                reason,
                ..
            } => {
                let decision_str = match decision {
                    PermissionDecision::AllowOnce => "allow_once",
                    PermissionDecision::AllowSession => "allow_session",
                    PermissionDecision::Deny => "deny",
                };
                let affected = tx
                    .execute(
                        "UPDATE pending_approvals
                         SET status = 'resolved', decision = ?2, resolved_at = ?3
                         WHERE approval_id = ?1 AND status = 'pending'",
                        params![approval_id.as_str(), decision_str, now_str],
                    )
                    .map_err(lingxi_storage::StorageError::from)?;
                if affected == 0 {
                    return Err(OrchestrationError::Domain(DomainError::NotFound(format!(
                        "pending approval {approval_id} not found"
                    ))));
                }
                tx.execute(
                    "UPDATE threads SET status = 'running', updated_at = ?2 WHERE thread_id = ?1",
                    params![thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "approval.resolved".to_string(),
                    serde_json::json!({
                        "approval_id": approval_id,
                        "thread_id": thread_id,
                        "decision": decision,
                        "reason": reason,
                    }),
                ));
            }

            OrchestrationCommand::CompleteTurn {
                thread_id,
                turn_id,
                message_id,
                provider_session_id,
                error,
                ..
            } => {
                let final_thread_status = if error.is_some() { "error" } else { "idle" };
                let final_turn_status = if error.is_some() {
                    "failed"
                } else {
                    "completed"
                };
                tx.execute(
                    "UPDATE messages SET is_streaming = 0, updated_at = ?2 WHERE message_id = ?1",
                    params![message_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                tx.execute(
                    "UPDATE turns SET status = ?2, error_message = ?3, completed_at = ?4 WHERE turn_id = ?1",
                    params![turn_id.as_str(), final_turn_status, error, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                tx.execute(
                    "UPDATE threads
                     SET status = ?2,
                         provider_session_id = COALESCE(?3, provider_session_id),
                         unread = 1,
                         updated_at = ?4
                     WHERE thread_id = ?1",
                    params![
                        thread_id.as_str(),
                        final_thread_status,
                        provider_session_id,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "turn.completed".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "turn_id": turn_id,
                        "error": error,
                    }),
                ));
                if error.is_none() {
                    outbox_effects.push(OutboxEffect {
                        effect_id: Uuid::new_v4().to_string(),
                        command_id: cmd_id.clone(),
                        thread_id: thread_id.clone(),
                        effect_type: "capture_checkpoint".to_string(),
                        payload: serde_json::json!({
                            "thread_id": thread_id,
                            "turn_id": turn_id,
                        }),
                    });
                }
            }

            OrchestrationCommand::CancelTurn {
                thread_id, turn_id, ..
            } => {
                tx.execute(
                    "UPDATE threads SET status = 'idle', updated_at = ?2 WHERE thread_id = ?1",
                    params![thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                if let Some(tid) = turn_id {
                    tx.execute(
                        "UPDATE turns SET status = 'cancelled', completed_at = ?2 WHERE turn_id = ?1",
                        params![tid.as_str(), now_str],
                    )
                    .map_err(lingxi_storage::StorageError::from)?;
                }
                tx.execute(
                    "UPDATE messages SET is_streaming = 0, updated_at = ?2 WHERE thread_id = ?1 AND is_streaming = 1",
                    params![thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "turn.cancelled".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "turn_id": turn_id,
                    }),
                ));
            }

            OrchestrationCommand::PauseThread { thread_id, .. } => {
                let status = fetch_thread_status(&tx, thread_id)?;
                if status == ThreadRunStatus::Paused {
                    return Err(OrchestrationError::Domain(DomainError::InvalidState(
                        "thread is already paused".into(),
                    )));
                }
                tx.execute(
                    "UPDATE threads SET status = 'paused', updated_at = ?2 WHERE thread_id = ?1",
                    params![thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.paused".to_string(),
                    serde_json::json!({ "thread_id": thread_id }),
                ));
            }

            OrchestrationCommand::ResumeThread { thread_id, .. } => {
                let status = fetch_thread_status(&tx, thread_id)?;
                if status != ThreadRunStatus::Paused && status != ThreadRunStatus::Error {
                    return Err(OrchestrationError::Domain(DomainError::InvalidState(
                        format!("cannot resume thread from status {status:?}"),
                    )));
                }
                tx.execute(
                    "UPDATE threads SET status = 'idle', updated_at = ?2 WHERE thread_id = ?1",
                    params![thread_id.as_str(), now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.resumed".to_string(),
                    serde_json::json!({ "thread_id": thread_id }),
                ));
            }

            OrchestrationCommand::SetModelSelection {
                thread_id,
                selection,
                ..
            } => {
                let json =
                    serde_json::to_string(selection).map_err(lingxi_storage::StorageError::from)?;
                tx.execute(
                    "UPDATE threads SET model_selection_json = ?2, updated_at = ?3 WHERE thread_id = ?1",
                    params![thread_id.as_str(), json, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.model_updated".to_string(),
                    serde_json::json!({ "thread_id": thread_id, "selection": selection }),
                ));
            }

            OrchestrationCommand::SetRuntimeMode {
                thread_id,
                runtime_mode,
                ..
            } => {
                let mode_str = serde_json::to_string(runtime_mode)
                    .map_err(lingxi_storage::StorageError::from)?
                    .trim_matches('"')
                    .to_string();
                tx.execute(
                    "UPDATE threads SET runtime_mode = ?2, updated_at = ?3 WHERE thread_id = ?1",
                    params![thread_id.as_str(), mode_str, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.runtime_mode_updated".to_string(),
                    serde_json::json!({ "thread_id": thread_id, "runtime_mode": runtime_mode }),
                ));
            }

            OrchestrationCommand::CompactContext {
                thread_id,
                summary_markdown,
                ..
            } => {
                let summary_id = MessageId::new();
                tx.execute(
                    "INSERT INTO messages (
                        message_id, thread_id, role, content, is_streaming, created_at, updated_at
                    ) VALUES (?1, ?2, 'system', ?3, 0, ?4, ?4)",
                    params![
                        summary_id.as_str(),
                        thread_id.as_str(),
                        summary_markdown,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.context_compacted".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "summary_message_id": summary_id,
                    }),
                ));
            }

            OrchestrationCommand::HandoffTask {
                source_thread_id,
                target_thread_id,
                target_provider,
                handoff_summary,
                ..
            } => {
                let msg_id = MessageId::new();
                tx.execute(
                    "INSERT INTO messages (
                        message_id, thread_id, role, content, is_streaming, created_at, updated_at
                    ) VALUES (?1, ?2, 'system', ?3, 0, ?4, ?4)",
                    params![
                        msg_id.as_str(),
                        target_thread_id.as_str(),
                        format!(
                            "[Handoff from {} -> {}]\n{}",
                            source_thread_id,
                            target_provider.display_name(),
                            handoff_summary
                        ),
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(target_thread_id.clone()),
                    "thread.handoff_created".to_string(),
                    serde_json::json!({
                        "source_thread_id": source_thread_id,
                        "target_thread_id": target_thread_id,
                        "target_provider": target_provider,
                    }),
                ));
            }

            OrchestrationCommand::CaptureCheckpoint { checkpoint, .. } => {
                tx.execute(
                    "INSERT INTO checkpoints (
                        checkpoint_id, thread_id, turn_id, turn_ordinal, git_ref, summary, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        checkpoint.id.as_str(),
                        checkpoint.thread_id.as_str(),
                        checkpoint.turn_id.as_str(),
                        checkpoint.turn_ordinal,
                        checkpoint.git_ref,
                        checkpoint.summary,
                        now_str
                    ],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(checkpoint.thread_id.clone()),
                    "checkpoint.captured".to_string(),
                    serde_json::json!({ "checkpoint": checkpoint }),
                ));
            }

            OrchestrationCommand::RevertToCheckpoint {
                thread_id,
                checkpoint_id,
                ..
            } => {
                let exists: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM checkpoints WHERE checkpoint_id = ?1 AND thread_id = ?2)",
                        params![checkpoint_id.as_str(), thread_id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(lingxi_storage::StorageError::from)?;
                if !exists {
                    return Err(OrchestrationError::Domain(DomainError::NotFound(format!(
                        "checkpoint {checkpoint_id} not found for thread {thread_id}"
                    ))));
                }
                pending_events.push((
                    Some(thread_id.clone()),
                    "checkpoint.revert_requested".to_string(),
                    serde_json::json!({
                        "thread_id": thread_id,
                        "checkpoint_id": checkpoint_id,
                    }),
                ));
                outbox_effects.push(OutboxEffect {
                    effect_id: Uuid::new_v4().to_string(),
                    command_id: cmd_id.clone(),
                    thread_id: thread_id.clone(),
                    effect_type: "revert_checkpoint".to_string(),
                    payload: serde_json::json!({
                        "thread_id": thread_id,
                        "checkpoint_id": checkpoint_id,
                    }),
                });
            }

            OrchestrationCommand::SetThreadArchived {
                thread_id,
                archived,
                ..
            } => {
                tx.execute(
                    "UPDATE threads SET archived = ?2, updated_at = ?3 WHERE thread_id = ?1",
                    params![thread_id.as_str(), *archived as i64, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.archived_changed".to_string(),
                    serde_json::json!({ "thread_id": thread_id, "archived": archived }),
                ));
            }

            OrchestrationCommand::SetThreadPinned {
                thread_id, pinned, ..
            } => {
                tx.execute(
                    "UPDATE threads SET pinned = ?2, updated_at = ?3 WHERE thread_id = ?1",
                    params![thread_id.as_str(), *pinned as i64, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.pinned_changed".to_string(),
                    serde_json::json!({ "thread_id": thread_id, "pinned": pinned }),
                ));
            }

            OrchestrationCommand::RenameThread {
                thread_id, title, ..
            } => {
                tx.execute(
                    "UPDATE threads SET title = ?2, updated_at = ?3 WHERE thread_id = ?1",
                    params![thread_id.as_str(), title, now_str],
                )
                .map_err(lingxi_storage::StorageError::from)?;
                pending_events.push((
                    Some(thread_id.clone()),
                    "thread.renamed".to_string(),
                    serde_json::json!({ "thread_id": thread_id, "title": title }),
                ));
            }
        }

        let mut stored_events = Vec::with_capacity(pending_events.len());
        let mut first_seq = 0i64;
        for (idx, (tid, event_type, payload)) in pending_events.into_iter().enumerate() {
            let event_id = EventId::new();
            let payload_json =
                serde_json::to_string(&payload).map_err(lingxi_storage::StorageError::from)?;
            tx.execute(
                "INSERT INTO orchestration_events (
                    event_id, command_id, thread_id, event_type, payload_json, occurred_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event_id.as_str(),
                    cmd_id.as_str(),
                    tid.as_ref().map(|t| t.as_str()),
                    event_type,
                    payload_json,
                    now_str
                ],
            )
            .map_err(lingxi_storage::StorageError::from)?;
            let seq = tx.last_insert_rowid();
            if idx == 0 {
                first_seq = seq;
            }
            stored_events.push(StoredDomainEvent {
                sequence: seq as u64,
                event_id,
                command_id: cmd_id.clone(),
                thread_id: tid,
                event_type,
                payload,
                occurred_at: now,
            });
        }

        tx.execute(
            "INSERT INTO orchestration_command_receipts (command_id, event_sequence, accepted_at)
             VALUES (?1, ?2, ?3)",
            params![cmd_id.as_str(), first_seq, now_str],
        )
        .map_err(lingxi_storage::StorageError::from)?;

        for effect in &outbox_effects {
            let payload_json = serde_json::to_string(&effect.payload)
                .map_err(lingxi_storage::StorageError::from)?;
            tx.execute(
                "INSERT INTO orchestration_effect_outbox (
                    effect_id, command_id, thread_id, effect_type, payload_json,
                    status, attempt_count, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, ?6, ?6)",
                params![
                    effect.effect_id,
                    effect.command_id.as_str(),
                    effect.thread_id.as_str(),
                    effect.effect_type,
                    payload_json,
                    now_str
                ],
            )
            .map_err(lingxi_storage::StorageError::from)?;
        }

        tx.commit().map_err(lingxi_storage::StorageError::from)?;
        let _ = ToolCallStatus::Pending;

        Ok(DispatchOutcome {
            deduplicated: false,
            events: stored_events,
            outbox_effects,
        })
    }
}

fn fetch_thread_status(
    tx: &rusqlite::Transaction<'_>,
    thread_id: &ThreadId,
) -> Result<ThreadRunStatus, OrchestrationError> {
    let status_str: Option<String> = tx
        .query_row(
            "SELECT status FROM threads WHERE thread_id = ?1",
            [thread_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(lingxi_storage::StorageError::from)?;

    let Some(s) = status_str else {
        return Err(OrchestrationError::Domain(DomainError::NotFound(format!(
            "thread {thread_id} not found"
        ))));
    };
    Ok(serde_json::from_str(&format!("\"{s}\"")).unwrap_or(ThreadRunStatus::Idle))
}
