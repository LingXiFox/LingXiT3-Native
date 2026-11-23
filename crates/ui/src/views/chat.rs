use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpuicn::avatar::{Avatar, AvatarSize};
use gpuicn::badge::{Badge, BadgeVariant};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::collapsible::{collapsible, collapsible_content, collapsible_trigger};
use gpuicn::markdown::markdown;
use gpuicn::message::{Message, MessageDensity};
use gpuicn::scroll_area::ScrollArea;
use gpuicn::theme::UiTheme;
use lingxi_domain::{
    MessageRole, PermissionDecision, ThreadRecord, ThreadRunStatus, ToolCallStatus,
};

use crate::components::{icon_xs, status_dot};
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, InspectorTab, UiAction, status_label};
use crate::tokens::{radius, space, surface, typo};

pub fn render_chat_pane(
    state: &AppUiState,
    thread: &ThreadRecord,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let d_git = Arc::clone(dispatcher);
    let d_diff = Arc::clone(dispatcher);
    let d_term = Arc::clone(dispatcher);

    let checkpoint_count = state.active_checkpoints().len();
    let active_tab = state.inspector_open.then_some(state.inspector_tab);

    let header = div()
        .h(space::THREAD_HEADER_HEIGHT)
        .w_full()
        .flex_shrink_0()
        .px(px(22.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.background)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(10.0))
                .min_w_0()
                .child(status_dot(thread.status, theme))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(typo::SECTION_TITLE)
                        .line_height(typo::SECTION_TITLE_LH)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child(thread.title.clone()),
                )
                .when_some(thread.branch.clone(), |el, branch| {
                    el.child(
                        Badge::new(branch)
                            .variant(BadgeVariant::Outline)
                            .icon(LucideIcon::GitBranch),
                    )
                })
                .child(
                    div()
                        .text_size(typo::CAPTION)
                        .text_color(theme.colors.muted_foreground)
                        .child(format!(
                            "{} · {}",
                            thread.model_selection.provider.display_name(),
                            thread.model_selection.model_slug
                        )),
                )
                .when(thread.status != ThreadRunStatus::Idle, |el| {
                    el.child(Badge::new(status_label(thread.status)).variant(
                        if thread.status == ThreadRunStatus::Error {
                            BadgeVariant::Destructive
                        } else {
                            BadgeVariant::Secondary
                        },
                    ))
                }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .flex_shrink_0()
                .when(checkpoint_count > 0, |el| {
                    el.child(
                        Badge::new(format!("{checkpoint_count} 检查点"))
                            .variant(BadgeVariant::Outline)
                            .icon(LucideIcon::Clock),
                    )
                })
                .child(
                    Button::new("chat.hdr.diff")
                        .variant(if active_tab == Some(InspectorTab::Diff) {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Sm)
                        .label("Diff")
                        .on_click(move |_, _, _| {
                            d_diff(UiAction::ToggleInspector(InspectorTab::Diff))
                        }),
                )
                .child(
                    Button::new("chat.hdr.git")
                        .variant(if active_tab == Some(InspectorTab::GitPr) {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Sm)
                        .label("Git")
                        .on_click(move |_, _, _| {
                            d_git(UiAction::ToggleInspector(InspectorTab::GitPr))
                        }),
                )
                .child(
                    Button::new("chat.hdr.term")
                        .variant(if active_tab == Some(InspectorTab::Terminal) {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Sm)
                        .label("终端")
                        .on_click(move |_, _, _| {
                            d_term(UiAction::ToggleInspector(InspectorTab::Terminal))
                        }),
                ),
        );

    let messages = state.active_messages();
    let approvals = state.active_approvals();

    let mut feed = div()
        .w_full()
        .max_w(space::CHAT_COLUMN_MAX_WIDTH)
        .mx_auto()
        .px(px(24.0))
        .py(px(24.0))
        .flex()
        .flex_col()
        .gap(px(22.0));

    if messages.is_empty() {
        feed = feed.child(
            div()
                .w_full()
                .px(px(18.0))
                .py(px(16.0))
                .rounded(radius::CARD)
                .bg(surface::quiet_panel(theme))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(typo::SECTION_TITLE)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child("新会话已就绪"),
                )
                .child(
                    div()
                        .text_size(typo::UI_BODY)
                        .line_height(typo::UI_BODY_LH)
                        .text_color(theme.colors.muted_foreground)
                        .child(
                            "在下方输入框描述你的任务，支持 @文件 引用工作区文件与 / 快捷命令。",
                        ),
                ),
        );
    } else {
        for (m_idx, msg) in messages.iter().enumerate() {
            let is_user = msg.role == MessageRole::User;
            let author_name = match msg.role {
                MessageRole::User => "主人",
                MessageRole::Assistant => "LingXi",
                MessageRole::System => "系统",
                MessageRole::Tool => "工具",
            };
            let ts: SharedString = msg.created_at.format("%H:%M").to_string().into();
            let avatar_fallback = if is_user { "主" } else { "灵" };
            let md_id: SharedString = format!("chat.msg.md.{}.{}", msg.id, m_idx).into();

            let mut row = Message::new(("chat.msg", m_idx))
                .density(MessageDensity::Standard)
                .avatar(
                    Avatar::new(("chat.msg.avatar", m_idx))
                        .size(AvatarSize::Sm)
                        .fallback(avatar_fallback),
                )
                .author(author_name)
                .timestamp(ts)
                .when(!is_user, |m| {
                    m.header_child(
                        div()
                            .text_size(typo::CAPTION)
                            .text_color(theme.colors.muted_foreground)
                            .child(thread.model_selection.provider.display_name()),
                    )
                })
                .child(
                    div()
                        .w_full()
                        .min_w_0()
                        .text_size(typo::CHAT_PROSE)
                        .line_height(typo::CHAT_PROSE_LH)
                        .child(markdown(md_id, msg.content.clone(), cx)),
                );

            for (t_idx, tool) in msg.tool_calls.iter().enumerate() {
                let tool_key = format!("{}:{}", msg.id, tool.id);
                let is_open = state.expanded_tool_calls.contains(&tool_key);
                let tool_key_click = tool_key.clone();
                let d_tog = Arc::clone(dispatcher);

                let (status_text, status_variant) = match tool.status {
                    ToolCallStatus::Pending | ToolCallStatus::Running => {
                        ("执行中", BadgeVariant::Default)
                    }
                    ToolCallStatus::Completed => ("完成", BadgeVariant::Secondary),
                    ToolCallStatus::Failed => ("失败", BadgeVariant::Destructive),
                    ToolCallStatus::Denied => ("已拒绝", BadgeVariant::Outline),
                };

                let mut body = div()
                    .px(px(12.0))
                    .py(px(10.0))
                    .rounded(radius::CONTROL)
                    .bg(surface::quiet_panel(theme))
                    .font_family(theme.fonts.mono.clone())
                    .text_size(typo::CODE)
                    .line_height(typo::CODE_LH)
                    .text_color(theme.colors.foreground)
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(format!("参数: {}", tool.input_preview));

                if let Some(output) = &tool.output_preview {
                    body = body.child(
                        div()
                            .pt(px(6.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .text_color(theme.colors.muted_foreground)
                            .child(output.clone()),
                    );
                }

                row = row.child(
                    div().w_full().pt(px(6.0)).child(
                        collapsible(("chat.tool", m_idx * 100 + t_idx), is_open, window, cx)
                            .child(
                                collapsible_trigger(
                                    ("chat.tool.trig", m_idx * 100 + t_idx),
                                    is_open,
                                    cx,
                                )
                                .w_full()
                                .justify_start()
                                .on_click(move |_, _, _| {
                                    d_tog(UiAction::ToggleToolCallExpanded(tool_key_click.clone()))
                                })
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(8.0))
                                        .child(icon_xs(
                                            LucideIcon::Wrench,
                                            theme.colors.muted_foreground,
                                        ))
                                        .child(
                                            div()
                                                .text_size(typo::UI_BODY)
                                                .font_weight(FontWeight::MEDIUM)
                                                .child(tool.name.clone()),
                                        )
                                        .child(Badge::new(status_text).variant(status_variant)),
                                ),
                            )
                            .child(collapsible_content(cx).child(body)),
                    ),
                );
            }

            if is_user {
                feed = feed.child(
                    div()
                        .w_full()
                        .rounded(radius::BUBBLE)
                        .bg(surface::user_bubble(theme))
                        .px(px(16.0))
                        .py(px(12.0))
                        .child(row),
                );
            } else {
                // Agent messages sit directly on Level 0 reading canvas with no distracting card border
                feed = feed.child(div().w_full().px(px(4.0)).py(px(2.0)).child(row));
            }
        }
    }

    for (a_idx, approval) in approvals.iter().enumerate() {
        let tid = thread.id.clone();
        let aid = approval.id.clone();
        let d_once = Arc::clone(dispatcher);
        let d_session = Arc::clone(dispatcher);
        let d_deny = Arc::clone(dispatcher);
        let d_modal = Arc::clone(dispatcher);
        let aid_modal = approval.id.clone();

        feed = feed.child(
            div()
                .w_full()
                .p(px(16.0))
                .rounded(radius::CARD)
                .border_1()
                .border_color(theme.colors.border)
                .bg(surface::quiet_panel(theme))
                .flex()
                .flex_col()
                .gap(px(10.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    lucide(LucideIcon::ShieldAlert)
                                        .size(px(16.0))
                                        .text_color(theme.colors.warning),
                                )
                                .child(
                                    div()
                                        .text_size(typo::UI_BODY)
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(theme.colors.foreground)
                                        .child(format!("请求权限确认 · {}", approval.tool_name)),
                                ),
                        )
                        .child(
                            Button::new(("chat.approval.detail", a_idx))
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Xs)
                                .label("详情")
                                .on_click(move |_, _, _| {
                                    d_modal(UiAction::OpenApprovalModal(aid_modal.clone()))
                                }),
                        ),
                )
                .child(
                    div()
                        .px(px(12.0))
                        .py(px(10.0))
                        .rounded(radius::CONTROL)
                        .bg(theme.colors.card)
                        .font_family(theme.fonts.mono.clone())
                        .text_size(typo::CODE)
                        .line_height(typo::CODE_LH)
                        .text_color(theme.colors.foreground)
                        .child(approval.summary.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child({
                            let t = tid.clone();
                            let a = aid.clone();
                            Button::new(("chat.approval.once", a_idx))
                                .variant(ButtonVariant::Default)
                                .size(ButtonSize::Sm)
                                .label("仅本次允许")
                                .on_click(move |_, _, _| {
                                    d_once(UiAction::ResolveApproval {
                                        thread_id: t.clone(),
                                        approval_id: a.clone(),
                                        decision: PermissionDecision::AllowOnce,
                                    })
                                })
                        })
                        .child({
                            let t = tid.clone();
                            let a = aid.clone();
                            Button::new(("chat.approval.session", a_idx))
                                .variant(ButtonVariant::Secondary)
                                .size(ButtonSize::Sm)
                                .label("本会话允许")
                                .on_click(move |_, _, _| {
                                    d_session(UiAction::ResolveApproval {
                                        thread_id: t.clone(),
                                        approval_id: a.clone(),
                                        decision: PermissionDecision::AllowSession,
                                    })
                                })
                        })
                        .child(
                            Button::new(("chat.approval.deny", a_idx))
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm)
                                .label("拒绝")
                                .on_click(move |_, _, _| {
                                    d_deny(UiAction::ResolveApproval {
                                        thread_id: tid.clone(),
                                        approval_id: aid.clone(),
                                        decision: PermissionDecision::Deny,
                                    })
                                }),
                        ),
                ),
        );
    }

    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(header)
        .child(
            ScrollArea::new("chat.timeline.scroll")
                .aria_label("对话消息列表")
                .flex_1()
                .min_h_0()
                .w_full()
                .child(feed),
        )
}
