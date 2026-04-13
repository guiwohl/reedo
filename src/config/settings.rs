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

    pub fn session_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("reedo");
        config_dir.join("session.json")
    }

    pub fn save_session(file_path: &std::path::Path, line: usize, col: usize) {
        let path = Self::session_path();
        let data = serde_json::json!({
            "file": file_path.to_string_lossy(),
            "line": line,
            "col": col,
        });
        let _ = std::fs::write(&path, data.to_string());
    }

    pub fn load_session() -> Option<(PathBuf, usize, usize)> {
        let path = Self::session_path();
        let content = std::fs::read_to_string(&path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&content).ok()?;
        let file = v.get("file")?.as_str()?;
        let line = v.get("line")?.as_u64()? as usize;
        let col = v.get("col")?.as_u64()? as usize;
        let file_path = PathBuf::from(file);
        if file_path.exists() {
            Some((file_path, line, col))
        } else {
            None
        }
    }

    pub fn apply_editorconfig(&mut self, file_path: &std::path::Path, project_root: &std::path::Path) {
        let ec_path = project_root.join(".editorconfig");
        if !ec_path.exists() {
            return;
        }
        let content = match std::fs::read_to_string(&ec_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mut in_matching_section = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let pattern = &trimmed[1..trimmed.len() - 1];
                in_matching_section = editorconfig_glob_matches(pattern, &file_name, ext);
                continue;
            }
            if !in_matching_section {
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim();
                match key.as_str() {
                    "indent_style" => {
                        self.use_spaces = value.eq_ignore_ascii_case("space");
                    }
                    "indent_size" => {
                        if let Ok(n) = value.parse::<usize>() {
                            self.indent_size = n;
                        }
                    }
                    _ => {}
                }
            }
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

fn editorconfig_glob_matches(pattern: &str, filename: &str, ext: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    // *.ext pattern
    if let Some(pat_ext) = pattern.strip_prefix("*.") {
        if pat_ext.contains(',') {
            // {rs,py,js} style
            let inner = pat_ext
                .strip_prefix('{')
                .and_then(|s| s.strip_suffix('}'))
                .unwrap_or(pat_ext);
            return inner.split(',').any(|e| e.trim() == ext);
        }
        return pat_ext == ext;
    }
    // exact filename match
    pattern == filename
}
