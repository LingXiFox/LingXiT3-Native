use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::composer::{Composer, ComposerState};
use gpuicn::select::{Select, SelectState};
use gpuicn::theme::UiTheme;

use crate::components::fox_brand_mark;
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, SettingsSection, UiAction};
use crate::tokens::{elevation, radius, space, typo};

fn quick_pill(
    id: &'static str,
    icon: LucideIcon,
    label: impl Into<SharedString>,
    theme: &UiTheme,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(id)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    lucide(icon)
                        .size(px(14.0))
                        .text_color(theme.colors.muted_foreground),
                )
                .child(
                    div()
                        .text_size(typo::CAPTION)
                        .text_color(theme.colors.foreground)
                        .child(label.into()),
                ),
        )
}

#[allow(clippy::too_many_arguments)]
pub fn render_welcome(
    state: &AppUiState,
    composer: &Entity<ComposerState>,
    provider_select: &Entity<SelectState>,
    mode_select: &Entity<SelectState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    cx: &App,
) -> impl IntoElement {
    let available_count = state
        .provider_statuses
        .iter()
        .filter(|p| p.install_state == lingxi_domain::ProviderInstallState::Installed)
        .count();
    let total_count = state.provider_statuses.len().max(12);

    let draft = composer.read(cx).value(cx).to_string();
    let can_send = !draft.trim().is_empty();
    let composer_handle = composer.clone();

    let d_send = Arc::clone(dispatcher);
    let d_open = Arc::clone(dispatcher);
    let d_import = Arc::clone(dispatcher);
    let d_cmd = Arc::clone(dispatcher);
    let d_providers = Arc::clone(dispatcher);
    let d_providers_footer = Arc::clone(dispatcher);

    let secondary_line: SharedString = match &state.legacy_db_path {
        Some(_) => {
            format!("本机已探测 {available_count}/{total_count} 个智能体 · 检测到原版 LingXi T3 数据可只读迁移")
                .into()
        }
        None => format!("本机已探测 {available_count}/{total_count} 个智能体 · 点击配置 Provider").into(),
    };

    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .py(px(28.0))
        .child(
            div()
                .w_full()
                .max_w(space::WELCOME_COMPOSER_MAX_WIDTH)
                .flex()
                .flex_col()
                .items_center()
                .gap(px(22.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(12.0))
                        .child(fox_brand_mark(44.0, theme))
                        .child(
                            div()
                                .text_size(typo::HERO_TITLE)
                                .line_height(typo::HERO_TITLE_LH)
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(theme.colors.foreground)
                                .text_center()
                                .child("今天想一起构建什么？"),
                        )
                        .child(
                            div()
                                .text_size(typo::UI_BODY)
                                .line_height(typo::UI_BODY_LH)
                                .text_color(theme.colors.muted_foreground)
                                .text_center()
                                .child("输入任务立即开始对话，或打开本地工作区继续协作"),
                        ),
                )
                .child(
                    div().w_full().child(
                        Composer::new(composer)
                            .w_full()
                            .rounded(radius::COMPOSER)
                            .bg(theme.colors.card)
                            .border_1()
                            .border_color(theme.colors.border)
                            .shadow(elevation::for_level1(theme))
                            .p(px(6.0))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap(px(8.0))
                                    .pt(px(6.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .child(Select::new(provider_select).w(px(148.0)))
                                            .child(Select::new(mode_select).w(px(138.0))),
                                    )
                                    .child(
                                        Button::new("welcome.composer.send")
                                            .variant(ButtonVariant::Default)
                                            .size(ButtonSize::IconSm)
                                            .aria_label("开始新会话")
                                            .disabled(!can_send)
                                            .on_click(move |_, window, cx| {
                                                let text =
                                                    composer_handle.read(cx).value(cx).to_string();
                                                if text.trim().is_empty() {
                                                    return;
                                                }
                                                composer_handle.update(cx, |c, cx| {
                                                    c.set_value("", window, cx);
                                                });
                                                d_send(UiAction::StartQuickThread { prompt: text });
                                            })
                                            .child(
                                                lucide(LucideIcon::ArrowUp)
                                                    .size(px(15.0))
                                                    .text_color(theme.colors.primary_foreground),
                                            ),
                                    ),
                            ),
                    ),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .child(quick_pill(
                            "welcome.pill.open",
                            LucideIcon::FolderOpen,
                            "打开本地工作区",
                            theme,
                            move |_, _, _| d_open(UiAction::OpenWorkspaceDialog),
                        ))
                        .child(quick_pill(
                            "welcome.pill.import",
                            LucideIcon::DatabaseBackup,
                            "只读导入原版",
                            theme,
                            move |_, _, _| {
                                d_import(UiAction::OpenSettings(SettingsSection::LegacyImport))
                            },
                        ))
                        .child(quick_pill(
                            "welcome.pill.cmd",
                            LucideIcon::Search,
                            "命令面板 ⌘K",
                            theme,
                            move |_, _, _| d_cmd(UiAction::OpenCommandPalette),
                        ))
                        .child(quick_pill(
                            "welcome.pill.providers",
                            LucideIcon::SlidersHorizontal,
                            "配置 Provider",
                            theme,
                            move |_, _, _| {
                                d_providers(UiAction::OpenSettings(SettingsSection::Providers))
                            },
                        )),
                )
                .child(
                    div()
                        .id("welcome.status_link")
                        .pt(px(6.0))
                        .cursor_pointer()
                        .text_size(typo::CAPTION)
                        .text_color(theme.colors.muted_foreground)
                        .hover(|s| s.text_color(theme.colors.foreground))
                        .on_click(move |_, _, _| {
                            d_providers_footer(UiAction::OpenSettings(SettingsSection::Providers))
                        })
                        .child(secondary_line),
                ),
        )
}
