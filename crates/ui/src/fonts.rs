use std::borrow::Cow;
use std::path::{Path, PathBuf};

use gpui_kit::FontFallbacks;

pub const PRIMARY_UI_FONT: &str = "Nunito Sans";
pub const PRIMARY_MONO_FONT: &str = "JetBrains Mono";
pub const PRIMARY_CJK_FONT: &str = "MiSans";

/// Returns embedded SIL OFL 1.1 fonts (Nunito Sans + JetBrains Mono) plus
/// locally bundled MiSans TTFs when available in the app bundle or local dev directory.
pub fn load_app_fonts() -> Vec<Cow<'static, [u8]>> {
    let mut fonts = vec![
        Cow::Borrowed(include_bytes!("../../../assets/fonts/NunitoSans.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf") as &[u8]),
        Cow::Borrowed(include_bytes!("../../../assets/fonts/JetBrainsMono-Medium.ttf") as &[u8]),
    ];

    for path in find_local_misans_paths() {
        if let Ok(bytes) = std::fs::read(&path) {
            tracing::debug!(path = %path.display(), "Loaded local MiSans font");
            fonts.push(Cow::Owned(bytes));
        }
    }

    fonts
}

fn find_local_misans_paths() -> Vec<PathBuf> {
    let filenames = [
        "MiSans-Regular.ttf",
        "MiSans-Medium.ttf",
        "MiSans-Semibold.ttf",
    ];
    let mut search_dirs: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe()
        && let Some(macos_dir) = exe.parent()
    {
        search_dirs.push(macos_dir.join("../Resources/fonts"));
    }

    if let Ok(cwd) = std::env::current_dir() {
        search_dirs.push(cwd.join("assets/fonts/local"));
    }

    let manifest_local = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/local");
    search_dirs.push(manifest_local);

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        search_dirs.push(home.join(".lingxi-t3-native/fonts"));
    }

    for dir in search_dirs {
        let mut found = Vec::new();
        for name in filenames {
            let p = dir.join(name);
            if p.is_file() {
                found.push(p);
            }
        }
        if !found.is_empty() {
            return found;
        }
    }

    Vec::new()
}

pub fn has_misans_available() -> bool {
    !find_local_misans_paths().is_empty()
}

/// UI font fallback chain: MiSans first for Simplified Chinese, then OS-native CJK fallbacks.
pub fn ui_font_fallbacks() -> FontFallbacks {
    FontFallbacks::from_fonts(vec![
        PRIMARY_CJK_FONT.to_string(),
        "PingFang SC".to_string(),
        "Hiragino Sans GB".to_string(),
        "Microsoft YaHei UI".to_string(),
        "Noto Sans CJK SC".to_string(),
    ])
}

/// Monospace fallback chain for code, Diff, and PTY terminal.
pub fn mono_font_fallbacks() -> FontFallbacks {
    FontFallbacks::from_fonts(vec![
        PRIMARY_CJK_FONT.to_string(),
        "PingFang SC".to_string(),
        "Microsoft YaHei UI".to_string(),
        "Noto Sans CJK SC".to_string(),
    ])
}
