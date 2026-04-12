use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_CONFIG: &str = r##"# ─── reedo configuration ──────────────────────────────────────
# location: ~/.config/reedo/reedo.conf.toml
# open this file in reedo with ctrl+,

# ─── editing ──────────────────────────────────────────────────

# number of spaces per indent level
# indent_size = 4

# use spaces instead of tabs
# use_spaces = true

# ─── autosave ─────────────────────────────────────────────────

# delay in ms after last edit before autosaving (0 to disable)
# autosave_delay_ms = 500

# ─── appearance ───────────────────────────────────────────────

# horizontal padding (chars) between gutter and text / text and edge
horizontal_padding = 4

# wrap long lines to the editor width
line_wrapping = true

# color theme — bundled: Default, reedo-dark, reedo-light,
#   catppuccin, dracula, gruvbox, nord, rose-pine, solarized-dark
# custom: drop a .toml in ~/.config/reedo/themes/
theme = "reedo-dark"
"##;

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
    #[serde(default = "default_line_wrapping")]
    pub line_wrapping: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub side_panel_open: bool,
}

fn default_indent_size() -> usize {
    4
}
fn default_use_spaces() -> bool {
    true
}
fn default_autosave_delay_ms() -> u64 {
    500
}
fn default_horizontal_padding() -> usize {
    4
}
fn default_line_wrapping() -> bool {
    true
}
fn default_theme() -> String {
    "reedo-dark".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            indent_size: default_indent_size(),
            use_spaces: default_use_spaces(),
            autosave_delay_ms: default_autosave_delay_ms(),
            horizontal_padding: default_horizontal_padding(),
            line_wrapping: default_line_wrapping(),
            theme: default_theme(),
            side_panel_open: false,
        }
    }
}

impl Settings {
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("reedo");
        config_dir.join("reedo.conf.toml")
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

    pub fn update_side_panel(open: bool) {
        let path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            let mut found = false;
            let new_content: String = content
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if trimmed.starts_with("side_panel_open") {
                        found = true;
                        format!("side_panel_open = {}", open)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let final_content = if found {
                new_content
            } else {
                format!("{}\nside_panel_open = {}", content.trim_end(), open)
            };
            let _ = std::fs::write(&path, final_content);
        }
    }

    pub fn update_theme(name: &str) {
        let path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            // replace existing theme line or append it
            let mut found = false;
            let new_content: String = content
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if trimmed.starts_with("theme") || trimmed.starts_with("# theme") {
                        found = true;
                        format!("theme = \"{}\"", name)
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let final_content = if found {
                new_content
            } else {
                format!("{}\ntheme = \"{}\"", content.trim_end(), name)
            };
            let _ = std::fs::write(&path, final_content);
        } else {
            // no config file — create one with just the theme
            let _ = std::fs::write(&path, format!("theme = \"{}\"\n", name));
        }
    }

    fn save_default(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if !path.exists() {
            let _ = std::fs::write(&path, DEFAULT_CONFIG);
            tracing::info!("created default config at {}", path.display());
        }
    }
}
