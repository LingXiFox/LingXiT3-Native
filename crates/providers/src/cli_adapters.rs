use std::process::Stdio;

use async_trait::async_trait;
use lingxi_acp::{AcpClient, AcpClientCapabilities, AcpClientInfo, AcpNormalizedEvent};
use lingxi_domain::{
    PermissionDecision, ProviderAuthState, ProviderInstallState, ProviderKind, ProviderStatus,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

use crate::ProviderError;
use crate::driver::{
    ProviderDriver, ProviderTurnEvent, TurnRequest, missing_provider_status, probe_binary_version,
};

macro_rules! define_cli_provider {
    ($struct_name:ident, $kind:expr, $bin:expr, $version_flag:expr, $acp_args:expr) => {
        pub struct $struct_name;

        #[async_trait]
        impl ProviderDriver for $struct_name {
            fn kind(&self) -> ProviderKind {
                $kind
            }

            async fn detect_status(&self, _custom_cmd: Option<&str>) -> ProviderStatus {
                let (bin, version) = probe_binary_version($bin, &[$version_flag]).await;
                let Some(binary_path) = bin else {
                    return missing_provider_status($kind, $bin);
                };
                ProviderStatus {
                    kind: $kind,
                    install_state: ProviderInstallState::Installed,
                    auth_state: ProviderAuthState::Unknown,
                    binary_path: Some(binary_path),
                    version,
                    models: Vec::new(),
                    config_options: Vec::new(),
                    status_message: Some(format!("{} detected", $kind.display_name())),
                }
            }

            async fn run_turn(
                &self,
                request: TurnRequest,
                events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
                _permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
            ) -> Result<Option<String>, ProviderError> {
                let acp_args: &[&str] = $acp_args;
                if !acp_args.is_empty() {
                    let args: Vec<String> = acp_args.iter().map(|s| (*s).to_string()).collect();
                    let client = AcpClient::spawn($bin, &args, &request.cwd, &[]).await?;
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
                    let _ = client.session_prompt(&sid, &request.prompt).await?;
                    while let Ok(ev) = client.updates_rx.lock().await.try_recv() {
                        if let AcpNormalizedEvent::TextDelta(t) = ev {
                            let _ = events_tx.send(ProviderTurnEvent::TextDelta(t));
                        }
                    }
                    let _ = events_tx.send(ProviderTurnEvent::Completed {
                        provider_session_id: Some(sid.clone()),
                    });
                    client.shutdown().await;
                    return Ok(Some(sid));
                }

                let mut cmd = Command::new($bin);
                cmd.arg(&request.prompt)
                    .current_dir(&request.cwd)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .kill_on_drop(true);
                let mut child = cmd
                    .spawn()
                    .map_err(|e| ProviderError::Execution($kind, e.to_string()))?;
                if let Some(stdout) = child.stdout.take() {
                    let mut lines = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if !line.is_empty() {
                            let _ =
                                events_tx.send(ProviderTurnEvent::TextDelta(format!("{line}\n")));
                        }
                    }
                }
                let status = child
                    .wait()
                    .await
                    .map_err(|e| ProviderError::Execution($kind, e.to_string()))?;
                if !status.success() {
                    return Err(ProviderError::Execution(
                        $kind,
                        format!("{} exited with {status}", $bin),
                    ));
                }
                let _ = events_tx.send(ProviderTurnEvent::Completed {
                    provider_session_id: None,
                });
                Ok(None)
            }
        }
    };
}

define_cli_provider!(
    CursorDriver,
    ProviderKind::Cursor,
    "cursor-agent",
    "--version",
    &[]
);
define_cli_provider!(GrokDriver, ProviderKind::Grok, "grok", "--version", &[]);
define_cli_provider!(
    OpenCodeDriver,
    ProviderKind::OpenCode,
    "opencode",
    "--version",
    &["acp"]
);
define_cli_provider!(PiDriver, ProviderKind::Pi, "pi", "--version", &[]);
define_cli_provider!(MuseDriver, ProviderKind::Muse, "muse", "--version", &[]);
define_cli_provider!(
    KiroDriver,
    ProviderKind::Kiro,
    "kiro-cli",
    "--version",
    &["acp"]
);
define_cli_provider!(FxDriver, ProviderKind::Fx, "fx", "--version", &[]);

pub struct AcpRegistryDriver;

#[async_trait]
impl ProviderDriver for AcpRegistryDriver {
    fn kind(&self) -> ProviderKind {
        ProviderKind::AcpRegistry
    }

    async fn detect_status(&self, custom_cmd: Option<&str>) -> ProviderStatus {
        let Some(cmd_line) = custom_cmd.map(str::trim).filter(|s| !s.is_empty()) else {
            return ProviderStatus {
                kind: ProviderKind::AcpRegistry,
                install_state: ProviderInstallState::Missing,
                auth_state: ProviderAuthState::Unknown,
                binary_path: None,
                version: None,
                models: Vec::new(),
                config_options: Vec::new(),
                status_message: Some("No custom ACP command configured".into()),
            };
        };
        let parts: Vec<&str> = cmd_line.split_whitespace().collect();
        let bin = parts[0];
        let (path, version) = probe_binary_version(bin, &["--version"]).await;
        match path {
            Some(p) => ProviderStatus {
                kind: ProviderKind::AcpRegistry,
                install_state: ProviderInstallState::Installed,
                auth_state: ProviderAuthState::Unknown,
                binary_path: Some(p),
                version,
                models: Vec::new(),
                config_options: Vec::new(),
                status_message: Some(format!("Configured custom ACP: {cmd_line}")),
            },
            None => missing_provider_status(ProviderKind::AcpRegistry, bin),
        }
    }

    async fn run_turn(
        &self,
        request: TurnRequest,
        events_tx: mpsc::UnboundedSender<ProviderTurnEvent>,
        _permission_rx: mpsc::UnboundedReceiver<(String, PermissionDecision)>,
    ) -> Result<Option<String>, ProviderError> {
        let cmd_line = request
            .custom_acp_command
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or(ProviderError::NotInstalled(ProviderKind::AcpRegistry))?;
        let parts: Vec<String> = cmd_line.split_whitespace().map(String::from).collect();
        let client = AcpClient::spawn(&parts[0], &parts[1..], &request.cwd, &[]).await?;
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
        let _ = client.session_prompt(&sid, &request.prompt).await?;
        while let Ok(ev) = client.updates_rx.lock().await.try_recv() {
            if let AcpNormalizedEvent::TextDelta(t) = ev {
                let _ = events_tx.send(ProviderTurnEvent::TextDelta(t));
            }
        }
        let _ = events_tx.send(ProviderTurnEvent::Completed {
            provider_session_id: Some(sid.clone()),
        });
        client.shutdown().await;
        Ok(Some(sid))
    }
}
