mod bench;
mod config;
mod runtime;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::Parser;
use gpui_kit::{
    AppContext as _, Bounds, Entity, KeyBinding, Window, WindowBounds, WindowOptions, actions, px,
    size,
};
use gpuicn::app::AppConfig;
use gpuicn::title_bar::title_bar_window_options;
use lingxi_domain::{
    ApprovalId, ChatMessage, CommandId, ContextBreakdown, GitFileDiff, GitStatusSummary, MessageId,
    MessageRole, ModelSelection, OrchestrationCommand, PendingApproval, ProjectId, ProviderKind,
    RuntimeMode, TerminalId, ThemeAppearance, ThreadId, ThreadRecord, ThreadRunStatus,
    ToolCallRecord, TurnId,
};
use lingxi_mcp::discover_skills;
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
use lingxi_ui::state::{
    ActiveModal, AppUiState, DiscoveredSkillView, InspectorTab, SettingsSection, TerminalTabView,
    UiAction,
};
use lingxi_ui::theme::{
    apply_appearance, lingxi_dark_theme, lingxi_light_theme, to_gpuicn_appearance,
};
use lingxi_workspace::{read_file_bounded, scan_workspace_files};
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

    /// Initial window size for QA / screenshot verification (e.g. 1440x900).
    #[arg(long)]
    window_size: Option<String>,

    /// Session theme override for QA / screenshot verification (light or dark).
    #[arg(long)]
    theme: Option<String>,

    /// Seed isolated QA workspace data (only allowed when --data-dir is explicitly set).
    #[arg(long)]
    qa_seed: bool,

    /// Open a specific view on startup for QA verification.
    #[arg(long)]
    open_view: Option<String>,
}

fn parse_window_size(raw: Option<&str>) -> (f32, f32) {
    if let Some(s) = raw
        && let Some((w, h)) = s.split_once('x').or_else(|| s.split_once('X'))
        && let (Ok(wf), Ok(hf)) = (w.trim().parse::<f32>(), h.trim().parse::<f32>())
    {
        return (wf.max(640.0), hf.max(480.0));
    }
    (1440.0, 900.0)
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

fn refresh_workspace_background(
    view: &Entity<LingXiShellView>,
    runtime: &AppRuntime,
    workspace_path: PathBuf,
    cx: &mut gpui_kit::AsyncApp,
) {
    let view_handle = view.clone();
    let wp = workspace_path.clone();
    let handle = runtime.spawn(async move {
        let status: Option<GitStatusSummary> = lingxi_git::git_status(&wp).await.ok();
        let diffs: Vec<GitFileDiff> = lingxi_git::git_diff(&wp).await.unwrap_or_default();
        let prs = lingxi_git::gh_list_pull_requests(&wp)
            .await
            .unwrap_or_default();
        let files = scan_workspace_files(&wp, 400).unwrap_or_default();
        (status, diffs, prs, files)
    });
    cx.spawn(async move |async_cx| {
        if let Ok((status, diffs, prs, files)) = handle.await {
            async_cx.update_entity(&view_handle, |shell, cx| {
                shell.state.git_status = status;
                if shell.state.selected_diff_path.is_none() {
                    shell.state.selected_diff_path = diffs.first().map(|d| d.path.clone());
                }
                shell.state.git_diffs = diffs;
                shell.state.pull_requests = prs;
                shell.state.workspace_files = files;
                cx.notify();
            });
        }
    })
    .detach();
}

#[allow(clippy::too_many_arguments)]
fn handle_ui_action(
    view: &Entity<LingXiShellView>,
    runtime: &AppRuntime,
    orchestrator: &Arc<Orchestrator>,
    terminal_mgr: &Arc<TerminalManager>,
    action: UiAction,
    cx: &mut gpui_kit::AsyncApp,
) {
    let mut trigger_workspace_refresh: Option<PathBuf> = None;

    cx.update_entity(view, |shell, cx| {
        match action {
            UiAction::SelectThread(tid) => {
                shell.state.select_thread(tid.clone());
                if !shell.state.messages_by_thread.contains_key(&tid)
                    && let Ok(msgs) = orchestrator.db().list_messages(&tid)
                {
                    shell.state.messages_by_thread.insert(tid.clone(), msgs);
                }
                if !shell.state.approvals_by_thread.contains_key(&tid)
                    && let Ok(apps) = orchestrator.db().list_pending_approvals(&tid)
                {
                    shell.state.approvals_by_thread.insert(tid.clone(), apps);
                }
                if !shell.state.checkpoints_by_thread.contains_key(&tid)
                    && let Ok(cps) = orchestrator.db().list_checkpoints(&tid)
                {
                    shell.state.checkpoints_by_thread.insert(tid.clone(), cps);
                }
                if let Some(proj) = shell.state.active_project() {
                    trigger_workspace_refresh = Some(PathBuf::from(&proj.workspace_root));
                }
            }
            UiAction::CloseTab(tid) => shell.state.close_tab(&tid),
            UiAction::CreateThread {
                project_id,
                worktree,
            } => {
                let orch = Arc::clone(orchestrator);
                let tid = ThreadId::new();
                let branch = worktree.then(|| format!("lingxi/{}", &tid.0[..8]));
                let provider = shell.state.selected_provider_for_new_thread;
                let title = "新会话".to_string();
                let selection = ModelSelection {
                    provider,
                    model_slug: "default".to_string(),
                    thinking_level: None,
                    planner_model: None,
                    executor_model: None,
                };
                let now = chrono::Utc::now();
                shell.state.threads.insert(
                    0,
                    ThreadRecord {
                        id: tid.clone(),
                        project_id: project_id.clone(),
                        title: title.clone(),
                        model_selection: selection.clone(),
                        runtime_mode: RuntimeMode::ApprovalRequired,
                        status: ThreadRunStatus::Idle,
                        branch: branch.clone(),
                        worktree_path: None,
                        provider_session_id: None,
                        pinned: false,
                        archived: false,
                        unread: false,
                        created_at: now,
                        updated_at: now,
                    },
                );
                shell.state.select_thread(tid.clone());
                let cmd = OrchestrationCommand::CreateThread {
                    command_id: CommandId::new(),
                    thread_id: tid,
                    project_id,
                    title,
                    model_selection: selection,
                    runtime_mode: RuntimeMode::ApprovalRequired,
                    branch,
                    worktree_path: None,
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
            }
            UiAction::SendTurn { thread_id, prompt } => {
                let now = chrono::Utc::now();
                let msg = ChatMessage {
                    id: MessageId::new(),
                    thread_id: thread_id.clone(),
                    turn_id: None,
                    role: MessageRole::User,
                    content: prompt.clone(),
                    reasoning: None,
                    is_streaming: false,
                    tool_calls: Vec::new(),
                    attachments: Vec::new(),
                    created_at: now,
                    updated_at: now,
                };
                shell
                    .state
                    .messages_by_thread
                    .entry(thread_id.clone())
                    .or_default()
                    .push(msg);
                if let Some(t) = shell.state.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.status = ThreadRunStatus::Running;
                }
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
                if let Some(t) = shell.state.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.status = ThreadRunStatus::Idle;
                }
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
            UiAction::StartQuickThread { prompt } => {
                let now = chrono::Utc::now();
                let project_id = if let Some(p) = shell.state.projects.first() {
                    p.id.clone()
                } else {
                    let cwd = std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .to_string_lossy()
                        .to_string();
                    let name = Path::new(&cwd)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Workspace")
                        .to_string();
                    let pid = ProjectId::new();
                    shell.state.projects.push(lingxi_domain::ProjectRecord {
                        id: pid.clone(),
                        title: name.clone(),
                        workspace_root: cwd.clone(),
                        default_model: None,
                        created_at: now,
                        updated_at: now,
                        archived: false,
                    });
                    shell.state.expanded_projects.insert(pid.clone());
                    let orch = Arc::clone(orchestrator);
                    let pid_db = pid.clone();
                    std::mem::drop(runtime.spawn(async move {
                        let _ = orch
                            .dispatch(OrchestrationCommand::CreateProject {
                                command_id: CommandId::new(),
                                project_id: pid_db,
                                title: name,
                                workspace_root: cwd,
                            })
                            .await;
                    }));
                    pid
                };

                let tid = ThreadId::new();
                let provider = shell.state.selected_provider_for_new_thread;
                let selection = ModelSelection {
                    provider,
                    model_slug: "default".to_string(),
                    thinking_level: None,
                    planner_model: None,
                    executor_model: None,
                };
                let title: String = prompt.chars().take(24).collect();
                shell.state.threads.insert(
                    0,
                    ThreadRecord {
                        id: tid.clone(),
                        project_id: project_id.clone(),
                        title: title.clone(),
                        model_selection: selection.clone(),
                        runtime_mode: RuntimeMode::ApprovalRequired,
                        status: ThreadRunStatus::Running,
                        branch: None,
                        worktree_path: None,
                        provider_session_id: None,
                        pinned: false,
                        archived: false,
                        unread: false,
                        created_at: now,
                        updated_at: now,
                    },
                );
                shell
                    .state
                    .messages_by_thread
                    .entry(tid.clone())
                    .or_default()
                    .push(ChatMessage {
                        id: MessageId::new(),
                        thread_id: tid.clone(),
                        turn_id: None,
                        role: MessageRole::User,
                        content: prompt.clone(),
                        reasoning: None,
                        is_streaming: false,
                        tool_calls: Vec::new(),
                        attachments: Vec::new(),
                        created_at: now,
                        updated_at: now,
                    });
                shell.state.select_thread(tid.clone());
                let orch = Arc::clone(orchestrator);
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch
                        .dispatch(OrchestrationCommand::CreateThread {
                            command_id: CommandId::new(),
                            thread_id: tid.clone(),
                            project_id,
                            title,
                            model_selection: selection,
                            runtime_mode: RuntimeMode::ApprovalRequired,
                            branch: None,
                            worktree_path: None,
                        })
                        .await;
                    let _ = orch
                        .dispatch(OrchestrationCommand::StartTurn {
                            command_id: CommandId::new(),
                            thread_id: tid,
                            turn_id: TurnId::new(),
                            message_id: MessageId::new(),
                            prompt,
                            attachments: Vec::new(),
                        })
                        .await;
                }));
            }
            UiAction::CompactContext(thread_id) => {
                let orch = Arc::clone(orchestrator);
                let cmd = OrchestrationCommand::CompactContext {
                    command_id: CommandId::new(),
                    thread_id,
                    summary_markdown: "用户请求手动压缩上下文。".to_string(),
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
                shell.state.status_banner = Some("已触发上下文压缩任务".to_string());
            }
            UiAction::ResolveApproval {
                thread_id,
                approval_id,
                decision,
            } => {
                if let Some(list) = shell.state.approvals_by_thread.get_mut(&thread_id) {
                    list.retain(|a| a.id != approval_id);
                }
                let orch = Arc::clone(orchestrator);
                let cmd = OrchestrationCommand::ResolvePermission {
                    command_id: CommandId::new(),
                    approval_id,
                    thread_id,
                    decision,
                    reason: None,
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
            }
            UiAction::RevertCheckpoint {
                thread_id,
                checkpoint_id,
            } => {
                let orch = Arc::clone(orchestrator);
                let cmd = OrchestrationCommand::RevertToCheckpoint {
                    command_id: CommandId::new(),
                    thread_id,
                    checkpoint_id,
                };
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch.dispatch(cmd).await;
                }));
            }
            UiAction::SelectModel {
                thread_id,
                selection,
            } => {
                if let Some(t) = shell.state.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.model_selection = selection;
                }
            }
            UiAction::SelectRuntimeMode { thread_id, mode } => {
                if let Some(t) = shell.state.threads.iter_mut().find(|t| t.id == thread_id) {
                    t.runtime_mode = mode;
                }
            }
            UiAction::OpenWorkspaceDialog => {
                let fallback_path = std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .to_string_lossy()
                    .to_string();
                let picked = pick_folder_macos().unwrap_or(fallback_path);
                let name = Path::new(&picked)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Workspace")
                    .to_string();
                let pid = ProjectId::new();
                let now = chrono::Utc::now();
                shell.state.projects.push(lingxi_domain::ProjectRecord {
                    id: pid.clone(),
                    title: name.clone(),
                    workspace_root: picked.clone(),
                    default_model: None,
                    created_at: now,
                    updated_at: now,
                    archived: false,
                });
                shell.state.expanded_projects.insert(pid.clone());
                let orch = Arc::clone(orchestrator);
                let root_for_db = picked.clone();
                std::mem::drop(runtime.spawn(async move {
                    let _ = orch
                        .dispatch(OrchestrationCommand::CreateProject {
                            command_id: CommandId::new(),
                            project_id: pid,
                            title: name,
                            workspace_root: root_for_db,
                        })
                        .await;
                }));
                trigger_workspace_refresh = Some(PathBuf::from(picked));
            }
            UiAction::RunLegacyImport => {
                if let Some(source) = AppPaths::discover_legacy_database() {
                    let db = orchestrator.db().clone();
                    let orch_for_import = Arc::clone(orchestrator);
                    let backups = runtime.paths.backups_dir.clone();
                    let view_handle = view.clone();
                    let handle = runtime
                        .spawn(async move { import_legacy_readonly(&db, &source, &backups) });
                    cx.spawn(async move |_weak, async_cx| {
                        if let Ok(Ok(report)) = handle.await {
                            let summary = format!(
                                "只读导入完成：{} 工作区 · {} 会话 · {} 消息",
                                report.projects_imported,
                                report.threads_imported,
                                report.messages_imported
                            );
                            async_cx.update_entity(&view_handle, |shell, cx| {
                                shell.state.last_import_summary = Some(summary);
                                if let Ok(projects) = orch_for_import.db().list_projects() {
                                    shell.state.projects = projects;
                                }
                                if let Ok(threads) = orch_for_import.db().list_threads() {
                                    shell.state.threads = threads;
                                }
                                cx.notify();
                            });
                        }
                    })
                    .detach();
                }
            }
            UiAction::ToggleProjectExpanded(pid) => {
                if !shell.state.expanded_projects.remove(&pid) {
                    shell.state.expanded_projects.insert(pid);
                }
            }
            UiAction::ToggleToolCallExpanded(key) => {
                if !shell.state.expanded_tool_calls.remove(&key) {
                    shell.state.expanded_tool_calls.insert(key);
                }
            }
            UiAction::ToggleSidebar => {
                shell.state.sidebar_collapsed = !shell.state.sidebar_collapsed;
            }
            UiAction::SetSidebarFilter(q) => {
                shell.state.sidebar_filter = q;
            }
            UiAction::ToggleInspector(tab) => {
                if shell.state.inspector_open && shell.state.inspector_tab == tab {
                    shell.state.inspector_open = false;
                } else {
                    shell.state.inspector_open = true;
                    shell.state.inspector_tab = tab;
                    if tab == InspectorTab::Terminal
                        && shell.state.terminal_tabs.is_empty()
                        && let Some(cwd) = shell
                            .state
                            .active_project()
                            .map(|p| PathBuf::from(&p.workspace_root))
                            .or_else(|| std::env::current_dir().ok())
                        && let Ok(tid) = terminal_mgr.spawn_shell(&cwd, 100, 30)
                    {
                        let id_str = tid.0.clone();
                        shell.state.terminal_tabs.push(TerminalTabView {
                            id: id_str.clone(),
                            title: "zsh #1".to_string(),
                            cwd: cwd.display().to_string(),
                            lines: Vec::new(),
                            running: true,
                        });
                        shell.state.active_terminal_id = Some(id_str);
                    }
                    if let Some(proj) = shell.state.active_project() {
                        trigger_workspace_refresh = Some(PathBuf::from(&proj.workspace_root));
                    }
                }
            }
            UiAction::CloseInspector => {
                shell.state.inspector_open = false;
            }
            UiAction::ToggleAppearance => {
                let next = toggle_appearance_mode(shell.state.settings.appearance);
                shell.state.settings.appearance = next;
                apply_appearance(cx, next);
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
            }
            UiAction::SetAppearance(next) => {
                shell.state.settings.appearance = next;
                apply_appearance(cx, next);
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
            }
            UiAction::OpenSettings(section) => {
                shell.state.settings_section = section;
                shell.state.open_modal(ActiveModal::Settings);
            }
            UiAction::OpenCommandPalette => {
                shell.state.open_modal(ActiveModal::CommandPalette);
            }
            UiAction::OpenUpdateModal => {
                shell.state.open_modal(ActiveModal::Update);
            }
            UiAction::OpenApprovalModal(aid) => {
                shell.state.active_approval_id = Some(aid);
                shell.state.open_modal(ActiveModal::PermissionApproval);
            }
            UiAction::CloseModal => {
                shell.state.close_modal();
            }
            UiAction::SelectDiffFile(path) => {
                shell.state.selected_diff_path = Some(path);
                shell.state.inspector_open = true;
                shell.state.inspector_tab = InspectorTab::Diff;
            }
            UiAction::OpenFilePreview(rel) => {
                if let Some(proj) = shell.state.active_project()
                    && let Ok(content) = read_file_bounded(Path::new(&proj.workspace_root), &rel)
                {
                    shell.state.file_preview_path = Some(rel);
                    shell.state.file_preview_content = Some(content);
                }
            }
            UiAction::RefreshGitAndWorkspace => {
                if let Some(proj) = shell.state.active_project() {
                    trigger_workspace_refresh = Some(PathBuf::from(&proj.workspace_root));
                }
                shell.state.resource_usage = sample_process_resources();
            }
            UiAction::SpawnTerminal => {
                let cwd = shell
                    .state
                    .active_project()
                    .map(|p| PathBuf::from(&p.workspace_root))
                    .unwrap_or_else(|| {
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    });
                if let Ok(tid) = terminal_mgr.spawn_shell(&cwd, 100, 30) {
                    let idx = shell.state.terminal_tabs.len() + 1;
                    let id_str = tid.0.clone();
                    shell.state.terminal_tabs.push(TerminalTabView {
                        id: id_str.clone(),
                        title: format!("zsh #{idx}"),
                        cwd: cwd.display().to_string(),
                        lines: Vec::new(),
                        running: true,
                    });
                    shell.state.active_terminal_id = Some(id_str);
                }
            }
            UiAction::SelectTerminalTab(id) => {
                shell.state.active_terminal_id = Some(id);
            }
            UiAction::SendTerminalInput {
                terminal_id,
                command,
            } => {
                let tid = TerminalId::from_string(terminal_id.clone());
                let payload = format!("{command}\n");
                let _ = terminal_mgr.write_input(&tid, payload.as_bytes());
                if let Some(tab) = shell
                    .state
                    .terminal_tabs
                    .iter_mut()
                    .find(|t| t.id == terminal_id)
                {
                    tab.lines.push(format!("$ {command}"));
                }
                let mgr = Arc::clone(terminal_mgr);
                let view_handle = view.clone();
                let tid_bg = tid;
                let term_id_bg = terminal_id;
                let handle = runtime.spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    mgr.snapshot_lines(&tid_bg).unwrap_or_default()
                });
                cx.spawn(async move |_weak, async_cx| {
                    if let Ok(lines) = handle.await {
                        async_cx.update_entity(&view_handle, |shell, cx| {
                            if let Some(tab) = shell
                                .state
                                .terminal_tabs
                                .iter_mut()
                                .find(|t| t.id == term_id_bg)
                                && !lines.is_empty()
                            {
                                tab.lines = lines;
                            }
                            cx.notify();
                        });
                    }
                })
                .detach();
            }
            UiAction::SetUpdateChannel(ch) => {
                shell.state.settings.update_channel = ch;
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
            }
            UiAction::SaveCustomAcpCommand(cmd) => {
                shell.state.settings.custom_acp_command = cmd;
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
                shell.state.status_banner = Some("已保存自定义 ACP 启动命令".to_string());
            }
            UiAction::SetRemotePort(port) => {
                shell.state.settings.remote_server_port = port;
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
                shell.state.status_banner = Some(format!("已更新远程端口为 {port}"));
            }
            UiAction::ToggleRemoteServer => {
                shell.state.remote_server_running = !shell.state.remote_server_running;
                if shell.state.remote_server_running {
                    let orch = Arc::clone(orchestrator);
                    let port = shell.state.settings.remote_server_port;
                    std::mem::drop(runtime.spawn(async move {
                        let _ = lingxi_remote::serve_on_port(orch, port).await;
                    }));
                }
            }
            UiAction::CheckUpdates => {
                shell.state.update_checking = true;
                let channel = shell.state.settings.update_channel;
                let view_handle = view.clone();
                let handle = runtime.spawn(async move {
                    lingxi_updater::check_for_updates(
                        lingxi_updater::DEFAULT_GITHUB_RELEASES_URL,
                        env!("CARGO_PKG_VERSION"),
                        channel,
                    )
                    .await
                });
                cx.spawn(async move |_weak, async_cx| {
                    let res = handle.await;
                    async_cx.update_entity(&view_handle, |shell, cx| {
                        shell.state.update_checking = false;
                        if let Ok(Ok(update)) = res {
                            shell.state.available_update = update;
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            UiAction::SwitchProviderFilter(p) => {
                shell.state.selected_provider_for_new_thread = p;
            }
        }
        cx.notify();
    });

    if let Some(wp) = trigger_workspace_refresh {
        refresh_workspace_background(view, runtime, wp, cx);
    }
}

#[cfg(target_os = "macos")]
fn pick_folder_macos() -> Option<String> {
    let out = std::process::Command::new("osascript")
        .arg("-e")
        .arg("POSIX path of (choose folder with prompt \"选择工作区项目目录\")")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!path.is_empty()).then_some(path)
}

#[cfg(not(target_os = "macos"))]
fn pick_folder_macos() -> Option<String> {
    None
}

fn seed_qa_workspace(
    orchestrator: &Orchestrator,
    rt: &tokio::runtime::Runtime,
) -> anyhow::Result<()> {
    if !orchestrator.db().list_projects()?.is_empty() {
        return Ok(());
    }
    let repo_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .to_string_lossy()
        .to_string();
    let p1 = ProjectId::from_string("qa-proj-lingxi");
    let p2 = ProjectId::from_string("qa-proj-docs");
    let t1 = ThreadId::from_string("qa-thread-ui");
    let t2 = ThreadId::from_string("qa-thread-acp");
    let t3 = ThreadId::from_string("qa-thread-docs");

    rt.block_on(async {
        let _ = orchestrator
            .dispatch(OrchestrationCommand::CreateProject {
                command_id: CommandId::new(),
                project_id: p1.clone(),
                title: "LingXiT3-Native".to_string(),
                workspace_root: repo_dir.clone(),
            })
            .await;
        let _ = orchestrator
            .dispatch(OrchestrationCommand::CreateProject {
                command_id: CommandId::new(),
                project_id: p2.clone(),
                title: "LingXi-Docs".to_string(),
                workspace_root: repo_dir,
            })
            .await;

        let _ = orchestrator
            .dispatch(OrchestrationCommand::CreateThread {
                command_id: CommandId::new(),
                thread_id: t1.clone(),
                project_id: p1.clone(),
                title: "重构 GPUI 原生三栏界面与组件复用".to_string(),
                model_selection: ModelSelection {
                    provider: ProviderKind::Codex,
                    model_slug: "gpt-5-codex".to_string(),
                    thinking_level: Some("high".to_string()),
                    planner_model: None,
                    executor_model: None,
                },
                runtime_mode: RuntimeMode::ApprovalRequired,
                branch: Some("feat/ui-rework".to_string()),
                worktree_path: None,
            })
            .await;

        let _ = orchestrator
            .dispatch(OrchestrationCommand::CreateThread {
                command_id: CommandId::new(),
                thread_id: t2,
                project_id: p1,
                title: "ACP JSON-RPC 2.0 流式增量解析联调".to_string(),
                model_selection: ModelSelection {
                    provider: ProviderKind::Claude,
                    model_slug: "claude-sonnet-4-5".to_string(),
                    thinking_level: None,
                    planner_model: None,
                    executor_model: None,
                },
                runtime_mode: RuntimeMode::FullAccess,
                branch: Some("main".to_string()),
                worktree_path: None,
            })
            .await;

        let _ = orchestrator
            .dispatch(OrchestrationCommand::CreateThread {
                command_id: CommandId::new(),
                thread_id: t3,
                project_id: p2,
                title: "整理多平台额度监控与快捷键说明".to_string(),
                model_selection: ModelSelection {
                    provider: ProviderKind::Antigravity,
                    model_slug: "gemini-3.1-pro".to_string(),
                    thinking_level: None,
                    planner_model: None,
                    executor_model: None,
                },
                runtime_mode: RuntimeMode::Auto,
                branch: None,
                worktree_path: None,
            })
            .await;

        let _ = orchestrator
            .dispatch(OrchestrationCommand::StartTurn {
                command_id: CommandId::new(),
                thread_id: t1,
                turn_id: TurnId::new(),
                message_id: MessageId::new(),
                prompt: "请按照 gpuicn 官方 Coss UI 规范检查 @crates/ui/src/shell.rs 的三栏布局与弹窗层级。".to_string(),
                attachments: Vec::new(),
            })
            .await;
    });

    Ok(())
}

fn enrich_qa_ui_state(ui_state: &mut AppUiState) {
    let tid = ThreadId::from_string("qa-thread-ui");
    if ui_state.threads.iter().any(|t| t.id == tid) {
        ui_state.select_thread(tid.clone());
        let now = chrono::Utc::now();
        ui_state.messages_by_thread.insert(
            tid.clone(),
            vec![
                ChatMessage {
                    id: MessageId::new(),
                    thread_id: tid.clone(),
                    turn_id: None,
                    role: MessageRole::User,
                    content: "请按照 gpuicn 官方 Coss UI 规范检查 `crates/ui/src/shell.rs` 的三栏布局与弹窗层级，确保右侧面板默认收起、弹窗互斥不重叠。".to_string(),
                    reasoning: None,
                    is_streaming: false,
                    tool_calls: Vec::new(),
                    attachments: Vec::new(),
                    created_at: now,
                    updated_at: now,
                },
                ChatMessage {
                    id: MessageId::new(),
                    thread_id: tid.clone(),
                    turn_id: None,
                    role: MessageRole::Assistant,
                    content: "已完成界面外壳与原生 `gpuicn` 组件映射重构：\n\n1. **窗口与三栏骨架**：采用 `TitleBar`、`Sidebar` 与 `Resizable` 搭建左中右结构，右侧工具面板默认收起。\n2. **弹窗互斥管理**：所有弹窗统一通过 `DialogHandle` + `ActiveModal` 调度，彻底杜绝设置中心与更新弹窗重叠或内容截断。\n\n```rust\npub fn sync(&self, active: ActiveModal, window: &mut Window, cx: &mut App) {\n    // Close inactive modal handles before opening the active one\n}\n```".to_string(),
                    reasoning: None,
                    is_streaming: false,
                    tool_calls: vec![ToolCallRecord {
                        id: lingxi_domain::ToolCallId::from_string("call_qa_1"),
                        name: "read_file".to_string(),
                        input_preview: "crates/ui/src/shell.rs (L1..L240)".to_string(),
                        output_preview: Some("Verified TitleBar, Sidebar, Composer, DiffView, and DialogHandle integration.".to_string()),
                        status: lingxi_domain::ToolCallStatus::Completed,
                        duration_ms: Some(18),
                    }],
                    attachments: Vec::new(),
                    created_at: now,
                    updated_at: now,
                },
            ],
        );
        ui_state.approvals_by_thread.insert(
            tid.clone(),
            vec![PendingApproval {
                id: ApprovalId::from_string("qa-approval-1"),
                thread_id: tid.clone(),
                turn_id: TurnId::from_string("qa-turn-1"),
                tool_name: "run_command".to_string(),
                summary: "cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings".to_string(),
                raw_input: serde_json::json!({
                    "command": "cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings"
                }),
                created_at: now,
            }],
        );
        ui_state.context_by_thread.insert(
            tid,
            ContextBreakdown {
                system_tokens: 4_200,
                messages_tokens: 18_600,
                tools_tokens: 6_400,
                cache_read_tokens: 14_800,
                cache_write_tokens: 1_200,
                context_window_limit: 200_000,
            },
        );
    }
}

fn apply_open_view(ui_state: &mut AppUiState, open_view: &str) {
    match open_view {
        "welcome" => {
            ui_state.active_thread_id = None;
            ui_state.open_tabs.clear();
            ui_state.inspector_open = false;
            ui_state.close_modal();
        }
        "chat" => {
            ui_state.inspector_open = false;
            ui_state.close_modal();
        }
        "inspector-diff" => {
            ui_state.inspector_open = true;
            ui_state.inspector_tab = InspectorTab::Diff;
        }
        "inspector-terminal" => {
            ui_state.inspector_open = true;
            ui_state.inspector_tab = InspectorTab::Terminal;
            if ui_state.terminal_tabs.is_empty() {
                ui_state.terminal_tabs.push(TerminalTabView {
                    id: "qa-term-1".to_string(),
                    title: "zsh #1".to_string(),
                    cwd: std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .display()
                        .to_string(),
                    lines: vec![
                        "$ cargo check -p lingxi-ui".to_string(),
                        "Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.41s"
                            .to_string(),
                    ],
                    running: true,
                });
                ui_state.active_terminal_id = Some("qa-term-1".to_string());
            }
        }
        "inspector-git" => {
            ui_state.inspector_open = true;
            ui_state.inspector_tab = InspectorTab::GitPr;
        }
        "inspector-files" => {
            ui_state.inspector_open = true;
            ui_state.inspector_tab = InspectorTab::Files;
        }
        "inspector-metrics" => {
            ui_state.inspector_open = true;
            ui_state.inspector_tab = InspectorTab::Metrics;
        }
        "command" => ui_state.open_modal(ActiveModal::CommandPalette),
        "update" => ui_state.open_modal(ActiveModal::Update),
        "approval" => {
            ui_state.active_approval_id = Some(ApprovalId::from_string("qa-approval-1"));
            ui_state.open_modal(ActiveModal::PermissionApproval);
        }
        "settings-general" => {
            ui_state.settings_section = SettingsSection::General;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-appearance" => {
            ui_state.settings_section = SettingsSection::Appearance;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-providers" => {
            ui_state.settings_section = SettingsSection::Providers;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-keybindings" => {
            ui_state.settings_section = SettingsSection::Keybindings;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-mcp" => {
            ui_state.settings_section = SettingsSection::McpSkills;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-remote" => {
            ui_state.settings_section = SettingsSection::Remote;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-import" => {
            ui_state.settings_section = SettingsSection::LegacyImport;
            ui_state.open_modal(ActiveModal::Settings);
        }
        "settings-about" => {
            ui_state.settings_section = SettingsSection::About;
            ui_state.open_modal(ActiveModal::Settings);
        }
        _ => {}
    }
}

fn discover_all_local_skills() -> Vec<DiscoveredSkillView> {
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for root in [
            home.join(".codex/skills"),
            home.join(".claude/skills"),
            home.join(".gemini/antigravity-cli/skills"),
        ] {
            for s in discover_skills(&root) {
                if !out
                    .iter()
                    .any(|existing: &DiscoveredSkillView| existing.name == s.name)
                {
                    out.push(DiscoveredSkillView {
                        name: s.name,
                        description: s.description,
                        path: s.path,
                    });
                }
            }
        }
    }
    out
}

fn main() -> anyhow::Result<()> {
    init_logging();
    let cli = Cli::parse();

    let paths = Arc::new(AppPaths::resolve(cli.data_dir.as_deref()));
    paths.ensure_dirs()?;
    if !paths.settings_path.exists() {
        save_settings(&paths.settings_path, &lingxi_domain::AppSettings::default())?;
    }
    let mut settings = load_settings(&paths.settings_path)?;
    if let Some(theme_arg) = cli.theme.as_deref() {
        settings.appearance = match theme_arg.to_lowercase().as_str() {
            "light" => ThemeAppearance::Light,
            "dark" => ThemeAppearance::Dark,
            _ => settings.appearance,
        };
    }

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
    let terminal_mgr = Arc::new(TerminalManager::new());
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

    if cli.qa_seed && cli.data_dir.is_some() {
        seed_qa_workspace(&orchestrator, &rt)?;
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
        ui_state
            .approvals_by_thread
            .insert(tid.clone(), db.list_pending_approvals(&tid)?);
        ui_state
            .checkpoints_by_thread
            .insert(tid, db.list_checkpoints(&ui_state.threads[0].id)?);
    }
    if cli.qa_seed && cli.data_dir.is_some() {
        enrich_qa_ui_state(&mut ui_state);
    }
    ui_state.legacy_db_path = AppPaths::discover_legacy_database().map(|p| p.display().to_string());
    ui_state.discovered_skills = discover_all_local_skills();
    ui_state.resource_usage = sample_process_resources();
    ui_state.quota_snapshots = build_quota_snapshots(&[]);
    if let Some(view_name) = cli.open_view.as_deref() {
        apply_open_view(&mut ui_state, view_name);
    }

    let (win_w, win_h) = parse_window_size(cli.window_size.as_deref());
    let runtime_for_ui = app_runtime.clone();
    let orch_for_ui = Arc::clone(&orchestrator);
    let term_for_ui = Arc::clone(&terminal_mgr);
    let registry_for_bg = Arc::clone(&provider_registry);
    let custom_acp_for_bg = settings.custom_acp_command.clone();
    let initial_workspace = ui_state
        .active_project()
        .map(|p| PathBuf::from(&p.workspace_root));

    let window_opts = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: gpui_kit::point(px(60.0), px(40.0)),
            size: size(px(win_w), px(win_h)),
        })),
        ..title_bar_window_options()
    };

    gpuicn::app::run(
        AppConfig::new(APP_DISPLAY_NAME)
            .fonts(lingxi_ui::fonts::load_app_fonts())
            .themes(lingxi_light_theme(), lingxi_dark_theme())
            .appearance(to_gpuicn_appearance(settings.appearance))
            .window_options(window_opts),
        move |window: &mut Window, cx| {
            window.set_window_title(APP_DISPLAY_NAME);
            window.resize(size(px(win_w), px(win_h)));
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
            let term_for_loop = Arc::clone(&term_for_ui);
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

            if let Some(wp) = initial_workspace {
                let entity_for_ws = entity.clone();
                let rt_ws = runtime_for_ui.clone();
                cx.spawn(async move |_weak, cx| {
                    refresh_workspace_background(&entity_for_ws, &rt_ws, wp, cx);
                })
                .detach();
            }

            cx.spawn(async move |_weak, cx| {
                while let Some(action) = action_rx.recv().await {
                    handle_ui_action(
                        &entity,
                        &runtime_for_loop,
                        &orch_for_loop,
                        &term_for_loop,
                        action,
                        cx,
                    );
                }
            })
            .detach();
            LingXiShellView::new(ui_state, dispatcher, window, cx)
        },
    );

    Ok(())
}
