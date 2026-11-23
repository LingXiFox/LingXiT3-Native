use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeAppearance {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    #[default]
    Stable,
    Nightly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeybindingEntry {
    pub action: String,
    pub shortcut: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub appearance: ThemeAppearance,
    pub ui_scale: f32,
    pub font_family: String,
    pub mono_font_family: String,
    pub language: String,
    pub update_channel: UpdateChannel,
    pub auto_check_updates: bool,
    pub remote_server_enabled: bool,
    pub remote_server_port: u16,
    pub custom_acp_command: Option<String>,
    pub keybindings: Vec<KeybindingEntry>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            appearance: ThemeAppearance::System,
            ui_scale: 1.0,
            font_family: "Geist".to_string(),
            mono_font_family: "Geist Mono".to_string(),
            language: "zh-CN".to_string(),
            update_channel: UpdateChannel::Stable,
            auto_check_updates: true,
            remote_server_enabled: false,
            remote_server_port: 3777,
            custom_acp_command: None,
            keybindings: default_keybindings(),
        }
    }
}

pub fn default_keybindings() -> Vec<KeybindingEntry> {
    vec![
        KeybindingEntry {
            action: "command_palette".to_string(),
            shortcut: "cmd-k".to_string(),
        },
        KeybindingEntry {
            action: "new_thread".to_string(),
            shortcut: "cmd-n".to_string(),
        },
        KeybindingEntry {
            action: "toggle_sidebar".to_string(),
            shortcut: "cmd-b".to_string(),
        },
        KeybindingEntry {
            action: "toggle_terminal".to_string(),
            shortcut: "cmd-j".to_string(),
        },
        KeybindingEntry {
            action: "open_settings".to_string(),
            shortcut: "cmd-,".to_string(),
        },
    ]
}
