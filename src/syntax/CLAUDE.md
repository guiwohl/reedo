# syntax/

Tree-sitter integration and custom highlighters.

## Files

| File | Purpose |
|---|---|
| `highlight.rs` | Highlighter engine. Manages parser, query, style computation. Also contains .env and markdown highlighters. |
| `languages.rs` | All 18 language configs (grammar + highlight query + extensions). |

## How Highlighting Works

1. On file open, `Highlighter::detect_language()` matches extension (or filename for Makefile)
2. `set_language()` sets the parser language, compiles the highlight query, maps capture names to `HighlightStyle` via theme colors
3. `parse()` runs tree-sitter on the full source text
4. `compute_styles()` walks query matches and builds a `HashMap<line, Vec<(start_col, end_col, style)>>`
5. `style_for(line, col)` returns the style for a specific character position

## Gotchas

- Queries must only use named node types like `(comment) @comment`. String literal patterns like `"fn" @keyword` break across grammar versions with "Invalid node type" errors.
- Markdown uses a completely separate path — `markdown_style_for_line()` operates on `&[char]` (not `&str`) to avoid byte-index panics on multi-byte UTF-8 characters like `—`.
- `compute_code_block_lines()` precomputes which lines are inside fenced code blocks in one O(n) pass. This is called once per frame to avoid O(n^2) in the render loop.
- The `capture_name_to_style()` function takes `ThemeColors` — syntax colors come from the active theme.
