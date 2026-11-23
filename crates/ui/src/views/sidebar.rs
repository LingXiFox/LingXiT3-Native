use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::input::{Input, InputState};
use gpuicn::kbd::Kbd;
use gpuicn::sidebar::{
    Sidebar, SidebarItem, SidebarItemSize, sidebar_badge, sidebar_group_label, sidebar_separator,
};
use gpuicn::theme::UiTheme;

use crate::components::{icon_xs, status_dot};
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, InspectorTab, SettingsSection, UiAction};

pub fn render_sidebar(
    state: &AppUiState,
    filter_input: &Entity<InputState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    cx: &App,
) -> impl IntoElement {
    let collapsed = state.sidebar_collapsed;

    let d_add_proj = Arc::clone(dispatcher);
    let d_import = Arc::clone(dispatcher);
    let d_metrics = Arc::clone(dispatcher);
    let d_settings = Arc::clone(dispatcher);

    let header = if collapsed {
        div().flex().justify_center().child(
            Button::new("sb.add_project_collapsed")
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::IconSm)
                .aria_label("打开本地工作区")
                .on_click(move |_, _, _| d_add_proj(UiAction::OpenWorkspaceDialog))
                .child(
                    lucide(LucideIcon::FolderPlus)
                        .size(px(16.0))
                        .text_color(theme.colors.sidebar_foreground),
                ),
        )
    } else {
        div()
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
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.sidebar_muted_foreground())
                            .child("工作区与会话"),
                    )
                    .child(
                        Button::new("sb.add_project")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .aria_label("打开本地工作区")
                            .on_click(move |_, _, _| d_add_proj(UiAction::OpenWorkspaceDialog))
                            .child(
                                lucide(LucideIcon::FolderPlus)
                                    .size(px(15.0))
                                    .text_color(theme.colors.sidebar_foreground),
                            ),
                    ),
            )
            .child(Input::new(filter_input).aria_label("搜索工作区或会话"))
    };

    let footer = div()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            SidebarItem::new("sb.footer.import", "数据迁移")
                .collapsed(collapsed)
                .size(SidebarItemSize::Small)
                .icon(lucide(LucideIcon::DatabaseBackup).size(px(15.0)))
                .on_activate(move |_, _, _| {
                    d_import(UiAction::OpenSettings(SettingsSection::LegacyImport))
                }),
        )
        .child(
            SidebarItem::new("sb.footer.metrics", "额度与监控")
                .collapsed(collapsed)
                .size(SidebarItemSize::Small)
                .selected(state.inspector_open && state.inspector_tab == InspectorTab::Metrics)
                .icon(lucide(LucideIcon::Gauge).size(px(15.0)))
                .on_activate(move |_, _, _| {
                    d_metrics(UiAction::ToggleInspector(InspectorTab::Metrics))
                }),
        )
        .child(
            SidebarItem::new("sb.footer.settings", "设置中心")
                .collapsed(collapsed)
                .size(SidebarItemSize::Small)
                .icon(lucide(LucideIcon::Settings).size(px(15.0)))
                .trailing(Kbd::new("⌘,"))
                .on_activate(move |_, _, _| {
                    d_settings(UiAction::OpenSettings(SettingsSection::General))
                }),
        );

    let mut sidebar = Sidebar::new("lingxi.sidebar", "工作区导航")
        .surface(true)
        .header(header)
        .footer(footer)
        .content_padding(px(8.0));

    if collapsed {
        for (idx, thread) in state.threads.iter().take(12).enumerate() {
            let tid = thread.id.clone();
            let d_sel = Arc::clone(dispatcher);
            let selected = state.active_thread_id.as_ref() == Some(&thread.id);
            sidebar = sidebar.child(
                SidebarItem::new(("sb.rail_thread", idx), thread.title.clone())
                    .collapsed(true)
                    .selected(selected)
                    .icon(lucide(LucideIcon::MessageSquare).size(px(15.0)))
                    .on_activate(move |_, _, _| d_sel(UiAction::SelectThread(tid.clone()))),
            );
        }
        return sidebar;
    }

    if state.projects.is_empty() {
        let d_open = Arc::clone(dispatcher);
        sidebar = sidebar.child(
            div()
                .p(px(12.0))
                .rounded(px(8.0))
                .border_1()
                .border_color(theme.colors.sidebar_border)
                .bg(theme.colors.card.opacity(0.6))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.sidebar_foreground)
                        .child("暂无已打开工作区"),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.sidebar_muted_foreground())
                        .child("点击下方按钮打开项目目录，或从设置中只读导入原版会话。"),
                )
                .child(
                    Button::new("sb.empty.open")
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm)
                        .label("打开工作区")
                        .on_click(move |_, _, _| d_open(UiAction::OpenWorkspaceDialog)),
                ),
        );
        return sidebar;
    }

    for (p_idx, project) in state.projects.iter().enumerate() {
        let expanded = state.expanded_projects.contains(&project.id);
        let pid_toggle = project.id.clone();
        let pid_new = project.id.clone();
        let pid_wt = project.id.clone();
        let d_tog = Arc::clone(dispatcher);
        let d_new = Arc::clone(dispatcher);
        let d_wt = Arc::clone(dispatcher);

        let threads = state.filtered_threads_for_project(&project.id);

        if p_idx > 0 {
            sidebar = sidebar.child(sidebar_separator(cx));
        }

        let project_row = div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(4.0))
            .py(px(2.0))
            .child(
                div()
                    .id(("sb.proj", p_idx))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .on_click(move |_, _, _| {
                        d_tog(UiAction::ToggleProjectExpanded(pid_toggle.clone()))
                    })
                    .child(icon_xs(
                        if expanded {
                            LucideIcon::ChevronDown
                        } else {
                            LucideIcon::ChevronRight
                        },
                        theme.colors.sidebar_muted_foreground(),
                    ))
                    .child(icon_xs(
                        LucideIcon::Folder,
                        theme.colors.sidebar_muted_foreground(),
                    ))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.sidebar_foreground)
                            .child(project.title.clone()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .child(
                        Button::new(("sb.proj.new_wt", p_idx))
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .aria_label("新建 Worktree 会话")
                            .on_click(move |_, _, _| {
                                d_wt(UiAction::CreateThread {
                                    project_id: pid_wt.clone(),
                                    worktree: true,
                                })
                            })
                            .child(icon_xs(
                                LucideIcon::GitBranch,
                                theme.colors.sidebar_muted_foreground(),
                            )),
                    )
                    .child(
                        Button::new(("sb.proj.new", p_idx))
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::IconSm)
                            .aria_label("新建会话")
                            .on_click(move |_, _, _| {
                                d_new(UiAction::CreateThread {
                                    project_id: pid_new.clone(),
                                    worktree: false,
                                })
                            })
                            .child(icon_xs(
                                LucideIcon::Plus,
                                theme.colors.sidebar_muted_foreground(),
                            )),
                    ),
            );

        sidebar = sidebar.child(project_row);

        if expanded {
            if threads.is_empty() {
                sidebar = sidebar.child(sidebar_group_label("暂无匹配会话", cx));
            } else {
                for (t_idx, thread) in threads.into_iter().enumerate() {
                    let selected = state.active_thread_id.as_ref() == Some(&thread.id);
                    let tid = thread.id.clone();
                    let d_sel = Arc::clone(dispatcher);
                    let item_id: SharedString = format!("sb.thread.{p_idx}.{t_idx}").into();

                    let mut item = SidebarItem::new(item_id, thread.title.clone())
                        .selected(selected)
                        .size(SidebarItemSize::Small)
                        .icon(status_dot(thread.status, theme))
                        .on_activate(move |_, _, _| d_sel(UiAction::SelectThread(tid.clone())));

                    if let Some(branch) = &thread.branch {
                        item = item.trailing(sidebar_badge(branch.clone(), cx));
                    } else if thread.unread {
                        item = item.trailing(sidebar_badge("未读", cx));
                    }

                    sidebar = sidebar.child(item);
                }
            }
        }
    }

    sidebar
}
