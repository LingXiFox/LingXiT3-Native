use chrono::Utc;
use lingxi_domain::PeerId;
use lingxi_storage::Database;
use rusqlite::params;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::RemoteError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcScope {
    Read,
    Write,
    Admin,
}

impl RpcScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Admin => "admin",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PairedPeerRecord {
    pub peer_id: PeerId,
    pub device_name: String,
    pub scopes: Vec<String>,
    pub revoked: bool,
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("{digest:x}")
}

pub fn issue_pairing_token(
    db: &Database,
    device_name: &str,
    scopes: &[RpcScope],
) -> Result<(PeerId, String), RemoteError> {
    let peer_id = PeerId::new();
    let raw_token = format!("lx_{}", Uuid::new_v4().simple());
    let token_hash = hash_token(&raw_token);
    let scope_list: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
    let scopes_json =
        serde_json::to_string(&scope_list).map_err(|e| RemoteError::Transport(e.to_string()))?;
    let now = Utc::now().to_rfc3339();

    let conn = db.lock();
    conn.execute(
        "INSERT INTO remote_peers (peer_id, device_name, token_hash, scopes_json, created_at, revoked)
         VALUES (?1, ?2, ?3, ?4, ?5, 0)",
        params![peer_id.as_str(), device_name, token_hash, scopes_json, now],
    )
    .map_err(|e| RemoteError::Transport(e.to_string()))?;

    Ok((peer_id, raw_token))
}

pub fn verify_token(
    db: &Database,
    raw_token: &str,
    required_scope: RpcScope,
) -> Result<PairedPeerRecord, RemoteError> {
    let token_hash = hash_token(raw_token);
    let conn = db.lock();
    let row: Option<(String, String, String, i64)> = conn
        .query_row(
            "SELECT peer_id, device_name, scopes_json, revoked
             FROM remote_peers WHERE token_hash = ?1",
            [&token_hash],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();

    let Some((peer_id, device_name, scopes_json, revoked)) = row else {
        return Err(RemoteError::Unauthorized("invalid pairing token".into()));
    };
    if revoked != 0 {
        return Err(RemoteError::Unauthorized("device token revoked".into()));
    }
    let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();
    let needed = required_scope.as_str();
    let allowed = scopes.iter().any(|s| s == "admin" || s == needed);
    if !allowed {
        return Err(RemoteError::Unauthorized(format!(
            "missing required scope '{needed}'"
        )));
    }
    let _ = conn.execute(
        "UPDATE remote_peers SET last_seen_at = ?2 WHERE peer_id = ?1",
        params![peer_id, Utc::now().to_rfc3339()],
    );

    Ok(PairedPeerRecord {
        peer_id: PeerId::from_string(peer_id),
        device_name,
        scopes,
        revoked: false,
    })
}

pub fn revoke_peer(db: &Database, peer_id: &PeerId) -> Result<bool, RemoteError> {
    let conn = db.lock();
    let affected = conn
        .execute(
            "UPDATE remote_peers SET revoked = 1 WHERE peer_id = ?1",
            [peer_id.as_str()],
        )
        .map_err(|e| RemoteError::Transport(e.to_string()))?;
    Ok(affected > 0)
}
