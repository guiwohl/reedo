use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

const LOGO: &[&str] = &[
    "               _       ",
    " _ __ ___  ___| |_ ___ ",
    "| '__/ _ \\/ _ \\ __/ _ \\",
    "| | |  __/  __/ || (_) |",
    "|_|  \\___|\\___|\\__\\___/ ",
];

const HINTS: &[(&str, &str)] = &[
    ("Ctrl+E / e", "file tree"),
    ("Ctrl+P", "fuzzy finder"),
    ("Ctrl+F", "search"),
    ("F4", "side panel"),
    ("F1", "all keybinds"),
    ("Ctrl+Q", "quit"),
];

pub struct WelcomeScreen<'a> {
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for WelcomeScreen<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let total_lines = LOGO.len() + 2 + HINTS.len();
        let start_y = area.height.saturating_sub(total_lines as u16) / 3;
        let logo_width = LOGO.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

        let accent = self.theme.popup_accent();
        let dim = self.theme.popup_dim();
        let fg = self.theme.fg();

        // logo
        let logo_x = area.x + area.width.saturating_sub(logo_width) / 2;
        for (i, line) in LOGO.iter().enumerate() {
            let y = area.y + start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let mut x = logo_x;
            for ch in line.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(accent).add_modifier(Modifier::BOLD));
                });
                x += 1;
            }
        }

        // hints
        let hints_start = area.y + start_y + LOGO.len() as u16 + 2;
        for (i, (key, desc)) in HINTS.iter().enumerate() {
            let y = hints_start + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let line = format!("{:>12}  {}", key, desc);
            let center_x = area.x + area.width.saturating_sub(line.len() as u16) / 2;
            let mut x = center_x;
            let key_len = 12;
            for (ci, ch) in line.chars().enumerate() {
                if x >= area.x + area.width {
                    break;
                }
                let style = if ci < key_len {
                    Style::default().fg(fg)
                } else {
                    Style::default().fg(dim)
                };
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                x += 1;
            }
        }
    }
}
