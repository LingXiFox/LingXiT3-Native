use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::base::Button as BaseButton;
use gpui_kit::{
    Anchor, App, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant, style_button};
use gpuicn::popover::{Popover, popover_popup};
use gpuicn::theme::UiTheme;
use lingxi_domain::{ModelSelection, ProviderInstallState, ProviderKind, RuntimeMode, ThreadId};

use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, UiAction};
use crate::tokens::{elevation, radius, text, typo};

fn thinking_label(level: Option<&str>) -> &'static str {
    match level.unwrap_or("high") {
        "low" => "低",
        "medium" => "中",
        "max" | "xhigh" => "极高",
        _ => "高",
    }
}

fn short_model_name(selection: &ModelSelection) -> String {
    let slug = selection.model_slug.trim();
    if slug.is_empty() || slug == "default" {
        return selection.provider.display_name().to_string();
    }
    slug.to_string()
}

fn runtime_mode_short_label(mode: RuntimeMode) -> &'static str {
    match mode {
        RuntimeMode::ApprovalRequired => "确认",
        RuntimeMode::AutoAcceptEdits => "自动编辑",
        RuntimeMode::Auto => "自动",
        RuntimeMode::FullAccess => "完全自主",
    }
}

fn runtime_mode_desc(mode: RuntimeMode) -> &'static str {
    match mode {
        RuntimeMode::ApprovalRequired => "执行高风险命令或写入前确认",
        RuntimeMode::AutoAcceptEdits => "自动接受文件修改，命令仍需确认",
        RuntimeMode::Auto => "按任务上下文自动调度执行",
        RuntimeMode::FullAccess => "全自主运行，不再弹出确认",
    }
}

/// Lightweight Claude Desktop-style ghost model + thinking level selector.
pub fn render_model_picker(
    id_prefix: &'static str,
    state: &AppUiState,
    thread_id: Option<ThreadId>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _window: &mut Window,
    _cx: &mut App,
) -> impl IntoElement + use<> {
    let current = state.effective_model_selection();
    let model_text = short_model_name(&current);
    let effort_text = thinking_label(current.thinking_level.as_deref());
    let tooltip_label: SharedString = format!(
        "{} · {} · 思考等级: {}",
        current.provider.display_name(),
        current.model_slug,
        effort_text
    )
    .into();

    let trigger_base = style_button(
        BaseButton::new(SharedString::from(format!("{id_prefix}.trigger")))
            .aria_label(tooltip_label.clone())
            .h(px(28.0))
            .px(px(8.0))
            .rounded(radius::CONTROL)
            .gap(px(5.0))
            .child(
                div()
                    .max_w(px(148.0))
                    .truncate()
                    .text_size(typo::CAPTION)
                    .font_weight(FontWeight::NORMAL)
                    .text_color(text::secondary(theme))
                    .child(model_text),
            )
            .child(
                div()
                    .text_size(typo::MICRO)
                    .font_weight(FontWeight::NORMAL)
                    .text_color(text::tertiary(theme))
                    .child(effort_text),
            )
            .child(
                lucide(LucideIcon::ChevronDown)
                    .size(px(12.0))
                    .text_color(text::tertiary(theme)),
            ),
        false,
        ButtonVariant::Ghost,
        ButtonSize::Sm,
        theme,
    );

    let providers = state.provider_statuses.clone();
    let dispatcher = Arc::clone(dispatcher);
    let theme_clone = theme.clone();

    Popover::new(SharedString::from(format!("{id_prefix}.popover")))
        .anchor(Anchor::BottomRight)
        .offset(px(6.0))
        .trigger(trigger_base)
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let levels = [
                ("low", "低"),
                ("medium", "中"),
                ("high", "高"),
                ("max", "极高"),
            ];
            let active_level = current.thinking_level.as_deref().unwrap_or("high");

            let mut effort_row = div().flex().items_center().gap(px(4.0));
            for (idx, (slug, label)) in levels.into_iter().enumerate() {
                let is_active = active_level == slug;
                let mut next_sel = current.clone();
                next_sel.thinking_level = Some(slug.to_string());
                let tid = thread_id.clone();
                let d_eff = Arc::clone(&dispatcher);
                let pop = popover.clone();

                effort_row = effort_row.child(
                    Button::new((id_prefix, idx))
                        .variant(if is_active {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::Xs)
                        .flex_1()
                        .label(label)
                        .on_click(move |_, window, cx| {
                            if let Some(tid) = tid.clone() {
                                d_eff(UiAction::SelectModel {
                                    thread_id: tid,
                                    selection: next_sel.clone(),
                                });
                            } else {
                                d_eff(UiAction::SelectDefaultModel(next_sel.clone()));
                            }
                            pop.update(cx, |p, cx| p.dismiss(window, cx));
                        }),
                );
            }

            let mut list = div()
                .id(SharedString::from(format!("{id_prefix}.list")))
                .max_h(px(248.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(2.0));

            for (idx, kind) in ProviderKind::ALL.into_iter().enumerate() {
                let status = providers.iter().find(|s| s.kind == kind);
                let installed = status
                    .map(|s| s.install_state == ProviderInstallState::Installed)
                    .unwrap_or(false);
                let default_slug = status
                    .and_then(|s| s.models.iter().find(|m| m.is_default).or(s.models.first()))
                    .map(|m| m.slug.clone())
                    .unwrap_or_else(|| "default".to_string());
                let is_selected = current.provider == kind;

                let next_sel = ModelSelection {
                    provider: kind,
                    model_slug: default_slug.clone(),
                    thinking_level: current
                        .thinking_level
                        .clone()
                        .or_else(|| Some("high".to_string())),
                    planner_model: None,
                    executor_model: None,
                };
                let tid = thread_id.clone();
                let d_prov = Arc::clone(&dispatcher);
                let pop = popover.clone();

                list = list.child(
                    div()
                        .id((id_prefix, 100 + idx))
                        .px(px(8.0))
                        .py(px(6.0))
                        .rounded(radius::CONTROL)
                        .cursor_pointer()
                        .when(is_selected, |el| el.bg(theme_clone.colors.muted))
                        .hover(|s| s.bg(theme_clone.colors.muted))
                        .on_click(move |_, window, cx| {
                            if let Some(tid) = tid.clone() {
                                d_prov(UiAction::SelectModel {
                                    thread_id: tid,
                                    selection: next_sel.clone(),
                                });
                            } else {
                                d_prov(UiAction::SelectDefaultModel(next_sel.clone()));
                            }
                            pop.update(cx, |p, cx| p.dismiss(window, cx));
                        })
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .items_center()
                                .gap(px(7.0))
                                .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(
                                    if installed {
                                        theme_clone.colors.success
                                    } else {
                                        theme_clone.colors.border
                                    },
                                ))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(typo::CAPTION)
                                                .font_weight(if is_selected {
                                                    FontWeight::MEDIUM
                                                } else {
                                                    FontWeight::NORMAL
                                                })
                                                .text_color(text::primary(&theme_clone))
                                                .child(kind.display_name()),
                                        )
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(typo::MICRO)
                                                .text_color(text::tertiary(&theme_clone))
                                                .child(default_slug),
                                        ),
                                ),
                        )
                        .when(is_selected, |el| {
                            el.child(
                                lucide(LucideIcon::Check)
                                    .size(px(14.0))
                                    .text_color(theme_clone.colors.primary),
                            )
                        }),
                );
            }

            popover_popup(
                SharedString::from(format!("{id_prefix}.popup")),
                "选择模型与思考等级",
                cx,
            )
            .w(px(284.0))
            .rounded(radius::MODAL)
            .border_1()
            .border_color(theme_clone.colors.border)
            .bg(theme_clone.colors.popover)
            .shadow(elevation::for_level2(&theme_clone))
            .p(px(10.0))
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .px(px(2.0))
                            .text_size(typo::MICRO)
                            .text_color(text::tertiary(&theme_clone))
                            .child("思考等级"),
                    )
                    .child(effort_row),
            )
            .child(div().h(px(1.0)).w_full().bg(theme_clone.colors.border))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .px(px(2.0))
                            .text_size(typo::MICRO)
                            .text_color(text::tertiary(&theme_clone))
                            .child("智能体与模型"),
                    )
                    .child(list),
            )
        })
}

/// Lightweight ghost runtime-mode selector (borderless by default).
pub fn render_mode_picker(
    id_prefix: &'static str,
    state: &AppUiState,
    thread_id: Option<ThreadId>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _window: &mut Window,
    _cx: &mut App,
) -> impl IntoElement + use<> {
    let mode = state.effective_runtime_mode();
    let short_label = runtime_mode_short_label(mode);
    let tooltip_label: SharedString = format!("执行模式：{}", runtime_mode_desc(mode)).into();

    let trigger_base = style_button(
        BaseButton::new(SharedString::from(format!("{id_prefix}.trigger")))
            .aria_label(tooltip_label.clone())
            .h(px(28.0))
            .px(px(8.0))
            .rounded(radius::CONTROL)
            .gap(px(5.0))
            .child(
                lucide(LucideIcon::ShieldCheck)
                    .size(px(13.0))
                    .text_color(text::tertiary(theme)),
            )
            .child(
                div()
                    .text_size(typo::CAPTION)
                    .text_color(text::secondary(theme))
                    .child(short_label),
            )
            .child(
                lucide(LucideIcon::ChevronDown)
                    .size(px(11.0))
                    .text_color(text::tertiary(theme)),
            ),
        false,
        ButtonVariant::Ghost,
        ButtonSize::Sm,
        theme,
    );

    let dispatcher = Arc::clone(dispatcher);
    let theme_clone = theme.clone();

    Popover::new(SharedString::from(format!("{id_prefix}.mode.popover")))
        .anchor(Anchor::BottomLeft)
        .offset(px(6.0))
        .trigger(trigger_base)
        .content(move |_, _, cx| {
            let popover = cx.entity();
            let modes = [
                RuntimeMode::ApprovalRequired,
                RuntimeMode::AutoAcceptEdits,
                RuntimeMode::Auto,
                RuntimeMode::FullAccess,
            ];

            let mut list = div().flex().flex_col().gap(px(2.0));
            for (idx, m) in modes.into_iter().enumerate() {
                let selected = m == mode;
                let tid = thread_id.clone();
                let d_m = Arc::clone(&dispatcher);
                let pop = popover.clone();

                list = list.child(
                    div()
                        .id((id_prefix, 200 + idx))
                        .px(px(8.0))
                        .py(px(6.0))
                        .rounded(radius::CONTROL)
                        .cursor_pointer()
                        .when(selected, |el| el.bg(theme_clone.colors.muted))
                        .hover(|s| s.bg(theme_clone.colors.muted))
                        .on_click(move |_, window, cx| {
                            if let Some(tid) = tid.clone() {
                                d_m(UiAction::SelectRuntimeMode {
                                    thread_id: tid,
                                    mode: m,
                                });
                            } else {
                                d_m(UiAction::SelectDefaultRuntimeMode(m));
                            }
                            pop.update(cx, |p, cx| p.dismiss(window, cx));
                        })
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .text_size(typo::CAPTION)
                                        .font_weight(if selected {
                                            FontWeight::MEDIUM
                                        } else {
                                            FontWeight::NORMAL
                                        })
                                        .text_color(text::primary(&theme_clone))
                                        .child(runtime_mode_short_label(m)),
                                )
                                .child(
                                    div()
                                        .text_size(typo::MICRO)
                                        .text_color(text::tertiary(&theme_clone))
                                        .child(runtime_mode_desc(m)),
                                ),
                        )
                        .when(selected, |el| {
                            el.child(
                                lucide(LucideIcon::Check)
                                    .size(px(14.0))
                                    .text_color(theme_clone.colors.primary),
                            )
                        }),
                );
            }

            popover_popup(
                SharedString::from(format!("{id_prefix}.mode.popup")),
                "选择执行权限模式",
                cx,
            )
            .w(px(244.0))
            .rounded(radius::MODAL)
            .border_1()
            .border_color(theme_clone.colors.border)
            .bg(theme_clone.colors.popover)
            .shadow(elevation::for_level2(&theme_clone))
            .p(px(8.0))
            .child(list)
        })
}
