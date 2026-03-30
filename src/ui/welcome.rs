use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

const LOGO: &[&str] = &[
    r"  ██╗  ██╗██╗██╗      ██████╗ ",
    r"  ██║ ██╔╝██║██║     ██╔═══██╗",
    r"  █████╔╝ ██║██║     ██║   ██║",
    r"  ██╔═██╗ ██║██║     ██║   ██║",
    r"  ██║  ██╗██║███████╗╚██████╔╝",
    r"  ╚═╝  ╚═╝╚═╝╚══════╝ ╚═════╝ ",
];

const HINTS: &[&str] = &[
    "",
    "  a minimal text editor",
    "",
    "  i          enter insert mode",
    "  ctrl+e     file explorer",
    "  ctrl+p     fuzzy finder",
    "  ctrl+f     search in file",
    "  ctrl+h     find & replace",
    "  ctrl+q     quit",
];

pub struct WelcomeScreen;

impl Widget for WelcomeScreen {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let total_lines = LOGO.len() + HINTS.len();
        let start_y = area
            .height
            .saturating_sub(total_lines as u16)
            / 3;

        let logo_color = Color::Rgb(137, 180, 250);
        let hint_color = Color::Rgb(86, 95, 137);
        let key_color = Color::Rgb(166, 227, 161);

        for (i, line) in LOGO.iter().enumerate() {
            let y = area.y + start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let x_offset = area.width.saturating_sub(line.len() as u16) / 2;
            let mut x = area.x + x_offset;
            for ch in line.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(
                        Style::default()
                            .fg(logo_color)
                            .add_modifier(Modifier::BOLD),
                    );
                });
                x += 1;
            }
        }

        for (i, line) in HINTS.iter().enumerate() {
            let y = area.y + start_y + LOGO.len() as u16 + i as u16;
            if y >= area.y + area.height {
                break;
            }
            if line.is_empty() {
                continue;
            }

            let x_offset = area.width.saturating_sub(30) / 2;
            let mut x = area.x + x_offset;

            // parse "  key     description" format
            let trimmed = line.trim_start();
            let spaces = line.len() - trimmed.len();
            x += spaces as u16;

            let parts: Vec<&str> = trimmed.splitn(2, "  ").collect();
            if parts.len() == 2 {
                // key part
                for ch in parts[0].chars() {
                    if x >= area.x + area.width {
                        break;
                    }
                    buf.cell_mut((x, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(key_color));
                    });
                    x += 1;
                }
                // spacing
                let padding = trimmed.find(parts[1]).unwrap_or(parts[0].len() + 2);
                while x < area.x + x_offset + padding as u16 {
                    x += 1;
                }
                // description
                for ch in parts[1].chars() {
                    if x >= area.x + area.width {
                        break;
                    }
                    buf.cell_mut((x, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(hint_color));
                    });
                    x += 1;
                }
            } else {
                for ch in trimmed.chars() {
                    if x >= area.x + area.width {
                        break;
                    }
                    buf.cell_mut((x, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(hint_color));
                    });
                    x += 1;
                }
            }
        }
    }
}
