pub fn closing_pair(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '<' => Some('>'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        _ => None,
    }
}

pub fn is_closing(ch: char) -> bool {
    matches!(ch, ')' | ']' | '}' | '>' | '"' | '\'' | '`')
}

pub fn find_matching_bracket(
    buffer: &crate::editor::buffer::Buffer,
    line: usize,
    col: usize,
) -> Option<(usize, usize)> {
    let text = buffer.line_text(line);
    let chars: Vec<char> = text.chars().collect();

    let ch = if col < chars.len() {
        chars[col]
    } else if col > 0 && col - 1 < chars.len() {
        return find_matching_bracket_at(buffer, line, col - 1);
    } else {
        return None;
    };

    if matches!(ch, '(' | '[' | '{' | ')' | ']' | '}') {
        find_matching_bracket_at(buffer, line, col)
    } else if col > 0 && col - 1 < chars.len() {
        let prev = chars[col - 1];
        if matches!(prev, '(' | '[' | '{' | ')' | ']' | '}') {
            find_matching_bracket_at(buffer, line, col - 1)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn find_matching_bracket_at(
    buffer: &crate::editor::buffer::Buffer,
    line: usize,
    col: usize,
) -> Option<(usize, usize)> {
    let text = buffer.line_text(line);
    let chars: Vec<char> = text.chars().collect();
    if col >= chars.len() {
        return None;
    }
    let ch = chars[col];
    let (target, forward) = match ch {
        '(' => (')', true),
        '[' => (']', true),
        '{' => ('}', true),
        ')' => ('(', false),
        ']' => ('[', false),
        '}' => ('{', false),
        _ => return None,
    };

    let mut depth = 0i32;
    let line_count = buffer.line_count();

    if forward {
        let mut l = line;
        while l < line_count {
            let lt = buffer.line_text(l);
            let lc: Vec<char> = lt.chars().collect();
            let start = if l == line { col } else { 0 };
            for ci in start..lc.len() {
                if lc[ci] == ch {
                    depth += 1;
                } else if lc[ci] == target {
                    depth -= 1;
                    if depth == 0 {
                        return Some((l, ci));
                    }
                }
            }
            l += 1;
        }
    } else {
        let mut l = line as i64;
        while l >= 0 {
            let lt = buffer.line_text(l as usize);
            let lc: Vec<char> = lt.chars().collect();
            let start = if l as usize == line { col } else { lc.len().saturating_sub(1) };
            for ci in (0..=start).rev() {
                if ci >= lc.len() {
                    continue;
                }
                if lc[ci] == ch {
                    depth += 1;
                } else if lc[ci] == target {
                    depth -= 1;
                    if depth == 0 {
                        return Some((l as usize, ci));
                    }
                }
            }
            l -= 1;
        }
    }

    None
}
