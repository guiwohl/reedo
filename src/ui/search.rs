use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub cursor_pos: usize,
    pub matches: Vec<(usize, usize)>, // (line, col) of each match
    pub current_match: usize,
}

impl SearchState {
    pub fn reset(&mut self) {
        self.query.clear();
        self.cursor_pos = 0;
        self.matches.clear();
        self.current_match = 0;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.query.remove(self.cursor_pos);
        }
    }

    pub fn find_matches(&mut self, buffer: &crate::editor::buffer::Buffer) {
        self.matches.clear();
        if self.query.is_empty() {
            return;
        }
        for line_idx in 0..buffer.line_count() {
            let line_text = buffer.line_text(line_idx);
            let mut start = 0;
            while let Some(pos) = line_text[start..].find(&self.query) {
                self.matches.push((line_idx, start + pos));
                start += pos + 1;
            }
        }
        if !self.matches.is_empty() && self.current_match >= self.matches.len() {
            self.current_match = 0;
        }
    }

    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = if self.current_match == 0 {
                self.matches.len() - 1
            } else {
                self.current_match - 1
            };
        }
    }

    pub fn current_pos(&self) -> Option<(usize, usize)> {
        self.matches.get(self.current_match).copied()
    }
}

pub struct SearchBar<'a> {
    pub state: &'a SearchState,
}

impl<'a> Widget for SearchBar<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = Color::Rgb(30, 30, 46);
        let fg = Color::Rgb(192, 202, 245);
        let accent = Color::Rgb(137, 180, 250);

        // fill background
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg));
            });
        }

        let label = " / ";
        let match_info = if self.state.matches.is_empty() {
            if self.state.query.is_empty() {
                String::new()
            } else {
                " [no matches]".to_string()
            }
        } else {
            format!(
                " [{}/{}]",
                self.state.current_match + 1,
                self.state.matches.len()
            )
        };

        let display = format!("{}{}{}", label, self.state.query, match_info);

        let mut x = area.x;
        for (i, ch) in display.chars().enumerate() {
            if x >= area.x + area.width {
                break;
            }
            let style = if i < label.len() {
                Style::default().fg(accent).bg(bg)
            } else {
                Style::default().fg(fg).bg(bg)
            };
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(style);
            });
            x += 1;
        }
    }
}
