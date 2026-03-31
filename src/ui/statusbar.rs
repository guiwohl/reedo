use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

use crate::app::App;

pub struct StatusBar<'a> {
    pub app: &'a App,
}

impl<'a> Widget for StatusBar<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = self.app.theme.statusbar_bg();
        let fg = self.app.theme.statusbar_fg();
        let mode_fg = match self.app.mode {
            crate::editor::mode::Mode::Normal => Color::Rgb(137, 180, 250),
            crate::editor::mode::Mode::Insert => Color::Rgb(166, 227, 161),
        };
        let git_fg = Color::Rgb(203, 166, 247);
        let sep_fg = Color::Rgb(69, 71, 90);

        // fill background
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg));
            });
        }

        // [MODE]
        let mode_str = format!(" {} ", self.app.mode.label());

        // line/total
        let line_info = format!(
            " {}/{} ",
            self.app.cursor.pos.line + 1,
            self.app.buffer.line_count()
        );

        // git status
        let git_str = self
            .app
            .git_info
            .as_ref()
            .map(|g| format!(" │ {} ", g.status_line()))
            .unwrap_or_default();

        // filename + dirty
        let dirty = if self.app.buffer.dirty { " [+]" } else { "" };
        let fname = self
            .app
            .buffer
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[new]".to_string());

        let file_part = format!(" {}{}", fname, dirty);

        let left = format!("{}{}{}{}", mode_str, line_info, git_str, file_part);

        let mode_end = mode_str.len();
        let line_end = mode_end + line_info.len();
        let git_end = line_end + git_str.len();

        let mut x = area.x;
        for (i, ch) in left.chars().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let style = if i < mode_end {
                Style::default().fg(Color::Rgb(30, 30, 46)).bg(mode_fg)
            } else if i < line_end {
                Style::default().fg(fg).bg(bg)
            } else if i < git_end {
                if ch == '│' {
                    Style::default().fg(sep_fg).bg(bg)
                } else {
                    Style::default().fg(git_fg).bg(bg)
                }
            } else {
                Style::default().fg(fg).bg(bg)
            };
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(style);
            });
            x += 1;
        }

        // flash message — right-aligned, visible for 2.5s
        if let Some((ref msg, ref when)) = self.app.flash_message {
            let elapsed = when.elapsed().as_millis();
            if elapsed < 2500 {
                let flash_fg = if elapsed < 2000 {
                    Color::Rgb(166, 227, 161) // green
                } else {
                    // fade to dim in last 500ms
                    Color::Rgb(86, 95, 137)
                };
                let display = format!(" {} ", msg);
                let start_x = area.x + area.width - display.len() as u16;
                let mut fx = start_x;
                for ch in display.chars() {
                    if fx >= area.x + area.width {
                        break;
                    }
                    buf.cell_mut((fx, area.y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(flash_fg).bg(bg));
                    });
                    fx += 1;
                }
            }
        }
    }
}
