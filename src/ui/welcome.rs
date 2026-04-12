use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

const PENGUIN: &[&str] = &[
    "      .-\"\"\"-.     ",
    "     /        \\    ",
    "    |  ○    ○  |   ",
    "    |    ▼     |   ",
    "     \\  .__,  /    ",
    "      '-.__.-'     ",
];

const TITLE: &str = "reedo";

const HINTS: &[(&str, &str)] = &[
    ("Ctrl+E / e", "file tree"),
    ("Ctrl+P", "fuzzy finder"),
    ("Ctrl+F", "search"),
    ("Ctrl+L", "goto line"),
    ("F4", "side panel"),
    ("F1", "all keybinds"),
    ("Ctrl+Q", "quit"),
];

pub struct WelcomeScreen<'a> {
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for WelcomeScreen<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = self.theme.bg();
        let accent = self.theme.popup_accent();
        let dim = self.theme.popup_dim();
        let fg = self.theme.fg();
        let keyword = Color::Rgb(255, 158, 100);

        // fill bg with theme color
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        let total_lines = PENGUIN.len() + 2 + HINTS.len() + 1;
        let start_y = area.y + area.height.saturating_sub(total_lines as u16) / 3;

        // penguin
        let penguin_width = PENGUIN.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
        let penguin_x = area.x + area.width.saturating_sub(penguin_width) / 2;

        for (i, line) in PENGUIN.iter().enumerate() {
            let y = start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let mut x = penguin_x;
            for ch in line.chars() {
                if x >= area.x + area.width {
                    break;
                }
                let color = match ch {
                    '○' => accent,
                    '▼' => keyword,
                    _ => fg,
                };
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(color).bg(bg));
                });
                x += 1;
            }
        }

        // title
        let title_y = start_y + PENGUIN.len() as u16 + 1;
        if title_y < area.y + area.height {
            let title_x = area.x + area.width.saturating_sub(TITLE.len() as u16) / 2;
            let mut x = title_x;
            for ch in TITLE.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.cell_mut((x, title_y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(
                        Style::default()
                            .fg(accent)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    );
                });
                x += 1;
            }
        }

        // hints
        let hints_start = title_y + 2;
        for (i, (key, desc)) in HINTS.iter().enumerate() {
            let y = hints_start + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let line = format!("{:>12}  {}", key, desc);
            let center_x = area.x + area.width.saturating_sub(line.len() as u16) / 2;
            let mut x = center_x;
            let key_end = 12;
            for (ci, ch) in line.chars().enumerate() {
                if x >= area.x + area.width {
                    break;
                }
                let style = if ci < key_end {
                    Style::default().fg(fg).bg(bg)
                } else {
                    Style::default().fg(dim).bg(bg)
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
