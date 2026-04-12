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
        let theme_cursor_bg = t.cursor_bg();
        let theme_cursor_fg = t.cursor_fg();
        let theme_cursorline = t.cursorline();

        // active line number color: brighter version of gutter
        let active_line_num_color = match theme_fg {
            Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
            _ => Color::White,
        };

        // indent guide color: very subtle
        let indent_guide_color = match theme_bg {
            Color::Rgb(r, g, b) => Color::Rgb(
                r.saturating_add(18),
                g.saturating_add(18),
                b.saturating_add(18),
            ),
            _ => Color::DarkGray,
        };

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
        let text_area_width = area.width.saturating_sub(gutter_width + h_padding * 2);

        let viewport_height = area.height as usize;
        let cursor_line = self.app.cursor.pos.line;

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

        let wrap_enabled = self.app.line_wrapping;
        let tw = text_area_width as usize;
        let indent_size = self.app.indent_size;

        // build screen rows: Vec<(file_line, char_offset)>
        let mut screen_rows: Vec<(usize, usize)> = Vec::new();
        let mut file_line = self.app.viewport_top;
        while screen_rows.len() < viewport_height && file_line < line_count {
            let line_len = self.app.buffer.line_text(file_line).chars().count();
            if wrap_enabled && tw > 0 && line_len > tw {
                let mut offset = 0;
                while offset < line_len && screen_rows.len() < viewport_height {
                    screen_rows.push((file_line, offset));
                    offset += tw;
                }
            } else {
                screen_rows.push((file_line, 0));
            }
            file_line += 1;
        }
        while screen_rows.len() < viewport_height {
            screen_rows.push((usize::MAX, 0));
        }

        for (row, &(file_line, char_offset)) in screen_rows.iter().enumerate() {
            let y = area.y + row as u16;
            let is_cursor_line = file_line == cursor_line;

            if file_line < line_count {
                let is_first_wrap = char_offset == 0;
                let line_bg = if is_cursor_line {
                    theme_cursorline
                } else {
                    theme_bg
                };

                // fill cursorline background across full width
                if is_cursor_line {
                    for x in area.x..area.x + area.width {
                        buf.cell_mut((x, y)).map(|cell| {
                            cell.set_style(Style::default().bg(line_bg));
                        });
                    }
                }

                // git gutter mark (only on first wrap row)
                if has_git_gutter && is_first_wrap {
                    let mark_x = area.x;
                    if let Some(mark) = self.app.gutter_marks.get(&file_line) {
                        let (ch, color) = match mark {
                            GutterMark::Added => ('▎', Color::Rgb(166, 227, 161)),
                            GutterMark::Modified => ('▎', Color::Rgb(249, 226, 175)),
                            GutterMark::Deleted => ('▁', Color::Rgb(247, 118, 142)),
                        };
                        buf.cell_mut((mark_x, y)).map(|cell| {
                            cell.set_char(ch);
                            cell.set_style(Style::default().fg(color).bg(line_bg));
                        });
                    }
                }

                // line number (only on first wrap row)
                if is_first_wrap {
                    let num_display = if self.app.relative_line_numbers && !is_cursor_line {
                        let dist = if file_line > cursor_line {
                            file_line - cursor_line
                        } else {
                            cursor_line - file_line
                        };
                        format!("{:>width$} ", dist, width = max_line_num_width)
                    } else {
                        format!("{:>width$} ", file_line + 1, width = max_line_num_width)
                    };
                    let gutter_style = if is_cursor_line {
                        Style::default()
                            .fg(active_line_num_color)
                            .bg(line_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme_gutter).bg(line_bg)
                    };
                    for (i, ch) in num_display.chars().enumerate() {
                        let x = area.x + git_gutter_width + i as u16;
                        if x < area.x + area.width {
                            buf.cell_mut((x, y)).map(|cell| {
                                cell.set_char(ch);
                                cell.set_style(gutter_style);
                            });
                        }
                    }
                }

                // text content
                let line_text = self.app.buffer.line_text(file_line);
                let scroll_col = if wrap_enabled && tw > 0 {
                    char_offset
                } else {
                    self.app.viewport_left
                };
                let visible_chars: Vec<char> = line_text.chars().skip(scroll_col).collect();

                // indent guides: render before text so text overwrites them
                if indent_size > 0 && is_first_wrap && !wrap_enabled {
                    let leading_spaces = line_text.chars().take_while(|c| *c == ' ').count();
                    let mut guide_col = indent_size;
                    while guide_col < leading_spaces {
                        let screen_col = guide_col.saturating_sub(self.app.viewport_left);
                        let x = text_area_x + screen_col as u16;
                        if x < text_area_x + text_area_width && guide_col >= self.app.viewport_left
                        {
                            buf.cell_mut((x, y)).map(|cell| {
                                cell.set_char('│');
                                cell.set_style(
                                    Style::default().fg(indent_guide_color).bg(line_bg),
                                );
                            });
                        }
                        guide_col += indent_size;
                    }
                }

                // markdown code block bg tinting
                let md_in_code = is_md && md_code_lines.get(file_line).copied().unwrap_or(false);
                let code_block_bg = if md_in_code {
                    match theme_bg {
                        Color::Rgb(r, g, b) => Some(Color::Rgb(
                            r.saturating_add(10),
                            g.saturating_add(10),
                            b.saturating_add(6),
                        )),
                        _ => None,
                    }
                } else {
                    None
                };

                // fill code block bg across text area
                if let Some(cb_bg) = code_block_bg {
                    let effective_bg = if is_cursor_line { line_bg } else { cb_bg };
                    for x in text_area_x..text_area_x + text_area_width {
                        buf.cell_mut((x, y)).map(|cell| {
                            cell.set_style(Style::default().bg(effective_bg));
                        });
                    }
                }

                for (i, &ch) in visible_chars.iter().enumerate() {
                    if i as u16 >= text_area_width {
                        break;
                    }
                    let x = text_area_x + i as u16;
                    let file_col = scroll_col + i;

                    // whitespace visualization
                    let display_ch = if self.app.show_whitespace {
                        match ch {
                            ' ' => '·',
                            '\t' => '→',
                            _ => ch,
                        }
                    } else {
                        ch
                    };

                    // syntax highlighting
                    let mut style = if is_md {
                        let line_chars: Vec<char> = line_text.chars().collect();
                        if let Some(hs) =
                            highlight::markdown_style_for_line(&line_chars, file_col, md_in_code)
                        {
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

                    // whitespace chars get dimmed
                    if self.app.show_whitespace && (ch == ' ' || ch == '\t') {
                        style = Style::default().fg(indent_guide_color);
                    }

                    // apply code block bg
                    if let Some(cb_bg) = code_block_bg {
                        if !is_cursor_line {
                            style = style.bg(cb_bg);
                        }
                    }

                    // apply cursorline bg
                    if is_cursor_line {
                        style = style.bg(line_bg);
                    }

                    // selection overlay (takes priority)
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
                        cell.set_char(display_ch);
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

        // cursor rendering
        let cursor_col = self.app.cursor.pos.col;
        let mut cursor_y = area.y;
        let mut cursor_x = text_area_x;
        for (row, &(fl, co)) in screen_rows.iter().enumerate() {
            if fl == cursor_line {
                let col_in_row = if wrap_enabled && tw > 0 {
                    if cursor_col >= co && cursor_col < co + tw {
                        Some(cursor_col - co)
                    } else {
                        None
                    }
                } else {
                    Some(cursor_col.saturating_sub(self.app.viewport_left))
                };
                if let Some(c) = col_in_row {
                    cursor_y = area.y + row as u16;
                    cursor_x = text_area_x + c as u16;
                    break;
                }
            }
        }

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

        // minimap scrollbar (1-char wide on right edge)
        if line_count > viewport_height && area.width > 10 {
            let scrollbar_x = area.x + area.width - 1;
            let bar_height = viewport_height;
            let thumb_size = ((viewport_height as f64 / line_count as f64) * bar_height as f64)
                .max(1.0) as usize;
            let thumb_pos = if line_count > viewport_height {
                ((self.app.viewport_top as f64 / (line_count - viewport_height) as f64)
                    * (bar_height - thumb_size) as f64) as usize
            } else {
                0
            };

            let track_color = match theme_bg {
                Color::Rgb(r, g, b) => Color::Rgb(
                    r.saturating_add(12),
                    g.saturating_add(12),
                    b.saturating_add(12),
                ),
                _ => Color::DarkGray,
            };
            let thumb_color = match theme_bg {
                Color::Rgb(r, g, b) => Color::Rgb(
                    r.saturating_add(40),
                    g.saturating_add(40),
                    b.saturating_add(40),
                ),
                _ => Color::Gray,
            };

            for i in 0..bar_height {
                let y = area.y + i as u16;
                let in_thumb = i >= thumb_pos && i < thumb_pos + thumb_size;

                // check for git marks at this position in the file
                let file_line_at_pos = (i as f64 / bar_height as f64 * line_count as f64) as usize;
                let mark_color = self.app.gutter_marks.get(&file_line_at_pos).map(|m| match m {
                    GutterMark::Added => Color::Rgb(166, 227, 161),
                    GutterMark::Modified => Color::Rgb(249, 226, 175),
                    GutterMark::Deleted => Color::Rgb(247, 118, 142),
                });

                let (ch, fg, bg_color) = if in_thumb {
                    ('┃', mark_color.unwrap_or(thumb_color), theme_bg)
                } else if let Some(mc) = mark_color {
                    ('│', mc, theme_bg)
                } else {
                    (' ', track_color, track_color)
                };

                buf.cell_mut((scrollbar_x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(Style::default().fg(fg).bg(bg_color));
                });
            }
        }

        // sticky scroll: show current function/heading at top when scrolled past
        if self.app.viewport_top > 0 && area.height > 3 {
            let breadcrumb = find_breadcrumb(self.app);
            if let Some(text) = breadcrumb {
                let sticky_y = area.y;
                let sticky_bg = match theme_bg {
                    Color::Rgb(r, g, b) => Color::Rgb(
                        r.saturating_add(15),
                        g.saturating_add(15),
                        b.saturating_add(15),
                    ),
                    _ => theme_bg,
                };
                let sticky_fg = theme_gutter;

                // fill sticky line bg
                for x in text_area_x..text_area_x + text_area_width {
                    buf.cell_mut((x, sticky_y)).map(|cell| {
                        cell.set_char(' ');
                        cell.set_style(Style::default().bg(sticky_bg));
                    });
                }

                let truncated: String = text.chars().take(text_area_width as usize).collect();
                let mut x = text_area_x;
                for ch in truncated.chars() {
                    if x >= text_area_x + text_area_width {
                        break;
                    }
                    buf.cell_mut((x, sticky_y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(
                            Style::default()
                                .fg(sticky_fg)
                                .bg(sticky_bg)
                                .add_modifier(Modifier::ITALIC),
                        );
                    });
                    x += 1;
                }
            }
        }
    }
}

fn find_breadcrumb(app: &crate::app::App) -> Option<String> {
    let is_md = app
        .buffer
        .file_path
        .as_ref()
        .map(|p| crate::syntax::highlight::is_markdown_file(p))
        .unwrap_or(false);

    if is_md {
        // find nearest heading above viewport_top
        for line in (0..app.viewport_top).rev() {
            let text = app.buffer.line_text(line);
            let trimmed = text.trim_start();
            if trimmed.starts_with('#') {
                let level = trimmed.chars().take_while(|c| *c == '#').count();
                if level <= 6 {
                    return Some(text.to_string());
                }
            }
        }
    } else {
        // for code: find nearest function definition above viewport
        for line in (0..app.viewport_top).rev() {
            let text = app.buffer.line_text(line);
            let trimmed = text.trim_start();
            // heuristic: lines starting with fn/def/func/function/pub fn/async fn/class/impl
            if trimmed.starts_with("fn ")
                || trimmed.starts_with("pub fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("def ")
                || trimmed.starts_with("function ")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("const ")
                || trimmed.starts_with("export ")
            {
                return Some(text.to_string());
            }
        }
    }
    None
}
