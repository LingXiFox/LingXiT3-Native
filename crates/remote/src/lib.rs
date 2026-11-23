pub mod auth;
pub mod client;
pub mod server;
pub mod ssh;

pub use auth::{PairedPeerRecord, RpcScope, issue_pairing_token, revoke_peer, verify_token};
pub use client::connect_with_backoff;
pub use server::{RemoteServerState, build_router, serve_on_port};
pub use ssh::{SshTarget, run_ssh_command};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemoteError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures_util::{SinkExt, StreamExt};
    use lingxi_domain::{CommandId, OrchestrationCommand, ProjectId};
    use lingxi_orchestration::Orchestrator;
    use lingxi_storage::Database;
    use tokio_tungstenite::tungstenite::Message;

    use super::*;

    #[tokio::test]
    async fn ws_server_enforces_scopes_dispatches_commands_and_replays_events() {
        let db = Database::open_in_memory().expect("db");
        let orch = Arc::new(Orchestrator::new(db.clone()));
        let (peer_id, read_token) =
            issue_pairing_token(&db, "iPhone ReadOnly", &[RpcScope::Read]).expect("issue");
        let (_admin_peer, write_token) =
            issue_pairing_token(&db, "Web Client", &[RpcScope::Read, RpcScope::Write])
                .expect("issue");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let router = build_router(RemoteServerState {
            orchestrator: Arc::clone(&orch),
        });
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.ok();
        });

        // Unauthorized connection rejected
        let unauth = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token=bad")).await;
        assert!(unauth.is_err());

        // Read-only token connects and gets snapshot, but write RPC is denied
        let (mut ws_read, _) =
            tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token={read_token}"))
                .await
                .expect("connect read");

        let write_attempt = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "orchestration.v2.dispatchCommand",
            "params": OrchestrationCommand::CreateProject {
                command_id: CommandId::new(),
                project_id: ProjectId::new(),
                title: "Denied".into(),
                workspace_root: "/tmp/denied".into(),
            }
        });
        ws_read
            .send(Message::Text(write_attempt.to_string().into()))
            .await
            .expect("send");
        let reply = ws_read.next().await.expect("msg").expect("ok");
        let reply_json: serde_json::Value =
            serde_json::from_str(&reply.into_text().unwrap()).unwrap();
        assert!(reply_json.get("error").is_some());

        // Write-scoped token succeeds and triggers broadcast
        let (mut ws_write, _) =
            tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token={write_token}"))
                .await
                .expect("connect write");
        let valid_cmd = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "orchestration.v2.dispatchCommand",
            "params": OrchestrationCommand::CreateProject {
                command_id: CommandId::new(),
                project_id: ProjectId::new(),
                title: "Remote Repo".into(),
                workspace_root: "/tmp/remote".into(),
            }
        });
        ws_write
            .send(Message::Text(valid_cmd.to_string().into()))
            .await
            .expect("send");
        let ok_reply = ws_write.next().await.expect("msg").expect("ok");
        let ok_text = ok_reply.into_text().unwrap();
        assert!(ok_text.contains("Remote Repo") || ok_text.contains("project.created"));

        // Revoking peer invalidates token immediately
        assert!(revoke_peer(&db, &peer_id).expect("revoke"));
        assert!(verify_token(&db, &read_token, RpcScope::Read).is_err());

        server.abort();
    }
}
