use ropey::Rope;
use std::path::PathBuf;

use crate::editor::cursor::Position;
use crate::editor::undo::{Operation, UndoStack};

pub struct Buffer {
    pub rope: Rope,
    pub file_path: Option<PathBuf>,
    pub dirty: bool,
    pub undo_stack: UndoStack,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            rope: Rope::new(),
            file_path: None,
            dirty: false,
            undo_stack: UndoStack::default(),
        }
    }
}

impl Buffer {
    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(Self {
            rope: Rope::from_str(&text),
            file_path: Some(path.to_path_buf()),
            dirty: false,
            undo_stack: UndoStack::default(),
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref path) = self.file_path {
            let text = self.rope.to_string();
            std::fs::write(path, &text)?;
            self.dirty = false;
            tracing::info!("saved file: {}", path.display());
        }
        Ok(())
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.rope.len_lines() {
            return 0;
        }
        let line_slice = self.rope.line(line);
        let len = line_slice.len_chars();
        if len > 0 && line_slice.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    pub fn line_text(&self, line: usize) -> String {
        if line >= self.rope.len_lines() {
            return String::new();
        }
        let line_slice = self.rope.line(line);
        let s = line_slice.to_string();
        s.trim_end_matches('\n').to_string()
    }

    pub fn insert_char(&mut self, pos: Position, ch: char) -> Position {
        let idx = self.pos_to_char_idx(pos);
        self.rope.insert_char(idx, ch);
        self.dirty = true;

        let new_pos = Position::new(pos.line, pos.col + 1);
        self.undo_stack
            .push_op(Operation::InsertChar { pos, ch }, new_pos);
        new_pos
    }

    pub fn insert_newline(&mut self, pos: Position) -> Position {
        let idx = self.pos_to_char_idx(pos);
        self.rope.insert_char(idx, '\n');
        self.dirty = true;

        let new_pos = Position::new(pos.line + 1, 0);
        self.undo_stack
            .push_op(Operation::InsertNewline { pos }, new_pos);
        new_pos
    }

    pub fn delete_char_backward(&mut self, pos: Position) -> Option<Position> {
        if pos.col > 0 {
            let idx = self.pos_to_char_idx(pos);
            let ch = self.rope.char(idx - 1);
            self.rope.remove(idx - 1..idx);
            self.dirty = true;
            let new_pos = Position::new(pos.line, pos.col - 1);
            self.undo_stack
                .push_op(Operation::DeleteChar { pos: new_pos, ch }, new_pos);
            Some(new_pos)
        } else if pos.line > 0 {
            let prev_line_len = self.line_len(pos.line - 1);
            let idx = self.pos_to_char_idx(pos);
            self.rope.remove(idx - 1..idx);
            self.dirty = true;
            let new_pos = Position::new(pos.line - 1, prev_line_len);
            self.undo_stack
                .push_op(Operation::DeleteNewline { pos: new_pos }, new_pos);
            Some(new_pos)
        } else {
            None
        }
    }

    pub fn delete_char_forward(&mut self, pos: Position) -> bool {
        let idx = self.pos_to_char_idx(pos);
        if idx < self.rope.len_chars() {
            let ch = self.rope.char(idx);
            self.rope.remove(idx..idx + 1);
            self.dirty = true;
            if ch == '\n' {
                self.undo_stack
                    .push_op(Operation::DeleteNewline { pos }, pos);
            } else {
                self.undo_stack
                    .push_op(Operation::DeleteChar { pos, ch }, pos);
            }
            true
        } else {
            false
        }
    }

    pub fn insert_text(&mut self, pos: Position, text: &str) -> Position {
        let idx = self.pos_to_char_idx(pos);
        self.rope.insert(idx, text);
        self.dirty = true;

        let lines: Vec<&str> = text.split('\n').collect();
        let new_pos = if lines.len() == 1 {
            Position::new(pos.line, pos.col + text.len())
        } else {
            Position::new(
                pos.line + lines.len() - 1,
                lines.last().map_or(0, |l| l.len()),
            )
        };

        self.undo_stack.push_op(
            Operation::InsertText {
                pos,
                text: text.to_string(),
            },
            new_pos,
        );
        new_pos
    }

    pub fn delete_range(&mut self, start: Position, end: Position) -> String {
        let start_idx = self.pos_to_char_idx(start);
        let end_idx = self.pos_to_char_idx(end);
        let text: String = self.rope.slice(start_idx..end_idx).to_string();
        self.rope.remove(start_idx..end_idx);
        self.dirty = true;
        self.undo_stack.push_op(
            Operation::DeleteText {
                start,
                end,
                text: text.clone(),
            },
            start,
        );
        text
    }

    pub fn apply_undo(&mut self) -> Option<Position> {
        let group = self.undo_stack.undo()?.clone();
        let cursor_pos = group.cursor_before;
        for op in group.ops.iter().rev() {
            self.apply_reverse_op(op);
        }
        self.dirty = true;
        Some(cursor_pos)
    }

    pub fn apply_redo(&mut self) -> Option<Position> {
        let group = self.undo_stack.redo()?.clone();
        let cursor_pos = group.cursor_after;
        for op in group.ops.iter() {
            self.apply_forward_op(op);
        }
        self.dirty = true;
        Some(cursor_pos)
    }

    fn apply_reverse_op(&mut self, op: &Operation) {
        match op {
            Operation::InsertChar { pos, .. } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.remove(idx..idx + 1);
            }
            Operation::DeleteChar { pos, ch } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.insert_char(idx, *ch);
            }
            Operation::InsertNewline { pos } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.remove(idx..idx + 1);
            }
            Operation::DeleteNewline { pos } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.insert_char(idx, '\n');
            }
            Operation::InsertText { pos, text } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.remove(idx..idx + text.len());
            }
            Operation::DeleteText { start, text, .. } => {
                let idx = self.pos_to_char_idx(*start);
                self.rope.insert(idx, text);
            }
        }
    }

    fn apply_forward_op(&mut self, op: &Operation) {
        match op {
            Operation::InsertChar { pos, ch } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.insert_char(idx, *ch);
            }
            Operation::DeleteChar { pos, .. } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.remove(idx..idx + 1);
            }
            Operation::InsertNewline { pos } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.insert_char(idx, '\n');
            }
            Operation::DeleteNewline { pos } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.remove(idx..idx + 1);
            }
            Operation::InsertText { pos, text } => {
                let idx = self.pos_to_char_idx(*pos);
                self.rope.insert(idx, text);
            }
            Operation::DeleteText { start, end, .. } => {
                let start_idx = self.pos_to_char_idx(*start);
                let end_idx = self.pos_to_char_idx(*end);
                self.rope.remove(start_idx..end_idx);
            }
        }
    }

    fn pos_to_char_idx(&self, pos: Position) -> usize {
        if pos.line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.line_len(pos.line);
        line_start + pos.col.min(line_len)
    }
}
