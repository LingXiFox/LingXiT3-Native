use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::AcpError;
use crate::protocol::{
    AcpClientCapabilities, AcpClientInfo, AcpInitializeResult, AcpNormalizedEvent,
    AcpSessionNewResult, JsonRpcRequest, JsonRpcResponse, normalize_session_update,
};

type PendingMap = Arc<Mutex<HashMap<i64, oneshot::Sender<Result<Value, AcpError>>>>>;

#[derive(Debug, Clone)]
pub struct AcpPermissionRequest {
    pub request_id: Value,
    pub tool_title: String,
    pub raw_input: Value,
    pub options: Vec<Value>,
}

pub struct AcpClient {
    stdin: Arc<Mutex<ChildStdin>>,
    pending: PendingMap,
    next_id: AtomicI64,
    child: Arc<Mutex<Child>>,
    pub updates_rx: Mutex<mpsc::UnboundedReceiver<AcpNormalizedEvent>>,
    pub permission_rx: Mutex<mpsc::UnboundedReceiver<AcpPermissionRequest>>,
}

impl AcpClient {
    pub async fn spawn(
        command: &str,
        args: &[String],
        cwd: &Path,
        env_overrides: &[(String, String)],
    ) -> Result<Self, AcpError> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for (k, v) in env_overrides {
            cmd.env(k, v);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| AcpError::Transport(format!("failed to spawn {command}: {e}")))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AcpError::Transport("missing child stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AcpError::Transport("missing child stdout".into()))?;

        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let (updates_tx, updates_rx) = mpsc::unbounded_channel();
        let (permission_tx, permission_rx) = mpsc::unbounded_channel();

        let pending_reader = Arc::clone(&pending);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(val) = serde_json::from_str::<Value>(trimmed) else {
                    tracing::debug!(line = trimmed, "ignoring non-JSON stdout line from ACP");
                    continue;
                };

                if let Some(method) = val.get("method").and_then(Value::as_str) {
                    if let Some(id) = val.get("id").cloned() {
                        // Server -> client request (e.g. session/request_permission)
                        if method == "session/request_permission" {
                            let params = val.get("params").cloned().unwrap_or(Value::Null);
                            let tool_title = params
                                .pointer("/toolCall/title")
                                .and_then(Value::as_str)
                                .unwrap_or("Tool call")
                                .to_string();
                            let raw_input = params
                                .pointer("/toolCall/rawInput")
                                .cloned()
                                .unwrap_or(Value::Null);
                            let options = params
                                .get("options")
                                .and_then(Value::as_array)
                                .cloned()
                                .unwrap_or_default();
                            let _ = permission_tx.send(AcpPermissionRequest {
                                request_id: id,
                                tool_title,
                                raw_input,
                                options,
                            });
                        }
                    } else if method == "session/update"
                        && let Some(params) = val.get("params")
                        && let Some(ev) = normalize_session_update(params)
                    {
                        let _ = updates_tx.send(ev);
                    }
                    continue;
                }

                if let Some(id) = val.get("id").and_then(Value::as_i64)
                    && let Some(tx) = pending_reader.lock().await.remove(&id)
                {
                    if let Some(err) = val.get("error") {
                        let code = err.get("code").and_then(Value::as_i64).unwrap_or(-32000);
                        let msg = err
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unknown ACP error")
                            .to_string();
                        let _ = tx.send(Err(AcpError::Protocol { code, message: msg }));
                    } else {
                        let res = val.get("result").cloned().unwrap_or(Value::Null);
                        let _ = tx.send(Ok(res));
                    }
                }
            }
        });

        Ok(Self {
            stdin: Arc::new(Mutex::new(stdin)),
            pending,
            next_id: AtomicI64::new(1),
            child: Arc::new(Mutex::new(child)),
            updates_rx: Mutex::new(updates_rx),
            permission_rx: Mutex::new(permission_rx),
        })
    }

    pub async fn request(&self, method: &str, params: Value) -> Result<Value, AcpError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Value::Number(id.into()),
            method: method.to_string(),
            params: Some(params),
        };
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let mut payload =
            serde_json::to_string(&req).map_err(|e| AcpError::Transport(e.to_string()))?;
        payload.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(payload.as_bytes())
                .await
                .map_err(|e| AcpError::Transport(e.to_string()))?;
            stdin
                .flush()
                .await
                .map_err(|e| AcpError::Transport(e.to_string()))?;
        }

        tokio::time::timeout(std::time::Duration::from_secs(60), rx)
            .await
            .map_err(|_| AcpError::Timeout(format!("ACP request {method} timed out")))?
            .map_err(|_| AcpError::Transport("ACP response channel dropped".into()))?
    }

    pub async fn respond_permission(
        &self,
        request_id: Value,
        option_id: Option<&str>,
    ) -> Result<(), AcpError> {
        let result = match option_id {
            Some(opt) => serde_json::json!({
                "outcome": { "outcome": "selected", "optionId": opt }
            }),
            None => serde_json::json!({
                "outcome": { "outcome": "cancelled" }
            }),
        };
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request_id,
            result: Some(result),
            error: None,
        };
        let mut line =
            serde_json::to_string(&resp).map_err(|e| AcpError::Transport(e.to_string()))?;
        line.push('\n');
        let mut stdin = self.stdin.lock().await;
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| AcpError::Transport(e.to_string()))?;
        stdin
            .flush()
            .await
            .map_err(|e| AcpError::Transport(e.to_string()))?;
        Ok(())
    }

    pub async fn initialize(
        &self,
        client_info: AcpClientInfo,
        capabilities: AcpClientCapabilities,
    ) -> Result<AcpInitializeResult, AcpError> {
        let res = self
            .request(
                "initialize",
                serde_json::json!({
                    "protocolVersion": 1,
                    "clientInfo": client_info,
                    "clientCapabilities": capabilities,
                }),
            )
            .await?;
        serde_json::from_value(res).map_err(|e| AcpError::Transport(e.to_string()))
    }

    pub async fn session_new(&self, cwd: &Path) -> Result<AcpSessionNewResult, AcpError> {
        let res = self
            .request(
                "session/new",
                serde_json::json!({
                    "cwd": cwd.to_string_lossy(),
                    "mcpServers": [],
                }),
            )
            .await?;
        serde_json::from_value(res).map_err(|e| AcpError::Transport(e.to_string()))
    }

    pub async fn session_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<Option<String>, AcpError> {
        let res = self
            .request(
                "session/prompt",
                serde_json::json!({
                    "sessionId": session_id,
                    "prompt": [{ "type": "text", "text": prompt }],
                }),
            )
            .await?;
        Ok(res
            .get("stopReason")
            .and_then(Value::as_str)
            .map(String::from))
    }

    pub async fn session_set_config_option(
        &self,
        session_id: &str,
        config_id: &str,
        value: &str,
    ) -> Result<(), AcpError> {
        self.request(
            "session/set_config_option",
            serde_json::json!({
                "sessionId": session_id,
                "configId": config_id,
                "value": value,
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn session_cancel(&self, session_id: &str) -> Result<(), AcpError> {
        self.request(
            "session/cancel",
            serde_json::json!({ "sessionId": session_id }),
        )
        .await?;
        Ok(())
    }

    pub async fn shutdown(&self) {
        let mut child = self.child.lock().await;
        let _ = child.kill().await;
    }
}
