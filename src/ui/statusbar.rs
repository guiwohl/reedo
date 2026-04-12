use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
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
        let dim_fg = Color::Rgb(86, 95, 137);

        // fill background
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg));
            });
        }

        // [MODE] badge
        let mode_str = format!(" {} ", self.app.mode.label());

        // line/col info
        let line_info = format!(
            " {}:{} ",
            self.app.cursor.pos.line + 1,
            self.app.cursor.pos.col + 1,
        );

        // total lines
        let total_info = format!("/{} ", self.app.buffer.line_count());

        // git status
        let git_str = self
            .app
            .git_info
            .as_ref()
            .map(|g| format!(" {} ", g.status_line()))
            .unwrap_or_default();

        // file path (relative to project root if possible)
        let dirty = if self.app.buffer.dirty { " [+]" } else { "" };
        let fname = if let (Some(file_path), Some(root)) =
            (&self.app.buffer.file_path, &self.app.project_root)
        {
            file_path
                .strip_prefix(root)
                .ok()
                .map(|rel| rel.display().to_string())
                .unwrap_or_else(|| {
                    file_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "[new]".to_string())
                })
        } else {
            self.app
                .buffer
                .file_path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "[new]".to_string())
        };
        let file_part = format!(" {}{}", fname, dirty);

        // build left side
        let left = format!("{}{}{}", mode_str, line_info, total_info);
        let mode_end = mode_str.len();
        let line_end = mode_end + line_info.len();

        let mut x = area.x;
        for (i, ch) in left.chars().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let style = if i < mode_end {
                Style::default()
                    .fg(Color::Rgb(30, 30, 46))
                    .bg(mode_fg)
                    .add_modifier(Modifier::BOLD)
            } else if i < line_end {
                Style::default().fg(fg).bg(bg)
            } else {
                Style::default().fg(dim_fg).bg(bg)
            };
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(style);
            });
            x += 1;
        }

        // separator
        if !git_str.is_empty() {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(sep_fg).bg(bg));
            });
            x += 1;

            for ch in git_str.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.cell_mut((x, area.y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(git_fg).bg(bg));
                });
                x += 1;
            }
        }

        // separator + file
        buf.cell_mut((x, area.y)).map(|cell| {
            cell.set_char('│');
            cell.set_style(Style::default().fg(sep_fg).bg(bg));
        });
        x += 1;

        for ch in file_part.chars() {
            if x >= area.x + area.width {
                break;
            }
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(fg).bg(bg));
            });
            x += 1;
        }

        // word count for markdown files (right side, before flash)
        let is_md = self
            .app
            .buffer
            .file_path
            .as_ref()
            .map(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "md" || e == "markdown")
                    .unwrap_or(false)
            })
            .unwrap_or(false);

        if is_md {
            let word_count: usize = (0..self.app.buffer.line_count())
                .map(|i| {
                    self.app
                        .buffer
                        .line_text(i)
                        .split_whitespace()
                        .count()
                })
                .sum();
            let wc_str = format!(" {} words ", word_count);
            let wc_start = (area.x + area.width).saturating_sub(wc_str.len() as u16);
            let mut wx = wc_start;
            for ch in wc_str.chars() {
                if wx >= area.x + area.width {
                    break;
                }
                buf.cell_mut((wx, area.y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(dim_fg).bg(bg));
                });
                wx += 1;
            }
        }

        // flash message — right-aligned
        if let Some((ref msg, ref when)) = self.app.flash_message {
            let elapsed = when.elapsed().as_millis();
            if elapsed < 2500 {
                let flash_fg = if elapsed < 2000 {
                    Color::Rgb(166, 227, 161)
                } else {
                    Color::Rgb(86, 95, 137)
                };
                let display = format!(" {} ", msg);
                let start_x =
                    (area.x + area.width).saturating_sub(display.len() as u16);
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
