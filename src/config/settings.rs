use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_indent_size")]
    pub indent_size: usize,
    #[serde(default = "default_use_spaces")]
    pub use_spaces: bool,
    #[serde(default = "default_autosave_delay_ms")]
    pub autosave_delay_ms: u64,
    #[serde(default = "default_horizontal_padding")]
    pub horizontal_padding: usize,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_indent_size() -> usize { 4 }
fn default_use_spaces() -> bool { true }
fn default_autosave_delay_ms() -> u64 { 500 }
fn default_horizontal_padding() -> usize { 0 }
fn default_theme() -> String { "kilo-dark".to_string() }

impl Default for Settings {
    fn default() -> Self {
        Self {
            indent_size: default_indent_size(),
            use_spaces: default_use_spaces(),
            autosave_delay_ms: default_autosave_delay_ms(),
            horizontal_padding: default_horizontal_padding(),
            theme: default_theme(),
        }
    }
}

impl Settings {
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("kilo");
        config_dir.join("kilo.conf.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(settings) => {
                        tracing::info!("loaded config from {}", path.display());
                        return settings;
                    }
                    Err(e) => {
                        tracing::warn!("failed to parse config: {}", e);
                    }
                },
                Err(e) => {
                    tracing::warn!("failed to read config: {}", e);
                }
            }
        }
        let settings = Self::default();
        settings.save_default();
        settings
    }

    fn save_default(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if !path.exists() {
            let content = toml::to_string_pretty(self).unwrap_or_default();
            let _ = std::fs::write(&path, content);
            tracing::info!("created default config at {}", path.display());
        }
    }
}
