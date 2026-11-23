use std::process::Stdio;

use async_trait::async_trait;
use lingxi_domain::{
    PermissionDecision, ProviderAuthState, ProviderInstallState, ProviderKind, ProviderStatus,
    RuntimeMode,
};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::ProviderError;
use crate::driver::{
    ProviderDriver, ProviderTurnEvent, TurnRequest, missing_provider_status, probe_binary_version,
};

pub struct ClaudeDriver;

pub fn parse_claude_stream_json_line(line: &str) -> Vec<ProviderTurnEvent> {
    let Ok(val) = serde_json::from_str::<Value>(line.trim()) else {
        return Vec::new();
    };
    let mut events = Vec::new();
    let kind = val.get("type").and_then(Value::as_str).unwrap_or_default();

    match kind {
        "assistant" => {
            if let Some(content) = val.pointer("/message/content").and_then(Value::as_array) {
                for block in content {
                    let btype = block
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if btype == "text"
                        && let Some(text) = block.get("text").and_then(Value::as_str)
                    {
                        events.push(ProviderTurnEvent::TextDelta(text.to_string()));
                    } else if btype == "thinking"
                        && let Some(thinking) = block.get("thinking").and_then(Value::as_str)
                    {
                        events.push(ProviderTurnEvent::ReasoningDelta(thinking.to_string()));
                    } else if btype == "tool_use" {
                        events.push(ProviderTurnEvent::ToolCall {
                            id: block
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or("tool")
                                .to_string(),
                            name: block
                                .get("name")
                                .and_then(Value::as_str)
                                .unwrap_or("tool")
                                .to_string(),
                            input: block.get("input").cloned().unwrap_or(Value::Null),
                        });
                    }
                }
            }
        }
        "result" => {
            let sid = val
                .get("session_id")
                .and_then(Value::as_str)
                .map(String::from);
            if val.get("is_error").and_then(Value::as_bool) == Some(true) {
                let err_msg = val
                    .get("result")
                    .and_then(Value::as_str)
                    .unwrap_or("Claude execution failed")
                    .to_string();
                events.push(ProviderTurnEvent::Failed(err_msg));
            } else {
                events.push(ProviderTurnEvent::Completed {
                    provider_session_id: sid,
                });
            }
        }
        _ => {}
    }

    events
}

#[async_trait]
impl ProviderDriver for ClaudeDriver {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    async fn detect_status(&self, _custom_cmd: Option<&str>) -> ProviderStatus {
        let (bin, version) = probe_binary_version("claude", &["--version"]).await;
        let Some(binary_path) = bin else {
            return missing_provider_status(ProviderKind::Claude, "claude");
        };
        ProviderStatus {
            kind: ProviderKind::Claude,
            install_state: ProviderInstallState::Installed,
            auth_state: ProviderAuthState::Unknown,
            binary_path: Some(binary_path),
            version,
            models: Vec::new(),
            config_options: Vec::new(),
            status_message: Some("Claude Code CLI detected".into()),
        }
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
        _permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
    ) -> Result<Option<String>, ProviderError> {
        let perm_mode = match request.runtime_mode {
            RuntimeMode::FullAccess => "bypassPermissions",
            RuntimeMode::AutoAcceptEdits => "acceptEdits",
            RuntimeMode::Auto | RuntimeMode::ApprovalRequired => "default",
        };
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg(&request.prompt)
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .arg("--permission-mode")
            .arg(perm_mode)
            .current_dir(&request.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if let Some(ref sid) = request.provider_session_id {
            cmd.arg("--resume").arg(sid);
        }
        if !request.model_selection.model_slug.is_empty() {
            cmd.arg("--model").arg(&request.model_selection.model_slug);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| ProviderError::Execution(ProviderKind::Claude, e.to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ProviderError::Execution(ProviderKind::Claude, "missing stdout".into())
        })?;
        let mut lines = BufReader::new(stdout).lines();
        let mut session_id = request.provider_session_id;
        while let Ok(Some(line)) = lines.next_line().await {
            for ev in parse_claude_stream_json_line(&line) {
                if let ProviderTurnEvent::Completed {
                    provider_session_id: Some(ref sid),
                } = ev
                {
                    session_id = Some(sid.clone());
                }
                let _ = events_tx.send(ev);
            }
        }
        let status = child
            .wait()
            .await
            .map_err(|e| ProviderError::Execution(ProviderKind::Claude, e.to_string()))?;
        if !status.success() {
            return Err(ProviderError::Execution(
                ProviderKind::Claude,
                format!("claude exited with {status}"),
            ));
        }
        Ok(session_id)
    }
}
