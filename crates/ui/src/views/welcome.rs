use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, prelude::FluentBuilder as _, px,
};
use gpuicn::badge::{Badge, BadgeVariant};
use gpuicn::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpuicn::kbd::Kbd;
use gpuicn::theme::UiTheme;

use crate::components::fox_brand_mark;
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, SettingsSection, UiAction};

#[allow(clippy::too_many_arguments)]
fn quick_action_card(
    id: &'static str,
    icon: LucideIcon,
    title: &'static str,
    description: impl Into<SharedString>,
    shortcut: Option<&'static str>,
    theme: &UiTheme,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut gpui_kit::Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w_full()
        .p(px(16.0))
        .rounded(px(10.0))
        .bg(theme.colors.card)
        .border_1()
        .border_color(theme.colors.border)
        .cursor_pointer()
        .hover(|s| {
            s.border_color(theme.colors.primary.opacity(0.6))
                .bg(theme.colors.accent.opacity(0.35))
        })
        .on_click(on_click)
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
                        .w(px(32.0))
                        .h(px(32.0))
                        .rounded(px(8.0))
                        .bg(theme.colors.accent)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(lucide(icon).size(px(16.0)).text_color(theme.colors.primary)),
                )
                .when_some(shortcut, |el, sc| el.child(Kbd::new(sc))),
        )
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.foreground)
                .text_left()
                .child(title),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(theme.colors.muted_foreground)
                .text_left()
                .line_height(px(19.0))
                .child(description.into()),
        )
}

pub fn render_welcome(
    state: &AppUiState,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _cx: &App,
) -> impl IntoElement {
    let available_providers: Vec<_> = state
        .provider_statuses
        .iter()
        .filter(|p| p.install_state == lingxi_domain::ProviderInstallState::Installed)
        .collect();
    let total_providers = state.provider_statuses.len().max(12);

    let d_open = Arc::clone(dispatcher);
    let d_new = Arc::clone(dispatcher);
    let d_import = Arc::clone(dispatcher);
    let d_providers = Arc::clone(dispatcher);

    let first_project_id = state.projects.first().map(|p| p.id.clone());

    let legacy_desc: SharedString = match &state.legacy_db_path {
        Some(path) => format!("检测到原版数据库：{path}（仅只读导入，绝不修改原库）").into(),
        None => "从 ~/.lingxi-t3 只读迁移工作区与历史会话（自动备份）".into(),
    };

    let provider_desc: SharedString = format!(
        "已检测到 {} / {} 个本地智能体 CLI，支持自定义 ACP 命令",
        available_providers.len(),
        total_providers
    )
    .into();

    div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .items_center()
        .justify_center()
        .p(px(28.0))
        .child(
            Empty::new()
                .max_w(px(720.0))
                .header(
                    EmptyHeader::new()
                        .media(EmptyMedia::new().child(fox_brand_mark(48.0, theme)))
                        .title(EmptyTitle::new().child("欢迎使用 LingXi T3 Native"))
                        .description(EmptyDescription::new().child(
                            "基于 Rust + GPUI 构建的原生多智能体编程工作台。选择左侧工作区会话，或从下方快捷入口开始。",
                        )),
                )
                .content(
                    EmptyContent::new()
                        .w_full()
                        .max_w(px(640.0))
                        .gap(px(16.0))
                        .child(
                            div()
                                .w_full()
                                .grid()
                                .grid_cols(2)
                                .gap(px(12.0))
                                .child(quick_action_card(
                                    "welcome.open_project",
                                    LucideIcon::FolderOpen,
                                    "打开本地工作区",
                                    "添加本地 Git 仓库或项目文件夹到工作区树",
                                    None,
                                    theme,
                                    move |_, _, _| d_open(UiAction::OpenWorkspaceDialog),
                                ))
                                .child(quick_action_card(
                                    "welcome.new_thread",
                                    LucideIcon::MessageSquarePlus,
                                    "新建 Agent 会话",
                                    "立即启动一个新的多智能体协作会话",
                                    Some("⌘N"),
                                    theme,
                                    move |_, _, _| {
                                        if let Some(pid) = &first_project_id {
                                            d_new(UiAction::CreateThread {
                                                project_id: pid.clone(),
                                                worktree: false,
                                            });
                                        } else {
                                            d_new(UiAction::OpenWorkspaceDialog);
                                        }
                                    },
                                ))
                                .child(quick_action_card(
                                    "welcome.import",
                                    LucideIcon::DatabaseBackup,
                                    "只读导入原版数据",
                                    legacy_desc,
                                    None,
                                    theme,
                                    move |_, _, _| {
                                        d_import(UiAction::OpenSettings(
                                            SettingsSection::LegacyImport,
                                        ))
                                    },
                                ))
                                .child(quick_action_card(
                                    "welcome.providers",
                                    LucideIcon::Cpu,
                                    "配置智能体 Provider",
                                    provider_desc,
                                    Some("⌘,"),
                                    theme,
                                    move |_, _, _| {
                                        d_providers(UiAction::OpenSettings(
                                            SettingsSection::Providers,
                                        ))
                                    },
                                )),
                        )
                        .when(!available_providers.is_empty(), |el| {
                            el.child(
                                div()
                                    .w_full()
                                    .pt(px(8.0))
                                    .flex()
                                    .flex_wrap()
                                    .items_center()
                                    .justify_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.muted_foreground)
                                            .mr(px(4.0))
                                            .child("本机已就绪:"),
                                    )
                                    .children(available_providers.into_iter().take(6).map(|p| {
                                        let label = match &p.version {
                                            Some(v) => {
                                                format!("{} ({v})", p.kind.display_name())
                                            }
                                            None => p.kind.display_name().to_string(),
                                        };
                                        Badge::new(label)
                                            .variant(BadgeVariant::Secondary)
                                            .icon(LucideIcon::Check)
                                    })),
                            )
                        }),
                ),
        )
}
