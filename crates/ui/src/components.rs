use gpui_kit::{Div, IntoElement, ParentElement, Styled, div, px};
use gpuicn::theme::UiTheme;
use lingxi_domain::{ContextBreakdown, ThreadRunStatus};

pub fn status_dot(status: ThreadRunStatus, theme: &UiTheme) -> Div {
    let color = match status {
        ThreadRunStatus::Idle => theme.colors.muted_foreground,
        ThreadRunStatus::Running => theme.colors.primary,
        ThreadRunStatus::WaitingApproval => theme.colors.warning,
        ThreadRunStatus::Paused => theme.colors.info,
        ThreadRunStatus::Error => theme.colors.destructive,
    };
    div().w(px(7.0)).h(px(7.0)).rounded_full().bg(color)
}

pub fn hairline_panel(theme: &UiTheme) -> Div {
    div()
        .bg(theme.colors.card)
        .border_1()
        .border_color(theme.colors.border)
        .rounded(px(6.0))
}

pub fn context_meter(breakdown: &ContextBreakdown, theme: &UiTheme) -> impl IntoElement {
    let ratio = breakdown.utilization_ratio() as f32;
    let pct = (ratio * 100.0).round() as u32;
    let bar_width = (ratio.clamp(0.0, 1.0) * 100.0).max(2.0);
    let fill = if pct >= 85 {
        theme.colors.destructive
    } else if pct >= 65 {
        theme.colors.warning
    } else {
        theme.colors.primary
    };

    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .text_xs()
                .child(format!(
                    "上下文 {}/{} ({pct}%)",
                    breakdown.total_used(),
                    breakdown.context_window_limit
                ))
                .child(format!(
                    "缓存命中 {:.0}%",
                    breakdown.cache_hit_ratio() * 100.0
                )),
        )
        .child(
            div()
                .w_full()
                .h(px(6.0))
                .rounded_full()
                .bg(theme.colors.muted)
                .overflow_hidden()
                .child(
                    div()
                        .h_full()
                        .w(gpui_kit::relative(bar_width / 100.0))
                        .bg(fill),
                ),
        )
}
