use async_trait::async_trait;
use lingxi_acp::{
    AcpClient, AcpClientCapabilities, AcpClientInfo, AcpFsCapabilities, AcpNormalizedEvent,
    AcpSessionConfigOption, normalize_config_options,
};
use lingxi_domain::{
    ModelCapabilities, PermissionDecision, ProviderAuthState, ProviderInstallState, ProviderKind,
    ProviderModel, ProviderStatus, RuntimeMode,
};
use regex::Regex;
use semver::Version;
use tokio::sync::mpsc;

use crate::ProviderError;
use crate::driver::{
    ProviderDriver, ProviderTurnEvent, TurnRequest, missing_provider_status, probe_binary_version,
};

pub struct AntigravityDriver;

pub fn parse_antigravity_semver(raw: Option<&str>) -> Option<Version> {
    let re = Regex::new(r"(\d+\.\d+\.\d+)").ok()?;
    let m = re.find(raw?.trim())?;
    Version::parse(m.as_str()).ok()
}

/// Older releases (< 1.5.1) only expose all models to allowlisted clients such as zed.
pub fn antigravity_client_info(version: Option<&str>) -> AcpClientInfo {
    let open_access = Version::new(1, 5, 1);
    if let Some(ver) = parse_antigravity_semver(version)
        && ver >= open_access
    {
        AcpClientInfo {
            name: "lingxi-t3-native".to_string(),
            title: Some("LingXi T3 Native".to_string()),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    } else {
        AcpClientInfo {
            name: "zed".to_string(),
            title: None,
            version: "0.0.0".to_string(),
        }
    }
}

pub fn antigravity_permission_mode(mode: RuntimeMode) -> &'static str {
    match mode {
        RuntimeMode::FullAccess => "yolo",
        RuntimeMode::AutoAcceptEdits => "auto_edit",
        RuntimeMode::Auto => "auto",
        RuntimeMode::ApprovalRequired => "default",
    }
}

/// Builds models from real ACP `configOptions`. Planner/Executor split is ONLY enabled
/// when the protocol response genuinely exposes planner/executor config options.
pub fn parse_antigravity_models(config_options: &[AcpSessionConfigOption]) -> Vec<ProviderModel> {
    let descriptors = normalize_config_options(config_options);
    let has_planner_executor = descriptors.iter().any(|d| {
        let id = d.id.to_lowercase();
        id.contains("planner") || id.contains("executor")
    });
    let supports_thinking = descriptors.iter().any(|d| {
        d.id == "thinking"
            || d.id == "reasoning_effort"
            || d.category.as_deref() == Some("thought_level")
    });
    let supports_orchestrator = descriptors.iter().any(|d| {
        d.options
            .iter()
            .any(|o| o.value.to_lowercase().contains("orchestrator"))
    });

    let Some(model_opt) = descriptors
        .iter()
        .find(|d| d.id == "model" || d.category.as_deref() == Some("model"))
    else {
        return Vec::new();
    };

    let current = model_opt.current_value.as_deref();
    model_opt
        .options
        .iter()
        .map(|opt| ProviderModel {
            slug: opt.value.clone(),
            name: opt.label.clone(),
            is_default: current == Some(opt.value.as_str()),
            is_custom: false,
            capabilities: ModelCapabilities {
                supports_thinking,
                supports_images: true,
                supports_resume: true,
                supports_orchestrator,
                supports_planner_executor_split: has_planner_executor,
                option_descriptors: descriptors.clone(),
            },
        })
        .collect()
}

#[async_trait]
impl ProviderDriver for AntigravityDriver {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Antigravity
    }

    async fn detect_status(&self, _custom_cmd: Option<&str>) -> ProviderStatus {
        let (bin, version) = probe_binary_version("agy", &["--version"]).await;
        let Some(binary_path) = bin else {
            return missing_provider_status(ProviderKind::Antigravity, "agy");
        };
        ProviderStatus {
            kind: ProviderKind::Antigravity,
            install_state: ProviderInstallState::Installed,
            auth_state: ProviderAuthState::Unknown,
            binary_path: Some(binary_path),
            version,
            models: Vec::new(),
            config_options: Vec::new(),
            status_message: Some("Ready via `agy acp`; models discovered on session start".into()),
        }
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
        mut permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
    ) -> Result<Option<String>, ProviderError> {
        let (_, version) = probe_binary_version("agy", &["--version"]).await;
        let client = AcpClient::spawn("agy", &["acp".to_string()], &request.cwd, &[]).await?;

        client
            .initialize(
                antigravity_client_info(version.as_deref()),
                AcpClientCapabilities {
                    fs: AcpFsCapabilities {
                        read_text_file: true,
                        write_text_file: false,
                    },
                    terminal: false,
                },
            )
            .await?;

        let session = client.session_new(&request.cwd).await?;
        let sid = session.session_id.clone();

        if !request.model_selection.model_slug.is_empty() {
            let _ = client
                .session_set_config_option(&sid, "model", &request.model_selection.model_slug)
                .await;
        }
        if let Some(level) = &request.model_selection.thinking_level {
            let _ = client
                .session_set_config_option(&sid, "thinking", level)
                .await;
        }

        let prompt_fut = client.session_prompt(&sid, &request.prompt);
        tokio::pin!(prompt_fut);

        loop {
            let mut updates_rx = client.updates_rx.lock().await;
            let mut perm_rx = client.permission_rx.lock().await;
            tokio::select! {
                res = &mut prompt_fut => {
                    drop(updates_rx);
                    drop(perm_rx);
                    match res {
                        Ok(_) => {
                            let _ = events_tx.send(ProviderTurnEvent::Completed {
                                provider_session_id: Some(sid.clone()),
                            });
                            client.shutdown().await;
                            return Ok(Some(sid.clone()));
                        }
                        Err(err) => {
                            client.shutdown().await;
                            return Err(ProviderError::Acp(err));
                        }
                    }
                }
                Some(ev) = updates_rx.recv() => {
                    match ev {
                        AcpNormalizedEvent::TextDelta(t) => {
                            let _ = events_tx.send(ProviderTurnEvent::TextDelta(t));
                        }
                        AcpNormalizedEvent::ReasoningDelta(r) => {
                            let _ = events_tx.send(ProviderTurnEvent::ReasoningDelta(r));
                        }
                        AcpNormalizedEvent::ToolCall { id, name, input } => {
                            let _ = events_tx.send(ProviderTurnEvent::ToolCall { id, name, input });
                        }
                        AcpNormalizedEvent::ToolCallUpdate { id, status, output } => {
                            let _ = events_tx.send(ProviderTurnEvent::ToolCallUpdate { id, status, output });
                        }
                        AcpNormalizedEvent::Usage { input_tokens, output_tokens, cache_read_tokens } => {
                            let _ = events_tx.send(ProviderTurnEvent::Usage {
                                input_tokens,
                                output_tokens,
                                cache_read_tokens,
                            });
                        }
                        AcpNormalizedEvent::Completed { .. } | AcpNormalizedEvent::Error(_) => {}
                    }
                }
                Some(req) = perm_rx.recv() => {
                    let req_id_str = req.request_id.to_string();
                    let _ = events_tx.send(ProviderTurnEvent::PermissionRequested {
                        id: req_id_str.clone(),
                        tool_name: req.tool_title.clone(),
                        summary: req.tool_title,
                        raw_input: req.raw_input,
                    });
                    if let Some((_, decision)) = permission_rx.recv().await {
                        let option_id = match decision {
                            PermissionDecision::AllowOnce | PermissionDecision::AllowSession => {
                                req.options.first().and_then(|o| o.get("optionId")).and_then(|v| v.as_str())
                            }
                            PermissionDecision::Deny => None,
                        };
                        let _ = client.respond_permission(req.request_id, option_id).await;
                    }
                }
            }
        }
    }
}
