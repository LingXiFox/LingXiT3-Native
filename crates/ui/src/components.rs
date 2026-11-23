use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{Div, FontWeight, IntoElement, ParentElement, Styled, div, px};
use gpuicn::theme::UiTheme;
use lingxi_domain::ThreadRunStatus;

pub fn status_dot(status: ThreadRunStatus, theme: &UiTheme) -> Div {
    let color = match status {
        ThreadRunStatus::Idle => theme.colors.muted_foreground.opacity(0.45),
        ThreadRunStatus::Running => theme.colors.primary,
        ThreadRunStatus::WaitingApproval => theme.colors.warning,
        ThreadRunStatus::Paused => theme.colors.info,
        ThreadRunStatus::Error => theme.colors.destructive,
    };
    div()
        .w(px(7.0))
        .h(px(7.0))
        .rounded_full()
        .flex_shrink_0()
        .bg(color)
}

pub fn icon_sm(name: LucideIcon, color: gpui_kit::Rgba) -> impl IntoElement {
    lucide(name).size(px(15.0)).text_color(color)
}

pub fn icon_xs(name: LucideIcon, color: gpui_kit::Rgba) -> impl IntoElement {
    lucide(name).size(px(13.0)).text_color(color)
}

pub fn fox_brand_mark(size_px: f32, theme: &UiTheme) -> Div {
    div()
        .w(px(size_px))
        .h(px(size_px))
        .rounded(px((size_px * 0.28).round()))
        .bg(theme.colors.primary)
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.colors.primary_foreground)
        .font_weight(FontWeight::BOLD)
        .text_size(px((size_px * 0.48).round()))
        .child("灵")
}
