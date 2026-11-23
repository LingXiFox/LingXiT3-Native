use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use lingxi_acp::{
    AcpClient, AcpClientCapabilities, AcpClientInfo, AcpNormalizedEvent, AcpSessionConfigOption,
    normalize_config_options,
};
use lingxi_domain::{
    ModelCapabilities, PermissionDecision, ProviderAuthState, ProviderInstallState, ProviderKind,
    ProviderModel, ProviderStatus,
};
use regex::Regex;
use semver::Version;
use tokio::sync::mpsc;

use crate::ProviderError;
use crate::driver::{
    ProviderDriver, ProviderTurnEvent, TurnRequest, missing_provider_status, probe_binary_version,
};

pub const DSH_MIN_VERSION: (u64, u64, u64) = (0, 2, 0);

pub struct DshDriver;

pub fn parse_dsh_version(raw: Option<&str>) -> Option<Version> {
    let re = Regex::new(r"\b(\d+\.\d+\.\d+)(?:-[0-9A-Za-z.-]+)?\b").ok()?;
    let m = re.find(raw?.trim())?;
    Version::parse(m.as_str()).ok()
}

pub fn is_dsh_version_compatible(raw: Option<&str>) -> bool {
    let Some(ver) = parse_dsh_version(raw) else {
        return false;
    };
    ver >= Version::new(DSH_MIN_VERSION.0, DSH_MIN_VERSION.1, DSH_MIN_VERSION.2)
}

/// Parses DSH `session/new` `configOptions`, preserving JSON `["provider","model"]` wire tuples
/// and disambiguating duplicate display names with their group label.
pub fn parse_dsh_models_from_config_options(
    config_options: &[AcpSessionConfigOption],
) -> Vec<ProviderModel> {
    let descriptors = normalize_config_options(config_options);
    let reasoning_desc = descriptors
        .iter()
        .find(|d| d.id == "reasoning_effort" || d.category.as_deref() == Some("thought_level"))
        .cloned();

    let Some(model_opt) = descriptors
        .iter()
        .find(|d| d.id == "model" || d.category.as_deref() == Some("model"))
    else {
        return Vec::new();
    };

    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for entry in &model_opt.options {
        let base = if entry.label.trim().is_empty() {
            entry.value.clone()
        } else {
            entry.label.trim().to_string()
        };
        *name_counts.entry(base).or_insert(0) += 1;
    }

    let mut seen = HashSet::new();
    let mut models = Vec::new();
    for entry in &model_opt.options {
        if entry.value.trim().is_empty() || !seen.insert(entry.value.clone()) {
            continue;
        }
        let base_name = if entry.label.trim().is_empty() {
            entry.value.clone()
        } else {
            entry.label.trim().to_string()
        };
        let is_dup = name_counts.get(&base_name).copied().unwrap_or(0) > 1;
        let display_name = match (is_dup, &entry.group) {
            (true, Some(g)) => format!("{base_name} ({g})"),
            _ => base_name,
        };
        let option_descriptors = reasoning_desc.clone().into_iter().collect::<Vec<_>>();
        let supports_thinking = !option_descriptors.is_empty();
        models.push(ProviderModel {
            slug: entry.value.clone(),
            name: display_name,
            is_default: model_opt.current_value.as_deref() == Some(entry.value.as_str()),
            is_custom: false,
            capabilities: ModelCapabilities {
                supports_thinking,
                supports_images: true,
                supports_resume: true,
                supports_orchestrator: false,
                supports_planner_executor_split: false,
                option_descriptors,
            },
        });
    }
    models
}

#[async_trait]
impl ProviderDriver for DshDriver {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Dsh
    }

    async fn detect_status(&self, _custom_cmd: Option<&str>) -> ProviderStatus {
        let (bin, version) = probe_binary_version("dsh", &["--version"]).await;
        let Some(binary_path) = bin else {
            return missing_provider_status(ProviderKind::Dsh, "dsh");
        };
        let compatible = is_dsh_version_compatible(version.as_deref());
        ProviderStatus {
            kind: ProviderKind::Dsh,
            install_state: if compatible {
                ProviderInstallState::Installed
            } else {
                ProviderInstallState::IncompatibleVersion
            },
            auth_state: ProviderAuthState::Unknown,
            binary_path: Some(binary_path),
            version,
            models: Vec::new(),
            config_options: Vec::new(),
            status_message: Some(if compatible {
                "Ready via `dsh --profile acp`".into()
            } else {
                "DSH version must be >= 0.2.0".into()
            }),
        }
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
        _permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
    ) -> Result<Option<String>, ProviderError> {
        let client = AcpClient::spawn(
            "dsh",
            &["--profile".to_string(), "acp".to_string()],
            &request.cwd,
            &[],
        )
        .await?;

        client
            .initialize(
                AcpClientInfo {
                    name: "lingxi-t3-native".into(),
                    title: Some("LingXi T3 Native".into()),
                    version: env!("CARGO_PKG_VERSION").into(),
                },
                AcpClientCapabilities::default(),
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
                .session_set_config_option(&sid, "reasoning_effort", level)
                .await;
        }

        let prompt_fut = client.session_prompt(&sid, &request.prompt);
        tokio::pin!(prompt_fut);

        loop {
            let mut updates_rx = client.updates_rx.lock().await;
            tokio::select! {
                res = &mut prompt_fut => {
                    drop(updates_rx);
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
            }
        }
    }
}
