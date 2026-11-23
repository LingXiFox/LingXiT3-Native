use std::path::{Path, PathBuf};

use chrono::Utc;
use lingxi_domain::{ModelSelection, ProviderKind};
use rusqlite::{Connection, OpenFlags, params};
use uuid::Uuid;

use crate::{Database, StorageError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyImportReport {
    pub import_id: String,
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub projects_imported: usize,
    pub threads_imported: usize,
    pub messages_imported: usize,
}

/// Imports projects, threads, and messages from an existing LingXi T3 Electron SQLite
/// database in strict read-only mode. Creates a timestamped backup of the target native
/// database beforehand so the import can be rolled back cleanly.
pub fn import_legacy_readonly(
    target_db: &Database,
    legacy_db_path: &Path,
    backups_dir: &Path,
) -> Result<LegacyImportReport, StorageError> {
    if !legacy_db_path.exists() {
        return Err(StorageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("legacy database not found: {}", legacy_db_path.display()),
        )));
    }

    std::fs::create_dir_all(backups_dir)?;
    let import_id = Uuid::new_v4().to_string();
    let ts = Utc::now().format("%Y%m%d%H%M%S");
    let backup_path = backups_dir.join(format!("native-pre-import-{ts}-{import_id}.sqlite"));

    if let Some(target_path) = target_db.path() {
        if target_path.exists() {
            std::fs::copy(target_path, &backup_path)?;
        }
    } else {
        std::fs::write(&backup_path, b"in-memory-marker")?;
    }

    let legacy_conn = Connection::open_with_flags(
        legacy_db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    let mut projects_imported = 0usize;
    let mut threads_imported = 0usize;
    let mut messages_imported = 0usize;

    let mut target_conn = target_db.lock();
    let tx = target_conn.transaction()?;
    let now_str = Utc::now().to_rfc3339();

    if table_exists(&legacy_conn, "projection_projects")? {
        let mut stmt = legacy_conn.prepare(
            "SELECT project_id, title, workspace_root, created_at, updated_at
             FROM projection_projects WHERE deleted_at IS NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;
        for row in rows {
            let (pid, title, root, created, updated) = row?;
            let changed = tx.execute(
                "INSERT OR IGNORE INTO projects (project_id, title, workspace_root, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![pid, title, root, created, updated],
            )?;
            projects_imported += changed;
        }
    }

    if table_exists(&legacy_conn, "projection_threads")? {
        let default_model_json = serde_json::to_string(&ModelSelection {
            provider: ProviderKind::Codex,
            model_slug: "default".into(),
            thinking_level: None,
            planner_model: None,
            executor_model: None,
        })?;
        let mut stmt = legacy_conn.prepare(
            "SELECT thread_id, project_id, title, branch, worktree_path, created_at, updated_at
             FROM projection_threads WHERE deleted_at IS NULL",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (tid, pid, title, branch, worktree_path, created, updated) = row?;
            let changed = tx.execute(
                "INSERT OR IGNORE INTO threads (
                    thread_id, project_id, title, model_selection_json, runtime_mode,
                    status, branch, worktree_path, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, 'approval-required', 'idle', ?5, ?6, ?7, ?8)",
                params![
                    tid,
                    pid,
                    title,
                    default_model_json,
                    branch,
                    worktree_path,
                    created,
                    updated
                ],
            )?;
            threads_imported += changed;
        }
    }

    if table_exists(&legacy_conn, "projection_thread_messages")? {
        let mut stmt = legacy_conn.prepare(
            "SELECT message_id, thread_id, turn_id, role, text, created_at, updated_at
             FROM projection_thread_messages",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        for row in rows {
            let (mid, tid, turn_id, role, text, created, updated) = row?;
            let changed = tx.execute(
                "INSERT OR IGNORE INTO messages (
                    message_id, thread_id, turn_id, role, content, is_streaming, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7)",
                params![mid, tid, turn_id, role, text, created, updated],
            )?;
            messages_imported += changed;
        }
    }

    tx.execute(
        "INSERT INTO import_audit_log (
            import_id, source_path, backup_path, projects_imported, threads_imported,
            messages_imported, status, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'completed', ?7)",
        params![
            import_id,
            legacy_db_path.to_string_lossy(),
            backup_path.to_string_lossy(),
            projects_imported as i64,
            threads_imported as i64,
            messages_imported as i64,
            now_str
        ],
    )?;

    tx.commit()?;

    Ok(LegacyImportReport {
        import_id,
        source_path: legacy_db_path.to_path_buf(),
        backup_path,
        projects_imported,
        threads_imported,
        messages_imported,
    })
}

pub fn rollback_import(target_db_path: &Path, backup_path: &Path) -> Result<(), StorageError> {
    if !backup_path.exists() {
        return Err(StorageError::Rollback(format!(
            "backup file not found: {}",
            backup_path.display()
        )));
    }
    std::fs::copy(backup_path, target_db_path)?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, StorageError> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
        [table],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn file_sha256(path: &Path) -> Vec<u8> {
        let bytes = std::fs::read(path).expect("read");
        Sha256::digest(&bytes).to_vec()
    }

    #[test]
    fn readonly_import_preserves_legacy_sha256_and_supports_rollback() {
        let dir = tempfile::tempdir().expect("tempdir");
        let legacy_path = dir.path().join("statev2.sqlite");
        let native_path = dir.path().join("native.sqlite");
        let backups_dir = dir.path().join("backups");

        // Seed a realistic legacy Electron database
        {
            let conn = Connection::open(&legacy_path).expect("open legacy");
            conn.execute_batch(
                "CREATE TABLE projection_projects (
                    project_id TEXT PRIMARY KEY,
                    title TEXT NOT NULL,
                    workspace_root TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                CREATE TABLE projection_threads (
                    thread_id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    branch TEXT,
                    worktree_path TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    deleted_at TEXT
                );
                CREATE TABLE projection_thread_messages (
                    message_id TEXT PRIMARY KEY,
                    thread_id TEXT NOT NULL,
                    turn_id TEXT,
                    role TEXT NOT NULL,
                    text TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO projection_projects VALUES ('p-1', 'Legacy Project', '/tmp/legacy', '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z', NULL);
                INSERT INTO projection_threads VALUES ('t-1', 'p-1', 'Legacy Thread', 'main', NULL, '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z', NULL);
                INSERT INTO projection_thread_messages VALUES ('m-1', 't-1', NULL, 'user', 'hello from electron', '2026-11-01T00:00:00Z', '2026-11-01T00:00:00Z');",
            )
            .expect("seed legacy");
        }
        let hash_before = file_sha256(&legacy_path);

        let native_db = Database::open(&native_path).expect("open native");
        assert_eq!(native_db.list_projects().expect("p").len(), 0);

        let report =
            import_legacy_readonly(&native_db, &legacy_path, &backups_dir).expect("import");
        assert_eq!(report.projects_imported, 1);
        assert_eq!(report.threads_imported, 1);
        assert_eq!(report.messages_imported, 1);

        let hash_after = file_sha256(&legacy_path);
        assert_eq!(hash_before, hash_after, "legacy DB must remain untouched");

        assert_eq!(native_db.list_projects().expect("p").len(), 1);
        assert_eq!(native_db.list_threads().expect("t").len(), 1);

        drop(native_db);
        rollback_import(&native_path, &report.backup_path).expect("rollback");
        let restored = Database::open(&native_path).expect("reopen");
        assert_eq!(restored.list_projects().expect("p").len(), 0);
    }
}
