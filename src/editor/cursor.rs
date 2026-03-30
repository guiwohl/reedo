use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self { line: 0, col: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn new(anchor: Position, head: Position) -> Self {
        Self { anchor, head }
    }

    pub fn start(&self) -> Position {
        if self.anchor.line < self.head.line
            || (self.anchor.line == self.head.line && self.anchor.col <= self.head.col)
        {
            self.anchor
        } else {
            self.head
        }
    }

    pub fn end(&self) -> Position {
        if self.anchor.line < self.head.line
            || (self.anchor.line == self.head.line && self.anchor.col <= self.head.col)
        {
            self.head
        } else {
            self.anchor
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Cursor {
    pub pos: Position,
    pub selection: Option<Selection>,
    pub desired_col: usize,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            pos: Position::default(),
            selection: None,
            desired_col: 0,
        }
    }
}

impl Cursor {
    pub fn move_to(&mut self, line: usize, col: usize, selecting: bool) {
        if selecting {
            if self.selection.is_none() {
                self.selection = Some(Selection::new(self.pos, self.pos));
            }
        } else {
            self.selection = None;
        }

        self.pos = Position::new(line, col);

        if let Some(ref mut sel) = self.selection {
            sel.head = self.pos;
        }
    }

    pub fn update_desired_col(&mut self) {
        self.desired_col = self.pos.col;
    }

    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn select_all(&mut self, last_line: usize, last_col: usize) {
        self.selection = Some(Selection::new(
            Position::new(0, 0),
            Position::new(last_line, last_col),
        ));
        self.pos = Position::new(last_line, last_col);
    }
}
