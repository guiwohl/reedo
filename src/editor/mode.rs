use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Mode {
    Normal,
    Insert,
}

impl Mode {
    pub fn label(&self) -> &str {
        match self {
            Mode::Normal => "NORMAL",
            Mode::Insert => "INSERT",
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Normal
    }
}
