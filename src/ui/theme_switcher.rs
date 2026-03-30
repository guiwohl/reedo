use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::config::theme::{self, Theme};

#[derive(Debug, Clone)]
pub struct ThemeSwitcherState {
    pub themes: Vec<Theme>,
    pub selected: usize,
}

impl Default for ThemeSwitcherState {
    fn default() -> Self {
        Self {
            themes: theme::bundled_themes(),
            selected: 0,
        }
    }
}

impl ThemeSwitcherState {
    pub fn reset(&mut self) {
        self.themes = theme::bundled_themes();
        // also scan custom themes dir
        let theme_dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
            .join("kilo")
            .join("themes");
        if theme_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&theme_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(t) = toml::from_str::<Theme>(&content) {
                                if !self.themes.iter().any(|existing| existing.name == t.name) {
                                    self.themes.push(t);
                                }
                            }
                        }
                    }
                }
            }
        }
        self.selected = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.themes.len() {
            self.selected += 1;
        }
    }

    pub fn selected_theme(&self) -> Option<&Theme> {
        self.themes.get(self.selected)
    }
}

pub struct ThemeSwitcherWidget<'a> {
    pub state: &'a ThemeSwitcherState,
}

impl<'a> Widget for ThemeSwitcherWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = Color::Rgb(24, 24, 37);
        let fg = Color::Rgb(192, 202, 245);
        let border_color = Color::Rgb(69, 71, 90);
        let selected_bg = Color::Rgb(45, 45, 65);
        let accent = Color::Rgb(137, 180, 250);

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // top border
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        // title
        if area.height > 1 {
            let title = " Switch Theme ";
            let mut x = area.x + 2;
            for ch in title.chars() {
                if x >= area.x + area.width { break; }
                buf.cell_mut((x, area.y + 1)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(accent).bg(bg).add_modifier(Modifier::BOLD));
                });
                x += 1;
            }
        }

        // separator
        if area.height > 2 {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, area.y + 2)).map(|cell| {
                    cell.set_char('─');
                    cell.set_style(Style::default().fg(border_color).bg(bg));
                });
            }
        }

        // theme list
        for (i, theme) in self.state.themes.iter().enumerate() {
            let y = area.y + 3 + i as u16;
            if y >= area.y + area.height { break; }

            let is_selected = i == self.state.selected;
            let line_bg = if is_selected { selected_bg } else { bg };

            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_style(Style::default().bg(line_bg));
                });
            }

            // color preview dots
            let preview_colors = [
                &theme.colors.keyword,
                &theme.colors.string,
                &theme.colors.function,
                &theme.colors.r#type,
                &theme.colors.number,
                &theme.colors.comment,
            ];

            let display = format!("  {} ", theme.name);
            let mut x = area.x;
            for ch in display.chars() {
                if x >= area.x + area.width { break; }
                let style = if is_selected {
                    Style::default().fg(accent).bg(line_bg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(fg).bg(line_bg)
                };
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                x += 1;
            }

            // color dots after the name
            for color_hex in &preview_colors {
                if x >= area.x + area.width { break; }
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char('●');
                    cell.set_style(Style::default().fg(theme::parse_hex_color(color_hex)).bg(line_bg));
                });
                x += 1;
            }
        }
    }
}
