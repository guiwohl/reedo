use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

const LOGO: &[&str] = &[r"(o<  -- Reedo!", r"//\", r"V_/_ "];

const HINT: &str = "press F1 for keybindings";

pub struct WelcomeScreen<'a> {
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for WelcomeScreen<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let total_lines = LOGO.len() + 2;
        let start_y = area.height.saturating_sub(total_lines as u16) / 3;
        let logo_width = LOGO.iter().map(|line| line.len()).max().unwrap_or(0) as u16;

        let logo_color = self.theme.popup_accent();
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
                    cell.set_style(Style::default().fg(logo_color).add_modifier(Modifier::BOLD));
                });
                x += 1;
            }
        }

        // hint
        let hint_y = area.y + start_y + LOGO.len() as u16 + 1;
        if hint_y < area.y + area.height {
            let x_offset = area.width.saturating_sub(HINT.len() as u16) / 2;
            let mut x = area.x + x_offset;
            for ch in HINT.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.cell_mut((x, hint_y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(self.theme.popup_border()));
                });
                x += 1;
            }
        }
    }
}
