use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::editor::brackets;
use crate::editor::clipboard;
use crate::editor::mode::Mode;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let ctrl_alt = ctrl && alt;
    let ctrl_shift = ctrl && shift;

    // clear pending key if the next key isn't completing a sequence
    let is_pending_completion = app.mode == Mode::Normal
        && !ctrl
        && matches!(key.code, KeyCode::Char(c) if
            (app.pending_key == Some('d') && c == 'd') ||
            (app.pending_key == Some('y') && c == 'y'));
    if !is_pending_completion {
        app.pending_key = None;
    }

    match key.code {
        // mode switching
        KeyCode::Esc => {
            app.buffer.undo_stack.finish_group();
            app.mode = Mode::Normal;
            app.cursor.clear_selection();
            tracing::debug!("mode -> Normal");
        }
        KeyCode::Char('i') if app.mode == Mode::Normal && !ctrl => {
            app.mode = Mode::Insert;
            tracing::debug!("mode -> Insert");
        }
        KeyCode::Insert if app.mode == Mode::Normal => {
            app.mode = Mode::Insert;
            tracing::debug!("mode -> Insert (Insert key)");
        }

        KeyCode::Char('z') if ctrl => {
            if let Some(pos) = app.buffer.apply_undo() {
                app.cursor.move_to(pos.line, pos.col, false);
                app.cursor.update_desired_col();
                app.mark_edited();
            }
        }
        KeyCode::Char('y') if ctrl => {
            if let Some(pos) = app.buffer.apply_redo() {
                app.cursor.move_to(pos.line, pos.col, false);
                app.cursor.update_desired_col();
                app.mark_edited();
            }
        }

        // ctrl+a select all
        KeyCode::Char('a') if ctrl => {
            let last_line = app.buffer.line_count().saturating_sub(1);
            let last_col = app.buffer.line_len(last_line);
            app.cursor.select_all(last_line, last_col);
            tracing::debug!("select all");
        }

        // ctrl+s save (manual, in addition to autosave)
        KeyCode::Char('s') if ctrl => {
            if let Err(e) = app.buffer.save() {
                tracing::error!("save failed: {}", e);
                app.flash("save failed");
            } else {
                if let Some(ref p) = app.buffer.file_path {
                    app.last_file_mtime = std::fs::metadata(p).ok().and_then(|m| m.modified().ok());
                }
                app.flash("saved");
            }
        }

        // clipboard: ctrl+c copy
        KeyCode::Char('c') if ctrl => {
            if let Some(sel) = &app.cursor.selection {
                let start = sel.start();
                let end = sel.end();
                let start_idx = app.buffer.rope.line_to_char(start.line) + start.col;
                let end_idx = app.buffer.rope.line_to_char(end.line) + end.col;
                let text: String = app.buffer.rope.slice(start_idx..end_idx).to_string();
                clipboard::copy_to_clipboard(&text);
                app.yank_buffer = Some(text);
            }
        }
        // clipboard: ctrl+x cut
        KeyCode::Char('x') if ctrl => {
            if let Some(sel) = app.cursor.selection.take() {
                let start = sel.start();
                let end = sel.end();
                app.buffer.undo_stack.begin_group(app.cursor.pos);
                let text = app.buffer.delete_range(start, end);
                app.buffer.undo_stack.finish_group();
                clipboard::copy_to_clipboard(&text);
                app.yank_buffer = Some(text);
                app.cursor.move_to(start.line, start.col, false);
                app.cursor.update_desired_col();
                app.mark_edited();
            }
        }
        // clipboard: ctrl+v paste
        KeyCode::Char('v') if ctrl => {
            if let Some(text) = clipboard::paste_from_clipboard() {
                delete_selection_if_any(app);
                app.buffer.undo_stack.begin_group(app.cursor.pos);
                let new_pos = app.buffer.insert_text(app.cursor.pos, &text);
                app.buffer.undo_stack.finish_group();
                app.cursor.move_to(new_pos.line, new_pos.col, false);
                app.cursor.update_desired_col();
                app.mark_edited();
            }
        }

        // popup keybinds
        KeyCode::Char('e') if ctrl || (app.mode == Mode::Normal && !shift) => {
            if app.popup == crate::app::Popup::FileTree {
                app.popup = crate::app::Popup::None;
            } else {
                if let Some(root) = app.project_root.clone() {
                    app.tree_state.build(&root);
                    if let Some(ref git) = app.git_info {
                        app.tree_state.apply_git_statuses(git);
                    }
                }
                app.popup = crate::app::Popup::FileTree;
            }
        }
        KeyCode::Char('f') if ctrl && !shift => {
            app.search_state.reset();
            app.popup = crate::app::Popup::Search;
        }
        KeyCode::Char('F') if ctrl => {
            app.project_search_state.reset();
            app.popup = crate::app::Popup::SearchProject;
        }
        KeyCode::Char('h') if ctrl && !shift => {
            app.replace_state.reset();
            app.popup = crate::app::Popup::Replace;
        }
        KeyCode::Char('H') if ctrl => {
            app.project_replace_state.reset();
            app.popup = crate::app::Popup::ReplaceProject;
        }
        KeyCode::Char('p') if ctrl => {
            app.fuzzy_state.reset();
            if let Some(root) = app.project_root.clone() {
                app.fuzzy_state.collect_files(&root);
            }
            app.popup = crate::app::Popup::FuzzyFinder;
        }
        KeyCode::Char(',') if ctrl => {
            let config_path = crate::config::settings::Settings::config_path();
            let _ = app.open_file(&config_path);
        }
        KeyCode::Char('t') if ctrl => {
            app.theme_switcher_state.reset();
            app.popup = crate::app::Popup::ThemeSwitcher;
        }
        KeyCode::Char(']') if ctrl => {
            app.padding_input = app.horizontal_padding.to_string();
            app.popup = crate::app::Popup::PaddingInput;
        }
        KeyCode::Char('\x1d') => {
            app.padding_input = app.horizontal_padding.to_string();
            app.popup = crate::app::Popup::PaddingInput;
        }
        KeyCode::F(2) => {
            app.padding_input = app.horizontal_padding.to_string();
            app.popup = crate::app::Popup::PaddingInput;
        }
        // keybind help — multiple bindings to cover terminal differences
        KeyCode::Char('/') if ctrl => {
            app.keybind_help_state.reset();
            app.popup = crate::app::Popup::KeybindHelp;
        }
        KeyCode::Char('?') if ctrl => {
            app.keybind_help_state.reset();
            app.popup = crate::app::Popup::KeybindHelp;
        }
        KeyCode::Char('?') if shift => {
            if app.mode == Mode::Normal {
                app.keybind_help_state.reset();
                app.popup = crate::app::Popup::KeybindHelp;
            }
        }
        KeyCode::Char('\x1f') => {
            app.keybind_help_state.reset();
            app.popup = crate::app::Popup::KeybindHelp;
        }
        KeyCode::F(1) => {
            app.keybind_help_state.reset();
            app.popup = crate::app::Popup::KeybindHelp;
        }

        // navigation: ctrl+alt+arrows = paragraph jump
        KeyCode::Up if ctrl_alt => {
            let new_line = find_prev_paragraph(app);
            app.cursor.move_to(new_line, 0, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::Down if ctrl_alt => {
            let new_line = find_next_paragraph(app);
            app.cursor.move_to(new_line, 0, shift);
            app.cursor.update_desired_col();
        }

        // navigation: ctrl+arrows = word jump
        KeyCode::Left if ctrl => {
            let (line, col) = find_prev_word(app);
            app.cursor.move_to(line, col, shift || ctrl_shift);
            app.cursor.update_desired_col();
        }
        KeyCode::Right if ctrl => {
            let (line, col) = find_next_word(app);
            app.cursor.move_to(line, col, shift || ctrl_shift);
            app.cursor.update_desired_col();
        }

        // navigation: arrows (with optional shift for selection)
        KeyCode::Left => {
            let (line, col) = if app.cursor.pos.col > 0 {
                (app.cursor.pos.line, app.cursor.pos.col - 1)
            } else if app.cursor.pos.line > 0 {
                let prev_len = app.buffer.line_len(app.cursor.pos.line - 1);
                (app.cursor.pos.line - 1, prev_len)
            } else {
                (0, 0)
            };
            app.cursor.move_to(line, col, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::Right => {
            let line_len = app.buffer.line_len(app.cursor.pos.line);
            let (line, col) = if app.cursor.pos.col < line_len {
                (app.cursor.pos.line, app.cursor.pos.col + 1)
            } else if app.cursor.pos.line < app.buffer.line_count().saturating_sub(1) {
                (app.cursor.pos.line + 1, 0)
            } else {
                (app.cursor.pos.line, app.cursor.pos.col)
            };
            app.cursor.move_to(line, col, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::Up => {
            if app.cursor.pos.line > 0 {
                let new_line = app.cursor.pos.line - 1;
                let new_col = app.cursor.desired_col.min(app.buffer.line_len(new_line));
                app.cursor.move_to(new_line, new_col, shift);
            }
        }
        KeyCode::Down => {
            let max_line = app.buffer.line_count().saturating_sub(1);
            if app.cursor.pos.line < max_line {
                let new_line = app.cursor.pos.line + 1;
                let new_col = app.cursor.desired_col.min(app.buffer.line_len(new_line));
                app.cursor.move_to(new_line, new_col, shift);
            }
        }
        KeyCode::Home if ctrl => {
            app.cursor.move_to(0, 0, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::End if ctrl => {
            let last = app.buffer.line_count().saturating_sub(1);
            let col = app.buffer.line_len(last);
            app.cursor.move_to(last, col, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::Home => {
            app.cursor.move_to(app.cursor.pos.line, 0, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::End => {
            let len = app.buffer.line_len(app.cursor.pos.line);
            app.cursor.move_to(app.cursor.pos.line, len, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::PageUp => {
            let new_line = find_prev_paragraph(app);
            app.cursor.move_to(new_line, 0, shift);
            app.cursor.update_desired_col();
        }
        KeyCode::PageDown => {
            let new_line = find_next_paragraph(app);
            app.cursor.move_to(new_line, 0, shift);
            app.cursor.update_desired_col();
        }

        // normal mode commands with pending key (dd, yy)
        KeyCode::Char('d') if app.mode == Mode::Normal && !ctrl => {
            if app.pending_key == Some('d') {
                handle_normal_dd(app);
                app.pending_key = None;
            } else {
                app.pending_key = Some('d');
            }
            return;
        }
        KeyCode::Char('y') if app.mode == Mode::Normal && !ctrl => {
            if app.pending_key == Some('y') {
                handle_normal_yy(app);
                app.pending_key = None;
            } else {
                app.pending_key = Some('y');
            }
            return;
        }
        KeyCode::Char('x') if app.mode == Mode::Normal && !ctrl => {
            app.buffer.undo_stack.begin_group(app.cursor.pos);
            app.buffer.delete_char_forward(app.cursor.pos);
            app.buffer.undo_stack.finish_group();
            let line_len = app.buffer.line_len(app.cursor.pos.line);
            if app.cursor.pos.col > line_len {
                app.cursor.move_to(app.cursor.pos.line, line_len, false);
            }
        }
        KeyCode::Char('p') if app.mode == Mode::Normal && !ctrl => {
            handle_normal_paste(app);
        }
        KeyCode::Char('o') if app.mode == Mode::Normal && !ctrl && !shift => {
            handle_normal_o(app, false);
        }
        KeyCode::Char('O') if app.mode == Mode::Normal && !ctrl => {
            handle_normal_o(app, true);
        }

        // insert mode: typing — consecutive chars grouped into one undo batch
        // group breaks on: space, Enter, Esc, navigation, mode switch
        KeyCode::Char(ch) if app.mode == Mode::Insert && !ctrl => {
            delete_selection_if_any(app);

            // auto-close: if typing a closing bracket and it's already the next char, just skip over it
            if brackets::is_closing(ch) {
                let line_text = app.buffer.line_text(app.cursor.pos.line);
                let chars: Vec<char> = line_text.chars().collect();
                if app.cursor.pos.col < chars.len() && chars[app.cursor.pos.col] == ch {
                    app.cursor
                        .move_to(app.cursor.pos.line, app.cursor.pos.col + 1, false);
                    app.cursor.update_desired_col();
                    return;
                }
            }

            if ch == ' ' {
                app.buffer.undo_stack.finish_group();
            }
            let new_pos = app.buffer.insert_char(app.cursor.pos, ch);

            // auto-close: insert matching bracket
            if let Some(closing) = brackets::closing_pair(ch) {
                // for quotes, only auto-close if not already inside a pair
                let should_close = if ch == '"' || ch == '\'' || ch == '`' {
                    let line_text = app.buffer.line_text(new_pos.line);
                    let count = line_text.chars().filter(|&c| c == ch).count();
                    count % 2 != 0 // odd count means we just opened one
                } else {
                    true
                };
                if should_close {
                    app.buffer.insert_char(new_pos, closing);
                }
            }

            if ch == ' ' {
                app.buffer.undo_stack.finish_group();
            }
            app.cursor.move_to(new_pos.line, new_pos.col, false);
            app.cursor.update_desired_col();
            app.mark_edited();
        }
        KeyCode::Enter if app.mode == Mode::Insert => {
            delete_selection_if_any(app);
            app.buffer.undo_stack.finish_group();
            app.buffer.undo_stack.begin_group(app.cursor.pos);

            // smart indent: detect current line's indentation
            // check char before cursor (not end of line, due to auto-close brackets)
            let current_line = app.buffer.line_text(app.cursor.pos.line);
            let indent: String = current_line
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            let char_before_cursor = if app.cursor.pos.col > 0 {
                current_line.chars().nth(app.cursor.pos.col - 1)
            } else {
                None
            };
            let extra_indent = match char_before_cursor {
                Some('{') | Some('(') | Some('[') | Some(':') => " ".repeat(app.indent_size),
                _ => String::new(),
            };

            let new_pos = app.buffer.insert_newline(app.cursor.pos);
            let full_indent = format!("{}{}", indent, extra_indent);
            let final_pos = if !full_indent.is_empty() {
                app.buffer.insert_text(new_pos, &full_indent)
            } else {
                new_pos
            };

            app.buffer.undo_stack.finish_group();
            app.cursor.move_to(final_pos.line, final_pos.col, false);
            app.cursor.update_desired_col();
            app.mark_edited();
        }
        KeyCode::Backspace if app.mode == Mode::Insert => {
            if app.cursor.has_selection() {
                delete_selection_if_any(app);
            } else {
                app.buffer.undo_stack.begin_group(app.cursor.pos);
                if let Some(new_pos) = app.buffer.delete_char_backward(app.cursor.pos) {
                    app.cursor.move_to(new_pos.line, new_pos.col, false);
                    app.cursor.update_desired_col();
                }
                app.buffer.undo_stack.finish_group();
            }
            app.mark_edited();
        }
        KeyCode::Delete if app.mode == Mode::Insert => {
            if app.cursor.has_selection() {
                delete_selection_if_any(app);
            } else {
                app.buffer.undo_stack.begin_group(app.cursor.pos);
                app.buffer.delete_char_forward(app.cursor.pos);
                app.buffer.undo_stack.finish_group();
            }
            app.mark_edited();
        }
        KeyCode::Tab if app.mode == Mode::Insert => {
            delete_selection_if_any(app);
            app.buffer.undo_stack.begin_group(app.cursor.pos);
            let spaces = " ".repeat(app.indent_size);
            let new_pos = app.buffer.insert_text(app.cursor.pos, &spaces);
            app.buffer.undo_stack.finish_group();
            app.cursor.move_to(new_pos.line, new_pos.col, false);
            app.cursor.update_desired_col();
            app.mark_edited();
        }

        // ctrl+w / ctrl+backspace: delete word backward
        KeyCode::Char('w') if ctrl && app.mode == Mode::Insert => {
            delete_word_backward(app);
        }
        KeyCode::Backspace if ctrl && app.mode == Mode::Insert => {
            delete_word_backward(app);
        }

        _ => {}
    }
}

fn delete_selection_if_any(app: &mut App) {
    if let Some(sel) = app.cursor.selection.take() {
        let start = sel.start();
        let end = sel.end();
        app.buffer.undo_stack.begin_group(app.cursor.pos);
        app.yank_buffer = Some(app.buffer.delete_range(start, end));
        app.buffer.undo_stack.finish_group();
        app.cursor.move_to(start.line, start.col, false);
        app.cursor.update_desired_col();
    }
}

fn delete_word_backward(app: &mut App) {
    let (target_line, target_col) = find_prev_word(app);
    let start = crate::editor::cursor::Position::new(target_line, target_col);
    let end = app.cursor.pos;
    if start != end {
        app.buffer.undo_stack.begin_group(app.cursor.pos);
        app.buffer.delete_range(start, end);
        app.buffer.undo_stack.finish_group();
        app.cursor.move_to(target_line, target_col, false);
        app.cursor.update_desired_col();
        app.mark_edited();
    }
}

fn handle_normal_dd(app: &mut App) {
    // dd: delete current line, yank it
    let line = app.cursor.pos.line;
    let line_count = app.buffer.line_count();
    if line_count == 0 {
        return;
    }

    let line_text = app.buffer.line_text(line);
    app.yank_buffer = Some(format!("{}\n", line_text));

    app.buffer.undo_stack.begin_group(app.cursor.pos);
    if line_count == 1 {
        let len = app.buffer.line_len(0);
        if len > 0 {
            let start = crate::editor::cursor::Position::new(0, 0);
            let end = crate::editor::cursor::Position::new(0, len);
            app.buffer.delete_range(start, end);
        }
        app.cursor.move_to(0, 0, false);
    } else {
        let start_idx = app.buffer.rope.line_to_char(line);
        let end_idx = if line < line_count - 1 {
            app.buffer.rope.line_to_char(line + 1)
        } else {
            app.buffer.rope.len_chars()
        };
        // also remove preceding newline if deleting last line
        let actual_start = if line == line_count - 1 && line > 0 {
            start_idx - 1
        } else {
            start_idx
        };
        app.buffer.rope.remove(actual_start..end_idx);
        app.buffer.dirty = true;

        let new_line = if line >= app.buffer.line_count() {
            app.buffer.line_count().saturating_sub(1)
        } else {
            line
        };
        let new_col = app.cursor.desired_col.min(app.buffer.line_len(new_line));
        app.cursor.move_to(new_line, new_col, false);
    }
    app.buffer.undo_stack.finish_group();
}

fn handle_normal_yy(app: &mut App) {
    let line = app.cursor.pos.line;
    let line_text = app.buffer.line_text(line);
    app.yank_buffer = Some(format!("{}\n", line_text));
    tracing::debug!("yy yanked line {}", line);
}

fn handle_normal_paste(app: &mut App) {
    if let Some(text) = app.yank_buffer.clone() {
        app.buffer.undo_stack.begin_group(app.cursor.pos);
        if text.ends_with('\n') {
            // line-wise paste: insert below current line
            let line = app.cursor.pos.line;
            let insert_line = line + 1;
            let idx = if insert_line >= app.buffer.line_count() {
                let len = app.buffer.rope.len_chars();
                app.buffer.rope.insert_char(len, '\n');
                len + 1
            } else {
                app.buffer.rope.line_to_char(insert_line)
            };
            let clean = text.trim_end_matches('\n');
            app.buffer.rope.insert(idx, clean);
            app.buffer.rope.insert_char(idx + clean.len(), '\n');
            app.buffer.dirty = true;
            app.cursor.move_to(insert_line, 0, false);
        } else {
            let new_pos = app.buffer.insert_text(app.cursor.pos, &text);
            app.cursor.move_to(new_pos.line, new_pos.col, false);
        }
        app.buffer.undo_stack.finish_group();
        app.cursor.update_desired_col();
    }
}

fn handle_normal_o(app: &mut App, above: bool) {
    app.buffer.undo_stack.begin_group(app.cursor.pos);
    let line = if above {
        app.cursor.pos.line
    } else {
        app.cursor.pos.line + 1
    };

    let idx = if line >= app.buffer.line_count() {
        let len = app.buffer.rope.len_chars();
        app.buffer.rope.insert_char(len, '\n');
        app.buffer.dirty = true;
        len + 1
    } else {
        let idx = app.buffer.rope.line_to_char(line);
        app.buffer.rope.insert_char(idx, '\n');
        app.buffer.dirty = true;
        idx
    };
    let _ = idx;
    app.buffer.undo_stack.finish_group();
    app.cursor.move_to(line, 0, false);
    app.cursor.update_desired_col();
    app.mode = Mode::Insert;
}

fn find_prev_word(app: &App) -> (usize, usize) {
    let line = app.cursor.pos.line;
    let col = app.cursor.pos.col;

    if col == 0 {
        if line > 0 {
            return (line - 1, app.buffer.line_len(line - 1));
        }
        return (0, 0);
    }

    let text = app.buffer.line_text(line);
    let chars: Vec<char> = text.chars().collect();
    let mut i = col.min(chars.len());

    // skip whitespace
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    // skip word chars
    if i > 0 && chars[i - 1].is_alphanumeric() || (i > 0 && chars[i - 1] == '_') {
        while i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
            i -= 1;
        }
    } else if i > 0 {
        // skip punctuation
        while i > 0
            && !chars[i - 1].is_alphanumeric()
            && chars[i - 1] != '_'
            && !chars[i - 1].is_whitespace()
        {
            i -= 1;
        }
    }

    (line, i)
}

fn find_next_word(app: &App) -> (usize, usize) {
    let line = app.cursor.pos.line;
    let col = app.cursor.pos.col;
    let text = app.buffer.line_text(line);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    if col >= len {
        if line < app.buffer.line_count().saturating_sub(1) {
            return (line + 1, 0);
        }
        return (line, col);
    }

    let mut i = col;

    // skip current word chars
    if chars[i].is_alphanumeric() || chars[i] == '_' {
        while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
            i += 1;
        }
    } else if !chars[i].is_whitespace() {
        while i < len && !chars[i].is_alphanumeric() && chars[i] != '_' && !chars[i].is_whitespace()
        {
            i += 1;
        }
    }

    // skip whitespace
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }

    (line, i)
}

fn find_prev_paragraph(app: &App) -> usize {
    let mut line = app.cursor.pos.line;
    // skip current blank lines
    while line > 0 && app.buffer.line_len(line) == 0 {
        line -= 1;
    }
    // find previous blank line
    while line > 0 && app.buffer.line_len(line) > 0 {
        line -= 1;
    }
    line
}

fn find_next_paragraph(app: &App) -> usize {
    let max = app.buffer.line_count().saturating_sub(1);
    let mut line = app.cursor.pos.line;
    // skip current non-blank lines
    while line < max && app.buffer.line_len(line) > 0 {
        line += 1;
    }
    // skip blank lines
    while line < max && app.buffer.line_len(line) == 0 {
        line += 1;
    }
    line
}
