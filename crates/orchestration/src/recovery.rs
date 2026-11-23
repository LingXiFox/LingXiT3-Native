use chrono::Utc;
use lingxi_storage::Database;
use rusqlite::params;

use crate::OrchestrationError;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    pub stale_effects_recovered: usize,
    pub interrupted_threads_recovered: usize,
    pub interrupted_turns_recovered: usize,
}

/// Scans for crashed/interrupted runs and expired outbox leases on startup.
pub fn recover_after_unclean_exit(db: &Database) -> Result<RecoveryReport, OrchestrationError> {
    let now_str = Utc::now().to_rfc3339();
    let mut conn = db.lock();
    let tx = conn
        .transaction()
        .map_err(lingxi_storage::StorageError::from)?;

    let stale_effects_recovered = tx
        .execute(
            "UPDATE orchestration_effect_outbox
             SET status = 'failed',
                 last_error = 'recovered after unclean shutdown',
                 lease_owner = NULL,
                 lease_expires_at = NULL,
                 updated_at = ?1
             WHERE status = 'running'",
            params![now_str],
        )
        .map_err(lingxi_storage::StorageError::from)?;

    let interrupted_turns_recovered = tx
        .execute(
            "UPDATE turns
             SET status = 'failed',
                 error_message = 'interrupted by unclean exit',
                 completed_at = ?1
             WHERE status = 'running'",
            params![now_str],
        )
        .map_err(lingxi_storage::StorageError::from)?;

    tx.execute(
        "UPDATE messages
         SET is_streaming = 0, updated_at = ?1
         WHERE is_streaming = 1",
        params![now_str],
    )
    .map_err(lingxi_storage::StorageError::from)?;

    let interrupted_threads_recovered = tx
        .execute(
            "UPDATE threads
             SET status = 'error', updated_at = ?1
             WHERE status IN ('running', 'waiting_approval')",
            params![now_str],
        )
        .map_err(lingxi_storage::StorageError::from)?;

    tx.commit().map_err(lingxi_storage::StorageError::from)?;

    Ok(RecoveryReport {
        stale_effects_recovered,
        interrupted_threads_recovered,
        interrupted_turns_recovered,
    })
}
