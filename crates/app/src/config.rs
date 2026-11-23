use std::path::Path;

use anyhow::Context;
use lingxi_domain::AppSettings;

pub fn load_settings(path: &Path) -> anyhow::Result<AppSettings> {
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read settings from {}", path.display()))?;
    let settings: AppSettings = toml::from_str(&raw)
        .with_context(|| format!("failed to parse settings TOML at {}", path.display()))?;
    Ok(settings)
}

pub fn save_settings(path: &Path, settings: &AppSettings) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let serialized = toml::to_string_pretty(settings)?;
    std::fs::write(path, serialized)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lingxi_domain::{ThemeAppearance, UpdateChannel};

    #[test]
    fn roundtrip_settings_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("settings.toml");
        let settings = AppSettings {
            appearance: ThemeAppearance::Dark,
            update_channel: UpdateChannel::Nightly,
            ..Default::default()
        };
        save_settings(&path, &settings).expect("save");
        let loaded = load_settings(&path).expect("load");
        assert_eq!(loaded.appearance, ThemeAppearance::Dark);
        assert_eq!(loaded.update_channel, UpdateChannel::Nightly);
    }
}
