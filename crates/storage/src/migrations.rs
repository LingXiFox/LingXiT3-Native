use rusqlite::Connection;

use crate::StorageError;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "001_initial_orchestration_and_projections",
    sql: r#"
CREATE TABLE IF NOT EXISTS projects (
    project_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    workspace_root TEXT NOT NULL,
    default_model_json TEXT,
    archived INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS threads (
    thread_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    model_selection_json TEXT NOT NULL,
    runtime_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    branch TEXT,
    worktree_path TEXT,
    provider_session_id TEXT,
    pinned INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    unread INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_threads_project_updated
    ON threads(project_id, archived, pinned DESC, updated_at DESC);

CREATE TABLE IF NOT EXISTS turns (
    turn_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES threads(thread_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL,
    status TEXT NOT NULL,
    prompt TEXT NOT NULL,
    error_message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_turns_thread_ordinal
    ON turns(thread_id, ordinal);

CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES threads(thread_id) ON DELETE CASCADE,
    turn_id TEXT,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    reasoning TEXT,
    is_streaming INTEGER NOT NULL DEFAULT 0,
    tool_calls_json TEXT NOT NULL DEFAULT '[]',
    attachments_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_thread_created
    ON messages(thread_id, created_at);

CREATE TABLE IF NOT EXISTS pending_approvals (
    approval_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES threads(thread_id) ON DELETE CASCADE,
    turn_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    summary TEXT NOT NULL,
    raw_input_json TEXT NOT NULL,
    status TEXT NOT NULL,
    decision TEXT,
    created_at TEXT NOT NULL,
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_approvals_thread_status
    ON pending_approvals(thread_id, status);

CREATE TABLE IF NOT EXISTS checkpoints (
    checkpoint_id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES threads(thread_id) ON DELETE CASCADE,
    turn_id TEXT NOT NULL,
    turn_ordinal INTEGER NOT NULL,
    git_ref TEXT NOT NULL,
    summary TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_thread_ordinal
    ON checkpoints(thread_id, turn_ordinal);

CREATE TABLE IF NOT EXISTS orchestration_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    command_id TEXT NOT NULL,
    thread_id TEXT,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_orch_events_thread_seq
    ON orchestration_events(thread_id, sequence);

CREATE TABLE IF NOT EXISTS orchestration_command_receipts (
    command_id TEXT PRIMARY KEY,
    event_sequence INTEGER NOT NULL,
    accepted_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS orchestration_effect_outbox (
    effect_id TEXT PRIMARY KEY,
    command_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    effect_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'succeeded', 'failed', 'cancelled')),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    lease_owner TEXT,
    lease_expires_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_error TEXT
);

CREATE INDEX IF NOT EXISTS idx_outbox_claim
    ON orchestration_effect_outbox(status, lease_expires_at, created_at);

CREATE TABLE IF NOT EXISTS drafts (
    thread_id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scheduled_tasks (
    task_id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    project_id TEXT,
    prompt TEXT NOT NULL,
    interval_seconds INTEGER NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    last_run_at TEXT,
    next_run_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS remote_peers (
    peer_id TEXT PRIMARY KEY,
    device_name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    scopes_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_seen_at TEXT,
    revoked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS import_audit_log (
    import_id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    backup_path TEXT NOT NULL,
    projects_imported INTEGER NOT NULL,
    threads_imported INTEGER NOT NULL,
    messages_imported INTEGER NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL
);
"#,
}];

pub fn run_migrations(conn: &mut Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    for migration in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [migration.version],
            |row| row.get(0),
        )?;
        if already_applied {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(migration.sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![migration.version, migration.name],
        )?;
        tx.commit()?;
    }

    Ok(())
}
