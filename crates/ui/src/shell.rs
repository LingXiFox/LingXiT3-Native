use std::sync::Arc;

use gpui_icons::LucideIcon;
use gpui_kit::{
    AppContext as _, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};
use gpuicn::command::{CommandEvent, CommandItem, CommandState};
use gpuicn::composer::{ComposerEvent, ComposerState, ComposerSuggestion};
use gpuicn::diff::{DiffFile, DiffState};
use gpuicn::input::{InputEvent, InputState};
use gpuicn::resizable::{PaneLimits, Resizable};
use gpuicn::select::{SelectEvent, SelectItem, SelectState};
use gpuicn::theme::UiTheme;
use lingxi_domain::{
    ModelSelection, ProviderKind, RuntimeMode, ThemeAppearance, ThreadId, UpdateChannel,
};

use crate::state::{AppUiState, SettingsSection, UiAction};
use crate::theme::tokens::{
    INSPECTOR_MAX_WIDTH, INSPECTOR_MIN_WIDTH, SIDEBAR_COLLAPSED_WIDTH, SIDEBAR_MAX_WIDTH,
    SIDEBAR_MIN_WIDTH,
};
use crate::views::chat::render_chat_pane;
use crate::views::composer::render_composer;
use crate::views::inspector::{git_diffs_to_unified_patch, render_inspector};
use crate::views::modals::{ModalHandles, render_modals};
use crate::views::sidebar::render_sidebar;
use crate::views::titlebar::render_titlebar;
use crate::views::welcome::render_welcome;

pub type ActionDispatcher = Arc<dyn Fn(UiAction) + Send + Sync>;

pub struct LingXiShellView {
    pub state: AppUiState,
    pub dispatcher: ActionDispatcher,
    composer: Entity<ComposerState>,
    sidebar_filter_input: Entity<InputState>,
    custom_acp_input: Entity<InputState>,
    remote_port_input: Entity<InputState>,
    terminal_input: Entity<InputState>,
    provider_select: Entity<SelectState>,
    mode_select: Entity<SelectState>,
    update_channel_select: Entity<SelectState>,
    command_state: Entity<CommandState>,
    diff_state: Entity<DiffState>,
    modal_handles: ModalHandles,
    last_diff_signature: String,
    last_command_signature: usize,
}

impl LingXiShellView {
    pub fn new(
        state: AppUiState,
        dispatcher: ActionDispatcher,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let composer = cx.new(|cx| {
            let mut c = ComposerState::new(6, window, cx);
            c.set_placeholder(
                "输入问题、任务描述，输入 @ 引用文件，/ 触发快捷命令…",
                window,
                cx,
            );
            c
        });
        let d_comp = Arc::clone(&dispatcher);
        cx.subscribe_in(
            &composer,
            window,
            move |this, comp, event, window, cx| match event {
                ComposerEvent::Submit(text) => {
                    let prompt = text.to_string();
                    if prompt.trim().is_empty() {
                        return;
                    }
                    if let Some(tid) = this.state.active_thread_id.clone() {
                        comp.update(cx, |c, cx| c.set_value("", window, cx));
                        d_comp(UiAction::SendTurn {
                            thread_id: tid,
                            prompt,
                        });
                    }
                }
                ComposerEvent::Autocomplete { trigger, query } => {
                    let q = query.to_lowercase();
                    let suggestions: Vec<ComposerSuggestion> = if *trigger == '@' {
                        this.state
                            .workspace_files
                            .iter()
                            .filter(|p| q.is_empty() || p.to_lowercase().contains(&q))
                            .take(10)
                            .map(|p| {
                                ComposerSuggestion::new(format!("@{p}"), p.clone())
                                    .icon(LucideIcon::FileCode)
                            })
                            .collect()
                    } else if *trigger == '/' {
                        [
                            ("/compact", "压缩上下文", "总结历史消息以释放上下文窗口"),
                            ("/diff", "打开 Diff 检查器", "查看工作区代码改动"),
                            ("/terminal", "打开内嵌终端", "启动本地 PTY Shell"),
                            ("/settings", "打开设置中心", "管理 Provider 与系统设置"),
                        ]
                        .into_iter()
                        .filter(|(cmd, label, _)| {
                            q.is_empty() || cmd.contains(&q) || label.to_lowercase().contains(&q)
                        })
                        .map(|(cmd, label, desc)| {
                            ComposerSuggestion::new(cmd, label)
                                .description(desc)
                                .icon(LucideIcon::Terminal)
                        })
                        .collect()
                    } else {
                        Vec::new()
                    };
                    comp.update(cx, |c, cx| c.set_suggestions(suggestions, cx));
                }
                ComposerEvent::AutocompleteClosed => {
                    comp.update(cx, |c, cx| c.set_suggestions(Vec::new(), cx));
                }
                _ => {}
            },
        )
        .detach();

        let sidebar_filter_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("搜索工作区或会话…"));
        let d_filter = Arc::clone(&dispatcher);
        cx.subscribe(&sidebar_filter_input, move |_, input, event, cx| {
            if let InputEvent::Change = event {
                let q = input.read(cx).value().to_string();
                d_filter(UiAction::SetSidebarFilter(q));
            }
        })
        .detach();

        let custom_acp_initial = state
            .settings
            .custom_acp_command
            .clone()
            .unwrap_or_default();
        let custom_acp_input = cx.new(|cx| {
            let mut inp =
                InputState::new(window, cx).placeholder("例如: npx @agentclientprotocol/server");
            if !custom_acp_initial.is_empty() {
                inp.set_value(custom_acp_initial, window, cx);
            }
            inp
        });

        let remote_port_initial = state.settings.remote_server_port.to_string();
        let remote_port_input = cx.new(|cx| {
            let mut inp = InputState::new(window, cx).placeholder("3773");
            inp.set_value(remote_port_initial, window, cx);
            inp
        });

        let terminal_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("输入 Shell 命令后按 Enter…"));
        let d_term = Arc::clone(&dispatcher);
        cx.subscribe_in(
            &terminal_input,
            window,
            move |this, input, event, window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    let cmd = input.read(cx).value().to_string();
                    if cmd.trim().is_empty() {
                        return;
                    }
                    if let Some(tid) = this
                        .state
                        .active_terminal_id
                        .clone()
                        .or_else(|| this.state.terminal_tabs.first().map(|t| t.id.clone()))
                    {
                        input.update(cx, |inp, cx| inp.set_value("", window, cx));
                        d_term(UiAction::SendTerminalInput {
                            terminal_id: tid,
                            command: cmd,
                        });
                    }
                }
            },
        )
        .detach();

        let provider_items = all_provider_select_items();
        let initial_provider = state
            .active_thread()
            .map(|t| provider_slug(t.model_selection.provider))
            .unwrap_or("codex");
        let provider_select = cx.new(|cx| {
            let mut s = SelectState::new(provider_items, window, cx);
            s.set_value(Some(initial_provider.into()), window, cx);
            s
        });
        let d_prov = Arc::clone(&dispatcher);
        cx.subscribe(&provider_select, move |this, _, event, _| {
            if let SelectEvent::Change(Some(val)) = event
                && let (Some(thread), Some(provider)) = (
                    this.state.active_thread(),
                    parse_provider_slug(val.as_ref()),
                )
            {
                let mut selection: ModelSelection = thread.model_selection.clone();
                selection.provider = provider;
                d_prov(UiAction::SelectModel {
                    thread_id: thread.id.clone(),
                    selection,
                });
            }
        })
        .detach();

        let mode_items = vec![
            SelectItem::new("approval", "操作前确认"),
            SelectItem::new("auto_edits", "自动接受编辑"),
            SelectItem::new("auto", "自动调度"),
            SelectItem::new("full_access", "完全自主执行"),
        ];
        let initial_mode = state
            .active_thread()
            .map(|t| runtime_mode_slug(t.runtime_mode))
            .unwrap_or("approval");
        let mode_select = cx.new(|cx| {
            let mut s = SelectState::new(mode_items, window, cx);
            s.set_value(Some(initial_mode.into()), window, cx);
            s
        });
        let d_mode = Arc::clone(&dispatcher);
        cx.subscribe(&mode_select, move |this, _, event, _| {
            if let SelectEvent::Change(Some(val)) = event
                && let Some(tid) = this.state.active_thread_id.clone()
            {
                let mode = match val.as_ref() {
                    "full_access" => RuntimeMode::FullAccess,
                    "auto_edits" => RuntimeMode::AutoAcceptEdits,
                    "auto" => RuntimeMode::Auto,
                    _ => RuntimeMode::ApprovalRequired,
                };
                d_mode(UiAction::SelectRuntimeMode {
                    thread_id: tid,
                    mode,
                });
            }
        })
        .detach();

        let channel_items = vec![
            SelectItem::new("stable", "Stable (稳定版)"),
            SelectItem::new("nightly", "Nightly (预览版)"),
        ];
        let initial_channel = match state.settings.update_channel {
            UpdateChannel::Stable => "stable",
            UpdateChannel::Nightly => "nightly",
        };
        let update_channel_select = cx.new(|cx| {
            let mut s = SelectState::new(channel_items, window, cx);
            s.set_value(Some(initial_channel.into()), window, cx);
            s
        });
        let d_chan = Arc::clone(&dispatcher);
        cx.subscribe(&update_channel_select, move |_, _, event, _| {
            if let SelectEvent::Change(Some(val)) = event {
                let ch = match val.as_ref() {
                    "nightly" => UpdateChannel::Nightly,
                    _ => UpdateChannel::Stable,
                };
                d_chan(UiAction::SetUpdateChannel(ch));
            }
        })
        .detach();

        let initial_cmd_items = build_command_items(&state);
        let command_state = cx.new(|cx| {
            let mut cmd = CommandState::new(initial_cmd_items, window, cx);
            cmd.set_placeholder("输入命令、工作区或会话名称…", window, cx);
            cmd
        });
        let d_cmd = Arc::clone(&dispatcher);
        cx.subscribe(&command_state, move |_, _, event, _| match event {
            CommandEvent::Confirm(value) => {
                dispatch_command_palette_value(value.as_ref(), &d_cmd);
                d_cmd(UiAction::CloseModal);
            }
            CommandEvent::Cancel => {
                d_cmd(UiAction::CloseModal);
            }
            _ => {}
        })
        .detach();

        let diff_state = cx.new(|cx| DiffState::new(Vec::<DiffFile>::new(), cx));
        let modal_handles = ModalHandles::new(cx);

        let mut view = Self {
            state,
            dispatcher,
            composer,
            sidebar_filter_input,
            custom_acp_input,
            remote_port_input,
            terminal_input,
            provider_select,
            mode_select,
            update_channel_select,
            command_state,
            diff_state,
            modal_handles,
            last_diff_signature: String::new(),
            last_command_signature: 0,
        };
        view.sync_diff_state(cx);
        view
    }

    pub fn sync_diff_state(&mut self, cx: &mut Context<Self>) {
        let sig = format!(
            "{}:{:?}",
            self.state.git_diffs.len(),
            self.state.selected_diff_path
        );
        if sig == self.last_diff_signature {
            return;
        }
        self.last_diff_signature = sig;
        let patch = git_diffs_to_unified_patch(
            &self.state.git_diffs,
            self.state.selected_diff_path.as_deref(),
        );
        let parsed = if patch.trim().is_empty() {
            Vec::new()
        } else {
            DiffFile::parse(&patch).unwrap_or_default()
        };
        self.diff_state.update(cx, |diff, cx| {
            diff.set_files(parsed, cx);
        });
    }

    fn sync_command_palette(&mut self, cx: &mut Context<Self>) {
        let sig = self.state.threads.len();
        if sig == self.last_command_signature {
            return;
        }
        self.last_command_signature = sig;
        let items = build_command_items(&self.state);
        self.command_state.update(cx, |cmd, cx| {
            cmd.set_items(items, cx);
        });
    }
}

impl Render for LingXiShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.modal_handles.sync(self.state.active_modal, window, cx);
        self.sync_diff_state(cx);
        self.sync_command_palette(cx);

        let theme = UiTheme::read(cx).clone();
        let d_sidebar_resize = Arc::clone(&self.dispatcher);
        let _ = d_sidebar_resize;

        let center_pane = if let Some(thread) = self.state.active_thread() {
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .bg(theme.colors.background)
                .flex()
                .flex_col()
                .child(render_chat_pane(
                    &self.state,
                    thread,
                    &self.dispatcher,
                    &theme,
                    window,
                    cx,
                ))
                .child(render_composer(
                    &self.state,
                    thread,
                    &self.composer,
                    &self.provider_select,
                    &self.mode_select,
                    &self.dispatcher,
                    &theme,
                    cx,
                ))
                .into_any_element()
        } else {
            render_welcome(&self.state, &self.dispatcher, &theme, cx).into_any_element()
        };

        let center_with_inspector = if self.state.inspector_open {
            let view_entity = cx.entity();
            let viewport_w = f32::from(window.viewport_size().width);
            let sidebar_w = if self.state.sidebar_collapsed {
                SIDEBAR_COLLAPSED_WIDTH
            } else {
                self.state.sidebar_width
            };
            let available_w = (viewport_w - sidebar_w).max(600.0);
            let first_w = (available_w - self.state.inspector_width).max(360.0);
            let inspector = render_inspector(
                &self.state,
                &self.diff_state,
                &self.terminal_input,
                &self.dispatcher,
                &theme,
                cx,
            );
            Resizable::new("shell.split.inspector", "调整工具面板宽度", px(first_w))
                .first_limits(PaneLimits::new(px(360.0), px(1600.0)))
                .second_limits(PaneLimits::new(
                    px(INSPECTOR_MIN_WIDTH),
                    px(INSPECTOR_MAX_WIDTH),
                ))
                .first(center_pane)
                .second(inspector)
                .on_resize(move |new_first_w, window, cx| {
                    let total = f32::from(window.viewport_size().width) - sidebar_w;
                    let new_insp_w = (total - f32::from(*new_first_w))
                        .clamp(INSPECTOR_MIN_WIDTH, INSPECTOR_MAX_WIDTH);
                    view_entity.update(cx, |this, cx| {
                        this.state.inspector_width = new_insp_w;
                        cx.notify();
                    });
                })
                .into_any_element()
        } else {
            center_pane
        };

        let sidebar_el = render_sidebar(
            &self.state,
            &self.sidebar_filter_input,
            &self.dispatcher,
            &theme,
            cx,
        );

        let body = if self.state.sidebar_collapsed {
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .flex()
                .child(
                    div()
                        .w(px(SIDEBAR_COLLAPSED_WIDTH))
                        .h_full()
                        .child(sidebar_el),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .flex()
                        .child(center_with_inspector),
                )
                .into_any_element()
        } else {
            let view_entity = cx.entity();
            Resizable::new(
                "shell.split.sidebar",
                "调整侧边栏宽度",
                px(self.state.sidebar_width),
            )
            .first_limits(PaneLimits::new(
                px(SIDEBAR_MIN_WIDTH),
                px(SIDEBAR_MAX_WIDTH),
            ))
            .second_limits(PaneLimits::new(px(460.0), px(2200.0)))
            .first(sidebar_el)
            .second(center_with_inspector)
            .on_resize(move |new_w, _, cx| {
                let w = f32::from(*new_w).clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
                view_entity.update(cx, |this, cx| {
                    this.state.sidebar_width = w;
                    cx.notify();
                });
            })
            .into_any_element()
        };

        let titlebar =
            render_titlebar(&self.state, &self.dispatcher, &theme, cx).into_any_element();
        let modals = render_modals(
            &self.state,
            &self.modal_handles,
            &self.command_state,
            &self.custom_acp_input,
            &self.remote_port_input,
            &self.update_channel_select,
            &self.dispatcher,
            &theme,
            window,
            cx,
        );

        div()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .font_family(theme.fonts.body.clone())
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(titlebar)
            .when_some(self.state.status_banner.clone(), |el, banner| {
                el.child(
                    div()
                        .w_full()
                        .px(px(16.0))
                        .py(px(6.0))
                        .border_b_1()
                        .border_color(theme.colors.border)
                        .bg(theme.colors.accent)
                        .text_size(px(12.5))
                        .text_color(theme.colors.foreground)
                        .child(banner),
                )
            })
            .child(div().flex_1().min_w_0().min_h_0().flex().child(body))
            .child(modals)
    }
}

pub fn toggle_appearance_mode(current: ThemeAppearance) -> ThemeAppearance {
    match current {
        ThemeAppearance::Light => ThemeAppearance::Dark,
        ThemeAppearance::Dark | ThemeAppearance::System => ThemeAppearance::Light,
    }
}

fn provider_slug(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Codex => "codex",
        ProviderKind::Claude => "claude",
        ProviderKind::Antigravity => "antigravity",
        ProviderKind::Dsh => "dsh",
        ProviderKind::Cursor => "cursor",
        ProviderKind::Grok => "grok",
        ProviderKind::OpenCode => "opencode",
        ProviderKind::Pi => "pi",
        ProviderKind::Muse => "muse",
        ProviderKind::Kiro => "kiro",
        ProviderKind::Fx => "fx",
        ProviderKind::AcpRegistry => "acp_registry",
    }
}

fn parse_provider_slug(slug: &str) -> Option<ProviderKind> {
    Some(match slug {
        "codex" => ProviderKind::Codex,
        "claude" => ProviderKind::Claude,
        "antigravity" => ProviderKind::Antigravity,
        "dsh" => ProviderKind::Dsh,
        "cursor" => ProviderKind::Cursor,
        "grok" => ProviderKind::Grok,
        "opencode" => ProviderKind::OpenCode,
        "pi" => ProviderKind::Pi,
        "muse" => ProviderKind::Muse,
        "kiro" => ProviderKind::Kiro,
        "fx" => ProviderKind::Fx,
        "acp_registry" => ProviderKind::AcpRegistry,
        _ => return None,
    })
}

fn all_provider_select_items() -> Vec<SelectItem> {
    ProviderKind::ALL
        .into_iter()
        .map(|p| SelectItem::new(provider_slug(p), p.display_name()))
        .collect()
}

fn runtime_mode_slug(mode: RuntimeMode) -> &'static str {
    match mode {
        RuntimeMode::ApprovalRequired => "approval",
        RuntimeMode::AutoAcceptEdits => "auto_edits",
        RuntimeMode::Auto => "auto",
        RuntimeMode::FullAccess => "full_access",
    }
}

fn build_command_items(state: &AppUiState) -> Vec<CommandItem> {
    state
        .command_palette_items()
        .into_iter()
        .map(|item| {
            let icon = if item.id.starts_with("thread:") {
                LucideIcon::MessageSquare
            } else if item.id.contains("settings") {
                LucideIcon::Settings
            } else if item.id.contains("terminal") {
                LucideIcon::Terminal
            } else if item.id.contains("diff") {
                LucideIcon::GitCompare
            } else if item.id.contains("theme") {
                LucideIcon::SunMoon
            } else if item.id.contains("update") {
                LucideIcon::Download
            } else {
                LucideIcon::Sparkles
            };
            let mut ci = CommandItem::new(item.id, item.title)
                .group(item.group)
                .description(item.subtitle)
                .icon(icon);
            if let Some(sc) = item.shortcut {
                ci = ci.shortcut(sc);
            }
            ci
        })
        .collect()
}

fn dispatch_command_palette_value(value: &str, dispatcher: &ActionDispatcher) {
    if let Some(tid) = value.strip_prefix("thread:") {
        dispatcher(UiAction::SelectThread(ThreadId::from_string(tid)));
        return;
    }
    match value {
        "cmd:new_thread" => {
            dispatcher(UiAction::OpenWorkspaceDialog);
        }
        "cmd:open_project" => dispatcher(UiAction::OpenWorkspaceDialog),
        "cmd:toggle_theme" => dispatcher(UiAction::ToggleAppearance),
        "cmd:toggle_sidebar" => dispatcher(UiAction::ToggleSidebar),
        "cmd:toggle_diff" => {
            dispatcher(UiAction::ToggleInspector(crate::state::InspectorTab::Diff))
        }
        "cmd:toggle_terminal" => dispatcher(UiAction::ToggleInspector(
            crate::state::InspectorTab::Terminal,
        )),
        "cmd:toggle_metrics" => dispatcher(UiAction::ToggleInspector(
            crate::state::InspectorTab::Metrics,
        )),
        "cmd:open_settings" => dispatcher(UiAction::OpenSettings(SettingsSection::General)),
        "cmd:open_updates" => dispatcher(UiAction::OpenUpdateModal),
        _ => {}
    }
}

pub fn _unused_shared_string_anchor() -> SharedString {
    SharedString::from("lingxi")
}
