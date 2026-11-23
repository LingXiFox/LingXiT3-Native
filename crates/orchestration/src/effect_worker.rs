use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{Duration as ChronoDuration, Utc};
use lingxi_domain::{
    ApprovalId, CheckpointId, CheckpointRecord, CommandId, MessageId, OrchestrationCommand,
    PermissionDecision, ThreadId, TurnId,
};
use rusqlite::params;
use tokio::sync::{Mutex, oneshot};

use crate::OrchestrationError;
use crate::orchestrator::{Orchestrator, OutboxEffect};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait TurnExecutor: Send + Sync {
    fn execute_turn(
        &self,
        thread_id: ThreadId,
        turn_id: TurnId,
        assistant_message_id: MessageId,
        prompt: String,
        orchestrator: Arc<Orchestrator>,
    ) -> BoxFuture<'_, Result<Option<String>, String>>;
}

pub trait CheckpointExecutor: Send + Sync {
    fn capture_checkpoint(
        &self,
        thread_id: ThreadId,
        turn_id: TurnId,
    ) -> BoxFuture<'_, Result<Option<CheckpointRecord>, String>>;

    fn revert_checkpoint(
        &self,
        thread_id: ThreadId,
        checkpoint_id: CheckpointId,
    ) -> BoxFuture<'_, Result<(), String>>;
}

pub struct NoopCheckpointExecutor;

impl CheckpointExecutor for NoopCheckpointExecutor {
    fn capture_checkpoint(
        &self,
        _thread_id: ThreadId,
        _turn_id: TurnId,
    ) -> BoxFuture<'_, Result<Option<CheckpointRecord>, String>> {
        Box::pin(async { Ok(None) })
    }

    fn revert_checkpoint(
        &self,
        _thread_id: ThreadId,
        _checkpoint_id: CheckpointId,
    ) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async { Ok(()) })
    }
}

pub struct EffectWorker {
    orchestrator: Arc<Orchestrator>,
    turn_executor: Arc<dyn TurnExecutor>,
    checkpoint_executor: Arc<dyn CheckpointExecutor>,
    approval_waiters: Arc<Mutex<HashMap<ApprovalId, oneshot::Sender<PermissionDecision>>>>,
    active_cancellations: Arc<Mutex<HashMap<ThreadId, oneshot::Sender<()>>>>,
    worker_id: String,
}

impl EffectWorker {
    pub fn new(
        orchestrator: Arc<Orchestrator>,
        turn_executor: Arc<dyn TurnExecutor>,
        checkpoint_executor: Arc<dyn CheckpointExecutor>,
    ) -> Self {
        Self {
            orchestrator,
            turn_executor,
            checkpoint_executor,
            approval_waiters: Arc::new(Mutex::new(HashMap::new())),
            active_cancellations: Arc::new(Mutex::new(HashMap::new())),
            worker_id: format!("worker-{}", uuid::Uuid::new_v4()),
        }
    }

    pub async fn register_approval_waiter(
        &self,
        approval_id: ApprovalId,
    ) -> oneshot::Receiver<PermissionDecision> {
        let (tx, rx) = oneshot::channel();
        self.approval_waiters.lock().await.insert(approval_id, tx);
        rx
    }

    pub async fn resolve_approval_waiter(
        &self,
        approval_id: &ApprovalId,
        decision: PermissionDecision,
    ) -> bool {
        if let Some(tx) = self.approval_waiters.lock().await.remove(approval_id) {
            let _ = tx.send(decision);
            true
        } else {
            false
        }
    }

    pub async fn cancel_thread_effect(&self, thread_id: &ThreadId) -> bool {
        if let Some(tx) = self.active_cancellations.lock().await.remove(thread_id) {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }

    /// Claims up to `limit` pending outbox effects with a 60s lease and executes them concurrently.
    pub async fn process_pending_once(&self, limit: usize) -> Result<usize, OrchestrationError> {
        let claimed = self.claim_effects(limit)?;
        let count = claimed.len();
        let mut handles = Vec::with_capacity(count);

        for effect in claimed {
            let orch = Arc::clone(&self.orchestrator);
            let turn_exec = Arc::clone(&self.turn_executor);
            let cp_exec = Arc::clone(&self.checkpoint_executor);
            let cancels = Arc::clone(&self.active_cancellations);
            let handle = tokio::spawn(async move {
                Self::run_single_effect(orch, turn_exec, cp_exec, cancels, effect).await;
            });
            handles.push(handle);
        }

        for h in handles {
            let _ = h.await;
        }
        Ok(count)
    }

    fn claim_effects(&self, limit: usize) -> Result<Vec<OutboxEffect>, OrchestrationError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let lease_expires = (now + ChronoDuration::seconds(60)).to_rfc3339();
        let db = self.orchestrator.db();
        let mut conn = db.lock();
        let tx = conn
            .transaction()
            .map_err(lingxi_storage::StorageError::from)?;

        let mut stmt = tx
            .prepare(
                "SELECT effect_id, command_id, thread_id, effect_type, payload_json
                 FROM orchestration_effect_outbox
                 WHERE status = 'pending'
                 ORDER BY created_at ASC
                 LIMIT ?1",
            )
            .map_err(lingxi_storage::StorageError::from)?;

        let rows = stmt
            .query_map([limit as i64], |row| {
                let payload_str: String = row.get(4)?;
                Ok(OutboxEffect {
                    effect_id: row.get(0)?,
                    command_id: CommandId::from_string(row.get::<_, String>(1)?),
                    thread_id: ThreadId::from_string(row.get::<_, String>(2)?),
                    effect_type: row.get(3)?,
                    payload: serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null),
                })
            })
            .map_err(lingxi_storage::StorageError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(lingxi_storage::StorageError::from)?;
        drop(stmt);

        for eff in &rows {
            tx.execute(
                "UPDATE orchestration_effect_outbox
                 SET status = 'running',
                     attempt_count = attempt_count + 1,
                     lease_owner = ?2,
                     lease_expires_at = ?3,
                     updated_at = ?4
                 WHERE effect_id = ?1",
                params![eff.effect_id, self.worker_id, lease_expires, now_str],
            )
            .map_err(lingxi_storage::StorageError::from)?;
        }

        tx.commit().map_err(lingxi_storage::StorageError::from)?;
        Ok(rows)
    }

    async fn run_single_effect(
        orchestrator: Arc<Orchestrator>,
        turn_executor: Arc<dyn TurnExecutor>,
        checkpoint_executor: Arc<dyn CheckpointExecutor>,
        active_cancellations: Arc<Mutex<HashMap<ThreadId, oneshot::Sender<()>>>>,
        effect: OutboxEffect,
    ) {
        let result: Result<(), String> = match effect.effect_type.as_str() {
            "run_provider_turn" => {
                let turn_id =
                    TurnId::from_string(effect.payload["turn_id"].as_str().unwrap_or_default());
                let msg_id = MessageId::from_string(
                    effect.payload["assistant_message_id"]
                        .as_str()
                        .unwrap_or_default(),
                );
                let prompt = effect.payload["prompt"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();

                let (cancel_tx, cancel_rx) = oneshot::channel();
                active_cancellations
                    .lock()
                    .await
                    .insert(effect.thread_id.clone(), cancel_tx);

                let turn_res = tokio::select! {
                    res = turn_executor.execute_turn(
                        effect.thread_id.clone(),
                        turn_id.clone(),
                        msg_id.clone(),
                        prompt,
                        Arc::clone(&orchestrator),
                    ) => res,
                    _ = cancel_rx => Err("turn cancelled".to_string()),
                };

                active_cancellations.lock().await.remove(&effect.thread_id);

                let (session_id, error_msg) = match turn_res {
                    Ok(sid) => (sid, None),
                    Err(err) => (None, Some(err)),
                };
                let _ = orchestrator
                    .dispatch(OrchestrationCommand::CompleteTurn {
                        command_id: CommandId::new(),
                        thread_id: effect.thread_id.clone(),
                        turn_id,
                        message_id: msg_id,
                        provider_session_id: session_id,
                        error: error_msg.clone(),
                    })
                    .await;
                match error_msg {
                    None => Ok(()),
                    Some(e) => Err(e),
                }
            }
            "capture_checkpoint" => {
                let turn_id =
                    TurnId::from_string(effect.payload["turn_id"].as_str().unwrap_or_default());
                match checkpoint_executor
                    .capture_checkpoint(effect.thread_id.clone(), turn_id)
                    .await
                {
                    Ok(Some(cp)) => {
                        let _ = orchestrator
                            .dispatch(OrchestrationCommand::CaptureCheckpoint {
                                command_id: CommandId::new(),
                                checkpoint: cp,
                            })
                            .await;
                        Ok(())
                    }
                    Ok(None) => Ok(()),
                    Err(e) => Err(e),
                }
            }
            "revert_checkpoint" => {
                let cp_id = CheckpointId::from_string(
                    effect.payload["checkpoint_id"].as_str().unwrap_or_default(),
                );
                checkpoint_executor
                    .revert_checkpoint(effect.thread_id.clone(), cp_id)
                    .await
            }
            other => Err(format!("unknown outbox effect type: {other}")),
        };

        let now_str = Utc::now().to_rfc3339();
        let db = orchestrator.db();
        let conn = db.lock();
        match result {
            Ok(()) => {
                let _ = conn.execute(
                    "UPDATE orchestration_effect_outbox
                     SET status = 'succeeded', lease_owner = NULL, lease_expires_at = NULL, updated_at = ?2
                     WHERE effect_id = ?1",
                    params![effect.effect_id, now_str],
                );
            }
            Err(err) => {
                let _ = conn.execute(
                    "UPDATE orchestration_effect_outbox
                     SET status = 'failed', last_error = ?2, lease_owner = NULL, lease_expires_at = NULL, updated_at = ?3
                     WHERE effect_id = ?1",
                    params![effect.effect_id, err, now_str],
                );
            }
        }
    }
}
