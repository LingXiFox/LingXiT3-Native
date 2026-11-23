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

pub struct CodexDriver;

pub fn parse_codex_jsonl_line(line: &str) -> Option<ProviderTurnEvent> {
    let val: Value = serde_json::from_str(line.trim()).ok()?;
    let kind = val.get("type").and_then(Value::as_str)?;
    match kind {
        "item.completed" | "agent_message" => {
            let text = val
                .pointer("/item/text")
                .or_else(|| val.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if text.is_empty() {
                None
            } else {
                Some(ProviderTurnEvent::TextDelta(text))
            }
        }
        "turn.completed" => Some(ProviderTurnEvent::Completed {
            provider_session_id: val
                .get("thread_id")
                .and_then(Value::as_str)
                .map(String::from),
        }),
        "error" => Some(ProviderTurnEvent::Failed(
            val.get("message")
                .and_then(Value::as_str)
                .unwrap_or("Codex error")
                .to_string(),
        )),
        _ => None,
    }
}

#[async_trait]
impl ProviderDriver for CodexDriver {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }

    async fn detect_status(&self, _custom_cmd: Option<&str>) -> ProviderStatus {
        let (bin, version) = probe_binary_version("codex", &["--version"]).await;
        let Some(binary_path) = bin else {
            return missing_provider_status(ProviderKind::Codex, "codex");
        };
        ProviderStatus {
            kind: ProviderKind::Codex,
            install_state: ProviderInstallState::Installed,
            auth_state: ProviderAuthState::Unknown,
            binary_path: Some(binary_path),
            version,
            models: Vec::new(),
            config_options: Vec::new(),
            status_message: Some("Codex CLI detected".into()),
        }
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
        _permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
    ) -> Result<Option<String>, ProviderError> {
        let approval_flag = match request.runtime_mode {
            RuntimeMode::FullAccess => "never",
            RuntimeMode::AutoAcceptEdits | RuntimeMode::Auto => "on-request",
            RuntimeMode::ApprovalRequired => "untrusted",
        };
        let mut cmd = Command::new("codex");
        cmd.arg("exec")
            .arg("--json")
            .arg("--ask-for-approval")
            .arg(approval_flag)
            .arg("-C")
            .arg(&request.cwd);
        if !request.model_selection.model_slug.is_empty() {
            cmd.arg("-m").arg(&request.model_selection.model_slug);
        }
        cmd.arg(&request.prompt)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd
            .spawn()
            .map_err(|e| ProviderError::Execution(ProviderKind::Codex, e.to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ProviderError::Execution(ProviderKind::Codex, "missing stdout".into())
        })?;
        let mut lines = BufReader::new(stdout).lines();
        let mut session_id = None;
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(ev) = parse_codex_jsonl_line(&line) {
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
            .map_err(|e| ProviderError::Execution(ProviderKind::Codex, e.to_string()))?;
        if !status.success() {
            return Err(ProviderError::Execution(
                ProviderKind::Codex,
                format!("codex exited with {status}"),
            ));
        }
        Ok(session_id)
    }
}
