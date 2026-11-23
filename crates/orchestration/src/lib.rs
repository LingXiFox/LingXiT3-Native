pub mod effect_worker;
pub mod orchestrator;
pub mod provider_executor;
pub mod recovery;

pub use effect_worker::{
    BoxFuture, CheckpointExecutor, EffectWorker, NoopCheckpointExecutor, TurnExecutor,
};
pub use orchestrator::{DispatchOutcome, Orchestrator, OutboxEffect};
pub use provider_executor::ProviderTurnExecutor;
pub use recovery::{RecoveryReport, recover_after_unclean_exit};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestrationError {
    #[error("domain error: {0}")]
    Domain(#[from] lingxi_domain::DomainError),
    #[error("storage error: {0}")]
    Storage(#[from] lingxi_storage::StorageError),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use lingxi_domain::{
        ApprovalId, CheckpointId, CheckpointRecord, CommandId, MessageId, ModelSelection,
        OrchestrationCommand, PendingApproval, PermissionDecision, ProjectId, ProviderKind,
        RuntimeMode, ThreadId, ThreadRunStatus, TurnId,
    };
    use lingxi_storage::Database;

    use super::*;

    struct MockTurnExecutor {
        worker: tokio::sync::OnceCell<Arc<EffectWorker>>,
        require_approval: bool,
    }

    impl TurnExecutor for MockTurnExecutor {
        fn execute_turn(
            &self,
            thread_id: ThreadId,
            turn_id: TurnId,
            assistant_message_id: MessageId,
            prompt: String,
            orchestrator: Arc<Orchestrator>,
        ) -> BoxFuture<'_, Result<Option<String>, String>> {
            Box::pin(async move {
                orchestrator
                    .dispatch(OrchestrationCommand::AppendAssistantDelta {
                        command_id: CommandId::new(),
                        thread_id: thread_id.clone(),
                        turn_id: turn_id.clone(),
                        message_id: assistant_message_id.clone(),
                        text_delta: format!("Echo: {prompt}"),
                        reasoning_delta: Some("thinking".into()),
                    })
                    .await
                    .map_err(|e| e.to_string())?;

                if self.require_approval {
                    let approval_id = ApprovalId::new();
                    let rx = self
                        .worker
                        .get()
                        .expect("worker set")
                        .register_approval_waiter(approval_id.clone())
                        .await;
                    orchestrator
                        .dispatch(OrchestrationCommand::RequestPermission {
                            command_id: CommandId::new(),
                            approval: PendingApproval {
                                id: approval_id,
                                thread_id: thread_id.clone(),
                                turn_id: turn_id.clone(),
                                tool_name: "run_command".into(),
                                summary: "cargo test".into(),
                                raw_input: serde_json::json!({"cmd": "cargo test"}),
                                created_at: Utc::now(),
                            },
                        })
                        .await
                        .map_err(|e| e.to_string())?;

                    let decision = rx.await.map_err(|e| e.to_string())?;
                    if decision == PermissionDecision::Deny {
                        return Err("permission denied by user".into());
                    }
                }

                Ok(Some("sess-mock-1".into()))
            })
        }
    }

    async fn seed_project_and_thread(orch: &Orchestrator) -> (ProjectId, ThreadId) {
        let pid = ProjectId::new();
        let tid = ThreadId::new();
        orch.dispatch(OrchestrationCommand::CreateProject {
            command_id: CommandId::new(),
            project_id: pid.clone(),
            title: "Test Repo".into(),
            workspace_root: "/tmp/test-repo".into(),
        })
        .await
        .expect("create project");

        orch.dispatch(OrchestrationCommand::CreateThread {
            command_id: CommandId::new(),
            thread_id: tid.clone(),
            project_id: pid.clone(),
            title: "Thread 1".into(),
            model_selection: ModelSelection {
                provider: ProviderKind::Codex,
                model_slug: "gpt-5".into(),
                thinking_level: None,
                planner_model: None,
                executor_model: None,
            },
            runtime_mode: RuntimeMode::ApprovalRequired,
            branch: Some("main".into()),
            worktree_path: None,
        })
        .await
        .expect("create thread");
        (pid, tid)
    }

    #[tokio::test]
    async fn command_idempotency_and_monotonic_event_sequences() {
        let db = Database::open_in_memory().expect("db");
        let orch = Orchestrator::new(db.clone());
        let cmd_id = CommandId::new();
        let pid = ProjectId::new();

        let first = orch
            .dispatch(OrchestrationCommand::CreateProject {
                command_id: cmd_id.clone(),
                project_id: pid.clone(),
                title: "Repo".into(),
                workspace_root: "/tmp/repo".into(),
            })
            .await
            .expect("first");
        assert!(!first.deduplicated);
        assert_eq!(first.events.len(), 1);
        assert_eq!(first.events[0].sequence, 1);

        let retry = orch
            .dispatch(OrchestrationCommand::CreateProject {
                command_id: cmd_id,
                project_id: pid,
                title: "Repo".into(),
                workspace_root: "/tmp/repo".into(),
            })
            .await
            .expect("retry");
        assert!(retry.deduplicated);
        assert_eq!(retry.events[0].sequence, 1);
        assert_eq!(db.list_projects().expect("projects").len(), 1);
    }

    #[tokio::test]
    async fn outbox_worker_executes_turn_streaming_and_approval_flow() {
        let db = Database::open_in_memory().expect("db");
        let orch = Arc::new(Orchestrator::new(db.clone()));
        let (_pid, tid) = seed_project_and_thread(&orch).await;

        let exec = Arc::new(MockTurnExecutor {
            worker: tokio::sync::OnceCell::new(),
            require_approval: true,
        });
        let worker = Arc::new(EffectWorker::new(
            Arc::clone(&orch),
            exec.clone(),
            Arc::new(NoopCheckpointExecutor),
        ));
        exec.worker.set(Arc::clone(&worker)).ok().unwrap();

        orch.dispatch(OrchestrationCommand::StartTurn {
            command_id: CommandId::new(),
            thread_id: tid.clone(),
            turn_id: TurnId::new(),
            message_id: MessageId::new(),
            prompt: "hello".into(),
            attachments: vec![],
        })
        .await
        .expect("start turn");

        let worker_bg = Arc::clone(&worker);
        let bg = tokio::spawn(async move { worker_bg.process_pending_once(4).await });

        // Wait briefly for approval to land in DB
        let mut approval_id = None;
        for _ in 0..50 {
            let pending = db.list_pending_approvals(&tid).expect("approvals");
            if let Some(a) = pending.first() {
                approval_id = Some(a.id.clone());
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let aid = approval_id.expect("approval created");
        orch.dispatch(OrchestrationCommand::ResolvePermission {
            command_id: CommandId::new(),
            approval_id: aid.clone(),
            thread_id: tid.clone(),
            decision: PermissionDecision::AllowOnce,
            reason: None,
        })
        .await
        .expect("resolve");
        assert!(
            worker
                .resolve_approval_waiter(&aid, PermissionDecision::AllowOnce)
                .await
        );

        let count = bg.await.expect("join").expect("worker");
        assert_eq!(count, 1);

        let msgs = db.list_messages(&tid).expect("msgs");
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[1].content, "Echo: hello");
        assert!(!msgs[1].is_streaming);

        let threads = db.list_threads().expect("threads");
        assert_eq!(threads[0].status, ThreadRunStatus::Idle);
        assert_eq!(
            threads[0].provider_session_id.as_deref(),
            Some("sess-mock-1")
        );
    }

    #[tokio::test]
    async fn crash_recovery_marks_running_threads_and_turns_failed() {
        let db = Database::open_in_memory().expect("db");
        let orch = Orchestrator::new(db.clone());
        let (_pid, tid) = seed_project_and_thread(&orch).await;

        orch.dispatch(OrchestrationCommand::StartTurn {
            command_id: CommandId::new(),
            thread_id: tid.clone(),
            turn_id: TurnId::new(),
            message_id: MessageId::new(),
            prompt: "crash test".into(),
            attachments: vec![],
        })
        .await
        .expect("start");

        let report = recover_after_unclean_exit(&db).expect("recover");
        assert_eq!(report.interrupted_threads_recovered, 1);
        assert_eq!(report.interrupted_turns_recovered, 1);

        let threads = db.list_threads().expect("threads");
        assert_eq!(threads[0].status, ThreadRunStatus::Error);

        // Resume from error back to idle
        orch.dispatch(OrchestrationCommand::ResumeThread {
            command_id: CommandId::new(),
            thread_id: tid,
        })
        .await
        .expect("resume");
        assert_eq!(
            db.list_threads().expect("t")[0].status,
            ThreadRunStatus::Idle
        );
    }

    #[tokio::test]
    async fn compaction_handoff_and_checkpoint_capture_work() {
        let db = Database::open_in_memory().expect("db");
        let orch = Orchestrator::new(db.clone());
        let (pid, tid1) = seed_project_and_thread(&orch).await;
        let tid2 = ThreadId::new();
        orch.dispatch(OrchestrationCommand::CreateThread {
            command_id: CommandId::new(),
            thread_id: tid2.clone(),
            project_id: pid,
            title: "Thread 2".into(),
            model_selection: ModelSelection {
                provider: ProviderKind::Claude,
                model_slug: "claude-sonnet".into(),
                thinking_level: None,
                planner_model: None,
                executor_model: None,
            },
            runtime_mode: RuntimeMode::Auto,
            branch: None,
            worktree_path: None,
        })
        .await
        .expect("t2");

        orch.dispatch(OrchestrationCommand::CompactContext {
            command_id: CommandId::new(),
            thread_id: tid1.clone(),
            summary_markdown: "## Compaction Summary".into(),
        })
        .await
        .expect("compact");

        orch.dispatch(OrchestrationCommand::HandoffTask {
            command_id: CommandId::new(),
            source_thread_id: tid1.clone(),
            target_thread_id: tid2.clone(),
            target_provider: ProviderKind::Claude,
            handoff_summary: "Continue refactor".into(),
        })
        .await
        .expect("handoff");

        let cp = CheckpointRecord {
            id: CheckpointId::new(),
            thread_id: tid1.clone(),
            turn_id: TurnId::new(),
            turn_ordinal: 1,
            git_ref: "refs/lingxi-t3/checkpoints/t1/1".into(),
            summary: "turn 1".into(),
            created_at: Utc::now(),
        };
        orch.dispatch(OrchestrationCommand::CaptureCheckpoint {
            command_id: CommandId::new(),
            checkpoint: cp.clone(),
        })
        .await
        .expect("cp");

        assert_eq!(db.list_messages(&tid1).expect("m1").len(), 1);
        assert_eq!(db.list_messages(&tid2).expect("m2").len(), 1);
        assert_eq!(db.list_checkpoints(&tid1).expect("cp").len(), 1);
    }
}
