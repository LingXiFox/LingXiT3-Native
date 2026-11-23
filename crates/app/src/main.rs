mod bench;
mod config;
mod runtime;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use gpui_kit::{AppContext as _, Entity, KeyBinding, Window, actions, px, size};
use gpuicn::app::AppConfig;
use lingxi_domain::{
    CommandId, MessageId, ModelSelection, OrchestrationCommand, ProjectId, ProviderKind,
    RuntimeMode, ThreadId, TurnId,
};
use lingxi_orchestration::{
    EffectWorker, NoopCheckpointExecutor, Orchestrator, ProviderTurnExecutor,
    recover_after_unclean_exit,
};
use lingxi_platform::{APP_DISPLAY_NAME, AppPaths, SingleInstanceGuard};
use lingxi_providers::ProviderRegistry;
use lingxi_storage::{Database, import_legacy_readonly};
use lingxi_telemetry::{build_quota_snapshots, sample_process_resources};
use lingxi_terminal::TerminalManager;
use lingxi_ui::shell::{ActionDispatcher, LingXiShellView, toggle_appearance_mode};
use lingxi_ui::state::{AppUiState, InspectorTab, SettingsSection, UiAction};
use lingxi_ui::theme::{
    apply_appearance, lingxi_dark_theme, lingxi_light_theme, to_gpuicn_appearance,
};
use tracing_subscriber::EnvFilter;

use crate::config::{load_settings, save_settings};
use crate::runtime::AppRuntime;

actions!(
    lingxi,
    [
        ToggleCommandPalette,
        ToggleSidebarAction,
        ToggleTerminalAction,
        OpenSettingsAction,
    ]
);

#[derive(Debug, Parser)]
#[command(
    name = "lingxi-t3-native",
    version,
    about = "LingXi T3 Native GPUI Desktop"
)]
struct Cli {
    /// Override the native application data directory (defaults to ~/.lingxi-t3-native).
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Perform headless startup initialization checks and exit immediately.
    #[arg(long)]
    check_startup: bool,

    /// Run headless server mode (without opening the GPUI window).
    #[arg(long)]
    headless_server: bool,

    /// Path to a legacy LingXi T3 SQLite database to inspect/import read-only.
    #[arg(long)]
    import_legacy: Option<PathBuf>,

    /// Run built-in performance benchmarks and print JSON metrics.
    #[arg(long)]
    bench: bool,

    /// Probe locally installed provider CLIs and print JSON statuses.
    #[arg(long)]
    probe_providers: bool,
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

fn handle_ui_action(
    view: &Entity<LingXiShellView>,
    runtime: &AppRuntime,
    orchestrator: &Arc<Orchestrator>,
    action: UiAction,
    cx: &mut gpui_kit::AsyncApp,
) {
    cx.update_entity(view, |shell, cx| {
        match action {
            UiAction::SelectThread(tid) => shell.state.select_thread(tid),
            UiAction::CloseTab(tid) => shell.state.close_tab(&tid),
            UiAction::CreateThread {
                project_id,
                worktree,
            } => {
                let orch = Arc::clone(orchestrator);
                let tid = ThreadId::new();
                let branch = worktree.then(|| format!("lingxi/{}", &tid.0[..8]));
                let cmd = OrchestrationCommand::CreateThread {
                    command_id: CommandId::new(),
                    thread_id: tid.clone(),
                    project_id,
                    title: "新会话".to_string(),
                    model_selection: ModelSelection {
                        provider: ProviderKind::Codex,
                        model_slug: "default".to_string(),
                        thinking_level: None,
                        planner_model: None,
                        executor_model: None,
                    },
                    runtime_mode: RuntimeMode::ApprovalRequired,
                    branch,
                    worktree_path: None,
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
                shell.state.select_thread(tid);
            }
            UiAction::SendTurn { thread_id, prompt } => {
                let orch = Arc::clone(orchestrator);
                let cmd = OrchestrationCommand::StartTurn {
                    command_id: CommandId::new(),
                    thread_id,
                    turn_id: TurnId::new(),
                    message_id: MessageId::new(),
                    prompt,
                    attachments: Vec::new(),
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
            }
            UiAction::CancelTurn(thread_id) => {
                let orch = Arc::clone(orchestrator);
                let cmd = OrchestrationCommand::CancelTurn {
                    command_id: CommandId::new(),
                    thread_id,
                    turn_id: None,
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
            }
            UiAction::RunLegacyImport => {
                if let Some(source) = AppPaths::discover_legacy_database() {
                    let db = orchestrator.db().clone();
                    let backups = runtime.paths.backups_dir.clone();
                    std::mem::drop(runtime.spawn(async move {
                        match import_legacy_readonly(&db, &source, &backups) {
                            Ok(report) => tracing::info!(
                                projects = report.projects_imported,
                                threads = report.threads_imported,
                                messages = report.messages_imported,
                                "Legacy read-only import completed"
                            ),
                            Err(err) => tracing::error!(%err, "Legacy read-only import failed"),
                        }
                    }));
                }
            }
            UiAction::ToggleProjectExpanded(pid) => {
                if !shell.state.expanded_projects.remove(&pid) {
                    shell.state.expanded_projects.insert(pid);
                }
            }
            UiAction::ToggleSidebar => {
                shell.state.sidebar_collapsed = !shell.state.sidebar_collapsed;
            }
            UiAction::ToggleInspector(tab) => {
                if shell.state.inspector_open && shell.state.inspector_tab == tab {
                    shell.state.inspector_open = false;
                } else {
                    shell.state.inspector_open = true;
                    shell.state.inspector_tab = tab;
                }
            }
            UiAction::ToggleAppearance => {
                let next = toggle_appearance_mode(shell.state.settings.appearance);
                shell.state.settings.appearance = next;
                apply_appearance(cx, next);
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
            }
            UiAction::OpenSettings(section) => {
                shell.state.settings_section = section;
                shell.state.settings_open = true;
            }
            UiAction::CloseSettings => shell.state.settings_open = false,
            UiAction::OpenCommandPalette => shell.state.command_palette_open = true,
            UiAction::CloseCommandPalette => shell.state.command_palette_open = false,
            UiAction::OpenUpdateModal => shell.state.update_modal_open = true,
            UiAction::CloseUpdateModal => shell.state.update_modal_open = false,
            other => {
                tracing::debug!(?other, "UI action queued for backend services");
            }
        }
        cx.notify();
    });
}

fn main() -> anyhow::Result<()> {
    init_logging();
    let cli = Cli::parse();

    let paths = Arc::new(AppPaths::resolve(cli.data_dir.as_deref()));
    paths.ensure_dirs()?;
    if !paths.settings_path.exists() {
        save_settings(&paths.settings_path, &lingxi_domain::AppSettings::default())?;
    }
    let settings = load_settings(&paths.settings_path)?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lingxi-bg")
        .build()?;

    let app_runtime = AppRuntime {
        paths: Arc::clone(&paths),
        settings: settings.clone(),
        tokio_handle: rt.handle().clone(),
    };

    let db = Database::open(&paths.database_path)?;
    let recovery = recover_after_unclean_exit(&db)?;
    if recovery.interrupted_threads_recovered > 0 {
        tracing::warn!(
            ?recovery,
            "Recovered interrupted runs after unclean shutdown"
        );
    }

    let provider_registry = Arc::new(ProviderRegistry::with_all_builtin());
    let orchestrator = Arc::new(Orchestrator::new(db.clone()));
    let _terminal_mgr = Arc::new(TerminalManager::new());
    let _effect_worker = Arc::new(EffectWorker::new(
        Arc::clone(&orchestrator),
        Arc::new(ProviderTurnExecutor {
            registry: Arc::clone(&provider_registry),
            custom_acp_command: settings.custom_acp_command.clone(),
        }),
        Arc::new(NoopCheckpointExecutor),
    ));

    if cli.check_startup {
        let _guard = SingleInstanceGuard::acquire(&paths.lock_path)?;
        let ping = rt.block_on(app_runtime.spawn(async { "ok" }))?;
        tracing::info!(
            home_dir = %paths.home_dir.display(),
            appearance = ?settings.appearance,
            tokio = ping,
            "LingXi T3 Native startup self-check passed"
        );
        return Ok(());
    }

    if cli.headless_server {
        let _guard = SingleInstanceGuard::acquire(&paths.lock_path)?;
        tracing::info!(
            port = settings.remote_server_port,
            "Starting headless remote WS/RPC server"
        );
        rt.block_on(lingxi_remote::serve_on_port(
            Arc::clone(&orchestrator),
            settings.remote_server_port,
        ))?;
        return Ok(());
    }

    if cli.bench {
        return bench::run_native_benchmarks(&rt);
    }

    if cli.probe_providers {
        let statuses =
            rt.block_on(provider_registry.detect_all(settings.custom_acp_command.as_deref()));
        println!("{}", serde_json::to_string_pretty(&statuses)?);
        return Ok(());
    }

    if let Some(legacy_path) = cli.import_legacy {
        let report = import_legacy_readonly(&db, &legacy_path, &paths.backups_dir)?;
        tracing::info!(
            import_id = %report.import_id,
            projects = report.projects_imported,
            threads = report.threads_imported,
            messages = report.messages_imported,
            "Read-only legacy import completed"
        );
        return Ok(());
    }

    let _guard = SingleInstanceGuard::acquire(&paths.lock_path)?;
    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel::<UiAction>();
    let dispatcher: ActionDispatcher = Arc::new(move |action| {
        let _ = action_tx.send(action);
    });

    let dispatcher_for_keys = Arc::clone(&dispatcher);
    let mut ui_state = AppUiState::new(settings.clone());
    ui_state.projects = db.list_projects()?;
    ui_state.threads = db.list_threads()?;
    for project in &ui_state.projects {
        ui_state.expanded_projects.insert(project.id.clone());
    }
    if let Some(tid) = ui_state.threads.first().map(|t| t.id.clone()) {
        ui_state.select_thread(tid.clone());
        ui_state
            .messages_by_thread
            .insert(tid.clone(), db.list_messages(&tid)?);
    }
    ui_state.resource_usage = sample_process_resources();
    ui_state.quota_snapshots = build_quota_snapshots(&[]);
    let _ = ProjectId::new();

    let runtime_for_ui = app_runtime.clone();
    let orch_for_ui = Arc::clone(&orchestrator);
    let registry_for_bg = Arc::clone(&provider_registry);
    let custom_acp_for_bg = settings.custom_acp_command.clone();

    gpuicn::app::run(
        AppConfig::new(APP_DISPLAY_NAME)
            .themes(lingxi_light_theme(), lingxi_dark_theme())
            .appearance(to_gpuicn_appearance(settings.appearance)),
        move |window: &mut Window, cx| {
            window.set_window_title(APP_DISPLAY_NAME);
            window.resize(size(px(1360.0), px(860.0)));
            cx.bind_keys([
                KeyBinding::new("cmd-k", ToggleCommandPalette, None),
                KeyBinding::new("ctrl-k", ToggleCommandPalette, None),
                KeyBinding::new("cmd-b", ToggleSidebarAction, None),
                KeyBinding::new("ctrl-b", ToggleSidebarAction, None),
                KeyBinding::new("cmd-j", ToggleTerminalAction, None),
                KeyBinding::new("ctrl-j", ToggleTerminalAction, None),
                KeyBinding::new("cmd-,", OpenSettingsAction, None),
                KeyBinding::new("ctrl-,", OpenSettingsAction, None),
            ]);
            let d1 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &ToggleCommandPalette, _| {
                d1(UiAction::OpenCommandPalette)
            });
            let d2 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &ToggleSidebarAction, _| {
                d2(UiAction::ToggleSidebar)
            });
            let d3 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &ToggleTerminalAction, _| {
                d3(UiAction::ToggleInspector(InspectorTab::Terminal))
            });
            let d4 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &OpenSettingsAction, _| {
                d4(UiAction::OpenSettings(SettingsSection::General))
            });

            let entity: Entity<LingXiShellView> = cx.entity();
            let runtime_for_loop = runtime_for_ui.clone();
            let orch_for_loop = Arc::clone(&orch_for_ui);
            let entity_for_probe = entity.clone();
            let (probe_tx, probe_rx) = tokio::sync::oneshot::channel();
            std::mem::drop(runtime_for_ui.spawn(async move {
                let statuses = registry_for_bg
                    .detect_all(custom_acp_for_bg.as_deref())
                    .await;
                let _ = probe_tx.send(statuses);
            }));
            cx.spawn(async move |_weak, cx| {
                if let Ok(statuses) = probe_rx.await {
                    cx.update_entity(&entity_for_probe, |shell, cx| {
                        shell.state.quota_snapshots = build_quota_snapshots(&statuses);
                        shell.state.provider_statuses = statuses;
                        cx.notify();
                    });
                }
            })
            .detach();
            cx.spawn(async move |_weak, cx| {
                while let Some(action) = action_rx.recv().await {
                    handle_ui_action(&entity, &runtime_for_loop, &orch_for_loop, action, cx);
                }
            })
            .detach();
            LingXiShellView::new(ui_state, dispatcher, window, cx)
        },
    );

    Ok(())
}
