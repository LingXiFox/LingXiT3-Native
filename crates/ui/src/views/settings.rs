use std::sync::Arc;

use gpui_icons::{LucideIcon, lucide};
use gpui_kit::{
    App, Entity, FontWeight, InteractiveElement as _, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement as _, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpuicn::button::{Button, ButtonSize, ButtonVariant};
use gpuicn::input::{Input, InputState};
use gpuicn::kbd::Kbd;
use gpuicn::scroll_area::ScrollArea;
use gpuicn::select::{Select, SelectState};
use gpuicn::separator::Separator;
use gpuicn::sidebar::{Sidebar, SidebarItem, SidebarItemSize, sidebar_group_label};
use gpuicn::switch::Switch;
use gpuicn::theme::UiTheme;
use lingxi_domain::ThemeAppearance;

use crate::fonts::has_misans_available;
use crate::shell::ActionDispatcher;
use crate::state::{AppUiState, SettingsSection, UiAction};
use crate::tokens::{elevation, radius, space, text, typo};

fn setting_value_text(value: impl Into<SharedString>, theme: &UiTheme) -> impl IntoElement {
    div()
        .text_size(typo::CAPTION)
        .text_color(text::secondary(theme))
        .child(value.into())
}

fn setting_mono_text(value: impl Into<SharedString>, theme: &UiTheme) -> impl IntoElement {
    div()
        .font_family(theme.fonts.mono.clone())
        .text_size(typo::CAPTION)
        .text_color(text::secondary(theme))
        .child(value.into())
}

fn setting_status_text(
    label: impl Into<SharedString>,
    active: bool,
    theme: &UiTheme,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(if active {
            theme.colors.success
        } else {
            theme.colors.border
        }))
        .child(
            div()
                .text_size(typo::CAPTION)
                .text_color(if active {
                    text::secondary(theme)
                } else {
                    text::tertiary(theme)
                })
                .child(label.into()),
        )
}

fn section_heading(title: &'static str, theme: &UiTheme) -> impl IntoElement {
    div()
        .pt(px(4.0))
        .pb(px(6.0))
        .text_size(typo::SECTION_TITLE)
        .line_height(typo::SECTION_TITLE_LH)
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.colors.foreground)
        .child(title)
}

fn setting_row(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    control: impl IntoElement,
    theme: &UiTheme,
) -> impl IntoElement {
    div()
        .w_full()
        .py(px(12.0))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(3.0))
                .child(
                    div()
                        .text_size(typo::UI_BODY)
                        .line_height(typo::UI_BODY_LH)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child(title.into()),
                )
                .child(
                    div()
                        .text_size(typo::CAPTION)
                        .line_height(typo::CAPTION_LH)
                        .text_color(theme.colors.muted_foreground)
                        .child(description.into()),
                ),
        )
        .child(div().flex_shrink_0().child(control))
}

#[allow(clippy::too_many_arguments)]
pub fn render_settings_popup_content(
    state: &AppUiState,
    custom_acp_input: &Entity<InputState>,
    remote_port_input: &Entity<InputState>,
    update_channel_select: &Entity<SelectState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let viewport = window.viewport_size();
    let modal_w = (viewport.width - px(56.0)).clamp(px(620.0), px(780.0));
    let modal_h = (viewport.height - px(120.0)).clamp(px(400.0), px(540.0));
    let nav_w = if viewport.width < px(1080.0) {
        px(168.0)
    } else {
        px(196.0)
    };

    let nav_items: [(SettingsSection, LucideIcon, &str); 8] = [
        (
            SettingsSection::General,
            LucideIcon::SlidersHorizontal,
            "常规",
        ),
        (
            SettingsSection::Appearance,
            LucideIcon::Palette,
            "外观与主题",
        ),
        (
            SettingsSection::Providers,
            LucideIcon::Cpu,
            "智能体 Provider",
        ),
        (SettingsSection::Keybindings, LucideIcon::Keyboard, "快捷键"),
        (
            SettingsSection::McpSkills,
            LucideIcon::Plug,
            "MCP 与 Skills",
        ),
        (SettingsSection::Remote, LucideIcon::Radio, "远程与配对"),
        (
            SettingsSection::LegacyImport,
            LucideIcon::DatabaseBackup,
            "数据迁移",
        ),
        (SettingsSection::About, LucideIcon::Info, "关于与更新"),
    ];

    let mut nav = Sidebar::new("settings.sidebar", "设置导航")
        .surface(true)
        .w(nav_w)
        .flex_shrink_0()
        .border_r_1()
        .border_color(theme.colors.sidebar_border)
        .content_padding(px(10.0))
        .header(
            div()
                .px(px(8.0))
                .py(px(6.0))
                .text_size(typo::SECTION_TITLE)
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.colors.sidebar_foreground)
                .child("设置中心"),
        )
        .child(sidebar_group_label("核心配置", cx));

    for (idx, (section, icon, label)) in nav_items.into_iter().enumerate() {
        if idx == 4 {
            nav = nav.child(sidebar_group_label("扩展与维护", cx));
        }
        let d_sec = Arc::clone(dispatcher);
        nav = nav.child(
            SidebarItem::new(("settings.nav", idx), label)
                .size(SidebarItemSize::Default)
                .h(space::SIDEBAR_ROW_HEIGHT)
                .selected(state.settings_section == section)
                .icon(lucide(icon).size(px(15.0)))
                .on_activate(move |_, _, _| d_sec(UiAction::OpenSettings(section))),
        );
    }

    let header = div()
        .h(px(48.0))
        .px(px(24.0))
        .border_b_1()
        .border_color(theme.colors.border)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(typo::CAPTION)
                        .text_color(theme.colors.muted_foreground)
                        .child("设置"),
                )
                .child(
                    lucide(LucideIcon::ChevronRight)
                        .size(px(14.0))
                        .text_color(theme.colors.muted_foreground),
                )
                .child(
                    div()
                        .text_size(typo::SECTION_TITLE)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.foreground)
                        .child(state.settings_section.label()),
                ),
        )
        .child({
            let d_close = Arc::clone(dispatcher);
            Button::new("settings.close")
                .size(ButtonSize::IconSm)
                .variant(ButtonVariant::Ghost)
                .aria_label("关闭设置")
                .on_click(move |_, _, _| d_close(UiAction::CloseModal))
                .child(
                    lucide(LucideIcon::X)
                        .size(px(15.0))
                        .text_color(theme.colors.muted_foreground),
                )
        });

    let section_body = match state.settings_section {
        SettingsSection::General => render_general(state, theme).into_any_element(),
        SettingsSection::Appearance => {
            render_appearance(state, dispatcher, theme).into_any_element()
        }
        SettingsSection::Providers => {
            render_providers(state, custom_acp_input, dispatcher, theme, cx).into_any_element()
        }
        SettingsSection::Keybindings => render_keybindings(theme).into_any_element(),
        SettingsSection::McpSkills => render_mcp_skills(state, theme).into_any_element(),
        SettingsSection::Remote => {
            render_remote(state, remote_port_input, dispatcher, theme, cx).into_any_element()
        }
        SettingsSection::LegacyImport => {
            render_legacy_import(state, dispatcher, theme).into_any_element()
        }
        SettingsSection::About => {
            render_about(state, update_channel_select, dispatcher, theme).into_any_element()
        }
    };

    div()
        .w(modal_w)
        .h(modal_h)
        .flex()
        .overflow_hidden()
        .rounded(radius::MODAL)
        .border_1()
        .border_color(theme.colors.border)
        .bg(theme.colors.background)
        .shadow(elevation::for_level3(theme))
        .font_family(theme.fonts.body.clone())
        .child(nav)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .min_h_0()
                .bg(theme.colors.background)
                .flex()
                .flex_col()
                .child(header)
                .child(
                    ScrollArea::new("settings.content.scroll")
                        .aria_label("设置内容")
                        .flex_1()
                        .min_h_0()
                        .child(
                            div()
                                .px(px(24.0))
                                .py(px(18.0))
                                .flex()
                                .flex_col()
                                .child(section_body),
                        ),
                ),
        )
}

fn render_general(state: &AppUiState, theme: &UiTheme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(section_heading("常规偏好", theme))
        .child(setting_row(
            "界面语言",
            "默认优先简体中文与技术术语混排",
            setting_value_text(state.settings.language.clone(), theme),
            theme,
        ))
        .child(Separator::new("set.gen.sep1").horizontal())
        .child(setting_row(
            "本地状态目录",
            "独立 SQLite WAL 数据库与运行时配置文件位置",
            setting_mono_text("~/.lingxi-t3-native", theme),
            theme,
        ))
        .child(Separator::new("set.gen.sep2").horizontal())
        .child(setting_row(
            "异常崩溃恢复",
            "启动时自动检测未完成 Turn 并重置为可继续状态",
            setting_status_text("已启用", true, theme),
            theme,
        ))
        .child(Separator::new("set.gen.sep3").horizontal())
        .child(setting_row(
            "已加载工作区与会话",
            "当前 SQLite 持久化状态统计",
            setting_value_text(
                format!(
                    "{} 工作区 · {} 会话",
                    state.projects.len(),
                    state.threads.len()
                ),
                theme,
            ),
            theme,
        ))
}

#[allow(clippy::too_many_arguments)]
fn appearance_card(
    id: &'static str,
    title: &'static str,
    subtitle: &'static str,
    selected: bool,
    preview_bg: gpui_kit::Rgba,
    preview_fg: gpui_kit::Rgba,
    theme: &UiTheme,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .p(px(12.0))
        .rounded(radius::CARD)
        .border_1()
        .border_color(if selected {
            theme.colors.primary
        } else {
            theme.colors.border
        })
        .bg(if selected {
            theme.colors.accent.opacity(0.45)
        } else {
            theme.colors.card
        })
        .cursor_pointer()
        .on_click(on_click)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .h(px(48.0))
                .rounded(radius::CONTROL)
                .border_1()
                .border_color(theme.colors.border)
                .bg(preview_bg)
                .p(px(8.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .w(px(32.0))
                        .h(px(5.0))
                        .rounded_full()
                        .bg(theme.colors.primary),
                )
                .child(
                    div()
                        .w(px(64.0))
                        .h(px(5.0))
                        .rounded_full()
                        .bg(preview_fg.opacity(0.28)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(typo::UI_BODY)
                        .font_weight(FontWeight::MEDIUM)
                        .child(title),
                )
                .when(selected, |el| {
                    el.child(
                        div()
                            .text_size(typo::MICRO)
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.colors.primary)
                            .child("当前"),
                    )
                }),
        )
        .child(
            div()
                .text_size(typo::CAPTION)
                .text_color(theme.colors.muted_foreground)
                .child(subtitle),
        )
}

fn render_appearance(
    state: &AppUiState,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    let d_light = Arc::clone(dispatcher);
    let d_dark = Arc::clone(dispatcher);
    let d_sys = Arc::clone(dispatcher);
    let current = state.settings.appearance;

    div()
        .flex()
        .flex_col()
        .child(section_heading("主题模式", theme))
        .child(
            div()
                .pb(px(14.0))
                .flex()
                .gap(px(12.0))
                .child(appearance_card(
                    "set.app.light",
                    "奶白浅色",
                    "Warm Cream + 狐橙高亮",
                    current == ThemeAppearance::Light,
                    gpui_kit::rgb(0xFAF7F2),
                    gpui_kit::rgb(0x1F1B18),
                    theme,
                    move |_, _, _| d_light(UiAction::SetAppearance(ThemeAppearance::Light)),
                ))
                .child(appearance_card(
                    "set.app.dark",
                    "炭黑深色",
                    "Layered Charcoal + 暖白文字",
                    current == ThemeAppearance::Dark,
                    gpui_kit::rgb(0x1A1816),
                    gpui_kit::rgb(0xF2ECE3),
                    theme,
                    move |_, _, _| d_dark(UiAction::SetAppearance(ThemeAppearance::Dark)),
                ))
                .child(appearance_card(
                    "set.app.system",
                    "跟随系统",
                    "自动同步 macOS 外观",
                    current == ThemeAppearance::System,
                    gpui_kit::rgb(0x282420),
                    gpui_kit::rgb(0xFAF7F2),
                    theme,
                    move |_, _, _| d_sys(UiAction::SetAppearance(ThemeAppearance::System)),
                )),
        )
        .child(Separator::new("set.app.sep1").horizontal())
        .child(setting_row(
            "字体排版栈",
            "英文/数字 Nunito Sans · 简体中文 MiSans · 代码 JetBrains Mono",
            setting_value_text("3-Font Stack", theme),
            theme,
        ))
        .child(Separator::new("set.app.sep2").horizontal())
        .child(setting_row(
            "减弱动态效果",
            "保持桌面端克制稳定的过渡表现",
            Switch::new("set.app.reduced_motion")
                .checked(true)
                .read_only(true),
            theme,
        ))
}

fn render_providers(
    state: &AppUiState,
    custom_acp_input: &Entity<InputState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _cx: &App,
) -> impl IntoElement {
    let d_save = Arc::clone(dispatcher);
    let input_handle = custom_acp_input.clone();

    let mut root = div()
        .flex()
        .flex_col()
        .child(section_heading("自定义 ACP Agent 启动命令", theme))
        .child(
            div()
                .pb(px(14.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(
                    div()
                        .text_size(typo::CAPTION)
                        .text_color(theme.colors.muted_foreground)
                        .child(
                            "通过 JSON-RPC 2.0 stdio 连接任意自定义 Agent Client Protocol 服务。",
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(custom_acp_input).aria_label("自定义 ACP 命令")),
                        )
                        .child(
                            Button::new("set.prov.save_acp")
                                .variant(ButtonVariant::Default)
                                .size(ButtonSize::Sm)
                                .label("保存")
                                .on_click(move |_, _, cx| {
                                    let val = input_handle.read(cx).value().trim().to_string();
                                    let opt = if val.is_empty() { None } else { Some(val) };
                                    d_save(UiAction::SaveCustomAcpCommand(opt));
                                }),
                        ),
                ),
        )
        .child(Separator::new("set.prov.top_sep").horizontal())
        .child(
            div()
                .pt(px(12.0))
                .pb(px(4.0))
                .flex()
                .items_center()
                .justify_between()
                .child(section_heading(
                    "本地已探测 Provider（12 路原生驱动）",
                    theme,
                ))
                .child(setting_value_text(
                    format!(
                        "{} / {} 可用",
                        state
                            .provider_statuses
                            .iter()
                            .filter(|p| {
                                p.install_state == lingxi_domain::ProviderInstallState::Installed
                            })
                            .count(),
                        state.provider_statuses.len()
                    ),
                    theme,
                )),
        );

    for (idx, p) in state.provider_statuses.iter().enumerate() {
        if idx > 0 {
            root = root.child(Separator::new(("set.prov.sep", idx)).horizontal());
        }
        let installed = p.install_state == lingxi_domain::ProviderInstallState::Installed;
        let detail: SharedString = match (&p.version, &p.binary_path) {
            (Some(v), Some(path)) => format!("{v} · {path}").into(),
            (Some(v), None) => v.clone().into(),
            (None, Some(path)) => path.clone().into(),
            (None, None) => "未在 PATH 或默认位置检测到可执行文件".into(),
        };
        root = root.child(setting_row(
            p.kind.display_name(),
            detail,
            setting_status_text(
                if installed { "已就绪" } else { "未安装" },
                installed,
                theme,
            ),
            theme,
        ));
    }

    root
}

fn render_keybindings(theme: &UiTheme) -> impl IntoElement {
    let bindings = [
        ("打开命令面板", "搜索命令、快捷跳转工作区会话", "⌘K"),
        ("展开 / 收起侧边栏", "切换左侧工作区与会话树", "⌘B"),
        ("切换内嵌终端", "打开或关闭右侧 PTY 终端面板", "⌘J"),
        ("打开设置中心", "直达应用设置中心", "⌘,"),
        (
            "发送消息",
            "在输入框直接发送当前消息（支持中文 IME）",
            "Enter",
        ),
        ("消息换行", "在输入框插入换行符", "⇧Enter"),
        ("关闭当前弹窗", "关闭设置、命令面板或更新弹窗", "Esc"),
    ];

    let mut col = div()
        .flex()
        .flex_col()
        .child(section_heading("全局快捷键", theme));
    for (idx, (title, desc, key)) in bindings.into_iter().enumerate() {
        if idx > 0 {
            col = col.child(Separator::new(("set.kb.sep", idx)).horizontal());
        }
        col = col.child(setting_row(title, desc, Kbd::new(key), theme));
    }
    col
}

fn render_mcp_skills(state: &AppUiState, theme: &UiTheme) -> impl IntoElement {
    let mut col = div()
        .flex()
        .flex_col()
        .child(section_heading("MCP 服务", theme))
        .child(setting_row(
            "内置 MCP HTTP Server",
            "基于 Axum 提供 /mcp JSON-RPC 工具注册与调度端点",
            setting_status_text(
                if state.mcp_server_running {
                    "就绪"
                } else {
                    "未启动"
                },
                state.mcp_server_running,
                theme,
            ),
            theme,
        ))
        .child(Separator::new("set.mcp.sep0").horizontal())
        .child(
            div()
                .pt(px(12.0))
                .pb(px(4.0))
                .flex()
                .items_center()
                .justify_between()
                .child(section_heading("已发现本地 Skills", theme))
                .child(setting_value_text(
                    format!("{} 个", state.discovered_skills.len()),
                    theme,
                )),
        );

    if state.discovered_skills.is_empty() {
        col = col.child(
            div()
                .py(px(10.0))
                .text_size(typo::UI_BODY)
                .text_color(theme.colors.muted_foreground)
                .child("未在 ~/.codex/skills 或 ~/.claude/skills 发现额外 Skill 清单。"),
        );
    } else {
        for (idx, skill) in state.discovered_skills.iter().take(20).enumerate() {
            if idx > 0 {
                col = col.child(Separator::new(("set.skill.sep", idx)).horizontal());
            }
            col = col.child(setting_row(
                skill.name.clone(),
                skill.description.clone(),
                setting_mono_text("SKILL.md", theme),
                theme,
            ));
        }
    }

    col
}

fn render_remote(
    state: &AppUiState,
    remote_port_input: &Entity<InputState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
    _cx: &App,
) -> impl IntoElement {
    let d_toggle = Arc::clone(dispatcher);
    let d_port = Arc::clone(dispatcher);
    let port_handle = remote_port_input.clone();

    div()
        .flex()
        .flex_col()
        .child(section_heading("远程访问与配对", theme))
        .child(setting_row(
            "远程 WS/RPC 服务",
            "允许同一局域网或 SSH 隧道客户端通过配对码接入",
            Switch::new("set.remote.switch")
                .checked(state.remote_server_running)
                .on_change(move |_, _, _| d_toggle(UiAction::ToggleRemoteServer)),
            theme,
        ))
        .child(Separator::new("set.remote.sep1").horizontal())
        .child(
            div()
                .py(px(12.0))
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .child(
                    div()
                        .text_size(typo::UI_BODY)
                        .font_weight(FontWeight::MEDIUM)
                        .child("监听端口"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .w(px(110.0))
                                .child(Input::new(remote_port_input).aria_label("远程服务端口")),
                        )
                        .child(
                            Button::new("set.remote.save_port")
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm)
                                .label("应用端口")
                                .on_click(move |_, _, cx| {
                                    if let Ok(port) =
                                        port_handle.read(cx).value().trim().parse::<u16>()
                                    {
                                        d_port(UiAction::SetRemotePort(port));
                                    }
                                }),
                        ),
                ),
        )
        .child(Separator::new("set.remote.sep2").horizontal())
        .child(setting_row(
            "安全边界",
            "仅接受作用域受限的 Pairing Token，禁止暴露明文凭据",
            setting_value_text("Scoped Token", theme),
            theme,
        ))
}

fn render_legacy_import(
    state: &AppUiState,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    let d_import = Arc::clone(dispatcher);
    let detected: SharedString = state
        .legacy_db_path
        .clone()
        .unwrap_or_else(|| "未检测到 ~/.lingxi-t3 原版数据库".to_string())
        .into();

    div()
        .flex()
        .flex_col()
        .child(section_heading("原版数据只读迁移", theme))
        .child(setting_row(
            "原版 LingXi T3 数据库探测",
            detected,
            setting_status_text(
                if state.legacy_db_path.is_some() {
                    "已检测到"
                } else {
                    "未找到"
                },
                state.legacy_db_path.is_some(),
                theme,
            ),
            theme,
        ))
        .child(Separator::new("set.imp.sep1").horizontal())
        .child(setting_row(
            "只读安全导入",
            "以 SQLITE_OPEN_READ_ONLY 打开原库，绝不写入或覆盖原版目录；导入前自动备份原生库。",
            Button::new("set.imp.run")
                .variant(ButtonVariant::Default)
                .size(ButtonSize::Sm)
                .disabled(state.legacy_db_path.is_none())
                .label("立即只读导入")
                .on_click(move |_, _, _| d_import(UiAction::RunLegacyImport)),
            theme,
        ))
        .when_some(state.last_import_summary.clone(), |el, summary| {
            el.child(Separator::new("set.imp.sep2").horizontal()).child(
                div()
                    .py(px(12.0))
                    .text_size(typo::UI_BODY)
                    .text_color(theme.colors.success)
                    .child(summary),
            )
        })
}

fn render_about(
    state: &AppUiState,
    update_channel_select: &Entity<SelectState>,
    dispatcher: &ActionDispatcher,
    theme: &UiTheme,
) -> impl IntoElement {
    let d_check = Arc::clone(dispatcher);
    let update_desc: SharedString = match &state.available_update {
        Some(u) => format!("发现新版本 {} ({})", u.version, u.published_at).into(),
        None if state.update_checking => "正在检查 GitHub Releases…".into(),
        None => "当前已是最新版本，支持 Stable / Nightly 双通道检查。".into(),
    };

    let misans_status = if has_misans_available() {
        "MiSans（已本地嵌入）"
    } else {
        "PingFang SC / 系统回退"
    };

    div()
        .flex()
        .flex_col()
        .child(section_heading("版本与更新", theme))
        .child(setting_row(
            "LingXi T3 Native",
            "Rust + GPUI + gpuicn + SQLite WAL + Tokio 原生桌面版",
            setting_mono_text(format!("v{}", env!("CARGO_PKG_VERSION")), theme),
            theme,
        ))
        .child(Separator::new("set.about.sep1").horizontal())
        .child(setting_row(
            "更新通道",
            "选择接收正式版或 Nightly 预览版",
            Select::new(update_channel_select).w(px(160.0)),
            theme,
        ))
        .child(Separator::new("set.about.sep2").horizontal())
        .child(setting_row(
            "检查更新",
            update_desc,
            Button::new("set.about.check")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Sm)
                .label("立即检查")
                .on_click(move |_, _, _| d_check(UiAction::CheckUpdates)),
            theme,
        ))
        .child(Separator::new("set.about.sep3").horizontal())
        .child(
            div()
                .pt(px(12.0))
                .child(section_heading("字体与开源许可", theme)),
        )
        .child(setting_row(
            "Nunito Sans & JetBrains Mono",
            "SIL Open Font License 1.1 · Copyright © Nunito / JetBrains Mono Project Authors",
            setting_value_text("SIL OFL 1.1", theme),
            theme,
        ))
        .child(Separator::new("set.about.sep4").horizontal())
        .child(setting_row(
            "MiSans 简体中文字体",
            "Copyright © Beijing Xiaomi Mobile Software Co., Ltd. · 仅随应用内部嵌入使用",
            setting_value_text(misans_status, theme),
            theme,
        ))
        .child(Separator::new("set.about.sep5").horizontal())
        .child(setting_row(
            "软件开源许可",
            "MIT License · 基于 GPUI、gpui-kit、gpuicn、Tokio、SQLite 构建",
            setting_value_text("MIT", theme),
            theme,
        ))
}
