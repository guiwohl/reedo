use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

#[derive(Debug, Clone, Default)]
pub struct SearchState {
    pub query: String,
    pub cursor_pos: usize,
    pub matches: Vec<(usize, usize)>, // (line, col) of each match
    pub current_match: usize,
    pub regex_mode: bool,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

impl SearchState {
    pub fn reset(&mut self) {
        if !self.query.is_empty() {
            if self.history.last().map(|s| s.as_str()) != Some(&self.query) {
                self.history.push(self.query.clone());
                if self.history.len() > 50 {
                    self.history.remove(0);
                }
            }
        }
        self.query.clear();
        self.cursor_pos = 0;
        self.matches.clear();
        self.current_match = 0;
        self.history_idx = None;
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
        self.history_idx = None;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.query.remove(self.cursor_pos);
            self.history_idx = None;
        }
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            Some(0) => return,
            Some(i) => i - 1,
            None => self.history.len() - 1,
        };
        self.history_idx = Some(idx);
        self.query = self.history[idx].clone();
        self.cursor_pos = self.query.len();
    }

    pub fn history_next(&mut self) {
        let idx = match self.history_idx {
            Some(i) => i + 1,
            None => return,
        };
        if idx >= self.history.len() {
            self.history_idx = None;
            self.query.clear();
            self.cursor_pos = 0;
        } else {
            self.history_idx = Some(idx);
            self.query = self.history[idx].clone();
            self.cursor_pos = self.query.len();
        }
    }

    pub fn toggle_regex(&mut self) {
        self.regex_mode = !self.regex_mode;
    }

    pub fn find_matches(&mut self, buffer: &crate::editor::buffer::Buffer) {
        self.matches.clear();
        if self.query.is_empty() {
            return;
        }

        let re = if self.regex_mode {
            regex::Regex::new(&self.query).ok()
        } else {
            None
        };

        for line_idx in 0..buffer.line_count() {
            let line_text = buffer.line_text(line_idx);
            if let Some(ref re) = re {
                for m in re.find_iter(&line_text) {
                    self.matches.push((line_idx, m.start()));
                }
            } else {
                let mut start = 0;
                while let Some(pos) = line_text[start..].find(&self.query) {
                    self.matches.push((line_idx, start + pos));
                    start += pos + 1;
                }
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
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for SearchBar<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = self.theme.statusbar_bg();
        let fg = self.theme.fg();
        let accent = self.theme.popup_accent();

        // fill background
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(bg));
            });
        }

        let label = if self.state.regex_mode { " /re " } else { " / " };
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
