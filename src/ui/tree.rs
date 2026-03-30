use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

const FOLDER_PALETTE: &[Color] = &[
    Color::Rgb(137, 180, 250), // blue
    Color::Rgb(166, 227, 161), // green
    Color::Rgb(249, 226, 175), // yellow
    Color::Rgb(203, 166, 247), // purple
    Color::Rgb(148, 226, 213), // teal
    Color::Rgb(250, 179, 135), // peach
    Color::Rgb(245, 194, 231), // pink
    Color::Rgb(116, 199, 236), // sapphire
];

#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub depth: usize,
    pub color: Color,
    pub git_status: Option<char>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TreeAction {
    None,
    NewFile,
    NewFolder,
    Rename,
    Delete,
}

#[derive(Debug, Clone, Default)]
pub struct TreeState {
    pub entries: Vec<TreeEntry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub open_dirs: HashSet<PathBuf>,
    pub root: Option<PathBuf>,
    pub action: TreeAction,
    pub input_buf: String,
    pub marked_for_move: Option<PathBuf>,
    folder_color_idx: usize,
}

impl Default for TreeAction {
    fn default() -> Self {
        TreeAction::None
    }
}

impl TreeState {
    pub fn build(&mut self, root: &Path) {
        self.root = Some(root.to_path_buf());
        self.entries.clear();
        self.folder_color_idx = 0;

        // virtual root entry — represents the project root folder
        let root_name = root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.entries.push(TreeEntry {
            path: root.to_path_buf(),
            name: root_name,
            is_dir: true,
            depth: 0,
            color: Color::Rgb(137, 180, 250),
            git_status: None,
        });

        self.build_dir(root, root, 0);
    }

    pub fn apply_git_statuses(&mut self, git_info: &crate::git::status::GitInfo) {
        let root = match &self.root {
            Some(r) => r.clone(),
            None => return,
        };
        for entry in &mut self.entries {
            if let Ok(rel) = entry.path.strip_prefix(&root) {
                entry.git_status = git_info.status_for_file(rel);
            }
        }
    }

    fn build_dir(&mut self, root: &Path, dir: &Path, depth: usize) {
        let mut children: Vec<PathBuf> = match std::fs::read_dir(dir) {
            Ok(entries) => entries.filter_map(|e| e.ok().map(|e| e.path())).collect(),
            Err(_) => return,
        };

        // sort: dirs first, then by extension, then alphabetical within same extension
        children.sort_by(|a, b| {
            let a_dir = a.is_dir();
            let b_dir = b.is_dir();
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => a.file_name().cmp(&b.file_name()),
                (false, false) => {
                    let a_ext = a.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let b_ext = b.extension().and_then(|e| e.to_str()).unwrap_or("");
                    match a_ext.cmp(b_ext) {
                        std::cmp::Ordering::Equal => a.file_name().cmp(&b.file_name()),
                        other => other,
                    }
                }
            }
        });

        // show all files except .git/
        children.retain(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            name != ".git"
        });

        let dir_color = FOLDER_PALETTE[self.folder_color_idx % FOLDER_PALETTE.len()];

        for child in children {
            let name = child
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let is_dir = child.is_dir();

            let color = if is_dir {
                self.folder_color_idx += 1;
                FOLDER_PALETTE[self.folder_color_idx % FOLDER_PALETTE.len()]
            } else {
                dir_color
            };

            self.entries.push(TreeEntry {
                path: child.clone(),
                name,
                is_dir,
                depth,
                color,
                git_status: None,
            });

            if is_dir && self.open_dirs.contains(&child) {
                self.build_dir(root, &child, depth + 1);
            }
        }
    }

    pub fn toggle_dir(&mut self) {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                let path = entry.path.clone();
                if self.open_dirs.contains(&path) {
                    self.open_dirs.remove(&path);
                } else {
                    self.open_dirs.insert(path);
                }
                if let Some(root) = self.root.clone() {
                    self.build(&root);
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
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            if self.selected >= self.scroll_offset + visible_height {
                self.scroll_offset = self.selected - visible_height + 1;
            }
        }
    }

    pub fn move_up_folders_only(&mut self) {
        let mut idx = self.selected;
        while idx > 0 {
            idx -= 1;
            if self.entries[idx].is_dir {
                self.selected = idx;
                if self.selected < self.scroll_offset {
                    self.scroll_offset = self.selected;
                }
                return;
            }
        }
    }

    pub fn move_down_folders_only(&mut self, visible_height: usize) {
        let mut idx = self.selected;
        while idx + 1 < self.entries.len() {
            idx += 1;
            if self.entries[idx].is_dir {
                self.selected = idx;
                if self.selected >= self.scroll_offset + visible_height {
                    self.scroll_offset = self.selected - visible_height + 1;
                }
                return;
            }
        }
    }

    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.entries.get(self.selected)
    }

    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.entries.get(self.selected).map(|e| &e.path)
    }

    pub fn start_action(&mut self, action: TreeAction) {
        self.action = action;
        self.input_buf.clear();
    }

    pub fn cancel_action(&mut self) {
        self.action = TreeAction::None;
        self.input_buf.clear();
    }

    pub fn confirm_new_file(&mut self) -> Option<PathBuf> {
        if self.input_buf.is_empty() {
            self.cancel_action();
            return None;
        }
        let parent = self.selected_dir();
        let new_path = parent.join(&self.input_buf);
        if let Some(dir) = new_path.parent() {
            if let Err(e) = std::fs::create_dir_all(dir) {
                tracing::error!("failed to create dir: {}", e);
                self.cancel_action();
                return None;
            }
        }
        if let Err(e) = std::fs::write(&new_path, "") {
            tracing::error!("failed to create file: {}", e);
            self.cancel_action();
            return None;
        }
        self.cancel_action();
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Some(new_path)
    }

    pub fn confirm_new_folder(&mut self) -> Option<PathBuf> {
        if self.input_buf.is_empty() {
            self.cancel_action();
            return None;
        }
        let parent = self.selected_dir();
        let new_path = parent.join(&self.input_buf);
        let _ = std::fs::create_dir_all(&new_path);
        self.open_dirs.insert(new_path.clone());
        self.cancel_action();
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Some(new_path)
    }

    pub fn confirm_rename(&mut self) -> Option<PathBuf> {
        if self.input_buf.is_empty() {
            self.cancel_action();
            return None;
        }
        if let Some(entry) = self.entries.get(self.selected) {
            let old_path = entry.path.clone();
            let new_path = old_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(&self.input_buf);
            let _ = std::fs::rename(&old_path, &new_path);
            self.cancel_action();
            if let Some(root) = self.root.clone() {
                self.build(&root);
            }
            return Some(new_path);
        }
        self.cancel_action();
        None
    }

    pub fn confirm_delete(&mut self) -> bool {
        if let Some(entry) = self.entries.get(self.selected) {
            let path = entry.path.clone();
            let result = if entry.is_dir {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            if result.is_ok() {
                self.cancel_action();
                if let Some(root) = self.root.clone() {
                    self.build(&root);
                }
                if self.selected >= self.entries.len() && self.selected > 0 {
                    self.selected -= 1;
                }
                return true;
            }
        }
        self.cancel_action();
        false
    }

    pub fn mark_for_move(&mut self) {
        if let Some(path) = self.selected_path().cloned() {
            self.marked_for_move = Some(path);
            // don't set action — allow normal navigation to continue
        }
    }

    pub fn confirm_move(&mut self) -> Option<PathBuf> {
        let marked = self.marked_for_move.take()?;
        // destination is the currently selected folder (or parent of selected file)
        let dest_dir = self.selected_dir();
        let file_name = marked.file_name()?;
        let new_path = dest_dir.join(file_name);
        if marked == new_path {
            return None; // same location, skip
        }
        let _ = std::fs::rename(&marked, &new_path);
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Some(new_path)
    }

    pub fn cancel_move(&mut self) {
        self.marked_for_move = None;
    }

    fn selected_dir(&self) -> PathBuf {
        if let Some(entry) = self.entries.get(self.selected) {
            if entry.is_dir {
                entry.path.clone()
            } else {
                entry.path.parent().unwrap_or(Path::new(".")).to_path_buf()
            }
        } else {
            self.root.clone().unwrap_or_else(|| PathBuf::from("."))
        }
    }
}

fn file_icon(name: &str, is_dir: bool, is_open: bool) -> &'static str {
    if is_dir {
        return if is_open {
            "\u{f07c}  " // nf-fa-folder_open
        } else {
            "\u{f07b}  " // nf-fa-folder
        };
    }
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "\u{e7a8}  ",      // nf-dev-rust
        "py" => "\u{e73c}  ",      // nf-dev-python
        "js" | "mjs" | "cjs" => "\u{e74e}  ", // nf-dev-javascript
        "ts" | "tsx" => "\u{e628}  ", // nf-seti-typescript
        "html" | "htm" => "\u{e736}  ", // nf-dev-html5
        "css" | "scss" => "\u{e749}  ", // nf-dev-css3
        "json" => "\u{e60b}  ",    // nf-seti-json
        "toml" => "\u{e615}  ",    // nf-seti-config
        "yaml" | "yml" => "\u{e615}  ",
        "md" => "\u{e73e}  ",      // nf-dev-markdown
        "sh" | "bash" | "zsh" => "\u{e795}  ", // nf-dev-terminal
        "php" => "\u{e73d}  ",     // nf-dev-php
        "c" | "h" => "\u{e61e}  ", // nf-seti-c
        "sql" => "\u{f1c0}  ",     // nf-fa-database
        "lock" => "\u{f023}  ",    // nf-fa-lock
        "txt" => "\u{f15c}  ",     // nf-fa-file_text
        "gitignore" => "\u{e702}  ", // nf-dev-git
        _ => "\u{f15b}  ",         // nf-fa-file
    }
}

pub struct FileTreeWidget<'a> {
    pub state: &'a TreeState,
}

impl<'a> Widget for FileTreeWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = Color::Rgb(24, 24, 37);
        let border_color = Color::Rgb(69, 71, 90);
        let selected_bg = Color::Rgb(45, 45, 65);
        let dim = Color::Rgb(86, 95, 137);
        let git_colors = |status: char| -> Color {
            match status {
                'M' => Color::Rgb(249, 226, 175), // yellow
                'A' => Color::Rgb(166, 227, 161), // green
                'D' => Color::Rgb(247, 118, 142), // red
                'U' => Color::Rgb(247, 118, 142), // red
                '?' => Color::Rgb(86, 95, 137),   // dim
                _ => Color::Rgb(192, 202, 245),
            }
        };

        // fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // right border
        let border_x = area.x + area.width - 1;
        for y in area.y..area.y + area.height {
            buf.cell_mut((border_x, y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        let content_width = area.width.saturating_sub(1) as usize;
        let visible_height = area.height as usize;

        // title — use project root name from entries[0]
        let root_name = self.state.entries.first()
            .map(|e| e.name.as_str())
            .unwrap_or("Explorer");
        let title = format!(" {} - Explorer ", root_name);
        let is_root_selected = self.state.selected == 0;
        let title_bg = if is_root_selected { selected_bg } else { bg };

        // fill title bg
        for lx in area.x..area.x + area.width - 1 {
            buf.cell_mut((lx, area.y)).map(|cell| {
                cell.set_style(Style::default().bg(title_bg));
            });
        }

        let mut x = area.x + 1;
        for ch in title.chars() {
            if x as usize >= area.x as usize + content_width {
                break;
            }
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(
                    Style::default()
                        .fg(Color::Rgb(137, 180, 250))
                        .bg(title_bg)
                        .add_modifier(Modifier::BOLD),
                );
            });
            x += 1;
        }

        // entries (skip index 0 = root, it's rendered as the title)
        let entries_start = 1usize;
        for i in 0..(visible_height.saturating_sub(1)) {
            let entry_idx = self.state.scroll_offset + i + 1; // +1 to skip root
            let y = area.y + (entries_start + i) as u16;
            if y >= area.y + area.height {
                break;
            }

            if let Some(entry) = self.state.entries.get(entry_idx) {
                let is_selected = entry_idx == self.state.selected;
                let line_bg = if is_selected { selected_bg } else { bg };

                // fill line bg
                for lx in area.x..area.x + area.width - 1 {
                    buf.cell_mut((lx, y)).map(|cell| {
                        cell.set_style(Style::default().bg(line_bg));
                    });
                }

                let indent = "  ".repeat(entry.depth);
                let is_open = entry.is_dir && self.state.open_dirs.contains(&entry.path);
                let icon = file_icon(&entry.name, entry.is_dir, is_open);
                let git_str = entry
                    .git_status
                    .map(|s| format!(" {}", s))
                    .unwrap_or_default();

                let move_indicator =
                    if self.state.marked_for_move.as_ref() == Some(&entry.path) {
                        " [moving]"
                    } else {
                        ""
                    };

                let display = format!(
                    " {}{}{}{}{}",
                    indent, icon, entry.name, git_str, move_indicator
                );

                // icon starts after " " + indent
                let icon_start = 1 + indent.len();
                let icon_end = icon_start + icon.chars().count();
                let name_start = icon_end;
                let name_end = name_start + entry.name.len();

                let mut cx = area.x;
                for (ci, ch) in display.chars().enumerate() {
                    if cx >= area.x + area.width - 1 {
                        break;
                    }
                    let style = if ci >= icon_start && ci < icon_end {
                        // icon same color as name
                        let mut s = Style::default().fg(entry.color).bg(line_bg);
                        if entry.is_dir {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        s
                    } else if ci >= name_start && ci < name_end {
                        let mut s = Style::default().fg(entry.color).bg(line_bg);
                        if entry.is_dir {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        s
                    } else if ci >= name_end && entry.git_status.is_some() {
                        Style::default()
                            .fg(git_colors(entry.git_status.unwrap()))
                            .bg(line_bg)
                    } else {
                        Style::default().fg(dim).bg(line_bg)
                    };
                    buf.cell_mut((cx, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(style);
                    });
                    cx += 1;
                }
            }
        }

        // action input at bottom
        if self.state.action != TreeAction::None {
            let action_y = area.y + area.height - 1;
            let label = match self.state.action {
                TreeAction::NewFile => " new file: ",
                TreeAction::NewFolder => " new folder: ",
                TreeAction::Rename => " rename: ",
                TreeAction::Delete => " delete? (y/n) ",
                TreeAction::None => "",
            };
            let display = format!("{}{}", label, self.state.input_buf);

            for lx in area.x..area.x + area.width - 1 {
                buf.cell_mut((lx, action_y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(
                        Style::default()
                            .bg(Color::Rgb(45, 45, 65)),
                    );
                });
            }

            let mut cx = area.x;
            for (ci, ch) in display.chars().enumerate() {
                if cx >= area.x + area.width - 1 {
                    break;
                }
                let style = if ci < label.len() {
                    Style::default()
                        .fg(Color::Rgb(249, 226, 175))
                        .bg(Color::Rgb(45, 45, 65))
                } else {
                    Style::default()
                        .fg(Color::Rgb(192, 202, 245))
                        .bg(Color::Rgb(45, 45, 65))
                };
                buf.cell_mut((cx, action_y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                cx += 1;
            }
        }
    }
}
