pub mod client;
pub mod protocol;

pub use client::{AcpClient, AcpPermissionRequest};
pub use protocol::{
    AcpAgentInfo, AcpClientCapabilities, AcpClientInfo, AcpFsCapabilities, AcpInitializeResult,
    AcpNormalizedEvent, AcpSessionConfigOption, AcpSessionNewResult, normalize_config_options,
    normalize_session_update,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AcpError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("protocol error ({code}): {message}")]
    Protocol { code: i64, message: String },
    #[error("timeout: {0}")]
    Timeout(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acp_client_handshakes_parses_config_options_and_streams_updates() {
        let script = r#"
import sys, json
for line in sys.stdin:
    req = json.loads(line)
    m = req.get("method")
    rid = req.get("id")
    if m == "initialize":
        print(json.dumps({"jsonrpc":"2.0","id":rid,"result":{"protocolVersion":1,"agentInfo":{"name":"fake-acp","version":"1.5.2"}}}), flush=True)
    elif m == "session/new":
        print(json.dumps({"jsonrpc":"2.0","id":rid,"result":{"sessionId":"s-1","configOptions":[{"id":"model","name":"Model","category":"model","type":"select","currentValue":"m1","options":[{"groupId":"g1","name":"Group A","options":[{"value":"m1","name":"Model One"}]}]}]}}), flush=True)
    elif m == "session/prompt":
        print(json.dumps({"jsonrpc":"2.0","method":"session/update","params":{"update":{"sessionUpdate":"agent_message_chunk","content":{"text":"Hello ACP"}}}}), flush=True)
        print(json.dumps({"jsonrpc":"2.0","id":rid,"result":{"stopReason":"end_turn"}}), flush=True)
"#;
        let dir = tempfile::tempdir().expect("tempdir");
        let script_path = dir.path().join("fake_acp.py");
        std::fs::write(&script_path, script).expect("write script");

        let client = AcpClient::spawn(
            "python3",
            &[script_path.to_string_lossy().to_string()],
            dir.path(),
            &[],
        )
        .await
        .expect("spawn");

        let init = client
            .initialize(
                AcpClientInfo {
                    name: "lingxi-t3-native".into(),
                    title: Some("LingXi T3 Native".into()),
                    version: "0.1.0".into(),
                },
                AcpClientCapabilities::default(),
            )
            .await
            .expect("init");
        assert_eq!(init.agent_info.unwrap().name, "fake-acp");

        let session = client.session_new(dir.path()).await.expect("session_new");
        let descriptors = normalize_config_options(&session.config_options);
        assert_eq!(descriptors[0].options[0].group.as_deref(), Some("Group A"));

        let stop = client
            .session_prompt(&session.session_id, "hi")
            .await
            .expect("prompt");
        assert_eq!(stop.as_deref(), Some("end_turn"));

        let update = client.updates_rx.lock().await.recv().await;
        assert_eq!(
            update,
            Some(AcpNormalizedEvent::TextDelta("Hello ACP".into()))
        );
        client.shutdown().await;
    }
}
