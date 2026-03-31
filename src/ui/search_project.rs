use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub line: usize,
    pub col: usize,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectSearchState {
    pub query: String,
    pub cursor_pos: usize,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub searching: bool,
}

impl ProjectSearchState {
    pub fn reset(&mut self) {
        self.query.clear();
        self.cursor_pos = 0;
        self.results.clear();
        self.selected = 0;
        self.scroll_offset = 0;
        self.searching = false;
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

    pub fn search(&mut self, root: &Path) {
        self.results.clear();
        self.selected = 0;
        self.scroll_offset = 0;
        if self.query.is_empty() {
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

            // skip binary files
            if is_binary(path) {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                for (line_idx, line) in content.lines().enumerate() {
                    let mut start = 0;
                    while let Some(pos) = line[start..].find(&self.query) {
                        let rel_path = path.strip_prefix(root).unwrap_or(path);
                        self.results.push(SearchResult {
                            path: rel_path.to_path_buf(),
                            line: line_idx,
                            col: start + pos,
                            text: line.to_string(),
                        });
                        start += pos + 1;
                        if self.results.len() > 1000 {
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self, visible_height: usize) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected - visible_height + 1;
            }
        }
    }

    pub fn selected_result(&self) -> Option<&SearchResult> {
        self.results.get(self.selected)
    }
}

fn is_binary(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
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

pub struct ProjectSearchWidget<'a> {
    pub state: &'a ProjectSearchState,
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for ProjectSearchWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = self.theme.popup_bg();
        let fg = self.theme.fg();
        let border_color = self.theme.popup_border();
        let selected_bg = self.theme.popup_selected();
        let accent = self.theme.popup_accent();
        let dim = self.theme.popup_dim();

        // fill bg
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // title + border
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        // input
        if area.height > 1 {
            let label = " Search Project: ";
            let count_info = if !self.state.results.is_empty() {
                format!("  ({} results)", self.state.results.len())
            } else {
                String::new()
            };
            let display = format!("{}{}{}", label, self.state.query, count_info);
            let mut x = area.x;
            for (i, ch) in display.chars().enumerate() {
                if x >= area.x + area.width {
                    break;
                }
                let style = if i < label.len() {
                    Style::default().fg(accent).bg(bg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(fg).bg(bg)
                };
                buf.cell_mut((x, area.y + 1)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                x += 1;
            }
        }

        // separator
        if area.height > 2 {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, area.y + 2)).map(|cell| {
                    cell.set_char('─');
                    cell.set_style(Style::default().fg(border_color).bg(bg));
                });
            }
        }

        // results
        let list_start = 3u16;
        let list_height = area.height.saturating_sub(list_start) as usize;

        for i in 0..list_height {
            let result_idx = self.state.scroll_offset + i;
            let y = area.y + list_start + i as u16;
            if y >= area.y + area.height {
                break;
            }

            if let Some(result) = self.state.results.get(result_idx) {
                let is_selected = result_idx == self.state.selected;
                let line_bg = if is_selected { selected_bg } else { bg };

                for lx in area.x..area.x + area.width {
                    buf.cell_mut((lx, y)).map(|cell| {
                        cell.set_style(Style::default().bg(line_bg));
                    });
                }

                let path_str = result.path.to_string_lossy();
                let line_num = format!(":{}", result.line + 1);
                let text_preview: String = result.text.trim().chars().take(60).collect();
                let display = format!("  {}{}: {}", path_str, line_num, text_preview);

                let path_end = 2 + path_str.len();

                let mut x = area.x;
                for (ci, ch) in display.chars().enumerate() {
                    if x >= area.x + area.width {
                        break;
                    }
                    let style = if ci < path_end {
                        Style::default().fg(accent).bg(line_bg)
                    } else if ci < path_end + line_num.len() {
                        Style::default().fg(dim).bg(line_bg)
                    } else {
                        Style::default().fg(fg).bg(line_bg)
                    };
                    buf.cell_mut((x, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(style);
                    });
                    x += 1;
                }
            }
        }
    }
}
