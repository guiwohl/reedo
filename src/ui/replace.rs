use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

#[derive(Debug, Clone, PartialEq)]
pub enum ReplaceField {
    Search,
    Replace,
}

#[derive(Debug, Clone)]
pub struct ReplaceState {
    pub search_query: String,
    pub replace_query: String,
    pub active_field: ReplaceField,
    pub search_cursor: usize,
    pub replace_cursor: usize,
    pub matches: Vec<(usize, usize)>,
    pub current_match: usize,
    pub awaiting_confirm: bool,
}

impl Default for ReplaceState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            replace_query: String::new(),
            active_field: ReplaceField::Search,
            search_cursor: 0,
            replace_cursor: 0,
            matches: Vec::new(),
            current_match: 0,
            awaiting_confirm: false,
        }
    }
}

impl ReplaceState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn insert_char(&mut self, ch: char) {
        match self.active_field {
            ReplaceField::Search => {
                self.search_query.insert(self.search_cursor, ch);
                self.search_cursor += 1;
            }
            ReplaceField::Replace => {
                self.replace_query.insert(self.replace_cursor, ch);
                self.replace_cursor += 1;
            }
        }
    }

    pub fn delete_char(&mut self) {
        match self.active_field {
            ReplaceField::Search => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    self.search_query.remove(self.search_cursor);
                }
            }
            ReplaceField::Replace => {
                if self.replace_cursor > 0 {
                    self.replace_cursor -= 1;
                    self.replace_query.remove(self.replace_cursor);
                }
            }
        }
    }

    pub fn toggle_field(&mut self) {
        self.active_field = match self.active_field {
            ReplaceField::Search => ReplaceField::Replace,
            ReplaceField::Replace => ReplaceField::Search,
        };
    }

    pub fn find_matches(&mut self, buffer: &crate::editor::buffer::Buffer) {
        self.matches.clear();
        if self.search_query.is_empty() {
            return;
        }
        for line_idx in 0..buffer.line_count() {
            let line_text = buffer.line_text(line_idx);
            let mut start = 0;
            while let Some(pos) = line_text[start..].find(&self.search_query) {
                self.matches.push((line_idx, start + pos));
                start += pos + 1;
            }
        }
        if !self.matches.is_empty() && self.current_match >= self.matches.len() {
            self.current_match = 0;
        }
        if !self.matches.is_empty() {
            self.awaiting_confirm = true;
        }
    }

    pub fn current_pos(&self) -> Option<(usize, usize)> {
        self.matches.get(self.current_match).copied()
    }

    pub fn skip_current(&mut self) {
        if !self.matches.is_empty() {
            self.current_match += 1;
            if self.current_match >= self.matches.len() {
                self.awaiting_confirm = false;
            }
        }
    }
}

pub struct ReplaceBar<'a> {
    pub state: &'a ReplaceState,
}

impl<'a> Widget for ReplaceBar<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        if area.height < 2 {
            return;
        }
        let bg = Color::Rgb(30, 30, 46);
        let fg = Color::Rgb(192, 202, 245);
        let accent = Color::Rgb(137, 180, 250);
        let active_bg = Color::Rgb(45, 45, 65);

        for row in 0..2u16.min(area.height) {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, area.y + row)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // search line
        let search_bg = if self.state.active_field == ReplaceField::Search {
            active_bg
        } else {
            bg
        };
        let search_label = " search: ";
        let search_display = format!("{}{}", search_label, self.state.search_query);
        let mut x = area.x;
        for (i, ch) in search_display.chars().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let style = if i < search_label.len() {
                Style::default().fg(accent).bg(search_bg)
            } else {
                Style::default().fg(fg).bg(search_bg)
            };
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(style);
            });
            x += 1;
        }

        // replace line
        let replace_bg = if self.state.active_field == ReplaceField::Replace {
            active_bg
        } else {
            bg
        };
        let replace_label = " replace: ";
        let match_info = if self.state.awaiting_confirm && !self.state.matches.is_empty() {
            format!(
                "  [{}/{}] y/n/a",
                self.state.current_match + 1,
                self.state.matches.len()
            )
        } else {
            String::new()
        };
        let replace_display = format!(
            "{}{}{}",
            replace_label, self.state.replace_query, match_info
        );
        x = area.x;
        for (i, ch) in replace_display.chars().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let style = if i < replace_label.len() {
                Style::default().fg(accent).bg(replace_bg)
            } else {
                Style::default().fg(fg).bg(replace_bg)
            };
            buf.cell_mut((x, area.y + 1)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(style);
            });
            x += 1;
        }
    }
}
