use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    AppContext as _, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, div,
    prelude::FluentBuilder, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::input::TextareaState;
use gpuicn::textarea::textarea;
use gpuicn::theme::UiTheme;
use lingxi_domain::{
    MessageRole, PermissionDecision, ProviderKind, RuntimeMode, ThemeAppearance, ThreadRunStatus,
    ToolCallStatus,
};

use crate::components::{context_meter, hairline_panel, status_dot};
use crate::markdown::{MarkdownBlock, parse_streaming_blocks};
use crate::state::{AppUiState, InspectorTab, SettingsSection, UiAction, status_label};

pub type ActionDispatcher = Arc<dyn Fn(UiAction) + Send + Sync>;

pub struct LingXiShellView {
    pub state: AppUiState,
    pub composer_input: Entity<TextareaState>,
    pub dispatcher: ActionDispatcher,
    _subscriptions: Vec<Subscription>,
}

impl LingXiShellView {
    pub fn new(
        state: AppUiState,
        dispatcher: ActionDispatcher,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let composer_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("向 Agent 下达任务，输入 @ 引用文件，Enter 发送 (Shift+Enter 换行)...")
        });
        Self {
            state,
            composer_input,
            dispatcher,
            _subscriptions: Vec::new(),
        }
    }

    pub fn update_state(&mut self, updater: impl FnOnce(&mut AppUiState), cx: &mut Context<Self>) {
        updater(&mut self.state);
        cx.notify();
    }

    fn dispatch(&self, action: UiAction) {
        (self.dispatcher)(action);
    }

    fn render_top_bar(&self, theme: &UiTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark =
            theme.colors.background == crate::theme::lingxi_dark_theme().colors.background;
        let open_tabs = self.state.open_tabs.clone();
        let active_id = self.state.active_thread_id.clone();

        div()
            .h(px(42.0))
            .w_full()
            .bg(theme.colors.sidebar)
            .border_b_1()
            .border_color(theme.colors.border)
            .px(px(10.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        Button::new("topbar.sidebar")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.dispatch(UiAction::ToggleSidebar);
                            }))
                            .child(
                                lucide(if self.state.sidebar_collapsed {
                                    LucideIcon::PanelLeftOpen
                                } else {
                                    LucideIcon::PanelLeftClose
                                })
                                .size(px(15.0))
                                .text_color(theme.colors.foreground),
                            ),
                    )
                    .child(div().flex().items_center().gap(px(4.0)).children(
                        open_tabs.into_iter().enumerate().map(|(idx, tid)| {
                            let is_active = active_id.as_ref() == Some(&tid);
                            let title = self
                                .state
                                .threads
                                .iter()
                                .find(|t| t.id == tid)
                                .map(|t| t.title.clone())
                                .unwrap_or_else(|| "会话".to_string());
                            let tid_select = tid.clone();
                            let tid_close = tid;
                            div()
                                .id(("tab", idx))
                                .px(px(10.0))
                                .py(px(4.0))
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(if is_active {
                                    theme.colors.primary
                                } else {
                                    theme.colors.border
                                })
                                .bg(if is_active {
                                    theme.colors.card
                                } else {
                                    theme.colors.sidebar
                                })
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, _| {
                                    this.dispatch(UiAction::SelectThread(tid_select.clone()));
                                }))
                                .child(div().text_xs().child(title))
                                .child(
                                    div()
                                        .id(("tab-close", idx))
                                        .text_xs()
                                        .text_color(theme.colors.muted_foreground)
                                        .on_click(cx.listener(move |this, _, _, _| {
                                            this.dispatch(UiAction::CloseTab(tid_close.clone()));
                                        }))
                                        .child("×"),
                                )
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        Button::new("topbar.cmd")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .label("命令面板 ⌘K")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.dispatch(UiAction::OpenCommandPalette);
                            })),
                    )
                    .child(
                        Button::new("topbar.theme")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.dispatch(UiAction::ToggleAppearance);
                            }))
                            .child(
                                lucide(if is_dark {
                                    LucideIcon::Sun
                                } else {
                                    LucideIcon::Moon
                                })
                                .size(px(15.0))
                                .text_color(theme.colors.foreground),
                            ),
                    )
                    .child(
                        Button::new("topbar.update")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.dispatch(UiAction::OpenUpdateModal);
                            }))
                            .child(lucide(LucideIcon::Download).size(px(15.0)).text_color(
                                if self.state.available_update.is_some() {
                                    theme.colors.primary
                                } else {
                                    theme.colors.foreground
                                },
                            )),
                    )
                    .child(
                        Button::new("topbar.settings")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .on_click(cx.listener(|this, _, _, _| {
                                this.dispatch(UiAction::OpenSettings(SettingsSection::General));
                            }))
                            .child(
                                lucide(LucideIcon::Settings)
                                    .size(px(15.0))
                                    .text_color(theme.colors.foreground),
                            ),
                    ),
            )
    }

    fn render_sidebar(&self, theme: &UiTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let projects = self.state.projects.clone();
        let active_id = self.state.active_thread_id.clone();

        div()
            .w(px(self.state.sidebar_width))
            .h_full()
            .bg(theme.colors.sidebar)
            .border_r_1()
            .border_color(theme.colors.sidebar_border)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(10.0))
                    .border_b_1()
                    .border_color(theme.colors.sidebar_border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("工作区与会话"),
                    )
                    .child(
                        Button::new("sidebar.open_workspace")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .label("+ 打开项目")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.dispatch(UiAction::OpenWorkspaceDialog);
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .p(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(projects.into_iter().enumerate().map(|(p_idx, project)| {
                        let expanded = self.state.expanded_projects.contains(&project.id);
                        let threads: Vec<_> = self
                            .state
                            .filtered_threads_for_project(&project.id)
                            .into_iter()
                            .cloned()
                            .collect();
                        let pid_toggle = project.id.clone();
                        let pid_new = project.id.clone();
                        let pid_wt = project.id.clone();
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(6.0))
                                    .py(px(4.0))
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .id(("proj", p_idx))
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        this.dispatch(UiAction::ToggleProjectExpanded(
                                            pid_toggle.clone(),
                                        ));
                                    }))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(if expanded { "▾" } else { "▸" })
                                            .child(project.title.clone()),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .child(
                                                Button::new(("new-wt", p_idx))
                                                    .variant(ButtonVariant::Ghost)
                                                    .size(ButtonSize::IconSm)
                                                    .on_click(cx.listener(move |this, _, _, _| {
                                                        this.dispatch(UiAction::CreateThread {
                                                            project_id: pid_wt.clone(),
                                                            worktree: true,
                                                        });
                                                    }))
                                                    .child(
                                                        lucide(LucideIcon::GitBranch)
                                                            .size(px(12.0))
                                                            .text_color(
                                                                theme.colors.muted_foreground,
                                                            ),
                                                    ),
                                            )
                                            .child(
                                                Button::new(("new-thread", p_idx))
                                                    .variant(ButtonVariant::Ghost)
                                                    .size(ButtonSize::IconSm)
                                                    .on_click(cx.listener(move |this, _, _, _| {
                                                        this.dispatch(UiAction::CreateThread {
                                                            project_id: pid_new.clone(),
                                                            worktree: false,
                                                        });
                                                    }))
                                                    .child(
                                                        lucide(LucideIcon::Plus)
                                                            .size(px(13.0))
                                                            .text_color(theme.colors.foreground),
                                                    ),
                                            ),
                                    ),
                            )
                            .when(expanded, |col| {
                                col.children(threads.into_iter().enumerate().map(
                                    |(t_idx, thread)| {
                                        let is_selected = active_id.as_ref() == Some(&thread.id);
                                        let tid = thread.id.clone();
                                        div()
                                            .id(SharedString::from(format!(
                                                "thread-row-{p_idx}-{t_idx}"
                                            )))
                                            .pl(px(18.0))
                                            .pr(px(8.0))
                                            .py(px(6.0))
                                            .rounded(px(6.0))
                                            .bg(if is_selected {
                                                theme.colors.sidebar_accent
                                            } else {
                                                theme.colors.sidebar
                                            })
                                            .border_1()
                                            .border_color(if is_selected {
                                                theme.colors.primary
                                            } else {
                                                theme.colors.sidebar
                                            })
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap(px(6.0))
                                            .cursor_pointer()
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                this.dispatch(UiAction::SelectThread(tid.clone()));
                                            }))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap(px(6.0))
                                                    .child(status_dot(thread.status, theme))
                                                    .child(
                                                        div().text_xs().child(thread.title.clone()),
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(theme.colors.muted_foreground)
                                                    .child(
                                                        thread
                                                            .model_selection
                                                            .provider
                                                            .display_name(),
                                                    ),
                                            )
                                    },
                                ))
                            })
                    })),
            )
    }

    fn render_chat_pane(&self, theme: &UiTheme, cx: &mut Context<Self>) -> impl IntoElement {
        let active_thread = self.state.active_thread().cloned();
        let messages = self.state.active_messages().to_vec();
        let approvals = self.state.active_approvals().to_vec();
        let context_info = active_thread
            .as_ref()
            .and_then(|t| self.state.context_by_thread.get(&t.id).cloned())
            .unwrap_or_default();
        let mentions = self.state.filtered_mentions();

        div()
            .flex_1()
            .h_full()
            .bg(theme.colors.background)
            .flex()
            .flex_col()
            .child(
                // Thread subheader
                div()
                    .px(px(14.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(if let Some(ref t) = active_thread {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(status_dot(t.status, theme))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(t.title.clone()),
                            )
                            .child(
                                div()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(theme.colors.muted)
                                    .text_xs()
                                    .child(status_label(t.status)),
                            )
                    } else {
                        div()
                            .text_sm()
                            .text_color(theme.colors.muted_foreground)
                            .child("未选中会话 — 请在左侧工作区选择或新建会话")
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                Button::new("chat.inspector.terminal")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .label("终端")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.dispatch(UiAction::ToggleInspector(
                                            InspectorTab::Terminal,
                                        ));
                                    })),
                            )
                            .child(
                                Button::new("chat.inspector.diff")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .label("Diff")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.dispatch(UiAction::ToggleInspector(
                                            InspectorTab::Diff,
                                        ));
                                    })),
                            )
                            .child(
                                Button::new("chat.inspector.git")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .label("Git/PR")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.dispatch(UiAction::ToggleInspector(
                                            InspectorTab::GitPr,
                                        ));
                                    })),
                            )
                            .child(
                                Button::new("chat.inspector.metrics")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Sm)
                                    .label("额度监控")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.dispatch(UiAction::ToggleInspector(
                                            InspectorTab::Metrics,
                                        ));
                                    })),
                            ),
                    ),
            )
            .child(
                // Timeline area
                div()
                    .flex_1()
                    .p(px(16.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .children(messages.into_iter().map(|msg| {
                        let is_user = msg.role == MessageRole::User;
                        let blocks = parse_streaming_blocks(&msg.content);
                        hairline_panel(theme)
                            .p(px(12.0))
                            .bg(if is_user {
                                theme.colors.accent
                            } else {
                                theme.colors.card
                            })
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.muted_foreground)
                                    .child(if is_user { "主人" } else { "Agent" }),
                            )
                            .children(blocks.into_iter().map(|block| {
                                match block {
                                    MarkdownBlock::Heading { text, .. } => div()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(text),
                                    MarkdownBlock::Paragraph(text) => div().text_sm().child(text),
                                    MarkdownBlock::CodeFence {
                                        language,
                                        code,
                                        closed,
                                    } => div()
                                        .p(px(10.0))
                                        .rounded(px(6.0))
                                        .bg(theme.colors.muted)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .flex()
                                        .flex_col()
                                        .gap(px(4.0))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.colors.muted_foreground)
                                                .child(format!(
                                                    "{}{}",
                                                    language.unwrap_or_else(|| "code".into()),
                                                    if closed { "" } else { " (streaming...)" }
                                                )),
                                        )
                                        .child(div().text_xs().child(code)),
                                    MarkdownBlock::ListItem(text) => {
                                        div().text_sm().child(format!("• {text}"))
                                    }
                                    MarkdownBlock::Blockquote(text) => div()
                                        .pl(px(10.0))
                                        .border_l_2()
                                        .border_color(theme.colors.primary)
                                        .text_sm()
                                        .text_color(theme.colors.muted_foreground)
                                        .child(text),
                                }
                            }))
                            .children(msg.tool_calls.into_iter().map(|tc| {
                                let badge_color = match tc.status {
                                    ToolCallStatus::Completed => theme.colors.success,
                                    ToolCallStatus::Failed | ToolCallStatus::Denied => {
                                        theme.colors.destructive
                                    }
                                    _ => theme.colors.primary,
                                };
                                div()
                                    .p(px(8.0))
                                    .rounded(px(6.0))
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .bg(theme.colors.background)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div().text_xs().child(format!(
                                            "Tool: {} · {}",
                                            tc.name, tc.input_preview
                                        )),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(badge_color)
                                            .child(format!("{:?}", tc.status)),
                                    )
                            }))
                    }))
                    .children(approvals.into_iter().enumerate().map(|(a_idx, approval)| {
                        let tid_allow = approval.thread_id.clone();
                        let aid_allow = approval.id.clone();
                        let tid_session = approval.thread_id.clone();
                        let aid_session = approval.id.clone();
                        let tid_deny = approval.thread_id.clone();
                        let aid_deny = approval.id.clone();
                        hairline_panel(theme)
                            .p(px(12.0))
                            .border_color(theme.colors.warning)
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(format!("权限请求: {}", approval.tool_name)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.colors.muted_foreground)
                                            .child(approval.summary),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        Button::new(("allow-once", a_idx))
                                            .variant(ButtonVariant::Default)
                                            .size(ButtonSize::Sm)
                                            .label("允许一次")
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                this.dispatch(UiAction::ResolveApproval {
                                                    thread_id: tid_allow.clone(),
                                                    approval_id: aid_allow.clone(),
                                                    decision: PermissionDecision::AllowOnce,
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new(("allow-session", a_idx))
                                            .variant(ButtonVariant::Outline)
                                            .size(ButtonSize::Sm)
                                            .label("本会话允许")
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                this.dispatch(UiAction::ResolveApproval {
                                                    thread_id: tid_session.clone(),
                                                    approval_id: aid_session.clone(),
                                                    decision: PermissionDecision::AllowSession,
                                                });
                                            })),
                                    )
                                    .child(
                                        Button::new(("deny", a_idx))
                                            .variant(ButtonVariant::Destructive)
                                            .size(ButtonSize::Sm)
                                            .label("拒绝")
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                this.dispatch(UiAction::ResolveApproval {
                                                    thread_id: tid_deny.clone(),
                                                    approval_id: aid_deny.clone(),
                                                    decision: PermissionDecision::Deny,
                                                });
                                            })),
                                    ),
                            )
                    })),
            )
            .child(
                // Composer dock
                div()
                    .p(px(12.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.card)
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .when(!mentions.is_empty(), |col| {
                        col.child(
                            hairline_panel(theme)
                                .p(px(6.0))
                                .flex()
                                .flex_wrap()
                                .gap(px(4.0))
                                .children(mentions.into_iter().map(|path| {
                                    div()
                                        .px(px(6.0))
                                        .py(px(2.0))
                                        .rounded(px(4.0))
                                        .bg(theme.colors.muted)
                                        .text_xs()
                                        .child(format!("@{path}"))
                                })),
                        )
                    })
                    .child(context_meter(&context_info, theme))
                    .child(textarea(&self.composer_input))
                    .child(self.render_composer_toolbar(active_thread.as_ref(), theme, cx)),
            )
    }

    fn render_composer_toolbar(
        &self,
        active_thread: Option<&lingxi_domain::ThreadRecord>,
        theme: &UiTheme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_running = active_thread
            .map(|t| t.status == ThreadRunStatus::Running)
            .unwrap_or(false);
        let provider_name = active_thread
            .map(|t| {
                format!(
                    "{} · {}",
                    t.model_selection.provider.display_name(),
                    t.model_selection.model_slug
                )
            })
            .unwrap_or_else(|| "选择模型".to_string());
        let mode_label = active_thread
            .map(|t| t.runtime_mode.label())
            .unwrap_or(RuntimeMode::ApprovalRequired.label());

        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .text_xs()
                            .child(provider_name),
                    )
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(theme.colors.border)
                            .text_xs()
                            .child(mode_label),
                    ),
            )
            .child(div().flex().items_center().gap(px(6.0)).when_some(
                active_thread.cloned(),
                |row, thread| {
                    let tid_compact = thread.id.clone();
                    let tid_send = thread.id.clone();
                    let tid_cancel = thread.id;
                    row.child(
                        Button::new("composer.compact")
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Sm)
                            .label("压缩上下文")
                            .on_click(cx.listener(move |this, _, _, _| {
                                this.dispatch(UiAction::CompactContext(tid_compact.clone()));
                            })),
                    )
                    .child(if is_running {
                        Button::new("composer.cancel")
                            .variant(ButtonVariant::Destructive)
                            .size(ButtonSize::Sm)
                            .label("停止")
                            .on_click(cx.listener(move |this, _, _, _| {
                                this.dispatch(UiAction::CancelTurn(tid_cancel.clone()));
                            }))
                    } else {
                        Button::new("composer.send")
                            .variant(ButtonVariant::Default)
                            .size(ButtonSize::Sm)
                            .label("发送")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.state.ime_composing {
                                    return;
                                }
                                let text = this.composer_input.read(cx).text().to_string();
                                if !text.trim().is_empty() {
                                    this.dispatch(UiAction::SendTurn {
                                        thread_id: tid_send.clone(),
                                        prompt: text,
                                    });
                                }
                            }))
                    })
                },
            ))
    }

    fn render_inspector(&self, theme: &UiTheme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(self.state.inspector_width))
            .h_full()
            .bg(theme.colors.card)
            .border_l_1()
            .border_color(theme.colors.border)
            .flex()
            .flex_col()
            .child(
                div()
                    .p(px(8.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(
                        match self.state.inspector_tab {
                            InspectorTab::Terminal => "终端 (PTY)",
                            InspectorTab::Diff => "代码变更 (Diff)",
                            InspectorTab::GitPr => "Git 与 PR",
                            InspectorTab::Files => "工作区文件",
                            InspectorTab::Metrics => "6 平台额度与资源",
                        },
                    ))
                    .child(
                        Button::new("inspector.close")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .on_click(cx.listener(|this, _, _, _| {
                                let tab = this.state.inspector_tab;
                                this.dispatch(UiAction::ToggleInspector(tab));
                            }))
                            .child(
                                lucide(LucideIcon::X)
                                    .size(px(14.0))
                                    .text_color(theme.colors.muted_foreground),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .p(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(match self.state.inspector_tab {
                        InspectorTab::Terminal => {
                            let active = self.state.terminal_tabs.first();
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.colors.muted_foreground)
                                        .child(
                                            active
                                                .map(|t| format!("{} ({})", t.title, t.cwd))
                                                .unwrap_or_else(|| "暂无活跃终端".into()),
                                        ),
                                )
                                .children(
                                    active
                                        .map(|t| t.lines.clone())
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|line| div().text_xs().child(line)),
                                )
                        }
                        InspectorTab::Diff => div().flex().flex_col().gap(px(6.0)).children(
                            self.state.git_diffs.iter().map(|diff| {
                                hairline_panel(theme)
                                    .p(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().text_xs().child(diff.path.clone()))
                                    .child(
                                        div().text_xs().text_color(theme.colors.primary).child(
                                            format!("+{} -{}", diff.additions, diff.deletions),
                                        ),
                                    )
                            }),
                        ),
                        InspectorTab::GitPr => div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div().text_xs().child(
                                    self.state
                                        .git_status
                                        .as_ref()
                                        .and_then(|s| s.branch.clone())
                                        .map(|b| format!("当前分支: {b}"))
                                        .unwrap_or_else(|| "未检测到 Git 仓库".into()),
                                ),
                            )
                            .children(self.state.pull_requests.iter().map(|pr| {
                                hairline_panel(theme).p(px(8.0)).child(
                                    div().text_xs().child(format!(
                                        "#{} {} [{}]",
                                        pr.number, pr.title, pr.state
                                    )),
                                )
                            })),
                        InspectorTab::Files => div().flex().flex_col().gap(px(4.0)).children(
                            self.state
                                .workspace_files
                                .iter()
                                .take(50)
                                .map(|f| div().text_xs().child(f.clone())),
                        ),
                        InspectorTab::Metrics => div()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.colors.muted_foreground)
                                    .child(format!(
                                        "进程 CPU {:.1}% · RSS {:.1} MiB",
                                        self.state.resource_usage.cpu_percent,
                                        self.state.resource_usage.memory_rss_bytes as f64
                                            / (1024.0 * 1024.0)
                                    )),
                            )
                            .children(self.state.quota_snapshots.iter().map(|q| {
                                hairline_panel(theme)
                                    .p(px(8.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(q.provider.display_name()),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.colors.muted_foreground)
                                            .child(q.source_note.clone()),
                                    )
                            })),
                    }),
            )
    }

    fn render_overlays(&self, theme: &UiTheme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .when(self.state.command_palette_open, |layer| {
                let items = self.state.command_palette_items();
                layer.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(theme.colors.overlay)
                        .flex()
                        .items_start()
                        .justify_center()
                        .pt(px(80.0))
                        .child(
                            hairline_panel(theme)
                                .w(px(540.0))
                                .p(px(14.0))
                                .flex()
                                .flex_col()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("命令面板"),
                                        )
                                        .child(
                                            Button::new("cmd.close")
                                                .variant(ButtonVariant::Ghost)
                                                .size(ButtonSize::Sm)
                                                .label("关闭")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.dispatch(UiAction::CloseCommandPalette);
                                                })),
                                        ),
                                )
                                .children(items.into_iter().take(8).map(|item| {
                                    div()
                                        .px(px(10.0))
                                        .py(px(6.0))
                                        .rounded(px(6.0))
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(div().text_xs().child(item.title))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(theme.colors.muted_foreground)
                                                .child(item.shortcut.unwrap_or_default()),
                                        )
                                })),
                        ),
                )
            })
            .when(self.state.settings_open, |layer| {
                layer.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(theme.colors.overlay)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            hairline_panel(theme)
                                .w(px(720.0))
                                .h(px(500.0))
                                .p(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(12.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .justify_between()
                                        .child(
                                            div()
                                                .text_base()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .child("设置中心"),
                                        )
                                        .child(
                                            Button::new("settings.close")
                                                .variant(ButtonVariant::Ghost)
                                                .size(ButtonSize::Sm)
                                                .label("关闭")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.dispatch(UiAction::CloseSettings);
                                                })),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(8.0))
                                        .child(
                                            Button::new("settings.import")
                                                .variant(ButtonVariant::Outline)
                                                .size(ButtonSize::Sm)
                                                .label("只读导入旧版 LingXi T3 数据")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.dispatch(UiAction::RunLegacyImport);
                                                })),
                                        )
                                        .child(
                                            Button::new("settings.check_update")
                                                .variant(ButtonVariant::Outline)
                                                .size(ButtonSize::Sm)
                                                .label("检查更新")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.dispatch(UiAction::CheckUpdates);
                                                })),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.colors.muted_foreground)
                                        .child(format!(
                                            "外观: {:?} · 语言: {} · 更新频道: {:?} · 远程端口: {}",
                                            self.state.settings.appearance,
                                            self.state.settings.language,
                                            self.state.settings.update_channel,
                                            self.state.settings.remote_server_port
                                        )),
                                )
                                .children(ProviderKind::ALL.iter().map(|kind| {
                                    div()
                                        .text_xs()
                                        .child(format!("Provider: {}", kind.display_name()))
                                })),
                        ),
                )
            })
            .when(self.state.update_modal_open, |layer| {
                layer.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(theme.colors.overlay)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            hairline_panel(theme)
                                .w(px(460.0))
                                .p(px(16.0))
                                .flex()
                                .flex_col()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("应用更新"),
                                )
                                .child(
                                    div().text_xs().child(
                                        self.state
                                            .available_update
                                            .as_ref()
                                            .map(|u| {
                                                format!(
                                                    "发现新版本 {} ({:?})",
                                                    u.version, u.channel
                                                )
                                            })
                                            .unwrap_or_else(|| "当前已是最新版本".into()),
                                    ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(8.0))
                                        .child(
                                            Button::new("update.check")
                                                .variant(ButtonVariant::Outline)
                                                .size(ButtonSize::Sm)
                                                .label("立即检查")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.dispatch(UiAction::CheckUpdates);
                                                })),
                                        )
                                        .child(
                                            Button::new("update.close")
                                                .variant(ButtonVariant::Default)
                                                .size(ButtonSize::Sm)
                                                .label("关闭")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.dispatch(UiAction::CloseUpdateModal);
                                                })),
                                        ),
                                ),
                        ),
                )
            })
    }
}

impl Render for LingXiShellView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = UiTheme::read(cx).clone();
        div()
            .relative()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .flex()
            .flex_col()
            .child(self.render_top_bar(&theme, cx))
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .flex()
                    .when(!self.state.sidebar_collapsed, |row| {
                        row.child(self.render_sidebar(&theme, cx))
                    })
                    .child(self.render_chat_pane(&theme, cx))
                    .when(self.state.inspector_open, |row| {
                        row.child(self.render_inspector(&theme, cx))
                    }),
            )
            .child(self.render_overlays(&theme, cx))
    }
}

pub fn toggle_appearance_mode(current: ThemeAppearance) -> ThemeAppearance {
    match current {
        ThemeAppearance::Light => ThemeAppearance::Dark,
        ThemeAppearance::Dark | ThemeAppearance::System => ThemeAppearance::Light,
    }
}
