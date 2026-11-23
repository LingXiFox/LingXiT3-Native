use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{DateTime, Utc};
use lingxi_domain::{
    ChatMessage, CheckpointRecord, MessageId, MessageRole, ModelSelection, PendingApproval,
    ProjectId, ProjectRecord, RuntimeMode, StoredDomainEvent, ThreadId, ThreadRecord,
    ThreadRunStatus,
};
use rusqlite::{Connection, OptionalExtension, params};

use crate::StorageError;
use crate::migrations::run_migrations;

#[derive(Clone)]
pub struct Database {
    path: Option<PathBuf>,
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(path)?;
        configure_connection(&mut conn)?;
        run_migrations(&mut conn)?;
        Ok(Self {
            path: Some(path.to_path_buf()),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut conn = Connection::open_in_memory()?;
        configure_connection(&mut conn)?;
        run_migrations(&mut conn)?;
        Ok(Self {
            path: None,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("database mutex poisoned")
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectRecord>, StorageError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT project_id, title, workspace_root, default_model_json, archived, created_at, updated_at
             FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let default_model_json: Option<String> = row.get(3)?;
            let default_model = default_model_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<ModelSelection>(s).ok());
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            Ok(ProjectRecord {
                id: ProjectId::from_string(row.get::<_, String>(0)?),
                title: row.get(1)?,
                workspace_root: row.get(2)?,
                default_model,
                archived: row.get::<_, i64>(4)? != 0,
                created_at: parse_dt(&created_str),
                updated_at: parse_dt(&updated_str),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_threads(&self) -> Result<Vec<ThreadRecord>, StorageError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT thread_id, project_id, title, model_selection_json, runtime_mode, status,
                    branch, worktree_path, provider_session_id, pinned, archived, unread,
                    created_at, updated_at
             FROM threads ORDER BY pinned DESC, updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let model_json: String = row.get(3)?;
            let mode_str: String = row.get(4)?;
            let status_str: String = row.get(5)?;
            let created_str: String = row.get(12)?;
            let updated_str: String = row.get(13)?;
            Ok(ThreadRecord {
                id: ThreadId::from_string(row.get::<_, String>(0)?),
                project_id: ProjectId::from_string(row.get::<_, String>(1)?),
                title: row.get(2)?,
                model_selection: serde_json::from_str(&model_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                runtime_mode: serde_json::from_str(&format!("\"{mode_str}\""))
                    .unwrap_or(RuntimeMode::ApprovalRequired),
                status: serde_json::from_str(&format!("\"{status_str}\""))
                    .unwrap_or(ThreadRunStatus::Idle),
                branch: row.get(6)?,
                worktree_path: row.get(7)?,
                provider_session_id: row.get(8)?,
                pinned: row.get::<_, i64>(9)? != 0,
                archived: row.get::<_, i64>(10)? != 0,
                unread: row.get::<_, i64>(11)? != 0,
                created_at: parse_dt(&created_str),
                updated_at: parse_dt(&updated_str),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_messages(&self, thread_id: &ThreadId) -> Result<Vec<ChatMessage>, StorageError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT message_id, thread_id, turn_id, role, content, reasoning, is_streaming,
                    tool_calls_json, attachments_json, created_at, updated_at
             FROM messages WHERE thread_id = ?1 ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([thread_id.as_str()], |row| {
            let role_str: String = row.get(3)?;
            let tc_json: String = row.get(7)?;
            let att_json: String = row.get(8)?;
            let created_str: String = row.get(9)?;
            let updated_str: String = row.get(10)?;
            Ok(ChatMessage {
                id: MessageId::from_string(row.get::<_, String>(0)?),
                thread_id: ThreadId::from_string(row.get::<_, String>(1)?),
                turn_id: row
                    .get::<_, Option<String>>(2)?
                    .map(lingxi_domain::TurnId::from_string),
                role: serde_json::from_str(&format!("\"{role_str}\""))
                    .unwrap_or(MessageRole::Assistant),
                content: row.get(4)?,
                reasoning: row.get(5)?,
                is_streaming: row.get::<_, i64>(6)? != 0,
                tool_calls: serde_json::from_str(&tc_json).unwrap_or_default(),
                attachments: serde_json::from_str(&att_json).unwrap_or_default(),
                created_at: parse_dt(&created_str),
                updated_at: parse_dt(&updated_str),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_pending_approvals(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<PendingApproval>, StorageError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT approval_id, thread_id, turn_id, tool_name, summary, raw_input_json, created_at
             FROM pending_approvals WHERE thread_id = ?1 AND status = 'pending' ORDER BY created_at ASC",
        )?;
        let rows = stmt.query_map([thread_id.as_str()], |row| {
            let raw_json: String = row.get(5)?;
            let created_str: String = row.get(6)?;
            Ok(PendingApproval {
                id: lingxi_domain::ApprovalId::from_string(row.get::<_, String>(0)?),
                thread_id: ThreadId::from_string(row.get::<_, String>(1)?),
                turn_id: lingxi_domain::TurnId::from_string(row.get::<_, String>(2)?),
                tool_name: row.get(3)?,
                summary: row.get(4)?,
                raw_input: serde_json::from_str(&raw_json).unwrap_or(serde_json::Value::Null),
                created_at: parse_dt(&created_str),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn list_checkpoints(
        &self,
        thread_id: &ThreadId,
    ) -> Result<Vec<CheckpointRecord>, StorageError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT checkpoint_id, thread_id, turn_id, turn_ordinal, git_ref, summary, created_at
             FROM checkpoints WHERE thread_id = ?1 ORDER BY turn_ordinal ASC",
        )?;
        let rows = stmt.query_map([thread_id.as_str()], |row| {
            let created_str: String = row.get(6)?;
            Ok(CheckpointRecord {
                id: lingxi_domain::CheckpointId::from_string(row.get::<_, String>(0)?),
                thread_id: ThreadId::from_string(row.get::<_, String>(1)?),
                turn_id: lingxi_domain::TurnId::from_string(row.get::<_, String>(2)?),
                turn_ordinal: row.get(3)?,
                git_ref: row.get(4)?,
                summary: row.get(5)?,
                created_at: parse_dt(&created_str),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn events_since(
        &self,
        since_sequence: u64,
        limit: usize,
    ) -> Result<Vec<StoredDomainEvent>, StorageError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT sequence, event_id, command_id, thread_id, event_type, payload_json, occurred_at
             FROM orchestration_events WHERE sequence > ?1 ORDER BY sequence ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since_sequence as i64, limit as i64], |row| {
            let payload_str: String = row.get(5)?;
            let occurred_str: String = row.get(6)?;
            Ok(StoredDomainEvent {
                sequence: row.get::<_, i64>(0)? as u64,
                event_id: lingxi_domain::EventId::from_string(row.get::<_, String>(1)?),
                command_id: lingxi_domain::CommandId::from_string(row.get::<_, String>(2)?),
                thread_id: row.get::<_, Option<String>>(3)?.map(ThreadId::from_string),
                event_type: row.get(4)?,
                payload: serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null),
                occurred_at: parse_dt(&occurred_str),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn get_draft(&self, thread_id: &ThreadId) -> Result<Option<String>, StorageError> {
        let conn = self.lock();
        conn.query_row(
            "SELECT content FROM drafts WHERE thread_id = ?1",
            [thread_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn save_draft(&self, thread_id: &ThreadId, content: &str) -> Result<(), StorageError> {
        let conn = self.lock();
        conn.execute(
            "INSERT INTO drafts (thread_id, content, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(thread_id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
            params![thread_id.as_str(), content, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }
}

fn configure_connection(conn: &mut Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA auto_vacuum = INCREMENTAL;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_apply_cleanly_and_drafts_roundtrip() {
        let db = Database::open_in_memory().expect("open");
        let tid = ThreadId::from_string("thread-draft");
        assert_eq!(db.get_draft(&tid).expect("get"), None);
        db.save_draft(&tid, "hello world").expect("save");
        assert_eq!(
            db.get_draft(&tid).expect("get"),
            Some("hello world".to_string())
        );
    }
}
