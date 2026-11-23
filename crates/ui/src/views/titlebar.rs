use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, prelude::FluentBuilder as _, px,
};
use gpuicn::badge::{Badge, BadgeVariant};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::kbd::Kbd;
use gpuicn::theme::UiTheme;
use gpuicn::title_bar::TitleBar;
use lingxi_domain::ThemeAppearance;

use crate::components::{fox_brand_mark, icon_xs};
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, SettingsSection, UiAction};
use crate::tokens::typo;

pub fn render_titlebar(
    state: &AppUiState,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _cx: &App,
) -> impl IntoElement {
    let sidebar_icon = if state.sidebar_collapsed {
        LucideIcon::PanelLeftOpen
    } else {
        LucideIcon::PanelLeftClose
    };
    let theme_icon = match state.settings.appearance {
        ThemeAppearance::Dark => LucideIcon::Sun,
        ThemeAppearance::Light | ThemeAppearance::System => LucideIcon::Moon,
    };

    let d_sidebar = Arc::clone(dispatcher);
    let d_cmd = Arc::clone(dispatcher);
    let d_inspector = Arc::clone(dispatcher);
    let d_theme = Arc::clone(dispatcher);
    let d_update = Arc::clone(dispatcher);
    let d_settings = Arc::clone(dispatcher);

    let active_project_name = state
        .active_project()
        .map(|p| p.title.clone())
        .unwrap_or_else(|| "LingXi T3 Native".to_string());

    let mut tab_strip = div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(4.0))
        .overflow_hidden();

    if state.open_tabs.is_empty() {
        tab_strip = tab_strip.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(6.0))
                .text_size(typo::CAPTION)
                .text_color(theme.colors.muted_foreground)
                .child(active_project_name),
        );
    } else {
        for (idx, tid) in state.open_tabs.iter().take(6).enumerate() {
            if let Some(thread) = state.threads.iter().find(|t| &t.id == tid) {
                let is_active = state.active_thread_id.as_ref() == Some(tid);
                let tid_select = tid.clone();
                let tid_close = tid.clone();
                let d_sel = Arc::clone(dispatcher);
                let d_cls = Arc::clone(dispatcher);
                let title: SharedString = thread.title.clone().into();
                tab_strip = tab_strip.child(
                    div()
                        .id(("titlebar-tab", idx))
                        .h(px(26.0))
                        .max_w(px(190.0))
                        .px(px(10.0))
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(if is_active {
                            theme.colors.border
                        } else {
                            gpui_kit::transparent_black().into()
                        })
                        .bg(if is_active {
                            theme.colors.card
                        } else {
                            gpui_kit::transparent_black().into()
                        })
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .cursor_pointer()
                        .on_click(move |_, _, _| d_sel(UiAction::SelectThread(tid_select.clone())))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(typo::CAPTION)
                                .text_color(if is_active {
                                    theme.colors.foreground
                                } else {
                                    theme.colors.muted_foreground
                                })
                                .child(title),
                        )
                        .child(
                            div()
                                .id(("titlebar-tab-close", idx))
                                .w(px(16.0))
                                .h(px(16.0))
                                .rounded(px(4.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .hover(|s| s.bg(theme.colors.muted))
                                .on_click(move |_, _, cx| {
                                    cx.stop_propagation();
                                    d_cls(UiAction::CloseTab(tid_close.clone()));
                                })
                                .child(icon_xs(LucideIcon::X, theme.colors.muted_foreground)),
                        ),
                );
            }
        }
    }

    let left_pad = if cfg!(target_os = "macos") {
        px(74.0)
    } else {
        px(10.0)
    };

    let current_inspector_tab = state.inspector_tab;

    TitleBar::new()
        .page(true)
        .pl(left_pad)
        .pr(px(12.0))
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .flex_shrink_0()
                .child(
                    Button::new("tb.sidebar")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .aria_label("切换侧边栏")
                        .on_click(move |_, _, _| d_sidebar(UiAction::ToggleSidebar))
                        .child(
                            lucide(sidebar_icon)
                                .size(px(15.0))
                                .text_color(theme.colors.foreground),
                        ),
                )
                .child(fox_brand_mark(20.0, theme))
                .child(
                    div()
                        .text_size(typo::UI_BODY)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.colors.foreground)
                        .child("LingXi T3"),
                ),
        )
        .child(tab_strip)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .flex_shrink_0()
                .child(
                    Button::new("tb.cmd")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm)
                        .on_click(move |_, _, _| d_cmd(UiAction::OpenCommandPalette))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .child(icon_xs(LucideIcon::Search, theme.colors.muted_foreground))
                                .child(
                                    div()
                                        .text_size(typo::CAPTION)
                                        .text_color(theme.colors.muted_foreground)
                                        .child("命令面板"),
                                )
                                .child(Kbd::new("⌘K")),
                        ),
                )
                .child(
                    Button::new("tb.inspector")
                        .variant(if state.inspector_open {
                            ButtonVariant::Secondary
                        } else {
                            ButtonVariant::Ghost
                        })
                        .size(ButtonSize::IconSm)
                        .aria_label("展开/收起工作区工具面板")
                        .on_click(move |_, _, _| {
                            d_inspector(UiAction::ToggleInspector(current_inspector_tab))
                        })
                        .child(lucide(LucideIcon::PanelRight).size(px(15.0)).text_color(
                            if state.inspector_open {
                                theme.colors.primary
                            } else {
                                theme.colors.foreground
                            },
                        )),
                )
                .child(
                    Button::new("tb.theme")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .aria_label("切换浅色/深色主题")
                        .on_click(move |_, _, _| d_theme(UiAction::ToggleAppearance))
                        .child(
                            lucide(theme_icon)
                                .size(px(15.0))
                                .text_color(theme.colors.foreground),
                        ),
                )
                .child(
                    Button::new("tb.update")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .aria_label("检查更新")
                        .on_click(move |_, _, _| d_update(UiAction::OpenUpdateModal))
                        .child(
                            div()
                                .relative()
                                .child(
                                    lucide(LucideIcon::Download)
                                        .size(px(15.0))
                                        .text_color(theme.colors.foreground),
                                )
                                .when(state.available_update.is_some(), |el| {
                                    el.child(
                                        div()
                                            .absolute()
                                            .top(px(-1.0))
                                            .right(px(-1.0))
                                            .w(px(6.0))
                                            .h(px(6.0))
                                            .rounded_full()
                                            .bg(theme.colors.primary),
                                    )
                                }),
                        ),
                )
                .child(
                    Button::new("tb.settings")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .aria_label("设置中心")
                        .on_click(move |_, _, _| {
                            d_settings(UiAction::OpenSettings(SettingsSection::General))
                        })
                        .child(
                            lucide(LucideIcon::Settings)
                                .size(px(15.0))
                                .text_color(theme.colors.foreground),
                        ),
                )
                .when(!state.approvals_by_thread.is_empty(), |el| {
                    let count: usize = state.approvals_by_thread.values().map(Vec::len).sum();
                    el.when(count > 0, |el2| {
                        el2.child(Badge::count(count, 9).variant(BadgeVariant::Default))
                    })
                }),
        )
}
