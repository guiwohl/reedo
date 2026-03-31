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

fn capture_name_to_style(name: &str, tc: &crate::config::theme::ThemeColors) -> HighlightStyle {
    use crate::config::theme::parse_hex_color;
    match name {
        "keyword" => HighlightStyle { fg: parse_hex_color(&tc.keyword), bold: true },
        "string" => HighlightStyle { fg: parse_hex_color(&tc.string), bold: false },
        "number" | "constant" => HighlightStyle { fg: parse_hex_color(&tc.number), bold: false },
        "comment" => HighlightStyle { fg: parse_hex_color(&tc.comment), bold: false },
        "function" | "function.macro" => HighlightStyle { fg: parse_hex_color(&tc.function), bold: false },
        "type" => HighlightStyle { fg: parse_hex_color(&tc.r#type), bold: false },
        "property" => HighlightStyle { fg: parse_hex_color(&tc.property), bold: false },
        "operator" => HighlightStyle { fg: parse_hex_color(&tc.operator), bold: false },
        "attribute" => HighlightStyle { fg: Color::Rgb(224, 175, 104), bold: false },
        "variable.builtin" => HighlightStyle { fg: Color::Rgb(247, 118, 142), bold: false },
        "variable" => HighlightStyle { fg: parse_hex_color(&tc.fg), bold: false },
        _ => HighlightStyle { fg: parse_hex_color(&tc.fg), bold: false },
    }
}

impl Highlighter {
    pub fn detect_language(file_path: &Path) -> Option<LangConfig> {
        let filename = file_path.file_name()?.to_str()?;
        let langs = languages::all_languages();

        // match by filename first (Dockerfile, Makefile, etc)
        let name_match = match filename {
            "Makefile" | "makefile" | "GNUmakefile" => Some("makefile"),
            _ => None,
        };
        if let Some(lang_name) = name_match {
            return langs.into_iter().find(|l| l.name == lang_name);
        }

        // then by extension
        let ext = file_path.extension()?.to_str()?;
        langs.into_iter().find(|l| l.extensions.contains(&ext))
    }

    pub fn set_language(&mut self, config: &LangConfig, theme_colors: &crate::config::theme::ThemeColors) {
        self.lang_name = config.name.to_string();
        self.parser.set_language(&config.language).ok();

        match Query::new(&config.language, config.highlight_query) {
            Ok(query) => {
                self.capture_styles = query
                    .capture_names()
                    .iter()
                    .map(|name| capture_name_to_style(name, theme_colors))
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

        // catch_unwind protects against tree-sitter C assertion failures
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut styles: HashMap<usize, Vec<(usize, usize, HighlightStyle)>> = HashMap::new();
            while let Some(m) = matches.next() {
                for cap in m.captures {
                    let style_idx = cap.index as usize;
                    if style_idx >= self.capture_styles.len() { continue; }
                    let style = self.capture_styles[style_idx];
                    let node = cap.node;
                    let start = node.start_position();
                    let end = node.end_position();

                    if start.row == end.row {
                        styles.entry(start.row).or_default().push((start.column, end.column, style));
                    } else {
                        styles.entry(start.row).or_default().push((start.column, usize::MAX, style));
                        for row in (start.row + 1)..end.row {
                            styles.entry(row).or_default().push((0, usize::MAX, style));
                        }
                        if end.column > 0 {
                            styles.entry(end.row).or_default().push((0, end.column, style));
                        }
                    }
                }
            }
            styles
        }));

        match result {
            Ok(styles) => self.line_styles = styles,
            Err(_) => {
                tracing::warn!("tree-sitter panicked during highlight for {}", self.lang_name);
                self.query = None;
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

pub fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e == "md" || e == "markdown")
        .unwrap_or(false)
}

pub fn markdown_style_for_line(chars: &[char], col: usize, in_code_block: bool) -> Option<HighlightStyle> {
    let len = chars.len();
    if len == 0 { return None; }

    // find leading whitespace count
    let leading = chars.iter().take_while(|c| c.is_whitespace()).count();
    let trimmed_start = leading;

    // fenced code block delimiter
    if len >= trimmed_start + 3
        && chars[trimmed_start] == '`'
        && chars.get(trimmed_start + 1) == Some(&'`')
        && chars.get(trimmed_start + 2) == Some(&'`')
    {
        return Some(HighlightStyle { fg: Color::Rgb(166, 227, 161), bold: false });
    }

    if in_code_block {
        return Some(HighlightStyle { fg: Color::Rgb(158, 206, 106), bold: false });
    }

    // headings
    if trimmed_start < len && chars[trimmed_start] == '#' {
        let hash_count = chars[trimmed_start..].iter().take_while(|&&c| c == '#').count();
        if hash_count <= 6 && chars.get(trimmed_start + hash_count) == Some(&' ') {
            return Some(HighlightStyle { fg: Color::Rgb(187, 154, 247), bold: true });
        }
    }

    // blockquote
    if trimmed_start < len && chars[trimmed_start] == '>' {
        return Some(HighlightStyle { fg: Color::Rgb(86, 95, 137), bold: false });
    }

    // list markers
    if trimmed_start < len {
        let c = chars[trimmed_start];
        if (c == '-' || c == '*' || c == '+') && chars.get(trimmed_start + 1) == Some(&' ') {
            if col <= trimmed_start + 1 {
                return Some(HighlightStyle { fg: Color::Rgb(137, 180, 250), bold: true });
            }
        }
    }

    // inline code `...`
    let mut in_bt = false;
    let mut bt_start = 0;
    for (i, &ch) in chars.iter().enumerate() {
        if ch == '`' {
            if in_bt {
                if col >= bt_start && col <= i {
                    return Some(HighlightStyle { fg: Color::Rgb(166, 227, 161), bold: false });
                }
                in_bt = false;
            } else {
                bt_start = i;
                in_bt = true;
            }
        }
    }

    // bold **...**
    let mut i = 0;
    while i + 1 < len {
        if chars[i] == '*' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len {
                if chars[i] == '*' && chars[i + 1] == '*' {
                    if col >= start && col <= i + 1 {
                        return Some(HighlightStyle { fg: Color::Rgb(255, 158, 100), bold: true });
                    }
                    i += 2;
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    // links [text](url)
    let mut i = 0;
    while i < len {
        if chars[i] == '[' {
            let link_start = i;
            i += 1;
            // find ](
            while i + 1 < len {
                if chars[i] == ']' && chars.get(i + 1) == Some(&'(') {
                    let bracket_end = i;
                    i += 2;
                    while i < len && chars[i] != ')' { i += 1; }
                    if i < len {
                        // col in [text] part
                        if col >= link_start && col <= bracket_end {
                            return Some(HighlightStyle { fg: Color::Rgb(137, 180, 250), bold: false });
                        }
                        // col in (url) part
                        if col > bracket_end && col <= i {
                            return Some(HighlightStyle { fg: Color::Rgb(86, 95, 137), bold: false });
                        }
                        i += 1;
                    }
                    break;
                }
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    None
}

pub fn compute_code_block_lines(buffer: &crate::editor::buffer::Buffer) -> Vec<bool> {
    let line_count = buffer.line_count();
    let mut result = vec![false; line_count];
    let mut in_block = false;
    for i in 0..line_count {
        let text = buffer.line_text(i);
        if text.trim_start().starts_with("```") {
            result[i] = in_block; // the delimiter line itself uses the previous state
            in_block = !in_block;
        } else {
            result[i] = in_block;
        }
    }
    result
}
