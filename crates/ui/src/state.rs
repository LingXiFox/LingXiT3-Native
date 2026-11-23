use std::collections::{HashMap, HashSet};

use lingxi_domain::{
    AppSettings, ApprovalId, AttachmentRef, ChatMessage, CheckpointId, CheckpointRecord,
    ContextBreakdown, GitFileDiff, GitStatusSummary, ModelSelection, PendingApproval,
    PermissionDecision, ProjectId, ProjectRecord, ProviderKind, ProviderQuotaSnapshot,
    ProviderStatus, PullRequestSummary, ResourceUsageSnapshot, RuntimeMode, ThemeAppearance,
    ThreadId, ThreadRecord, ThreadRunStatus, UpdateChannel, UpdateReleaseInfo,
};

use crate::theme::tokens::{INSPECTOR_DEFAULT_WIDTH, SIDEBAR_DEFAULT_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Diff,
    Terminal,
    GitPr,
    Files,
    Metrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSection {
    General,
    Appearance,
    Providers,
    Keybindings,
    McpSkills,
    Remote,
    LegacyImport,
    About,
}

impl SettingsSection {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "常规",
            Self::Appearance => "外观与主题",
            Self::Providers => "智能体 Provider",
            Self::Keybindings => "快捷键",
            Self::McpSkills => "MCP 与 Skills",
            Self::Remote => "远程与配对",
            Self::LegacyImport => "数据迁移与维护",
            Self::About => "关于与更新",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::Providers => "providers",
            Self::Keybindings => "keybindings",
            Self::McpSkills => "mcp",
            Self::Remote => "remote",
            Self::LegacyImport => "import",
            Self::About => "about",
        }
    }
}

/// Mutually exclusive modal state — only one modal can be open at any time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveModal {
    None,
    CommandPalette,
    Settings,
    Update,
    PermissionApproval,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalTabView {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub lines: Vec<String>,
    pub running: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSkillView {
    pub name: String,
    pub description: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub group: String,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AppUiState {
    pub settings: AppSettings,
    pub projects: Vec<ProjectRecord>,
    pub threads: Vec<ThreadRecord>,
    pub messages_by_thread: HashMap<ThreadId, Vec<ChatMessage>>,
    pub approvals_by_thread: HashMap<ThreadId, Vec<PendingApproval>>,
    pub checkpoints_by_thread: HashMap<ThreadId, Vec<CheckpointRecord>>,
    pub context_by_thread: HashMap<ThreadId, ContextBreakdown>,
    pub provider_statuses: Vec<ProviderStatus>,
    pub quota_snapshots: Vec<ProviderQuotaSnapshot>,
    pub resource_usage: ResourceUsageSnapshot,

    pub expanded_projects: HashSet<ProjectId>,
    pub expanded_tool_calls: HashSet<String>,
    pub open_tabs: Vec<ThreadId>,
    pub active_thread_id: Option<ThreadId>,
    pub sidebar_collapsed: bool,
    pub sidebar_width: f32,
    pub sidebar_filter: String,
    pub show_archived: bool,

    pub inspector_open: bool,
    pub inspector_tab: InspectorTab,
    pub inspector_width: f32,

    pub composer_drafts: HashMap<ThreadId, String>,
    pub composer_attachments: Vec<AttachmentRef>,
    pub mention_query: Option<String>,
    pub workspace_files: Vec<String>,
    pub ime_composing: bool,
    pub selected_provider_for_new_thread: ProviderKind,

    pub git_status: Option<GitStatusSummary>,
    pub git_diffs: Vec<GitFileDiff>,
    pub selected_diff_path: Option<String>,
    pub file_preview_path: Option<String>,
    pub file_preview_content: Option<String>,
    pub pull_requests: Vec<PullRequestSummary>,

    pub terminal_tabs: Vec<TerminalTabView>,
    pub active_terminal_id: Option<String>,

    pub active_modal: ActiveModal,
    pub active_approval_id: Option<ApprovalId>,
    pub command_palette_query: String,
    pub settings_section: SettingsSection,
    pub available_update: Option<UpdateReleaseInfo>,
    pub update_download_progress: Option<f32>,
    pub update_checking: bool,

    pub discovered_skills: Vec<DiscoveredSkillView>,
    pub mcp_server_running: bool,
    pub remote_server_running: bool,
    pub legacy_db_path: Option<String>,
    pub last_import_summary: Option<String>,
    pub status_banner: Option<String>,
}

impl AppUiState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings,
            projects: Vec::new(),
            threads: Vec::new(),
            messages_by_thread: HashMap::new(),
            approvals_by_thread: HashMap::new(),
            checkpoints_by_thread: HashMap::new(),
            context_by_thread: HashMap::new(),
            provider_statuses: Vec::new(),
            quota_snapshots: Vec::new(),
            resource_usage: ResourceUsageSnapshot::default(),
            expanded_projects: HashSet::new(),
            expanded_tool_calls: HashSet::new(),
            open_tabs: Vec::new(),
            active_thread_id: None,
            sidebar_collapsed: false,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            sidebar_filter: String::new(),
            show_archived: false,
            // Right auxiliary tool inspector is collapsed by default per Coding Harness spec.
            inspector_open: false,
            inspector_tab: InspectorTab::Diff,
            inspector_width: INSPECTOR_DEFAULT_WIDTH,
            composer_drafts: HashMap::new(),
            composer_attachments: Vec::new(),
            mention_query: None,
            workspace_files: Vec::new(),
            ime_composing: false,
            selected_provider_for_new_thread: ProviderKind::Codex,
            git_status: None,
            git_diffs: Vec::new(),
            selected_diff_path: None,
            file_preview_path: None,
            file_preview_content: None,
            pull_requests: Vec::new(),
            terminal_tabs: Vec::new(),
            active_terminal_id: None,
            active_modal: ActiveModal::None,
            active_approval_id: None,
            command_palette_query: String::new(),
            settings_section: SettingsSection::General,
            available_update: None,
            update_download_progress: None,
            update_checking: false,
            discovered_skills: Vec::new(),
            mcp_server_running: true,
            remote_server_running: false,
            legacy_db_path: None,
            last_import_summary: None,
            status_banner: None,
        }
    }

    pub fn open_modal(&mut self, modal: ActiveModal) {
        self.active_modal = modal;
    }

    pub fn close_modal(&mut self) {
        self.active_modal = ActiveModal::None;
    }

    pub fn active_project(&self) -> Option<&ProjectRecord> {
        if let Some(thread) = self.active_thread() {
            return self.projects.iter().find(|p| p.id == thread.project_id);
        }
        self.projects.first()
    }

    pub fn active_thread(&self) -> Option<&ThreadRecord> {
        let id = self.active_thread_id.as_ref()?;
        self.threads.iter().find(|t| &t.id == id)
    }

    pub fn active_messages(&self) -> &[ChatMessage] {
        self.active_thread_id
            .as_ref()
            .and_then(|id| self.messages_by_thread.get(id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn active_approvals(&self) -> &[PendingApproval] {
        self.active_thread_id
            .as_ref()
            .and_then(|id| self.approvals_by_thread.get(id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn current_modal_approval(&self) -> Option<&PendingApproval> {
        let approvals = self.active_approvals();
        if let Some(id) = &self.active_approval_id
            && let Some(found) = approvals.iter().find(|a| &a.id == id)
        {
            return Some(found);
        }
        approvals.first()
    }

    pub fn active_checkpoints(&self) -> &[CheckpointRecord] {
        self.active_thread_id
            .as_ref()
            .and_then(|id| self.checkpoints_by_thread.get(id))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn active_context(&self) -> Option<&ContextBreakdown> {
        self.active_thread_id
            .as_ref()
            .and_then(|id| self.context_by_thread.get(id))
    }

    pub fn active_draft(&self) -> &str {
        self.active_thread_id
            .as_ref()
            .and_then(|id| self.composer_drafts.get(id))
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn set_active_draft(&mut self, text: String) {
        self.mention_query = extract_active_mention(&text);
        if let Some(id) = &self.active_thread_id {
            self.composer_drafts.insert(id.clone(), text);
        }
    }

    pub fn select_thread(&mut self, thread_id: ThreadId) {
        if !self.open_tabs.contains(&thread_id) {
            self.open_tabs.push(thread_id.clone());
        }
        self.active_thread_id = Some(thread_id.clone());
        if let Some(thread) = self.threads.iter_mut().find(|t| t.id == thread_id) {
            thread.unread = false;
        }
    }

    pub fn close_tab(&mut self, thread_id: &ThreadId) {
        if let Some(pos) = self.open_tabs.iter().position(|id| id == thread_id) {
            self.open_tabs.remove(pos);
            if self.active_thread_id.as_ref() == Some(thread_id) {
                self.active_thread_id = if self.open_tabs.is_empty() {
                    None
                } else {
                    let next_idx = pos.min(self.open_tabs.len() - 1);
                    Some(self.open_tabs[next_idx].clone())
                };
            }
        }
    }

    pub fn filtered_threads_for_project(&self, project_id: &ProjectId) -> Vec<&ThreadRecord> {
        let query = self.sidebar_filter.trim().to_lowercase();
        let mut list: Vec<&ThreadRecord> = self
            .threads
            .iter()
            .filter(|t| &t.project_id == project_id)
            .filter(|t| self.show_archived || !t.archived)
            .filter(|t| {
                query.is_empty()
                    || t.title.to_lowercase().contains(&query)
                    || t.branch
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&query)
            })
            .collect();
        list.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        list
    }

    pub fn filtered_mentions(&self) -> Vec<&str> {
        let Some(query) = &self.mention_query else {
            return Vec::new();
        };
        let q = query.to_lowercase();
        self.workspace_files
            .iter()
            .map(String::as_str)
            .filter(|path| q.is_empty() || path.to_lowercase().contains(&q))
            .take(12)
            .collect()
    }

    pub fn command_palette_items(&self) -> Vec<CommandPaletteItem> {
        let mut items = vec![
            CommandPaletteItem {
                id: "cmd:new_thread".into(),
                title: "新建会话".into(),
                subtitle: "在当前工作区创建新的 Agent 会话".into(),
                group: "快捷命令".into(),
                shortcut: Some("secondary-n".into()),
            },
            CommandPaletteItem {
                id: "cmd:open_project".into(),
                title: "打开本地工作区".into(),
                subtitle: "添加项目目录到工作区树".into(),
                group: "快捷命令".into(),
                shortcut: None,
            },
            CommandPaletteItem {
                id: "cmd:toggle_theme".into(),
                title: "切换浅色 / 深色主题".into(),
                subtitle: "在奶白浅色与炭黑深色之间切换".into(),
                group: "快捷命令".into(),
                shortcut: None,
            },
            CommandPaletteItem {
                id: "cmd:toggle_sidebar".into(),
                title: "展开 / 收起侧边栏".into(),
                subtitle: "切换工作区与会话树可见性".into(),
                group: "快捷命令".into(),
                shortcut: Some("secondary-b".into()),
            },
            CommandPaletteItem {
                id: "cmd:toggle_diff".into(),
                title: "打开 Diff 检查器".into(),
                subtitle: "查看当前工作区变更差异".into(),
                group: "工作区工具".into(),
                shortcut: None,
            },
            CommandPaletteItem {
                id: "cmd:toggle_terminal".into(),
                title: "切换终端面板".into(),
                subtitle: "打开或隐藏内嵌 PTY 终端".into(),
                group: "工作区工具".into(),
                shortcut: Some("secondary-j".into()),
            },
            CommandPaletteItem {
                id: "cmd:toggle_metrics".into(),
                title: "查看额度与资源监控".into(),
                subtitle: "查看上下文占用、CPU/内存及多平台配额".into(),
                group: "工作区工具".into(),
                shortcut: None,
            },
            CommandPaletteItem {
                id: "cmd:open_settings".into(),
                title: "打开设置中心".into(),
                subtitle: "配置 Provider、外观、快捷键、MCP、远程与数据迁移".into(),
                group: "设置与维护".into(),
                shortcut: Some("secondary-,".into()),
            },
            CommandPaletteItem {
                id: "cmd:open_updates".into(),
                title: "检查应用更新".into(),
                subtitle: "查看 Stable / Nightly 版本更新状态".into(),
                group: "设置与维护".into(),
                shortcut: None,
            },
        ];

        for thread in &self.threads {
            items.push(CommandPaletteItem {
                id: format!("thread:{}", thread.id),
                title: thread.title.clone(),
                subtitle: format!(
                    "{} · {}",
                    thread.model_selection.provider.display_name(),
                    thread.model_selection.model_slug
                ),
                group: "会话列表".into(),
                shortcut: None,
            });
        }

        let q = self.command_palette_query.trim().to_lowercase();
        if q.is_empty() {
            return items;
        }
        items
            .into_iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&q) || item.subtitle.to_lowercase().contains(&q)
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum UiAction {
    SelectThread(ThreadId),
    CloseTab(ThreadId),
    CreateThread {
        project_id: ProjectId,
        worktree: bool,
    },
    ToggleProjectExpanded(ProjectId),
    ToggleToolCallExpanded(String),
    ToggleSidebar,
    SetSidebarFilter(String),
    ToggleInspector(InspectorTab),
    CloseInspector,
    ToggleAppearance,
    SetAppearance(ThemeAppearance),
    OpenSettings(SettingsSection),
    OpenCommandPalette,
    OpenUpdateModal,
    OpenApprovalModal(ApprovalId),
    CloseModal,
    SendTurn {
        thread_id: ThreadId,
        prompt: String,
    },
    CancelTurn(ThreadId),
    CompactContext(ThreadId),
    ResolveApproval {
        thread_id: ThreadId,
        approval_id: ApprovalId,
        decision: PermissionDecision,
    },
    RevertCheckpoint {
        thread_id: ThreadId,
        checkpoint_id: CheckpointId,
    },
    SelectModel {
        thread_id: ThreadId,
        selection: ModelSelection,
    },
    SelectRuntimeMode {
        thread_id: ThreadId,
        mode: RuntimeMode,
    },
    SelectDiffFile(String),
    OpenFilePreview(String),
    RefreshGitAndWorkspace,
    SpawnTerminal,
    SelectTerminalTab(String),
    SendTerminalInput {
        terminal_id: String,
        command: String,
    },
    OpenWorkspaceDialog,
    RunLegacyImport,
    CheckUpdates,
    SetUpdateChannel(UpdateChannel),
    SaveCustomAcpCommand(Option<String>),
    SetRemotePort(u16),
    ToggleRemoteServer,
    SwitchProviderFilter(ProviderKind),
}

pub fn status_label(status: ThreadRunStatus) -> &'static str {
    match status {
        ThreadRunStatus::Idle => "空闲",
        ThreadRunStatus::Running => "运行中",
        ThreadRunStatus::WaitingApproval => "待确认",
        ThreadRunStatus::Paused => "已暂停",
        ThreadRunStatus::Error => "异常",
    }
}

pub fn extract_active_mention(input: &str) -> Option<String> {
    let token = input.split_whitespace().last()?;
    let rest = token.strip_prefix('@')?;
    if rest.contains('\n') {
        return None;
    }
    Some(rest.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn extract_mention_parses_trailing_at_token() {
        assert_eq!(
            extract_active_mention("please inspect @src/main"),
            Some("src/main".to_string())
        );
        assert_eq!(extract_active_mention("hello @"), Some(String::new()));
        assert_eq!(extract_active_mention("normal text"), None);
    }

    #[test]
    fn inspector_is_collapsed_by_default_and_modals_are_mutually_exclusive() {
        let mut state = AppUiState::new(AppSettings::default());
        assert!(!state.inspector_open);
        assert_eq!(state.active_modal, ActiveModal::None);

        state.open_modal(ActiveModal::Settings);
        assert_eq!(state.active_modal, ActiveModal::Settings);

        state.open_modal(ActiveModal::Update);
        assert_eq!(state.active_modal, ActiveModal::Update);

        state.close_modal();
        assert_eq!(state.active_modal, ActiveModal::None);
    }

    #[test]
    fn tab_lifecycle_switches_to_neighbor_on_close() {
        let mut state = AppUiState::new(AppSettings::default());
        let p = ProjectId::from_string("p1");
        let t1 = ThreadId::from_string("t1");
        let t2 = ThreadId::from_string("t2");
        let now = Utc::now();
        for (id, title) in [(&t1, "One"), (&t2, "Two")] {
            state.threads.push(ThreadRecord {
                id: id.clone(),
                project_id: p.clone(),
                title: title.into(),
                model_selection: ModelSelection {
                    provider: ProviderKind::Codex,
                    model_slug: "gpt-5".into(),
                    thinking_level: None,
                    planner_model: None,
                    executor_model: None,
                },
                runtime_mode: RuntimeMode::ApprovalRequired,
                status: ThreadRunStatus::Idle,
                branch: None,
                worktree_path: None,
                provider_session_id: None,
                pinned: false,
                archived: false,
                unread: true,
                created_at: now,
                updated_at: now,
            });
        }
        state.select_thread(t1.clone());
        state.select_thread(t2.clone());
        assert_eq!(state.active_thread_id, Some(t2.clone()));
        state.close_tab(&t2);
        assert_eq!(state.active_thread_id, Some(t1));
    }

    #[test]
    fn command_palette_filters_by_query() {
        let mut state = AppUiState::new(AppSettings::default());
        state.command_palette_query = "终端".into();
        let items = state.command_palette_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "cmd:toggle_terminal");
    }
}
