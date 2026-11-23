mod config;
mod runtime;

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use gpui_kit::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, size};
use gpuicn::app::AppConfig;
use gpuicn::theme::UiTheme;
use lingxi_platform::{APP_DISPLAY_NAME, AppPaths, SingleInstanceGuard};
use lingxi_ui::theme::{lingxi_dark_theme, lingxi_light_theme, to_gpuicn_appearance};
use tracing_subscriber::EnvFilter;

use crate::config::{load_settings, save_settings};
use crate::runtime::AppRuntime;

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

struct BootstrapRootView {
    runtime: AppRuntime,
}

impl Render for BootstrapRootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = UiTheme::read(cx);
        div()
            .size_full()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(APP_DISPLAY_NAME),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.colors.muted_foreground)
                    .child(format!(
                        "Data root: {} | Appearance: {:?}",
                        self.runtime.paths.home_dir.display(),
                        self.runtime.settings.appearance
                    )),
            )
    }
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
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

    let runtime_for_ui = app_runtime.clone();
    gpuicn::app::run(
        AppConfig::new(APP_DISPLAY_NAME)
            .themes(lingxi_light_theme(), lingxi_dark_theme())
            .appearance(to_gpuicn_appearance(settings.appearance)),
        move |window, cx| {
            window.set_window_title(APP_DISPLAY_NAME);
            window.resize(size(px(1280.0), px(820.0)));
            let _ = cx;
            BootstrapRootView {
                runtime: runtime_for_ui,
            }
        },
    );

    Ok(())
}
