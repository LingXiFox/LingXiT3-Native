use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, px,
};
use gpuicn::composer::ComposerState;
use gpuicn::theme::UiTheme;

use crate::components::fox_brand_mark;
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, SettingsSection, UiAction};
use crate::tokens::{space, surface, text, typo};
use crate::views::composer::render_composer_box;

fn subtle_action_chip(
    id: &'static str,
    icon: LucideIcon,
    label: impl Into<SharedString>,
    theme: &UiTheme,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover_bg = surface::subtle_chip_hover(theme);
    div()
        .id(id)
        .h(px(30.0))
        .px(px(12.0))
        .rounded(px(10.0))
        .bg(surface::subtle_chip(theme))
        .hover(move |s| s.bg(hover_bg))
        .cursor_pointer()
        .on_click(on_click)
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            lucide(icon)
                .size(px(13.5))
                .text_color(text::tertiary(theme)),
        )
        .child(
            div()
                .text_size(typo::CAPTION)
                .font_weight(FontWeight::NORMAL)
                .text_color(text::secondary(theme))
                .child(label.into()),
        )
}

pub fn render_welcome(
    state: &AppUiState,
    composer: &Entity<ComposerState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let d_open = Arc::clone(dispatcher);
    let d_import = Arc::clone(dispatcher);
    let d_cmd = Arc::clone(dispatcher);
    let d_providers = Arc::clone(dispatcher);

    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px(px(32.0))
        .pb(px(36.0))
        .child(
            div()
                .w_full()
                .max_w(space::WELCOME_COMPOSER_MAX_WIDTH)
                .flex()
                .flex_col()
                .items_center()
                .gap(px(26.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(12.0))
                        .child(fox_brand_mark(30.0, theme))
                        .child(
                            div()
                                .text_size(typo::HERO_TITLE)
                                .line_height(typo::HERO_TITLE_LH)
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(text::primary(theme))
                                .child("今天想一起构建什么？"),
                        ),
                )
                .child(div().w_full().child(render_composer_box(
                    "welcome.composer",
                    state,
                    None,
                    false,
                    composer,
                    dispatcher,
                    theme,
                    window,
                    cx,
                )))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .child(subtle_action_chip(
                            "welcome.chip.open",
                            LucideIcon::FolderOpen,
                            "打开工作区",
                            theme,
                            move |_, _, _| d_open(UiAction::OpenWorkspaceDialog),
                        ))
                        .child(subtle_action_chip(
                            "welcome.chip.import",
                            LucideIcon::DatabaseBackup,
                            "导入原版",
                            theme,
                            move |_, _, _| {
                                d_import(UiAction::OpenSettings(SettingsSection::LegacyImport))
                            },
                        ))
                        .child(subtle_action_chip(
                            "welcome.chip.cmd",
                            LucideIcon::Search,
                            "命令面板",
                            theme,
                            move |_, _, _| d_cmd(UiAction::OpenCommandPalette),
                        ))
                        .child(subtle_action_chip(
                            "welcome.chip.providers",
                            LucideIcon::SlidersHorizontal,
                            "配置智能体",
                            theme,
                            move |_, _, _| {
                                d_providers(UiAction::OpenSettings(SettingsSection::Providers))
                            },
                        )),
                ),
        )
}
