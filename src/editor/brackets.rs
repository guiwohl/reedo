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
