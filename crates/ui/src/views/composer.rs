use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, IntoElement, ParentElement, Styled, div, prelude::FluentBuilder as _, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::composer::{Composer, ComposerState};
use gpuicn::select::{Select, SelectState};
use gpuicn::theme::UiTheme;
use lingxi_domain::{ThreadRecord, ThreadRunStatus};

use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, UiAction};
use crate::tokens::{elevation, radius, space, typo};

#[allow(clippy::too_many_arguments)]
pub fn render_composer(
    state: &AppUiState,
    thread: &ThreadRecord,
    composer: &Entity<ComposerState>,
    provider_select: &Entity<SelectState>,
    mode_select: &Entity<SelectState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    cx: &App,
) -> impl IntoElement {
    let is_running = thread.status == ThreadRunStatus::Running;
    let draft = composer.read(cx).value(cx).to_string();
    let can_send = !draft.trim().is_empty();

    let tid_send = thread.id.clone();
    let tid_cancel = thread.id.clone();
    let tid_compact = thread.id.clone();
    let d_send = Arc::clone(dispatcher);
    let d_cancel = Arc::clone(dispatcher);
    let d_compact = Arc::clone(dispatcher);
    let composer_handle = composer.clone();

    let context_label = state.active_context().map(|ctx| {
        let pct = (ctx.utilization_ratio() * 100.0).round() as u32;
        format!("上下文 {pct}%")
    });

    div()
        .w_full()
        .flex_shrink_0()
        .px(px(24.0))
        .pb(px(18.0))
        .pt(px(4.0))
        .child(
            div()
                .w_full()
                .max_w(space::CHAT_COLUMN_MAX_WIDTH)
                .mx_auto()
                .child(
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
                                .pt(px(4.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .flex_wrap()
                                        .child(Select::new(provider_select).w(px(148.0)))
                                        .child(Select::new(mode_select).w(px(138.0)))
                                        .when_some(context_label, |el, label| {
                                            el.child(
                                                div()
                                                    .px(px(8.0))
                                                    .text_size(typo::CAPTION)
                                                    .text_color(theme.colors.muted_foreground)
                                                    .child(label),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .flex_shrink_0()
                                        .child(
                                            Button::new("composer.compact")
                                                .variant(ButtonVariant::Ghost)
                                                .size(ButtonSize::Sm)
                                                .label("压缩")
                                                .on_click(move |_, _, _| {
                                                    d_compact(UiAction::CompactContext(
                                                        tid_compact.clone(),
                                                    ))
                                                }),
                                        )
                                        .when(is_running, |el| {
                                            el.child(
                                                Button::new("composer.stop")
                                                    .variant(ButtonVariant::Destructive)
                                                    .size(ButtonSize::Sm)
                                                    .label("停止")
                                                    .on_click(move |_, _, _| {
                                                        d_cancel(UiAction::CancelTurn(
                                                            tid_cancel.clone(),
                                                        ))
                                                    }),
                                            )
                                        })
                                        .when(!is_running, |el| {
                                            el.child(
                                                Button::new("composer.send")
                                                    .variant(ButtonVariant::Default)
                                                    .size(ButtonSize::IconSm)
                                                    .aria_label("发送消息")
                                                    .disabled(!can_send)
                                                    .on_click(move |_, window, cx| {
                                                        let text = composer_handle
                                                            .read(cx)
                                                            .value(cx)
                                                            .to_string();
                                                        if text.trim().is_empty() {
                                                            return;
                                                        }
                                                        composer_handle.update(cx, |c, cx| {
                                                            c.set_value("", window, cx);
                                                        });
                                                        d_send(UiAction::SendTurn {
                                                            thread_id: tid_send.clone(),
                                                            prompt: text,
                                                        });
                                                    })
                                                    .child(
                                                        lucide(LucideIcon::ArrowUp)
                                                            .size(px(15.0))
                                                            .text_color(
                                                                theme.colors.primary_foreground,
                                                            ),
                                                    ),
                                            )
                                        }),
                                ),
                        ),
                ),
        )
}
