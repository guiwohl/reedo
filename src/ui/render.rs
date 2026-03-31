use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::app::App;
use crate::git::status::GutterMark;
use crate::syntax::highlight;

pub struct EditorView<'a> {
    pub app: &'a App,
}

impl<'a> Widget for EditorView<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let t = &self.app.theme;
        let theme_bg = t.bg();
        let theme_fg = t.fg();
        let theme_gutter = t.gutter();
        let theme_selection = t.selection();
        let theme_cursor_bg = Color::Rgb(249, 226, 175);
        let theme_cursor_fg = Color::Rgb(24, 24, 37);

        // fill entire editor area with theme background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(theme_bg));
                });
            }
        }

        let line_count = self.app.buffer.line_count();
        let max_line_num_width = format!("{}", line_count).len().max(3);
        let has_git_gutter = !self.app.gutter_marks.is_empty();
        let git_gutter_width: u16 = if has_git_gutter { 1 } else { 0 };
        let gutter_width = git_gutter_width + max_line_num_width as u16 + 1;
        let h_padding = self.app.horizontal_padding as u16;

        let text_area_x = area.x + gutter_width + h_padding;
        let text_area_width = area
            .width
            .saturating_sub(gutter_width + h_padding * 2);

        let viewport_height = area.height as usize;

        let sel_start = self.app.cursor.selection.as_ref().map(|s| s.start());
        let sel_end = self.app.cursor.selection.as_ref().map(|s| s.end());

        let is_env = self
            .app
            .buffer
            .file_path
            .as_ref()
            .map(|p| highlight::is_env_file(p))
            .unwrap_or(false);

        let is_md = self
            .app
            .buffer
            .file_path
            .as_ref()
            .map(|p| highlight::is_markdown_file(p))
            .unwrap_or(false);

        let md_code_lines = if is_md {
            highlight::compute_code_block_lines(&self.app.buffer)
        } else {
            Vec::new()
        };

        for row in 0..viewport_height {
            let file_line = self.app.viewport_top + row;
            let y = area.y + row as u16;

            if file_line < line_count {
                // git gutter mark
                if has_git_gutter {
                    let mark_x = area.x;
                    if let Some(mark) = self.app.gutter_marks.get(&file_line) {
                        let (ch, color) = match mark {
                            GutterMark::Added => ('│', Color::Rgb(166, 227, 161)),    // green
                            GutterMark::Modified => ('│', Color::Rgb(249, 226, 175)), // yellow
                            GutterMark::Deleted => ('▸', Color::Rgb(247, 118, 142)),  // red
                        };
                        buf.cell_mut((mark_x, y)).map(|cell| {
                            cell.set_char(ch);
                            cell.set_style(Style::default().fg(color));
                        });
                    }
                }

                // line number
                let num_str = format!(
                    "{:>width$} ",
                    file_line + 1,
                    width = max_line_num_width
                );
                let gutter_style = Style::default().fg(theme_gutter);
                for (i, ch) in num_str.chars().enumerate() {
                    let x = area.x + git_gutter_width + i as u16;
                    if x < area.x + area.width {
                        buf.cell_mut((x, y)).map(|cell| {
                            cell.set_char(ch);
                            cell.set_style(gutter_style);
                        });
                    }
                }

                // text content with syntax highlighting
                let line_text = self.app.buffer.line_text(file_line);
                let scroll_col = self.app.viewport_left;
                let visible_chars: Vec<char> = line_text.chars().skip(scroll_col).collect();

                for (i, &ch) in visible_chars.iter().enumerate() {
                    if i as u16 >= text_area_width {
                        break;
                    }
                    let x = text_area_x + i as u16;
                    let file_col = scroll_col + i;

                    // syntax highlighting
                    let mut style = if is_md {
                        let in_code = md_code_lines.get(file_line).copied().unwrap_or(false);
                        let line_chars: Vec<char> = line_text.chars().collect();
                        if let Some(hs) = highlight::markdown_style_for_line(&line_chars, file_col, in_code) {
                            hs.to_ratatui_style()
                        } else {
                            Style::default().fg(theme_fg)
                        }
                    } else if self.app.highlighter.is_active() {
                        if let Some(hs) = self.app.highlighter.style_for(file_line, file_col) {
                            hs.to_ratatui_style()
                        } else {
                            Style::default().fg(theme_fg)
                        }
                    } else if is_env {
                        if let Some(hs) = highlight::env_style_for_line(&line_text, file_col) {
                            hs.to_ratatui_style()
                        } else {
                            Style::default().fg(theme_fg)
                        }
                    } else {
                        Style::default().fg(theme_fg)
                    };

                    // selection overlay
                    if let (Some(ss), Some(se)) = (sel_start, sel_end) {
                        let in_selection = if ss.line == se.line {
                            file_line == ss.line && file_col >= ss.col && file_col < se.col
                        } else if file_line == ss.line {
                            file_col >= ss.col
                        } else if file_line == se.line {
                            file_col < se.col
                        } else {
                            file_line > ss.line && file_line < se.line
                        };
                        if in_selection {
                            style = style.bg(theme_selection);
                        }
                    }

                    buf.cell_mut((x, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(style);
                    });
                }
            } else {
                let tilde_x = area.x + gutter_width.saturating_sub(2);
                buf.cell_mut((tilde_x, y)).map(|cell| {
                    cell.set_char('~');
                    cell.set_style(Style::default().fg(Color::DarkGray));
                });
            }
        }

        // cursor
        let cursor_screen_line = self
            .app
            .cursor
            .pos
            .line
            .saturating_sub(self.app.viewport_top);
        let cursor_screen_col = self
            .app
            .cursor
            .pos
            .col
            .saturating_sub(self.app.viewport_left);
        let cursor_y = area.y + cursor_screen_line as u16;
        let cursor_x = text_area_x + cursor_screen_col as u16;

        if cursor_y < area.y + area.height && cursor_x < area.x + area.width {
            match self.app.mode {
                crate::editor::mode::Mode::Normal => {
                    buf.cell_mut((cursor_x, cursor_y)).map(|cell| {
                        cell.set_style(
                            Style::default()
                                .fg(theme_cursor_fg)
                                .bg(theme_cursor_bg)
                                .add_modifier(Modifier::BOLD),
                        );
                    });
                }
                crate::editor::mode::Mode::Insert => {
                    buf.cell_mut((cursor_x, cursor_y)).map(|cell| {
                        cell.set_style(
                            cell.style()
                                .add_modifier(Modifier::UNDERLINED)
                                .fg(theme_cursor_bg),
                        );
                    });
                }
            }
        }
    }
}
