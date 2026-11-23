use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

pub const APP_BUNDLE_ID: &str = "com.lingxifox.lingxit3.native";
pub const APP_DISPLAY_NAME: &str = "LingXi T3 Native";
pub const APP_URL_SCHEME: &str = "lingxit3-native";
pub const NATIVE_HOME_DIR_NAME: &str = ".lingxi-t3-native";

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("keychain error: {0}")]
    Keychain(String),
    #[error("process launch failed: {0}")]
    Process(String),
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub home_dir: PathBuf,
    pub userdata_dir: PathBuf,
    pub database_path: PathBuf,
    pub backups_dir: PathBuf,
    pub attachments_dir: PathBuf,
    pub worktrees_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub settings_path: PathBuf,
    pub lock_path: PathBuf,
}

impl AppPaths {
    pub fn resolve(override_dir: Option<&Path>) -> Self {
        let home_dir = match override_dir {
            Some(dir) => dir.to_path_buf(),
            None => dirs::home_dir()
                .map(|h| h.join(NATIVE_HOME_DIR_NAME))
                .unwrap_or_else(|| PathBuf::from(NATIVE_HOME_DIR_NAME)),
        };
        let userdata_dir = home_dir.join("userdata");
        let database_path = userdata_dir.join("native.sqlite");
        let backups_dir = userdata_dir.join("backups");
        let attachments_dir = userdata_dir.join("attachments");
        let worktrees_dir = home_dir.join("worktrees");
        let logs_dir = home_dir.join("logs");
        let settings_path = home_dir.join("settings.toml");
        let lock_path = home_dir.join("instance.lock");
        Self {
            home_dir,
            userdata_dir,
            database_path,
            backups_dir,
            attachments_dir,
            worktrees_dir,
            logs_dir,
            settings_path,
            lock_path,
        }
    }

    pub fn ensure_dirs(&self) -> Result<(), PlatformError> {
        for dir in [
            &self.home_dir,
            &self.userdata_dir,
            &self.backups_dir,
            &self.attachments_dir,
            &self.worktrees_dir,
            &self.logs_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// Locates the legacy LingXi T3 Electron database for read-only import without modifying it.
    pub fn discover_legacy_database() -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        let candidates = [
            home.join(".lingxi-t3/userdata/statev2.sqlite"),
            home.join(".lingxi-t3/userdata/state.sqlite"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }
}

pub struct SingleInstanceGuard {
    lock_path: PathBuf,
    _file: File,
}

impl SingleInstanceGuard {
    pub fn acquire(lock_path: &Path) -> Result<Self, PlatformError> {
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_path)?;
        writeln!(file, "{}", std::process::id())?;
        Ok(Self {
            lock_path: lock_path.to_path_buf(),
            _file: file,
        })
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

pub fn set_clipboard_text(text: &str) -> Result<(), PlatformError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| PlatformError::Clipboard(e.to_string()))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| PlatformError::Clipboard(e.to_string()))
}

pub fn get_clipboard_text() -> Result<String, PlatformError> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| PlatformError::Clipboard(e.to_string()))?;
    clipboard
        .get_text()
        .map_err(|e| PlatformError::Clipboard(e.to_string()))
}

pub fn store_secret(account: &str, secret: &str) -> Result<(), PlatformError> {
    let entry = keyring::Entry::new(APP_BUNDLE_ID, account)
        .map_err(|e| PlatformError::Keychain(e.to_string()))?;
    entry
        .set_password(secret)
        .map_err(|e| PlatformError::Keychain(e.to_string()))
}

pub fn read_secret(account: &str) -> Result<Option<String>, PlatformError> {
    let entry = keyring::Entry::new(APP_BUNDLE_ID, account)
        .map_err(|e| PlatformError::Keychain(e.to_string()))?;
    match entry.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(PlatformError::Keychain(err.to_string())),
    }
}

pub fn send_desktop_notification(title: &str, body: &str) -> Result<(), PlatformError> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('"', "\\\""),
            title.replace('"', "\\\"")
        );
        Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| PlatformError::Process(e.to_string()))?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("notify-send").arg(title).arg(body).output();
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        tracing::info!(%title, %body, "desktop notification requested");
        Ok(())
    }
}

pub fn open_external_path(target: &str) -> Result<(), PlatformError> {
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = Command::new("open");
        c.arg(target);
        c
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = Command::new("xdg-open");
        c.arg(target);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", target]);
        c
    };
    cmd.spawn()
        .map_err(|e| PlatformError::Process(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_paths_are_isolated_from_electron_t3() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = AppPaths::resolve(Some(temp.path()));
        paths.ensure_dirs().expect("ensure dirs");
        assert!(paths.userdata_dir.exists());
        assert!(paths.database_path.ends_with("native.sqlite"));
        assert!(!paths.home_dir.to_string_lossy().ends_with(".lingxi-t3"));
    }

    #[test]
    fn single_instance_guard_creates_and_cleans_lock() {
        let temp = tempfile::tempdir().expect("tempdir");
        let lock_path = temp.path().join("instance.lock");
        {
            let _guard = SingleInstanceGuard::acquire(&lock_path).expect("acquire");
            assert!(lock_path.exists());
        }
        assert!(!lock_path.exists());
    }
}
