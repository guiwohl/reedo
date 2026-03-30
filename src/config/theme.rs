use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    #[serde(default = "default_bg")]
    pub bg: String,
    #[serde(default = "default_fg")]
    pub fg: String,
    #[serde(default = "default_gutter")]
    pub gutter: String,
    #[serde(default = "default_cursor_bg")]
    pub cursor_bg: String,
    #[serde(default = "default_cursor_fg")]
    pub cursor_fg: String,
    #[serde(default = "default_selection")]
    pub selection: String,
    #[serde(default = "default_statusbar_bg")]
    pub statusbar_bg: String,
    #[serde(default = "default_statusbar_fg")]
    pub statusbar_fg: String,
    #[serde(default = "default_keyword")]
    pub keyword: String,
    #[serde(default = "default_string")]
    pub string: String,
    #[serde(default = "default_comment")]
    pub comment: String,
    #[serde(default = "default_function")]
    pub function: String,
    #[serde(default = "default_type_color")]
    pub r#type: String,
    #[serde(default = "default_number")]
    pub number: String,
    #[serde(default = "default_operator")]
    pub operator: String,
    #[serde(default = "default_property")]
    pub property: String,
}

fn default_bg() -> String { "#1a1b26".into() }
fn default_fg() -> String { "#c0caf5".into() }
fn default_gutter() -> String { "#3b4261".into() }
fn default_cursor_bg() -> String { "#c0caf5".into() }
fn default_cursor_fg() -> String { "#1a1b26".into() }
fn default_selection() -> String { "#283457".into() }
fn default_statusbar_bg() -> String { "#1e1e2e".into() }
fn default_statusbar_fg() -> String { "#a6adc8".into() }
fn default_keyword() -> String { "#bb9af7".into() }
fn default_string() -> String { "#9ece6a".into() }
fn default_comment() -> String { "#565f89".into() }
fn default_function() -> String { "#7daeF7".into() }
fn default_type_color() -> String { "#2ac3de".into() }
fn default_number() -> String { "#ff9e64".into() }
fn default_operator() -> String { "#89ddff".into() }
fn default_property() -> String { "#73bac2".into() }

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            bg: default_bg(),
            fg: default_fg(),
            gutter: default_gutter(),
            cursor_bg: default_cursor_bg(),
            cursor_fg: default_cursor_fg(),
            selection: default_selection(),
            statusbar_bg: default_statusbar_bg(),
            statusbar_fg: default_statusbar_fg(),
            keyword: default_keyword(),
            string: default_string(),
            comment: default_comment(),
            function: default_function(),
            r#type: default_type_color(),
            number: default_number(),
            operator: default_operator(),
            property: default_property(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "kilo-dark".to_string(),
            colors: ThemeColors::default(),
        }
    }
}

pub fn parse_hex_color(hex: &str) -> ratatui::style::Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return ratatui::style::Color::Rgb(r, g, b);
        }
    }
    ratatui::style::Color::White
}

pub fn bundled_themes() -> Vec<Theme> {
    vec![
        Theme::default(), // kilo-dark
        Theme {
            name: "kilo-light".to_string(),
            colors: ThemeColors {
                bg: "#fafafa".into(),
                fg: "#383a42".into(),
                gutter: "#9ca0a4".into(),
                cursor_bg: "#383a42".into(),
                cursor_fg: "#fafafa".into(),
                selection: "#bfceff".into(),
                statusbar_bg: "#e5e5e5".into(),
                statusbar_fg: "#383a42".into(),
                keyword: "#a626a4".into(),
                string: "#50a14f".into(),
                comment: "#a0a1a7".into(),
                function: "#4078f2".into(),
                r#type: "#c18401".into(),
                number: "#986801".into(),
                operator: "#0184bc".into(),
                property: "#e45649".into(),
            },
        },
        Theme {
            name: "catppuccin".to_string(),
            colors: ThemeColors {
                bg: "#1e1e2e".into(),
                fg: "#cdd6f4".into(),
                gutter: "#585b70".into(),
                cursor_bg: "#f5e0dc".into(),
                cursor_fg: "#1e1e2e".into(),
                selection: "#45475a".into(),
                statusbar_bg: "#181825".into(),
                statusbar_fg: "#a6adc8".into(),
                keyword: "#cba6f7".into(),
                string: "#a6e3a1".into(),
                comment: "#6c7086".into(),
                function: "#89b4fa".into(),
                r#type: "#89dceb".into(),
                number: "#fab387".into(),
                operator: "#94e2d5".into(),
                property: "#f38ba8".into(),
            },
        },
    ]
}

pub fn load_theme(name: &str) -> Theme {
    // check for custom theme file
    let theme_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("kilo")
        .join("themes");

    let theme_file = theme_dir.join(format!("{}.toml", name));
    if theme_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&theme_file) {
            if let Ok(theme) = toml::from_str::<Theme>(&content) {
                tracing::info!("loaded custom theme: {}", name);
                return theme;
            }
        }
    }

    // fall back to bundled
    bundled_themes()
        .into_iter()
        .find(|t| t.name == name)
        .unwrap_or_default()
}
