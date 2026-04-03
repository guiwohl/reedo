use std::path::PathBuf;
use std::time::Instant;

use crate::config::settings::Settings;
use crate::config::theme::{self, Theme};
use crate::editor::buffer::Buffer;
use crate::editor::cursor::Cursor;
use crate::editor::mode::Mode;
use crate::git::status::{GitInfo, GutterMark};
use crate::syntax::highlight::Highlighter;
use crate::ui::fuzzy::FuzzyState;
use crate::ui::keybind_help::KeybindHelpState;
use crate::ui::replace::ReplaceState;
use crate::ui::replace_project::ProjectReplaceState;
use crate::ui::search::SearchState;
use crate::ui::search_project::ProjectSearchState;
use crate::ui::theme_switcher::ThemeSwitcherState;
use crate::ui::tree::TreeState;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Popup {
    None,
    FileTree,
    Search,
    SearchProject,
    Replace,
    ReplaceProject,
    FuzzyFinder,
    ThemeSwitcher,
    KeybindHelp,
    PaddingInput,
}

pub struct App {
    pub buffer: Buffer,
    pub cursor: Cursor,
    pub mode: Mode,
    pub highlighter: Highlighter,
    pub needs_reparse: bool,
    pub running: bool,
    pub viewport_top: usize,
    pub viewport_left: usize,
    pub viewport_height: usize,
    pub viewport_width: usize,
    pub horizontal_padding: usize,
    pub line_wrapping: bool,
    pub indent_size: usize,
    pub yank_buffer: Option<String>,
    pub last_edit_time: Option<Instant>,
    pub autosave_delay_ms: u64,
    pub last_file_mtime: Option<std::time::SystemTime>,
    pub last_external_check: Instant,
    pub flash_message: Option<(String, Instant)>,
    pub theme: Theme,
    pub pending_key: Option<char>,
    pub popup: Popup,
    pub tree_state: TreeState,
    pub search_state: SearchState,
    pub replace_state: ReplaceState,
    pub fuzzy_state: FuzzyState,
    pub project_search_state: ProjectSearchState,
    pub project_replace_state: ProjectReplaceState,
    pub theme_switcher_state: ThemeSwitcherState,
    pub keybind_help_state: KeybindHelpState,
    pub padding_input: String,
    pub project_root: Option<PathBuf>,
    pub git_info: Option<GitInfo>,
    pub gutter_marks: HashMap<usize, GutterMark>,
    pub last_git_refresh: Instant,
}

impl App {
    pub fn new(settings: Settings) -> Self {
        let loaded_theme = theme::load_theme(&settings.theme);
        Self {
            buffer: Buffer::default(),
            cursor: Cursor::default(),
            mode: Mode::default(),
            highlighter: Highlighter::default(),
            needs_reparse: false,
            running: true,
            viewport_top: 0,
            viewport_left: 0,
            viewport_height: 24,
            viewport_width: 80,
            horizontal_padding: settings.horizontal_padding,
            line_wrapping: settings.line_wrapping,
            indent_size: settings.indent_size,
            autosave_delay_ms: settings.autosave_delay_ms,
            yank_buffer: None,
            last_edit_time: None,
            last_file_mtime: None,
            last_external_check: Instant::now(),
            flash_message: None,
            theme: loaded_theme,
            pending_key: None,
            popup: Popup::None,
            tree_state: TreeState::default(),
            search_state: SearchState::default(),
            replace_state: ReplaceState::default(),
            fuzzy_state: FuzzyState::default(),
            project_search_state: ProjectSearchState::default(),
            project_replace_state: ProjectReplaceState::default(),
            theme_switcher_state: ThemeSwitcherState::default(),
            keybind_help_state: KeybindHelpState::default(),
            padding_input: String::new(),
            project_root: None,
            git_info: None,
            gutter_marks: HashMap::new(),
            last_git_refresh: Instant::now(),
        }
    }

    pub fn set_project_root(&mut self, path: PathBuf) {
        self.git_info = GitInfo::gather(&path);
        self.project_root = Some(path);
    }

    pub fn open_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.buffer = Buffer::from_file(path)?;
        self.cursor = Cursor::default();
        self.viewport_top = 0;
        self.viewport_left = 0;
        self.last_file_mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());

        // detect and setup syntax highlighting
        if let Some(config) = Highlighter::detect_language(path) {
            self.highlighter.set_language(&config, &self.theme.colors);
            let source = self.buffer.rope.to_string();
            self.highlighter.parse(&source);
            self.highlighter.compute_styles(&source);
        } else if crate::syntax::highlight::is_env_file(path) {
            // .env files use simple highlighting, no tree-sitter
            tracing::info!("detected .env file");
        }

        // compute git gutter marks
        if let Some(ref root) = self.project_root {
            self.gutter_marks = GitInfo::diff_for_file(root, path);
        }

        tracing::info!("opened file: {}", path.display());
        Ok(())
    }

    pub fn flash(&mut self, msg: impl Into<String>) {
        self.flash_message = Some((msg.into(), Instant::now()));
    }

    pub fn mark_edited(&mut self) {
        self.last_edit_time = Some(Instant::now());
        self.needs_reparse = true;
    }

    pub fn reparse_if_needed(&mut self) {
        if self.needs_reparse && self.highlighter.is_active() {
            let source = self.buffer.rope.to_string();
            self.highlighter.parse(&source);
            self.highlighter.compute_styles(&source);
            self.needs_reparse = false;
        }
    }

    pub fn check_autosave(&mut self) {
        if let Some(last_edit) = self.last_edit_time {
            if last_edit.elapsed().as_millis() >= self.autosave_delay_ms as u128
                && self.buffer.dirty
                && self.buffer.file_path.is_some()
            {
                if let Err(e) = self.buffer.save() {
                    tracing::error!("autosave failed: {}", e);
                    self.flash("save failed");
                } else {
                    self.flash("saved");
                    let path = self.buffer.file_path.clone();
                    if let Some(p) = path {
                        self.last_file_mtime =
                            std::fs::metadata(&p).ok().and_then(|m| m.modified().ok());
                    }
                }
                self.last_edit_time = None;
            }
        }
    }

    pub fn check_git_refresh(&mut self) {
        if self.last_git_refresh.elapsed().as_secs() >= 5 {
            if let Some(root) = &self.project_root {
                self.git_info = GitInfo::gather(root);
                if let Some(ref file_path) = self.buffer.file_path {
                    self.gutter_marks = GitInfo::diff_for_file(root, file_path);
                }
            }
            self.last_git_refresh = Instant::now();
        }
    }

    pub fn check_external_changes(&mut self) {
        if self.last_external_check.elapsed().as_secs() < 1 {
            return;
        }
        self.last_external_check = Instant::now();

        let path = match &self.buffer.file_path {
            Some(p) => p.clone(),
            None => return,
        };

        let current_mtime = match std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
        {
            Some(t) => t,
            None => return,
        };

        let changed = match self.last_file_mtime {
            Some(prev) => current_mtime != prev,
            None => false,
        };

        if !changed {
            return;
        }

        // file changed on disk
        if self.buffer.dirty {
            // local unsaved edits — don't reload, user's work takes priority
            tracing::info!("file changed externally but buffer is dirty, skipping reload");
            return;
        }

        // reload: preserve cursor position as best we can
        let old_line = self.cursor.pos.line;
        let old_col = self.cursor.pos.col;

        if let Ok(()) = self.reload_file(&path) {
            // clamp cursor to new file bounds
            let max_line = self.buffer.line_count().saturating_sub(1);
            let new_line = old_line.min(max_line);
            let new_col = old_col.min(self.buffer.line_len(new_line));
            self.cursor.move_to(new_line, new_col, false);
            self.cursor.update_desired_col();
            self.flash("reloaded — external change");
            tracing::info!("reloaded file after external change");
        }
    }

    fn reload_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.buffer = Buffer::from_file(path)?;
        self.last_file_mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        self.needs_reparse = true;

        // re-detect syntax
        if let Some(config) = Highlighter::detect_language(path) {
            self.highlighter.set_language(&config, &self.theme.colors);
            let source = self.buffer.rope.to_string();
            self.highlighter.parse(&source);
            self.highlighter.compute_styles(&source);
        }

        // refresh gutter marks
        if let Some(ref root) = self.project_root {
            self.gutter_marks = GitInfo::diff_for_file(root, path);
        }

        Ok(())
    }

    pub fn scroll_to_cursor(&mut self) {
        // vertical scroll
        if self.cursor.pos.line < self.viewport_top {
            self.viewport_top = self.cursor.pos.line;
        }
        let bottom = self.viewport_top + self.viewport_height.saturating_sub(2); // -2 for statusbar
        if self.cursor.pos.line >= bottom {
            self.viewport_top = self.cursor.pos.line - self.viewport_height.saturating_sub(3);
        }

        // horizontal scroll
        if self.line_wrapping {
            self.viewport_left = 0;
            return;
        }

        let gutter_width = format!("{}", self.buffer.line_count()).len().max(3) + 1;
        let text_width = self
            .viewport_width
            .saturating_sub(gutter_width + self.horizontal_padding * 2);
        if self.cursor.pos.col < self.viewport_left {
            self.viewport_left = self.cursor.pos.col;
        }
        if self.cursor.pos.col >= self.viewport_left + text_width {
            self.viewport_left = self.cursor.pos.col - text_width + 1;
        }
    }
}
