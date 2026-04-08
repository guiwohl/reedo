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
    pub fs_undo_stack: Vec<FsOperation>,
    pub fs_redo_stack: Vec<FsOperation>,
    folder_color_idx: usize,
}

#[derive(Debug, Clone)]
pub enum FsOperation {
    Move {
        from: PathBuf,
        to: PathBuf,
    },
    Create {
        path: PathBuf,
        is_dir: bool,
    },
    Delete {
        path: PathBuf,
        content: Option<String>,
        is_dir: bool,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
}

impl Default for TreeAction {
    fn default() -> Self {
        TreeAction::None
    }
}

impl TreeState {
    fn push_fs_op(&mut self, op: FsOperation) {
        self.fs_undo_stack.push(op);
        self.fs_redo_stack.clear();
    }

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

    pub fn build_git_only(&mut self, root: &Path, git_info: &crate::git::status::GitInfo) {
        self.root = Some(root.to_path_buf());
        self.entries.clear();
        self.folder_color_idx = 0;

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

        let mut paths: Vec<_> = git_info
            .file_statuses
            .iter()
            .map(|(p, s)| (p.clone(), *s))
            .collect();
        paths.sort_by(|a, b| a.0.cmp(&b.0));

        for (rel_path, status) in &paths {
            let full_path = root.join(rel_path);
            let name = rel_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let color = match status {
                'M' => Color::Rgb(249, 226, 175),
                'A' => Color::Rgb(166, 227, 161),
                'D' => Color::Rgb(247, 118, 142),
                '?' => Color::Rgb(148, 226, 213),
                _ => Color::Rgb(192, 202, 245),
            };
            self.entries.push(TreeEntry {
                path: full_path,
                name,
                is_dir: false,
                depth: 0,
                color,
                git_status: Some(*status),
            });
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

    pub fn selected_relative_path(&self) -> Option<String> {
        let entry = self.selected_entry()?;
        let root = self.root.as_ref()?;
        match entry.path.strip_prefix(root) {
            Ok(rel) if rel.as_os_str().is_empty() => Some(".".to_string()),
            Ok(rel) => Some(rel.display().to_string()),
            Err(_) => None,
        }
    }

    pub fn selected_full_path(&self) -> Option<String> {
        self.selected_entry()
            .map(|entry| entry.path.display().to_string())
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
        self.push_fs_op(FsOperation::Create {
            path: new_path.clone(),
            is_dir: false,
        });
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
        self.push_fs_op(FsOperation::Create {
            path: new_path.clone(),
            is_dir: true,
        });
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
            if std::fs::rename(&old_path, &new_path).is_ok() {
                self.push_fs_op(FsOperation::Rename {
                    from: old_path,
                    to: new_path.clone(),
                });
            }
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
            let is_dir = entry.is_dir;
            // save file content for undo (only for files, not dirs)
            let content = if !is_dir {
                std::fs::read_to_string(&path).ok()
            } else {
                None
            };
            let result = if is_dir {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
            if result.is_ok() {
                self.push_fs_op(FsOperation::Delete {
                    path: path.clone(),
                    content,
                    is_dir,
                });
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
        if std::fs::rename(&marked, &new_path).is_ok() {
            self.push_fs_op(FsOperation::Move {
                from: marked,
                to: new_path.clone(),
            });
        }
        if let Some(root) = self.root.clone() {
            self.build(&root);
        }
        Some(new_path)
    }

    pub fn cancel_move(&mut self) {
        self.marked_for_move = None;
    }

    pub fn undo_last_fs_op(&mut self) -> bool {
        let op = match self.fs_undo_stack.last().cloned() {
            Some(op) => op,
            None => return false,
        };
        let ok = self.apply_fs_op(&op, true);
        if ok {
            self.fs_undo_stack.pop();
            self.fs_redo_stack.push(op);
            if let Some(root) = self.root.clone() {
                self.build(&root);
            }
        }
        ok
    }

    pub fn redo_last_fs_op(&mut self) -> bool {
        let op = match self.fs_redo_stack.last().cloned() {
            Some(op) => op,
            None => return false,
        };
        let ok = self.apply_fs_op(&op, false);
        if ok {
            self.fs_redo_stack.pop();
            self.fs_undo_stack.push(op);
            if let Some(root) = self.root.clone() {
                self.build(&root);
            }
        }
        ok
    }

    fn apply_fs_op(&mut self, op: &FsOperation, reverse: bool) -> bool {
        match op {
            FsOperation::Move { from, to } | FsOperation::Rename { from, to } => {
                let (src, dest) = if reverse { (to, from) } else { (from, to) };
                std::fs::rename(src, dest).is_ok()
            }
            FsOperation::Create { path, is_dir } => {
                if reverse {
                    if *is_dir {
                        std::fs::remove_dir_all(path).is_ok()
                    } else {
                        std::fs::remove_file(path).is_ok()
                    }
                } else if *is_dir {
                    std::fs::create_dir_all(path).is_ok()
                } else {
                    std::fs::write(path, "").is_ok()
                }
            }
            FsOperation::Delete {
                path,
                content,
                is_dir,
            } => {
                if reverse {
                    if *is_dir {
                        std::fs::create_dir_all(path).is_ok()
                    } else {
                        let data = content.as_deref().unwrap_or("");
                        std::fs::write(path, data).is_ok()
                    }
                } else if *is_dir {
                    std::fs::remove_dir_all(path).is_ok()
                } else {
                    std::fs::remove_file(path).is_ok()
                }
            }
        }
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

pub fn file_icon_pub(name: &str, is_dir: bool, is_open: bool) -> &'static str {
    file_icon(name, is_dir, is_open)
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
        "rs" => "\u{e7a8}  ",                 // nf-dev-rust
        "py" => "\u{e73c}  ",                 // nf-dev-python
        "js" | "mjs" | "cjs" => "\u{e74e}  ", // nf-dev-javascript
        "ts" | "tsx" => "\u{e628}  ",         // nf-seti-typescript
        "html" | "htm" => "\u{e736}  ",       // nf-dev-html5
        "css" | "scss" => "\u{e749}  ",       // nf-dev-css3
        "json" => "\u{e60b}  ",               // nf-seti-json
        "toml" => "\u{e615}  ",               // nf-seti-config
        "yaml" | "yml" => "\u{e615}  ",
        "md" => "\u{e73e}  ",                  // nf-dev-markdown
        "sh" | "bash" | "zsh" => "\u{e795}  ", // nf-dev-terminal
        "php" => "\u{e73d}  ",                 // nf-dev-php
        "c" | "h" => "\u{e61e}  ",             // nf-seti-c
        "sql" => "\u{f1c0}  ",                 // nf-fa-database
        "lock" => "\u{f023}  ",                // nf-fa-lock
        "txt" => "\u{f15c}  ",                 // nf-fa-file_text
        "gitignore" => "\u{e702}  ",           // nf-dev-git
        _ => "\u{f15b}  ",                     // nf-fa-file
    }
}

#[cfg(test)]
mod tests {
    use super::{TreeEntry, TreeState};
    use ratatui::style::Color;
    use std::path::PathBuf;

    #[test]
    fn selected_relative_path_returns_dot_for_root() {
        let root = PathBuf::from("/tmp/reedo");
        let mut state = TreeState {
            root: Some(root.clone()),
            entries: vec![TreeEntry {
                path: root,
                name: "reedo".to_string(),
                is_dir: true,
                depth: 0,
                color: Color::Reset,
                git_status: None,
            }],
            ..TreeState::default()
        };

        state.selected = 0;

        assert_eq!(state.selected_relative_path().as_deref(), Some("."));
    }

    #[test]
    fn selected_relative_path_returns_path_below_root() {
        let root = PathBuf::from("/tmp/reedo");
        let child = root.join("src/main.rs");
        let state = TreeState {
            root: Some(root),
            entries: vec![
                TreeEntry {
                    path: PathBuf::from("/tmp/reedo"),
                    name: "reedo".to_string(),
                    is_dir: true,
                    depth: 0,
                    color: Color::Reset,
                    git_status: None,
                },
                TreeEntry {
                    path: child.clone(),
                    name: "main.rs".to_string(),
                    is_dir: false,
                    depth: 1,
                    color: Color::Reset,
                    git_status: None,
                },
            ],
            selected: 1,
            ..TreeState::default()
        };

        assert_eq!(
            state.selected_relative_path().as_deref(),
            Some("src/main.rs")
        );
        assert_eq!(
            state.selected_full_path().as_deref(),
            Some(child.to_str().unwrap())
        );
    }
}

pub struct FileTreeWidget<'a> {
    pub state: &'a TreeState,
    pub theme: &'a crate::config::theme::Theme,
}

pub fn tree_inner_area(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

pub fn tree_list_height(area: Rect, has_action: bool) -> usize {
    let inner_height = tree_inner_area(area).height as usize;
    inner_height
        .saturating_sub(1) // title row
        .saturating_sub(usize::from(has_action)) // optional action row
}

impl<'a> Widget for FileTreeWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        if area.width < 3 || area.height < 3 {
            return;
        }

        let bg = self.theme.popup_bg();
        let border_color = self.theme.popup_border();
        let selected_bg = self.theme.popup_selected();
        let dim = self.theme.popup_dim();
        let accent = self.theme.popup_accent();
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

        // border
        let right_x = area.x + area.width - 1;
        let bottom_y = area.y + area.height - 1;
        for x in area.x..=right_x {
            let ch = if x == area.x || x == right_x {
                '┌'
            } else {
                '─'
            };
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }
        for x in area.x..=right_x {
            let ch = if x == area.x || x == right_x {
                '└'
            } else {
                '─'
            };
            buf.cell_mut((x, bottom_y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }
        for y in area.y..area.y + area.height {
            buf.cell_mut((area.x, y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
            buf.cell_mut((right_x, y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }
        buf.cell_mut((area.x, area.y)).map(|cell| {
            cell.set_char('┌');
            cell.set_style(Style::default().fg(border_color).bg(bg));
        });
        buf.cell_mut((right_x, area.y)).map(|cell| {
            cell.set_char('┐');
            cell.set_style(Style::default().fg(border_color).bg(bg));
        });
        buf.cell_mut((area.x, bottom_y)).map(|cell| {
            cell.set_char('└');
            cell.set_style(Style::default().fg(border_color).bg(bg));
        });
        buf.cell_mut((right_x, bottom_y)).map(|cell| {
            cell.set_char('┘');
            cell.set_style(Style::default().fg(border_color).bg(bg));
        });

        let inner = tree_inner_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let content_width = inner.width as usize;
        let title_y = inner.y;

        // title — use project root name from entries[0]
        let root_name = self
            .state
            .entries
            .first()
            .map(|e| e.name.as_str())
            .unwrap_or("Explorer");
        let title = format!(" \u{f015}  {} - Explorer ", root_name); // nf-fa-home
        let is_root_selected = self.state.selected == 0;
        let title_bg = if is_root_selected { selected_bg } else { bg };

        // fill title bg
        for lx in inner.x..inner.x + inner.width {
            buf.cell_mut((lx, title_y)).map(|cell| {
                cell.set_style(Style::default().bg(title_bg));
            });
        }

        let mut x = inner.x;
        for ch in title.chars() {
            if x as usize >= inner.x as usize + content_width {
                break;
            }
            buf.cell_mut((x, title_y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(
                    Style::default()
                        .fg(accent)
                        .bg(title_bg)
                        .add_modifier(Modifier::BOLD),
                );
            });
            x += 1;
        }

        // entries (skip index 0 = root, it's rendered as the title)
        let entries_start = 1usize;
        for i in 0..tree_list_height(area, self.state.action != TreeAction::None) {
            let entry_idx = self.state.scroll_offset + i + 1; // +1 to skip root
            let y = inner.y + (entries_start + i) as u16;
            if y >= inner.y + inner.height {
                break;
            }

            if let Some(entry) = self.state.entries.get(entry_idx) {
                let is_selected = entry_idx == self.state.selected;
                let line_bg = if is_selected { selected_bg } else { bg };

                // fill line bg
                for lx in inner.x..inner.x + inner.width {
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

                let move_indicator = if self.state.marked_for_move.as_ref() == Some(&entry.path) {
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

                let mut cx = inner.x;
                for (ci, ch) in display.chars().enumerate() {
                    if cx >= inner.x + inner.width {
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
            let action_y = inner.y + inner.height - 1;
            let action_bg = self.theme.popup_selected();
            let label = match self.state.action {
                TreeAction::NewFile => " new file: ",
                TreeAction::NewFolder => " new folder: ",
                TreeAction::Rename => " rename: ",
                TreeAction::Delete => " delete? (y/n) ",
                TreeAction::None => "",
            };
            let display = format!("{}{}", label, self.state.input_buf);

            for lx in inner.x..inner.x + inner.width {
                buf.cell_mut((lx, action_y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(action_bg));
                });
            }

            let mut cx = inner.x;
            for (ci, ch) in display.chars().enumerate() {
                if cx >= inner.x + inner.width {
                    break;
                }
                let style = if ci < label.len() {
                    Style::default().fg(accent).bg(action_bg)
                } else {
                    Style::default().fg(self.theme.fg()).bg(action_bg)
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
