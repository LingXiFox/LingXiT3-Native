use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, div, prelude::FluentBuilder as _, px,
};
use gpuicn::badge::{Badge, BadgeVariant};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::diff::{DiffState, DiffView};
use gpuicn::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpuicn::input::{Input, InputState};
use gpuicn::meter::Meter;
use gpuicn::progress::Progress;
use gpuicn::scroll_area::ScrollArea;
use gpuicn::tabs::{Tab, Tabs, TabsSize, TabsVariant};
use gpuicn::theme::UiTheme;

use crate::components::icon_xs;
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, InspectorTab, UiAction};

fn tab_value(tab: InspectorTab) -> &'static str {
    match tab {
        InspectorTab::Diff => "diff",
        InspectorTab::Terminal => "terminal",
        InspectorTab::GitPr => "git",
        InspectorTab::Files => "files",
        InspectorTab::Metrics => "metrics",
    }
}

fn parse_tab(value: &str) -> InspectorTab {
    match value {
        "diff" => InspectorTab::Diff,
        "terminal" => InspectorTab::Terminal,
        "git" => InspectorTab::GitPr,
        "files" => InspectorTab::Files,
        "metrics" => InspectorTab::Metrics,
        _ => InspectorTab::Diff,
    }
}

pub fn render_inspector(
    state: &AppUiState,
    diff_state: &Entity<DiffState>,
    terminal_input: &Entity<InputState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _cx: &App,
) -> impl IntoElement {
    let d_tab = Arc::clone(dispatcher);
    let d_refresh = Arc::clone(dispatcher);
    let d_close = Arc::clone(dispatcher);

    let header = div()
        .h(px(44.0))
        .px(px(10.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(6.0))
        .child(
            Tabs::new("inspector.tabs")
                .variant(TabsVariant::Line)
                .size(TabsSize::Sm)
                .selected(tab_value(state.inspector_tab))
                .on_change(move |val, _, _| {
                    d_tab(UiAction::ToggleInspector(parse_tab(val.as_ref())));
                })
                .item(Tab::new("insp.tab.diff", "diff", "Diff"))
                .item(Tab::new("insp.tab.term", "terminal", "终端"))
                .item(Tab::new("insp.tab.git", "git", "Git"))
                .item(Tab::new("insp.tab.files", "files", "文件"))
                .item(Tab::new("insp.tab.metrics", "metrics", "监控")),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(
                    Button::new("insp.refresh")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .aria_label("刷新工作区状态")
                        .on_click(move |_, _, _| d_refresh(UiAction::RefreshGitAndWorkspace))
                        .child(icon_xs(
                            LucideIcon::RefreshCw,
                            theme.colors.muted_foreground,
                        )),
                )
                .child(
                    Button::new("insp.close")
                        .variant(ButtonVariant::Ghost)
                        .size(ButtonSize::IconSm)
                        .aria_label("收起右侧面板")
                        .on_click(move |_, _, _| d_close(UiAction::CloseInspector))
                        .child(icon_xs(LucideIcon::X, theme.colors.muted_foreground)),
                ),
        );

    let body = match state.inspector_tab {
        InspectorTab::Diff => {
            render_diff_tab(state, diff_state, dispatcher, theme).into_any_element()
        }
        InspectorTab::Terminal => {
            render_terminal_tab(state, terminal_input, dispatcher, theme).into_any_element()
        }
        InspectorTab::GitPr => render_git_tab(state, dispatcher, theme).into_any_element(),
        InspectorTab::Files => render_files_tab(state, dispatcher, theme).into_any_element(),
        InspectorTab::Metrics => render_metrics_tab(state, theme).into_any_element(),
    };

    div()
        .w_full()
        .h_full()
        .min_w_0()
        .min_h_0()
        .bg(theme.colors.background)
        .border_l_1()
        .border_color(theme.colors.border)
        .flex()
        .flex_col()
        .child(header)
        .child(body)
}

fn render_diff_tab(
    state: &AppUiState,
    diff_state: &Entity<DiffState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    if state.git_diffs.is_empty() {
        return div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Empty::new().header(
                    EmptyHeader::new()
                        .media(
                            EmptyMedia::new().child(
                                lucide(LucideIcon::GitCompare)
                                    .size(px(28.0))
                                    .text_color(theme.colors.muted_foreground),
                            ),
                        )
                        .title(EmptyTitle::new().child("工作区暂无变更"))
                        .description(
                            EmptyDescription::new()
                                .child("修改文件后将在这里展示原生语法高亮 Diff。"),
                        ),
                ),
            )
            .into_any_element();
    }

    let file_pills = div()
        .px(px(12.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .flex()
        .flex_wrap()
        .gap(px(6.0))
        .children(state.git_diffs.iter().enumerate().map(|(idx, diff)| {
            let selected = state
                .selected_diff_path
                .as_deref()
                .map(|p| p == diff.path)
                .unwrap_or(idx == 0);
            let path = diff.path.clone();
            let d_sel = Arc::clone(dispatcher);
            Button::new(("insp.diff.file", idx))
                .variant(if selected {
                    ButtonVariant::Secondary
                } else {
                    ButtonVariant::Ghost
                })
                .size(ButtonSize::Xs)
                .label(format!(
                    "{} (+{}/-{})",
                    diff.path, diff.additions, diff.deletions
                ))
                .on_click(move |_, _, _| d_sel(UiAction::SelectDiffFile(path.clone())))
        }));

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(file_pills)
        .child(
            div().flex_1().min_h_0().p(px(10.0)).child(
                DiffView::new("insp.diff.viewer", diff_state)
                    .read_only(true)
                    .header_visible(true)
                    .h_full()
                    .w_full(),
            ),
        )
        .into_any_element()
}

pub fn git_diffs_to_unified_patch(
    diffs: &[lingxi_domain::GitFileDiff],
    selected: Option<&str>,
) -> String {
    let mut patch = String::new();
    for diff in diffs {
        if let Some(sel) = selected
            && diff.path != sel
        {
            continue;
        }
        if !diff.patch.trim_start().starts_with("diff --git") {
            patch.push_str(&format!("diff --git a/{} b/{}\n", diff.path, diff.path));
        }
        patch.push_str(&diff.patch);
        if !patch.ends_with('\n') {
            patch.push('\n');
        }
    }
    patch
}

fn render_terminal_tab(
    state: &AppUiState,
    terminal_input: &Entity<InputState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    let d_spawn = Arc::clone(dispatcher);
    let d_send = Arc::clone(dispatcher);
    let active_term = state
        .active_terminal_id
        .as_ref()
        .and_then(|id| state.terminal_tabs.iter().find(|t| &t.id == id))
        .or_else(|| state.terminal_tabs.first());

    let toolbar = div()
        .px(px(12.0))
        .py(px(8.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(div().flex().items_center().gap(px(6.0)).children(
            state.terminal_tabs.iter().enumerate().map(|(idx, t)| {
                let is_active = active_term.map(|a| a.id == t.id).unwrap_or(false);
                let tid = t.id.clone();
                let d_sel = Arc::clone(dispatcher);
                Button::new(("insp.term.tab", idx))
                    .variant(if is_active {
                        ButtonVariant::Secondary
                    } else {
                        ButtonVariant::Ghost
                    })
                    .size(ButtonSize::Xs)
                    .label(t.title.clone())
                    .on_click(move |_, _, _| d_sel(UiAction::SelectTerminalTab(tid.clone())))
            }),
        ))
        .child(
            Button::new("insp.term.spawn")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Xs)
                .label("新建终端")
                .on_click(move |_, _, _| d_spawn(UiAction::SpawnTerminal)),
        );

    let output = if let Some(term) = active_term {
        let mut lines_col = div()
            .p(px(12.0))
            .font_family(theme.fonts.mono.clone())
            .text_size(px(12.5))
            .line_height(px(19.0))
            .text_color(theme.colors.foreground)
            .flex()
            .flex_col()
            .gap(px(2.0));
        if term.lines.is_empty() {
            lines_col = lines_col.child(
                div()
                    .text_color(theme.colors.muted_foreground)
                    .child(format!("Shell 就绪 ({})，请在下方输入命令。", term.cwd)),
            );
        } else {
            for line in term.lines.iter().rev().take(200).rev() {
                lines_col = lines_col.child(line.clone());
            }
        }
        ScrollArea::new("insp.term.scroll")
            .aria_label("终端输出")
            .flex_1()
            .min_h_0()
            .child(lines_col)
            .into_any_element()
    } else {
        Empty::new()
            .header(
                EmptyHeader::new()
                    .media(
                        EmptyMedia::new().child(
                            lucide(LucideIcon::Terminal)
                                .size(px(28.0))
                                .text_color(theme.colors.muted_foreground),
                        ),
                    )
                    .title(EmptyTitle::new().child("暂无活动 PTY 终端"))
                    .description(
                        EmptyDescription::new().child("点击右上角「新建终端」启动真实本地 Shell。"),
                    ),
            )
            .into_any_element()
    };

    let active_term_id = active_term.map(|t| t.id.clone());
    let input_handle = terminal_input.clone();

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(toolbar)
        .child(output)
        .when_some(active_term_id, |el, tid| {
            el.child(
                div()
                    .p(px(10.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(terminal_input).aria_label("终端命令输入")),
                    )
                    .child(
                        Button::new("insp.term.run")
                            .variant(ButtonVariant::Default)
                            .size(ButtonSize::Sm)
                            .label("执行")
                            .on_click(move |_, window, cx| {
                                let cmd = input_handle.read(cx).value().to_string();
                                if cmd.trim().is_empty() {
                                    return;
                                }
                                input_handle.update(cx, |inp, cx| inp.set_value("", window, cx));
                                d_send(UiAction::SendTerminalInput {
                                    terminal_id: tid.clone(),
                                    command: cmd,
                                });
                            }),
                    ),
            )
        })
}

fn render_git_tab(
    state: &AppUiState,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    let mut content = div().p(px(14.0)).flex().flex_col().gap(px(14.0));

    if let Some(git) = &state.git_status {
        let is_dirty = !git.staged_files.is_empty()
            || !git.modified_files.is_empty()
            || !git.untracked_files.is_empty();
        content = content.child(
            div()
                .p(px(12.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(theme.colors.border)
                .bg(theme.colors.card)
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
                                .text_size(px(13.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!(
                                    "分支: {}",
                                    git.branch.as_deref().unwrap_or("HEAD")
                                )),
                        )
                        .child(
                            Badge::new(if is_dirty {
                                "有未提交变更"
                            } else {
                                "工作区干净"
                            })
                            .variant(if is_dirty {
                                BadgeVariant::Default
                            } else {
                                BadgeVariant::Secondary
                            }),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.5))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!(
                            "Ahead {} · Behind {} · Staged {} · Modified {} · Untracked {}",
                            git.ahead,
                            git.behind,
                            git.staged_files.len(),
                            git.modified_files.len(),
                            git.untracked_files.len()
                        )),
                ),
        );
    }

    if !state.git_diffs.is_empty() {
        content = content.child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.muted_foreground)
                .child("变更文件"),
        );
        for (idx, diff) in state.git_diffs.iter().enumerate() {
            let path = diff.path.clone();
            let d_jump = Arc::clone(dispatcher);
            content = content.child(
                div()
                    .id(("insp.git.file", idx))
                    .px(px(10.0))
                    .py(px(8.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.card)
                    .cursor_pointer()
                    .hover(|s| s.border_color(theme.colors.primary))
                    .on_click(move |_, _, _| d_jump(UiAction::SelectDiffFile(path.clone())))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(13.0))
                            .child(diff.path.clone()),
                    )
                    .child(
                        Badge::new(format!("+{} / -{}", diff.additions, diff.deletions))
                            .variant(BadgeVariant::Outline),
                    ),
            );
        }
    }

    let checkpoints = state.active_checkpoints();
    if !checkpoints.is_empty() {
        content = content.child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.colors.muted_foreground)
                .child("会话检查点"),
        );
        for (idx, cp) in checkpoints.iter().enumerate() {
            let tid = cp.thread_id.clone();
            let cid = cp.id.clone();
            let d_rev = Arc::clone(dispatcher);
            content = content.child(
                div()
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.card)
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(cp.summary.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.muted_foreground)
                                    .child(cp.created_at.format("%m-%d %H:%M").to_string()),
                            ),
                    )
                    .child(
                        Button::new(("insp.cp.revert", idx))
                            .variant(ButtonVariant::Outline)
                            .size(ButtonSize::Xs)
                            .label("回滚")
                            .on_click(move |_, _, _| {
                                d_rev(UiAction::RevertCheckpoint {
                                    thread_id: tid.clone(),
                                    checkpoint_id: cid.clone(),
                                })
                            }),
                    ),
            );
        }
    }

    if state.git_status.is_none() && state.git_diffs.is_empty() && checkpoints.is_empty() {
        return Empty::new()
            .header(
                EmptyHeader::new()
                    .media(
                        EmptyMedia::new().child(
                            lucide(LucideIcon::GitBranch)
                                .size(px(28.0))
                                .text_color(theme.colors.muted_foreground),
                        ),
                    )
                    .title(EmptyTitle::new().child("暂无 Git 仓库信息"))
                    .description(EmptyDescription::new().child("打开 Git 项目工作区后点击刷新。")),
            )
            .into_any_element();
    }

    ScrollArea::new("insp.git.scroll")
        .aria_label("Git 与检查点")
        .flex_1()
        .min_h_0()
        .child(content)
        .into_any_element()
}

fn render_files_tab(
    state: &AppUiState,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    if state.workspace_files.is_empty() {
        return Empty::new()
            .header(
                EmptyHeader::new()
                    .media(
                        EmptyMedia::new().child(
                            lucide(LucideIcon::FolderTree)
                                .size(px(28.0))
                                .text_color(theme.colors.muted_foreground),
                        ),
                    )
                    .title(EmptyTitle::new().child("暂无工作区文件"))
                    .description(
                        EmptyDescription::new().child("打开项目工作区后可在此快速浏览文件。"),
                    ),
            )
            .into_any_element();
    }

    let mut list = div().p(px(10.0)).flex().flex_col().gap(px(4.0));
    for (idx, file) in state.workspace_files.iter().take(120).enumerate() {
        let selected = state.file_preview_path.as_deref() == Some(file.as_str());
        let path = file.clone();
        let d_open = Arc::clone(dispatcher);
        list = list.child(
            div()
                .id(("insp.file", idx))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(if selected {
                    theme.colors.accent
                } else {
                    gpui_kit::transparent_black().into()
                })
                .hover(|s| s.bg(theme.colors.muted))
                .cursor_pointer()
                .on_click(move |_, _, _| d_open(UiAction::OpenFilePreview(path.clone())))
                .text_size(px(12.5))
                .font_family(theme.fonts.mono.clone())
                .truncate()
                .child(file.clone()),
        );
    }

    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(
            ScrollArea::new("insp.files.list")
                .aria_label("工作区文件列表")
                .flex_1()
                .min_h_0()
                .child(list),
        )
        .when_some(state.file_preview_content.clone(), |el, preview| {
            el.child(
                div()
                    .h(px(220.0))
                    .border_t_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.card)
                    .child(
                        ScrollArea::new("insp.files.preview")
                            .aria_label("文件内容预览")
                            .h_full()
                            .child(
                                div()
                                    .p(px(12.0))
                                    .font_family(theme.fonts.mono.clone())
                                    .text_size(px(12.0))
                                    .line_height(px(18.0))
                                    .child(preview),
                            ),
                    ),
            )
        })
        .into_any_element()
}

fn render_metrics_tab(state: &AppUiState, theme: &UiTheme) -> impl IntoElement {
    let mut col = div().p(px(14.0)).flex().flex_col().gap(px(14.0));

    col = col.child(
        div()
            .p(px(12.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.card)
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("本地进程资源"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(13.0))
                    .child(format!(
                        "内存 RSS: {:.1} MB",
                        state.resource_usage.memory_rss_bytes as f64 / (1024.0 * 1024.0)
                    ))
                    .child(format!("CPU: {:.1}%", state.resource_usage.cpu_percent)),
            )
            .child(
                Meter::new("insp.metrics.cpu")
                    .aria_label("CPU 占用")
                    .range(0.0, 100.0)
                    .value(f64::from(state.resource_usage.cpu_percent).clamp(0.0, 100.0)),
            ),
    );

    if let Some(ctx) = state.active_context() {
        let pct = (ctx.utilization_ratio() * 100.0).clamp(0.0, 100.0);
        col = col.child(
            div()
                .p(px(12.0))
                .rounded(px(10.0))
                .border_1()
                .border_color(theme.colors.border)
                .bg(theme.colors.card)
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
                                .text_size(px(13.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("上下文窗口占用"),
                        )
                        .child(
                            Badge::new(format!(
                                "{}/{}",
                                ctx.total_used(),
                                ctx.context_window_limit
                            ))
                            .variant(BadgeVariant::Secondary),
                        ),
                )
                .child(Progress::new("insp.metrics.ctx").value(pct))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!(
                            "System {} · 对话 {} · 工具 {} · 缓存命中 {:.0}%",
                            ctx.system_tokens,
                            ctx.messages_tokens,
                            ctx.tools_tokens,
                            ctx.cache_hit_ratio() * 100.0
                        )),
                ),
        );
    }

    col = col.child(
        div()
            .text_size(px(13.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.colors.muted_foreground)
            .child("多平台额度监控"),
    );

    for (idx, snap) in state.quota_snapshots.iter().enumerate() {
        let provider_name: SharedString = snap.provider.display_name().into();
        let status_badge = if snap.available {
            "已连接"
        } else {
            "未安装"
        };

        let mut card = div()
            .id(("insp.quota.card", idx))
            .p(px(12.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.card)
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
                            .text_size(px(13.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(provider_name),
                    )
                    .child(Badge::new(status_badge).variant(if snap.available {
                        BadgeVariant::Secondary
                    } else {
                        BadgeVariant::Outline
                    })),
            );

        for (p_idx, pool) in snap.pools.iter().enumerate() {
            let remaining = pool.remaining_percent.unwrap_or(0.0).clamp(0.0, 100.0);
            card = card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_size(px(12.0))
                            .text_color(theme.colors.muted_foreground)
                            .child(pool.pool_name.clone())
                            .child(
                                pool.remaining_percent
                                    .map(|p| format!("{p:.0}%"))
                                    .unwrap_or_else(|| "未知".to_string()),
                            ),
                    )
                    .child(
                        Meter::new(("insp.quota.meter", idx * 20 + p_idx))
                            .range(0.0, 100.0)
                            .value(remaining),
                    ),
            );
        }

        if !snap.source_note.is_empty() {
            card = card.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.colors.muted_foreground)
                    .child(snap.source_note.clone()),
            );
        }

        col = col.child(card);
    }

    ScrollArea::new("insp.metrics.scroll")
        .aria_label("资源与额度监控")
        .flex_1()
        .min_h_0()
        .child(col)
}
