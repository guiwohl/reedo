use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

#[derive(Debug, Clone, Default)]
pub struct FuzzyState {
    pub query: String,
    pub cursor_pos: usize,
    pub all_files: Vec<PathBuf>,
    pub filtered: Vec<(PathBuf, i64)>, // (path, score)
    pub selected: usize,
    pub scroll_offset: usize,
}

impl FuzzyState {
    pub fn reset(&mut self) {
        self.query.clear();
        self.cursor_pos = 0;
        self.filtered.clear();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn collect_files(&mut self, root: &Path) {
        self.all_files.clear();
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                if let Ok(rel) = entry.path().strip_prefix(root) {
                    self.all_files.push(rel.to_path_buf());
                }
            }
        }
        self.filter();
    }

    pub fn insert_char(&mut self, ch: char) {
        self.query.insert(self.cursor_pos, ch);
        self.cursor_pos += 1;
        self.filter();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.query.remove(self.cursor_pos);
            self.filter();
        }
    }

    pub fn filter(&mut self) {
        self.selected = 0;
        self.scroll_offset = 0;

        if self.query.is_empty() {
            self.filtered = self
                .all_files
                .iter()
                .map(|p| (p.clone(), 0))
                .collect();
            return;
        }

        let query_lower = self.query.to_lowercase();
        let mut scored: Vec<(PathBuf, i64)> = self
            .all_files
            .iter()
            .filter_map(|p| {
                let name = p.to_string_lossy().to_lowercase();
                if fuzzy_match(&name, &query_lower) {
                    let score = fuzzy_score(&name, &query_lower);
                    Some((p.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        self.filtered = scored;
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
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected - visible_height + 1;
            }
        }
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.filtered.get(self.selected).map(|(p, _)| p)
    }
}

fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let mut hay_chars = haystack.chars();
    for n in needle.chars() {
        loop {
            match hay_chars.next() {
                Some(h) if h == n => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

fn fuzzy_score(haystack: &str, needle: &str) -> i64 {
    let mut score: i64 = 0;
    let mut hay_idx = 0;
    let hay_chars: Vec<char> = haystack.chars().collect();

    for n in needle.chars() {
        while hay_idx < hay_chars.len() {
            if hay_chars[hay_idx] == n {
                // consecutive matches score higher
                score += 10;
                // beginning of word bonus
                if hay_idx == 0
                    || hay_chars[hay_idx - 1] == '/'
                    || hay_chars[hay_idx - 1] == '_'
                    || hay_chars[hay_idx - 1] == '-'
                {
                    score += 5;
                }
                hay_idx += 1;
                break;
            }
            score -= 1; // penalty for skipping
            hay_idx += 1;
        }
    }

    // prefer shorter paths
    score -= (haystack.len() as i64) / 4;
    score
}

pub struct FuzzyFinderWidget<'a> {
    pub state: &'a FuzzyState,
}

impl<'a> Widget for FuzzyFinderWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = Color::Rgb(24, 24, 37);
        let fg = Color::Rgb(192, 202, 245);
        let border_color = Color::Rgb(69, 71, 90);
        let selected_bg = Color::Rgb(45, 45, 65);
        let accent = Color::Rgb(137, 180, 250);
        let dim = Color::Rgb(86, 95, 137);

        // fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // top border
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        // input line
        if area.height > 1 {
            let input_y = area.y + 1;
            let label = " > ";
            let display = format!("{}{}", label, self.state.query);
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
                buf.cell_mut((x, input_y)).map(|cell| {
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

        // file list
        let list_start_y = area.y + 3;
        let list_height = area.height.saturating_sub(3) as usize;

        for i in 0..list_height {
            let file_idx = self.state.scroll_offset + i;
            let y = list_start_y + i as u16;
            if y >= area.y + area.height {
                break;
            }

            if let Some((path, _)) = self.state.filtered.get(file_idx) {
                let is_selected = file_idx == self.state.selected;
                let line_bg = if is_selected { selected_bg } else { bg };
                let line_fg = if is_selected { accent } else { fg };

                // fill line bg
                for x in area.x..area.x + area.width {
                    buf.cell_mut((x, y)).map(|cell| {
                        cell.set_style(Style::default().bg(line_bg));
                    });
                }

                let path_str = path.to_string_lossy();
                // show directory in dim, filename in bright
                let (dir_part, file_part) = if let Some(parent) = path.parent() {
                    let parent_str = parent.to_string_lossy();
                    if parent_str.is_empty() {
                        (String::new(), path_str.to_string())
                    } else {
                        (format!("{}/", parent_str), path.file_name().unwrap().to_string_lossy().to_string())
                    }
                } else {
                    (String::new(), path_str.to_string())
                };

                let display = format!("  {}{}", dir_part, file_part);
                let dir_end = 2 + dir_part.len();
                let mut x = area.x;
                for (ci, ch) in display.chars().enumerate() {
                    if x >= area.x + area.width {
                        break;
                    }
                    let style = if ci < dir_end {
                        Style::default().fg(dim).bg(line_bg)
                    } else {
                        let mut s = Style::default().fg(line_fg).bg(line_bg);
                        if is_selected {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        s
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
