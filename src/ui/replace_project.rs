use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

#[derive(Debug, Clone)]
pub struct ProjectMatch {
    pub path: PathBuf,
    pub abs_path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PReplaceField {
    Search,
    Replace,
}

#[derive(Debug, Clone)]
pub struct ProjectReplaceState {
    pub search_query: String,
    pub replace_query: String,
    pub active_field: PReplaceField,
    pub search_cursor: usize,
    pub replace_cursor: usize,
    pub results: Vec<ProjectMatch>,
    pub current: usize,
    pub awaiting_confirm: bool,
    pub done: bool,
}

impl Default for ProjectReplaceState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            replace_query: String::new(),
            active_field: PReplaceField::Search,
            search_cursor: 0,
            replace_cursor: 0,
            results: Vec::new(),
            current: 0,
            awaiting_confirm: false,
            done: false,
        }
    }
}

impl ProjectReplaceState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn insert_char(&mut self, ch: char) {
        match self.active_field {
            PReplaceField::Search => {
                self.search_query.insert(self.search_cursor, ch);
                self.search_cursor += 1;
            }
            PReplaceField::Replace => {
                self.replace_query.insert(self.replace_cursor, ch);
                self.replace_cursor += 1;
            }
        }
    }

    pub fn delete_char(&mut self) {
        match self.active_field {
            PReplaceField::Search => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    self.search_query.remove(self.search_cursor);
                }
            }
            PReplaceField::Replace => {
                if self.replace_cursor > 0 {
                    self.replace_cursor -= 1;
                    self.replace_query.remove(self.replace_cursor);
                }
            }
        }
    }

    pub fn toggle_field(&mut self) {
        self.active_field = match self.active_field {
            PReplaceField::Search => PReplaceField::Replace,
            PReplaceField::Replace => PReplaceField::Search,
        };
    }

    pub fn search(&mut self, root: &Path) {
        self.results.clear();
        self.current = 0;
        self.done = false;
        if self.search_query.is_empty() {
            return;
        }

        let walker = ignore::WalkBuilder::new(root)
            .hidden(true)
            .git_ignore(false)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if is_binary_ext(ext) {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                for (line_idx, line) in content.lines().enumerate() {
                    let mut start = 0;
                    while let Some(pos) = line[start..].find(&self.search_query) {
                        let rel_path = path.strip_prefix(root).unwrap_or(path);
                        self.results.push(ProjectMatch {
                            path: rel_path.to_path_buf(),
                            abs_path: path.to_path_buf(),
                            line: line_idx,
                            col: start + pos,
                            text: line.to_string(),
                        });
                        start += pos + 1;
                        if self.results.len() > 1000 {
                            self.awaiting_confirm = !self.results.is_empty();
                            return;
                        }
                    }
                }
            }
        }
        self.awaiting_confirm = !self.results.is_empty();
    }

    pub fn apply_current(&mut self) -> bool {
        if self.current >= self.results.len() {
            self.done = true;
            self.awaiting_confirm = false;
            return false;
        }

        let result = &self.results[self.current];
        let abs = result.abs_path.clone();
        let line_idx = result.line;
        let col = result.col;

        if let Ok(content) = std::fs::read_to_string(&abs) {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            if line_idx < lines.len() {
                let line = &mut lines[line_idx];
                let end = col + self.search_query.len();
                if end <= line.len() && &line[col..end] == self.search_query {
                    line.replace_range(col..end, &self.replace_query);
                    let new_content = lines.join("\n");
                    let _ = std::fs::write(&abs, new_content);
                }
            }
        }

        self.current += 1;
        if self.current >= self.results.len() {
            self.done = true;
            self.awaiting_confirm = false;
        }
        true
    }

    pub fn skip_current(&mut self) {
        self.current += 1;
        if self.current >= self.results.len() {
            self.done = true;
            self.awaiting_confirm = false;
        }
    }

    pub fn current_match(&self) -> Option<&ProjectMatch> {
        self.results.get(self.current)
    }
}

fn is_binary_ext(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp"
            | "mp3" | "mp4" | "avi" | "mov" | "mkv"
            | "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar"
            | "exe" | "dll" | "so" | "dylib" | "o" | "a"
            | "wasm" | "pdf" | "doc" | "docx"
            | "ttf" | "otf" | "woff" | "woff2"
            | "sqlite" | "db"
    )
}

pub struct ProjectReplaceWidget<'a> {
    pub state: &'a ProjectReplaceState,
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for ProjectReplaceWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = self.theme.popup_bg();
        let fg = self.theme.fg();
        let accent = self.theme.popup_accent();
        let border_color = self.theme.popup_border();
        let active_bg = self.theme.popup_selected();
        let warn = Color::Rgb(249, 226, 175);

        // fill bg
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // border
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        // search input
        if area.height > 1 {
            let sbg = if self.state.active_field == PReplaceField::Search {
                active_bg
            } else {
                bg
            };
            let label = " search: ";
            let display = format!("{}{}", label, self.state.search_query);
            let mut x = area.x;
            for (i, ch) in display.chars().enumerate() {
                if x >= area.x + area.width { break; }
                let style = if i < label.len() {
                    Style::default().fg(accent).bg(sbg)
                } else {
                    Style::default().fg(fg).bg(sbg)
                };
                buf.cell_mut((x, area.y + 1)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                x += 1;
            }
        }

        // replace input
        if area.height > 2 {
            let rbg = if self.state.active_field == PReplaceField::Replace {
                active_bg
            } else {
                bg
            };
            let label = " replace: ";
            let status = if self.state.awaiting_confirm {
                format!(
                    "  [{}/{}] y=apply n=skip a=all",
                    self.state.current + 1,
                    self.state.results.len()
                )
            } else if self.state.done {
                " [done]".to_string()
            } else {
                String::new()
            };
            let display = format!("{}{}{}", label, self.state.replace_query, status);
            let mut x = area.x;
            for (i, ch) in display.chars().enumerate() {
                if x >= area.x + area.width { break; }
                let style = if i < label.len() {
                    Style::default().fg(accent).bg(rbg)
                } else {
                    Style::default().fg(fg).bg(rbg)
                };
                buf.cell_mut((x, area.y + 2)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                x += 1;
            }
        }

        // current match preview
        if area.height > 4 {
            if let Some(m) = self.state.current_match() {
                let preview_y = area.y + 4;
                let path_str = m.path.to_string_lossy();
                let display = format!(
                    "  {}:{} {}",
                    path_str,
                    m.line + 1,
                    m.text.trim()
                );
                let mut x = area.x;
                for ch in display.chars() {
                    if x >= area.x + area.width { break; }
                    buf.cell_mut((x, preview_y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(warn).bg(bg));
                    });
                    x += 1;
                }
            }
        }
    }
}
