use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, FontWeight, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpuicn::badge::{Badge, BadgeVariant};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::command::{CommandState, command_dialog_popup};
use gpuicn::dialog::{
    DialogChangeReason, DialogHandle, dialog_description, dialog_header, dialog_popup,
    dialog_title, dialog_with,
};
use gpuicn::input::InputState;
use gpuicn::kbd::Kbd;
use gpuicn::progress::Progress;
use gpuicn::select::SelectState;
use gpuicn::theme::UiTheme;
use lingxi_domain::PermissionDecision;

use crate::shell::ActionDispatcher;
use crate::state::{ActiveModal, AppUiState, UiAction};
use crate::tokens::{elevation, radius, surface, typo};
use crate::views::settings::render_settings_popup_content;

pub struct ModalHandles {
    pub command: DialogHandle,
    pub settings: DialogHandle,
    pub update: DialogHandle,
    pub approval: DialogHandle,
}

impl ModalHandles {
    pub fn new(_cx: &mut App) -> Self {
        Self {
            command: DialogHandle::new(false),
            settings: DialogHandle::new(false),
            update: DialogHandle::new(false),
            approval: DialogHandle::new(false),
        }
    }

    pub fn sync(&self, active: ActiveModal, window: &mut Window, cx: &mut App) {
        let targets = [
            (ActiveModal::CommandPalette, &self.command),
            (ActiveModal::Settings, &self.settings),
            (ActiveModal::Update, &self.update),
            (ActiveModal::PermissionApproval, &self.approval),
        ];
        for (kind, handle) in targets {
            let should_open = active == kind;
            if should_open && !handle.is_open() {
                handle.open(window, cx);
            } else if !should_open && handle.is_open() {
                handle.close(window, cx);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_modals(
    state: &AppUiState,
    handles: &ModalHandles,
    command_state: &Entity<CommandState>,
    custom_acp_input: &Entity<InputState>,
    remote_port_input: &Entity<InputState>,
    update_channel_select: &Entity<SelectState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let d_cmd_close = Arc::clone(dispatcher);
    let cmd_dialog = dialog_with(
        "modal.command",
        &handles.command,
        |_, cx| {
            div().px(px(28.0)).py(px(24.0)).child(
                div()
                    .w(px(580.0))
                    .max_w(px(580.0))
                    .rounded(radius::MODAL)
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.popover)
                    .shadow(elevation::for_level2(theme))
                    .font_family(theme.fonts.body.clone())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(
                        command_dialog_popup("modal.command.popup", command_state, cx)
                            .w_full()
                            .max_w_full()
                            .border_0()
                            .shadow(Vec::new()),
                    )
                    .child(
                        div()
                            .px(px(14.0))
                            .py(px(8.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .bg(surface::subtle_footer(theme))
                            .flex()
                            .items_center()
                            .gap(px(14.0))
                            .text_size(typo::CAPTION)
                            .text_color(theme.colors.muted_foreground)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(Kbd::new("↑"))
                                    .child(Kbd::new("↓"))
                                    .child("导航"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(Kbd::new("Enter"))
                                    .child("选择"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(Kbd::new("Esc"))
                                    .child("关闭"),
                            ),
                    ),
            )
        },
        window,
        cx,
    )
    .on_open_change(move |open, reason, _, _| {
        if !open && reason != DialogChangeReason::Imperative {
            d_cmd_close(UiAction::CloseModal);
        }
    });

    let d_set_close = Arc::clone(dispatcher);
    let settings_dialog = dialog_with(
        "modal.settings",
        &handles.settings,
        |window, cx| {
            div().px(px(28.0)).py(px(24.0)).child(
                dialog_popup("modal.settings.popup", "设置中心", cx)
                    .w_full()
                    .max_w(px(780.0))
                    .p_0()
                    .border_0()
                    .shadow(Vec::new())
                    .child(render_settings_popup_content(
                        state,
                        custom_acp_input,
                        remote_port_input,
                        update_channel_select,
                        dispatcher,
                        theme,
                        window,
                        cx,
                    )),
            )
        },
        window,
        cx,
    )
    .on_open_change(move |open, reason, _, _| {
        if !open && reason != DialogChangeReason::Imperative {
            d_set_close(UiAction::CloseModal);
        }
    });

    let d_upd_close = Arc::clone(dispatcher);
    let d_upd_later = Arc::clone(dispatcher);
    let d_upd_check = Arc::clone(dispatcher);
    let update_dialog = dialog_with(
        "modal.update",
        &handles.update,
        |_, cx| {
            let (title, desc): (&str, SharedString) = match &state.available_update {
                Some(u) => (
                    "发现可用更新",
                    format!(
                        "最新版本 {} 已发布（{}），点击下方检查或下载更新。",
                        u.version, u.published_at
                    )
                    .into(),
                ),
                None if state.update_checking => (
                    "正在检查更新",
                    "正在查询 GitHub Releases 更新通道…".into(),
                ),
                None => (
                    "应用更新检查",
                    format!(
                        "当前版本 v{}（通道：{:?}）。点击「立即检查更新」从官方发布源获取最新版本。",
                        env!("CARGO_PKG_VERSION"),
                        state.settings.update_channel
                    )
                    .into(),
                ),
            };

            div().px(px(28.0)).py(px(24.0)).child(
                dialog_popup("modal.update.popup", "应用更新检查", cx)
                    .w(px(460.0))
                    .max_w(px(460.0))
                    .rounded(radius::MODAL)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow(elevation::for_level3(theme))
                    .font_family(theme.fonts.body.clone())
                    .child(
                        dialog_header(cx)
                            .child(dialog_title("modal.update.title", cx).child(title))
                            .child(dialog_description("modal.update.desc", cx).child(desc)),
                    )
                    .when_some(state.available_update.as_ref(), |el, info| {
                        el.child(
                            div()
                                .p(px(12.0))
                                .rounded(radius::CONTROL)
                                .bg(surface::quiet_panel(theme))
                                .text_size(typo::UI_BODY)
                                .line_height(typo::UI_BODY_LH)
                                .text_color(theme.colors.foreground)
                                .child(info.notes.clone()),
                        )
                    })
                    .when_some(state.update_download_progress, |el, prog| {
                        el.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    Progress::new("modal.update.progress")
                                        .value(f64::from(prog) * 100.0),
                                )
                                .child(
                                    div()
                                        .text_size(typo::CAPTION)
                                        .text_color(theme.colors.muted_foreground)
                                        .child(format!("下载进度 {:.0}%", prog * 100.0)),
                                ),
                        )
                    })
                    .child(
                        div()
                            .w_full()
                            .pt(px(8.0))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .child(
                                Button::new("modal.update.later")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm)
                                    .label("稍后")
                                    .on_click(move |_, _, _| d_upd_later(UiAction::CloseModal)),
                            )
                            .child(
                                Button::new("modal.update.check")
                                    .variant(ButtonVariant::Default)
                                    .size(ButtonSize::Sm)
                                    .label("立即检查更新")
                                    .on_click(move |_, _, _| d_upd_check(UiAction::CheckUpdates)),
                            ),
                    ),
            )
        },
        window,
        cx,
    )
    .on_open_change(move |open, reason, _, _| {
        if !open && reason != DialogChangeReason::Imperative {
            d_upd_close(UiAction::CloseModal);
        }
    });

    let d_app_close = Arc::clone(dispatcher);
    let d_app_once = Arc::clone(dispatcher);
    let d_app_deny = Arc::clone(dispatcher);
    let d_app_session = Arc::clone(dispatcher);

    let active_approval = state.current_modal_approval().cloned();
    let active_tid = state.active_thread_id.clone();

    let tid_once = active_tid.clone();
    let aid_once = active_approval.as_ref().map(|a| a.id.clone());
    let tid_deny = active_tid.clone();
    let aid_deny = active_approval.as_ref().map(|a| a.id.clone());
    let tid_sess = active_tid;
    let aid_sess = active_approval.as_ref().map(|a| a.id.clone());

    let approval_dialog = dialog_with(
        "modal.approval",
        &handles.approval,
        |_, cx| {
            let (tool_name, cmd_preview) = match &active_approval {
                Some(a) => (a.tool_name.clone(), a.summary.clone()),
                None => ("未知工具".to_string(), "当前暂无待确认操作".to_string()),
            };

            div().px(px(28.0)).py(px(24.0)).child(
                dialog_popup("modal.approval.popup", "工具执行权限确认", cx)
                    .w(px(500.0))
                    .max_w(px(500.0))
                    .rounded(radius::MODAL)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow(elevation::for_level3(theme))
                    .font_family(theme.fonts.body.clone())
                    .child(
                        dialog_header(cx)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        lucide(LucideIcon::ShieldAlert)
                                            .size(px(18.0))
                                            .text_color(theme.colors.warning),
                                    )
                                    .child(
                                        dialog_title("modal.approval.title", cx)
                                            .child("工具执行权限确认"),
                                    ),
                            )
                            .child(dialog_description("modal.approval.desc", cx).child(format!(
                                "Agent 请求执行以下受控操作（工具：{tool_name}），请确认是否授权："
                            ))),
                    )
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(radius::CONTROL)
                            .bg(surface::quiet_panel(theme))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(typo::CAPTION)
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.colors.muted_foreground)
                                            .child("命令 / 参数预览"),
                                    )
                                    .child(Badge::new(tool_name).variant(BadgeVariant::Outline)),
                            )
                            .child(
                                div()
                                    .font_family(theme.fonts.mono.clone())
                                    .text_size(typo::CODE)
                                    .line_height(typo::CODE_LH)
                                    .text_color(theme.colors.foreground)
                                    .child(cmd_preview),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .pt(px(8.0))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .child(
                                Button::new("modal.approval.deny")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Sm)
                                    .label("拒绝")
                                    .on_click(move |_, _, _| {
                                        if let (Some(t), Some(a)) = (&tid_deny, &aid_deny) {
                                            d_app_deny(UiAction::ResolveApproval {
                                                thread_id: t.clone(),
                                                approval_id: a.clone(),
                                                decision: PermissionDecision::Deny,
                                            });
                                        }
                                        d_app_deny(UiAction::CloseModal);
                                    }),
                            )
                            .child(
                                Button::new("modal.approval.session")
                                    .variant(ButtonVariant::Secondary)
                                    .size(ButtonSize::Sm)
                                    .label("本会话允许")
                                    .on_click(move |_, _, _| {
                                        if let (Some(t), Some(a)) = (&tid_sess, &aid_sess) {
                                            d_app_session(UiAction::ResolveApproval {
                                                thread_id: t.clone(),
                                                approval_id: a.clone(),
                                                decision: PermissionDecision::AllowSession,
                                            });
                                        }
                                        d_app_session(UiAction::CloseModal);
                                    }),
                            )
                            .child(
                                Button::new("modal.approval.once")
                                    .variant(ButtonVariant::Default)
                                    .size(ButtonSize::Sm)
                                    .label("仅本次允许")
                                    .on_click(move |_, _, _| {
                                        if let (Some(t), Some(a)) = (&tid_once, &aid_once) {
                                            d_app_once(UiAction::ResolveApproval {
                                                thread_id: t.clone(),
                                                approval_id: a.clone(),
                                                decision: PermissionDecision::AllowOnce,
                                            });
                                        }
                                        d_app_once(UiAction::CloseModal);
                                    }),
                            ),
                    ),
            )
        },
        window,
        cx,
    )
    .on_open_change(move |open, reason, _, _| {
        if !open && reason != DialogChangeReason::Imperative {
            d_app_close(UiAction::CloseModal);
        }
    });

    div()
        .child(cmd_dialog)
        .child(settings_dialog)
        .child(update_dialog)
        .child(approval_dialog)
}
