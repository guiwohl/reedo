use crate::editor::cursor::Position;

#[derive(Debug, Clone)]
pub enum Operation {
    InsertChar {
        pos: Position,
        ch: char,
    },
    DeleteChar {
        pos: Position,
        ch: char,
    },
    InsertNewline {
        pos: Position,
    },
    DeleteNewline {
        pos: Position,
    },
    InsertText {
        pos: Position,
        text: String,
    },
    DeleteText {
        start: Position,
        end: Position,
        text: String,
    },
}

#[derive(Debug, Clone)]
pub struct UndoGroup {
    pub ops: Vec<Operation>,
    pub cursor_before: Position,
    pub cursor_after: Position,
}

pub struct UndoStack {
    undo: Vec<UndoGroup>,
    redo: Vec<UndoGroup>,
    current_group: Option<UndoGroup>,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            current_group: None,
        }
    }
}

impl UndoStack {
    pub fn begin_group(&mut self, cursor: Position) {
        self.finish_group();
        self.current_group = Some(UndoGroup {
            ops: Vec::new(),
            cursor_before: cursor,
            cursor_after: cursor,
        });
    }

    pub fn push_op(&mut self, op: Operation, cursor_after: Position) {
        if let Some(ref mut group) = self.current_group {
            group.ops.push(op);
            group.cursor_after = cursor_after;
        } else {
            self.current_group = Some(UndoGroup {
                ops: vec![op],
                cursor_before: cursor_after,
                cursor_after,
            });
        }
        self.redo.clear();
    }

    pub fn finish_group(&mut self) {
        if let Some(group) = self.current_group.take() {
            if !group.ops.is_empty() {
                self.undo.push(group);
            }
        }
    }

    pub fn undo(&mut self) -> Option<&UndoGroup> {
        self.finish_group();
        if let Some(group) = self.undo.pop() {
            self.redo.push(group);
            self.redo.last()
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<&UndoGroup> {
        if let Some(group) = self.redo.pop() {
            self.undo.push(group);
            self.undo.last()
        } else {
            None
        }
    }
}
