mod config;
mod runtime;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use gpui_kit::{AppContext as _, Entity, KeyBinding, Window, actions, px, size};
use gpuicn::app::AppConfig;
use lingxi_platform::{APP_DISPLAY_NAME, AppPaths, SingleInstanceGuard};
use lingxi_ui::shell::{ActionDispatcher, LingXiShellView, toggle_appearance_mode};
use lingxi_ui::state::{AppUiState, InspectorTab, SettingsSection, UiAction};
use lingxi_ui::theme::{
    apply_appearance, lingxi_dark_theme, lingxi_light_theme, to_gpuicn_appearance,
};
use tracing_subscriber::EnvFilter;

use crate::config::{load_settings, save_settings};
use crate::runtime::AppRuntime;

actions!(
    lingxi,
    [
        ToggleCommandPalette,
        ToggleSidebarAction,
        ToggleTerminalAction,
        OpenSettingsAction,
    ]
);

#[derive(Debug, Parser)]
#[command(
    name = "lingxi-t3-native",
    version,
    about = "LingXi T3 Native GPUI Desktop"
)]
struct Cli {
    /// Override the native application data directory (defaults to ~/.lingxi-t3-native).
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Perform headless startup initialization checks and exit immediately.
    #[arg(long)]
    check_startup: bool,

    /// Run headless server mode (without opening the GPUI window).
    #[arg(long)]
    headless_server: bool,

    /// Path to a legacy LingXi T3 SQLite database to inspect/import read-only.
    #[arg(long)]
    import_legacy: Option<PathBuf>,
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

fn handle_ui_action(
    view: &Entity<LingXiShellView>,
    runtime: &AppRuntime,
    action: UiAction,
    cx: &mut gpui_kit::AsyncApp,
) {
    cx.update_entity(view, |shell, cx| {
        match action {
            UiAction::SelectThread(tid) => shell.state.select_thread(tid),
            UiAction::CloseTab(tid) => shell.state.close_tab(&tid),
            UiAction::ToggleProjectExpanded(pid) => {
                if !shell.state.expanded_projects.remove(&pid) {
                    shell.state.expanded_projects.insert(pid);
                }
            }
            UiAction::ToggleSidebar => {
                shell.state.sidebar_collapsed = !shell.state.sidebar_collapsed;
            }
            UiAction::ToggleInspector(tab) => {
                if shell.state.inspector_open && shell.state.inspector_tab == tab {
                    shell.state.inspector_open = false;
                } else {
                    shell.state.inspector_open = true;
                    shell.state.inspector_tab = tab;
                }
            }
            UiAction::ToggleAppearance => {
                let next = toggle_appearance_mode(shell.state.settings.appearance);
                shell.state.settings.appearance = next;
                apply_appearance(cx, next);
                let _ = save_settings(&runtime.paths.settings_path, &shell.state.settings);
            }
            UiAction::OpenSettings(section) => {
                shell.state.settings_section = section;
                shell.state.settings_open = true;
            }
            UiAction::CloseSettings => shell.state.settings_open = false,
            UiAction::OpenCommandPalette => shell.state.command_palette_open = true,
            UiAction::CloseCommandPalette => shell.state.command_palette_open = false,
            UiAction::OpenUpdateModal => shell.state.update_modal_open = true,
            UiAction::CloseUpdateModal => shell.state.update_modal_open = false,
            other => {
                tracing::debug!(?other, "UI action queued for backend services");
            }
        }
        cx.notify();
    });
}

fn main() -> anyhow::Result<()> {
    init_logging();
    let cli = Cli::parse();

    let paths = Arc::new(AppPaths::resolve(cli.data_dir.as_deref()));
    paths.ensure_dirs()?;
    if !paths.settings_path.exists() {
        save_settings(&paths.settings_path, &lingxi_domain::AppSettings::default())?;
    }
    let settings = load_settings(&paths.settings_path)?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("lingxi-bg")
        .build()?;

    let app_runtime = AppRuntime {
        paths: Arc::clone(&paths),
        settings: settings.clone(),
        tokio_handle: rt.handle().clone(),
    };

    if cli.check_startup {
        let _guard = SingleInstanceGuard::acquire(&paths.lock_path)?;
        let ping = rt.block_on(app_runtime.spawn(async { "ok" }))?;
        tracing::info!(
            home_dir = %paths.home_dir.display(),
            appearance = ?settings.appearance,
            tokio = ping,
            "LingXi T3 Native startup self-check passed"
        );
        return Ok(());
    }

    if cli.headless_server {
        let _guard = SingleInstanceGuard::acquire(&paths.lock_path)?;
        tracing::info!(
            port = settings.remote_server_port,
            "Headless server mode initialized"
        );
        return Ok(());
    }

    if let Some(legacy_path) = cli.import_legacy {
        tracing::info!(path = %legacy_path.display(), "Legacy import target queued");
    }

    let _guard = SingleInstanceGuard::acquire(&paths.lock_path)?;
    let (action_tx, mut action_rx) = tokio::sync::mpsc::unbounded_channel::<UiAction>();
    let dispatcher: ActionDispatcher = Arc::new(move |action| {
        let _ = action_tx.send(action);
    });

    let dispatcher_for_keys = Arc::clone(&dispatcher);
    let ui_state = AppUiState::new(settings.clone());
    let runtime_for_ui = app_runtime.clone();

    gpuicn::app::run(
        AppConfig::new(APP_DISPLAY_NAME)
            .themes(lingxi_light_theme(), lingxi_dark_theme())
            .appearance(to_gpuicn_appearance(settings.appearance)),
        move |window: &mut Window, cx| {
            window.set_window_title(APP_DISPLAY_NAME);
            window.resize(size(px(1360.0), px(860.0)));
            cx.bind_keys([
                KeyBinding::new("cmd-k", ToggleCommandPalette, None),
                KeyBinding::new("ctrl-k", ToggleCommandPalette, None),
                KeyBinding::new("cmd-b", ToggleSidebarAction, None),
                KeyBinding::new("ctrl-b", ToggleSidebarAction, None),
                KeyBinding::new("cmd-j", ToggleTerminalAction, None),
                KeyBinding::new("ctrl-j", ToggleTerminalAction, None),
                KeyBinding::new("cmd-,", OpenSettingsAction, None),
                KeyBinding::new("ctrl-,", OpenSettingsAction, None),
            ]);
            let d1 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &ToggleCommandPalette, _| {
                d1(UiAction::OpenCommandPalette)
            });
            let d2 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &ToggleSidebarAction, _| {
                d2(UiAction::ToggleSidebar)
            });
            let d3 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &ToggleTerminalAction, _| {
                d3(UiAction::ToggleInspector(InspectorTab::Terminal))
            });
            let d4 = Arc::clone(&dispatcher_for_keys);
            gpui_kit::App::on_action(cx, move |_: &OpenSettingsAction, _| {
                d4(UiAction::OpenSettings(SettingsSection::General))
            });

            let entity = cx.entity();
            let runtime_for_loop = runtime_for_ui.clone();
            let _bg = runtime_for_ui.spawn(async {
                tracing::debug!("App background Tokio loop ready");
            });
            cx.spawn(async move |_weak, cx| {
                while let Some(action) = action_rx.recv().await {
                    handle_ui_action(&entity, &runtime_for_loop, action, cx);
                }
            })
            .detach();
            LingXiShellView::new(ui_state, dispatcher, window, cx)
        },
    );

    Ok(())
}
