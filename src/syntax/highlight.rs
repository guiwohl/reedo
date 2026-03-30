use std::collections::HashMap;
use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

use crate::syntax::languages::{self, LangConfig};

#[derive(Debug, Clone, Copy)]
pub struct HighlightStyle {
    pub fg: Color,
    pub bold: bool,
}

impl HighlightStyle {
    pub fn to_ratatui_style(self) -> Style {
        let mut s = Style::default().fg(self.fg);
        if self.bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        s
    }
}

pub struct Highlighter {
    parser: Parser,
    tree: Option<Tree>,
    query: Option<Query>,
    capture_styles: Vec<HighlightStyle>,
    lang_name: String,
    // spans per line: line_idx -> vec of (start_col, end_col, style)
    pub line_styles: HashMap<usize, Vec<(usize, usize, HighlightStyle)>>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self {
            parser: Parser::new(),
            tree: None,
            query: None,
            capture_styles: Vec::new(),
            lang_name: String::new(),
            line_styles: HashMap::new(),
        }
    }
}

// default dark minimal palette (Tokyo Night inspired)
fn capture_name_to_style(name: &str) -> HighlightStyle {
    match name {
        "keyword" => HighlightStyle { fg: Color::Rgb(187, 154, 247), bold: true },  // purple
        "string" => HighlightStyle { fg: Color::Rgb(158, 206, 106), bold: false },   // green
        "number" | "constant" => HighlightStyle { fg: Color::Rgb(255, 158, 100), bold: false }, // orange
        "comment" => HighlightStyle { fg: Color::Rgb(86, 95, 137), bold: false },    // dim gray
        "function" | "function.macro" => HighlightStyle { fg: Color::Rgb(125, 174, 247), bold: false }, // blue
        "type" => HighlightStyle { fg: Color::Rgb(42, 195, 222), bold: false },      // cyan
        "property" => HighlightStyle { fg: Color::Rgb(115, 186, 194), bold: false }, // teal
        "operator" => HighlightStyle { fg: Color::Rgb(137, 221, 255), bold: false }, // light blue
        "attribute" => HighlightStyle { fg: Color::Rgb(224, 175, 104), bold: false },// yellow
        "variable.builtin" => HighlightStyle { fg: Color::Rgb(247, 118, 142), bold: false }, // red
        _ => HighlightStyle { fg: Color::Rgb(192, 202, 245), bold: false },          // default text
    }
}

impl Highlighter {
    pub fn detect_language(file_path: &Path) -> Option<LangConfig> {
        let ext = file_path.extension()?.to_str()?;
        languages::all_languages()
            .into_iter()
            .find(|l| l.extensions.contains(&ext))
    }

    pub fn set_language(&mut self, config: &LangConfig) {
        self.lang_name = config.name.to_string();
        self.parser.set_language(&config.language).ok();

        match Query::new(&config.language, config.highlight_query) {
            Ok(query) => {
                self.capture_styles = query
                    .capture_names()
                    .iter()
                    .map(|name| capture_name_to_style(name))
                    .collect();
                self.query = Some(query);
                tracing::info!("loaded language: {}", config.name);
            }
            Err(e) => {
                tracing::warn!("failed to compile query for {}: {}", config.name, e);
                self.query = None;
            }
        }
    }

    pub fn parse(&mut self, source: &str) {
        self.tree = self.parser.parse(source, self.tree.as_ref());
    }

    pub fn compute_styles(&mut self, source: &str) {
        self.line_styles.clear();

        let tree = match &self.tree {
            Some(t) => t,
            None => return,
        };
        let query = match &self.query {
            Some(q) => q,
            None => return,
        };

        let mut cursor = QueryCursor::new();
        let root = tree.root_node();
        let mut matches = cursor.matches(query, root, source.as_bytes());

        while let Some(m) = matches.next() {
            for cap in m.captures {
                let style = self.capture_styles[cap.index as usize];
                let node = cap.node;
                let start = node.start_position();
                let end = node.end_position();

                if start.row == end.row {
                    self.line_styles
                        .entry(start.row)
                        .or_default()
                        .push((start.column, end.column, style));
                } else {
                    // multi-line: first line from start to end of line
                    self.line_styles
                        .entry(start.row)
                        .or_default()
                        .push((start.column, usize::MAX, style));
                    // middle lines: full line
                    for row in (start.row + 1)..end.row {
                        self.line_styles
                            .entry(row)
                            .or_default()
                            .push((0, usize::MAX, style));
                    }
                    // last line: start of line to end col
                    if end.column > 0 {
                        self.line_styles
                            .entry(end.row)
                            .or_default()
                            .push((0, end.column, style));
                    }
                }
            }
        }
    }

    pub fn style_for(&self, line: usize, col: usize) -> Option<HighlightStyle> {
        let spans = self.line_styles.get(&line)?;
        // return the last matching span (highest priority)
        spans
            .iter()
            .rev()
            .find(|(start, end, _)| col >= *start && col < *end)
            .map(|(_, _, s)| *s)
    }

    pub fn is_active(&self) -> bool {
        self.query.is_some()
    }
}

// simple regex-free .env highlighting
pub fn env_style_for_line(line_text: &str, col: usize) -> Option<HighlightStyle> {
    let trimmed = line_text.trim_start();
    if trimmed.starts_with('#') {
        return Some(HighlightStyle {
            fg: Color::Rgb(86, 95, 137),
            bold: false,
        });
    }
    if let Some(eq_pos) = line_text.find('=') {
        if col < eq_pos {
            return Some(HighlightStyle {
                fg: Color::Rgb(125, 174, 247),
                bold: true,
            });
        } else if col == eq_pos {
            return Some(HighlightStyle {
                fg: Color::Rgb(137, 221, 255),
                bold: false,
            });
        } else {
            return Some(HighlightStyle {
                fg: Color::Rgb(158, 206, 106),
                bold: false,
            });
        }
    }
    None
}

pub fn is_env_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == ".env" || n.starts_with(".env."))
        .unwrap_or(false)
}
