use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::base::Button as BaseButton;
use gpui_kit::{
    App, Entity, IntoElement, ParentElement, SharedString, StatefulInteractiveElement as _, Styled,
    Window, div, prelude::FluentBuilder as _, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant, style_button};
use gpuicn::composer::{Composer, ComposerState};
use gpuicn::theme::UiTheme;
use gpuicn::tooltip::tooltip;
use lingxi_domain::{ThreadId, ThreadRecord, ThreadRunStatus};

use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, UiAction};
use crate::tokens::{elevation, radius, space, surface, text};
use crate::views::model_picker::{render_mode_picker, render_model_picker};

#[allow(clippy::too_many_arguments)]
pub fn render_composer_box(
    id_prefix: &'static str,
    state: &AppUiState,
    thread_id: Option<ThreadId>,
    is_running: bool,
    composer: &Entity<ComposerState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let draft = composer.read(cx).value(cx).to_string();
    let can_send = !draft.trim().is_empty();

    let composer_handle = composer.clone();
    let d_attach = Arc::clone(dispatcher);
    let d_send = Arc::clone(dispatcher);
    let d_cancel = Arc::clone(dispatcher);
    let d_compact = Arc::clone(dispatcher);

    let tid_send = thread_id.clone();
    let tid_cancel = thread_id.clone();
    let tid_compact = thread_id.clone();

    let attach_btn_base = style_button(
        BaseButton::new(SharedString::from(format!("{id_prefix}.attach")))
            .aria_label("打开本地工作区或添加上下文")
            .on_click(move |_, _, _| d_attach(UiAction::OpenWorkspaceDialog))
            .child(
                lucide(LucideIcon::Plus)
                    .size(px(15.0))
                    .text_color(text::tertiary(theme)),
            ),
        false,
        ButtonVariant::Ghost,
        ButtonSize::IconSm,
        theme,
    );
    let attach_btn = tooltip(
        SharedString::from(format!("{id_prefix}.attach.tip")),
        "打开本地工作区",
        attach_btn_base,
        window,
        cx,
    );

    let context_pct = state
        .active_context()
        .map(|ctx| (ctx.utilization_ratio() * 100.0).round() as u32);

    let compact_control = if let (Some(tid_c), Some(pct)) = (tid_compact, context_pct) {
        let label: SharedString = format!("压缩上下文（当前占用 {pct}%）").into();
        let btn_base = style_button(
            BaseButton::new(SharedString::from(format!("{id_prefix}.compact")))
                .aria_label(label.clone())
                .on_click(move |_, _, _| d_compact(UiAction::CompactContext(tid_c.clone())))
                .child(
                    lucide(LucideIcon::Minimize2)
                        .size(px(13.5))
                        .text_color(text::tertiary(theme)),
                ),
            false,
            ButtonVariant::Ghost,
            ButtonSize::IconSm,
            theme,
        );
        Some(tooltip(
            SharedString::from(format!("{id_prefix}.compact.tip")),
            label,
            btn_base,
            window,
            cx,
        ))
    } else {
        None
    };

    let mode_picker = render_mode_picker(
        match thread_id {
            Some(_) => "chat.mode",
            None => "welcome.mode",
        },
        state,
        thread_id.clone(),
        dispatcher,
        theme,
        window,
        cx,
    );

    let model_picker = render_model_picker(
        match thread_id {
            Some(_) => "chat.model",
            None => "welcome.model",
        },
        state,
        thread_id,
        dispatcher,
        theme,
        window,
        cx,
    );

    Composer::new(composer)
        .w_full()
        .rounded(radius::COMPOSER)
        .bg(theme.colors.card)
        .border_1()
        .border_color(surface::composer_border(theme))
        .shadow(elevation::for_level1(theme))
        .px(px(12.0))
        .pt(px(10.0))
        .pb(px(8.0))
        .child(
            div()
                .w_full()
                .h(px(28.0))
                .pt(px(2.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .child(attach_btn)
                        .child(mode_picker),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(4.0))
                        .flex_shrink_0()
                        .child(model_picker)
                        .when_some(compact_control, |el, ctrl| el.child(ctrl))
                        .when(is_running, |el| {
                            let tid_stop = tid_cancel.clone();
                            let d_stop = Arc::clone(&d_cancel);
                            el.child(
                                Button::new(SharedString::from(format!("{id_prefix}.stop")))
                                    .variant(ButtonVariant::Destructive)
                                    .size(ButtonSize::IconSm)
                                    .aria_label("停止生成")
                                    .on_click(move |_, _, _| {
                                        if let Some(tid) = tid_stop.clone() {
                                            d_stop(UiAction::CancelTurn(tid));
                                        }
                                    })
                                    .child(
                                        lucide(LucideIcon::Square)
                                            .size(px(12.0))
                                            .text_color(theme.colors.destructive_foreground),
                                    ),
                            )
                        })
                        .when(!is_running, |el| {
                            el.child(
                                Button::new(SharedString::from(format!("{id_prefix}.send")))
                                    .variant(ButtonVariant::Default)
                                    .size(ButtonSize::IconSm)
                                    .aria_label("发送消息")
                                    .disabled(!can_send)
                                    .on_click(move |_, window, cx| {
                                        let text = composer_handle.read(cx).value(cx).to_string();
                                        if text.trim().is_empty() {
                                            return;
                                        }
                                        composer_handle.update(cx, |c, cx| {
                                            c.set_value("", window, cx);
                                        });
                                        if let Some(tid) = tid_send.clone() {
                                            d_send(UiAction::SendTurn {
                                                thread_id: tid,
                                                prompt: text,
                                            });
                                        } else {
                                            d_send(UiAction::StartQuickThread { prompt: text });
                                        }
                                    })
                                    .child(
                                        lucide(LucideIcon::ArrowUp)
                                            .size(px(15.0))
                                            .text_color(theme.colors.primary_foreground),
                                    ),
                            )
                        }),
                ),
        )
}

pub fn render_composer(
    state: &AppUiState,
    thread: &ThreadRecord,
    composer: &Entity<ComposerState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement + use<> {
    let is_running = thread.status == ThreadRunStatus::Running;

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
                .child(render_composer_box(
                    "chat.composer",
                    state,
                    Some(thread.id.clone()),
                    is_running,
                    composer,
                    dispatcher,
                    theme,
                    window,
                    cx,
                )),
        )
}
