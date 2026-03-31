# Syntax Highlighting

## Supported Languages (17 tree-sitter + custom highlighters)

| Language | Extensions | Engine |
|---|---|---|
| Rust | `.rs` | tree-sitter |
| Python | `.py`, `.pyi` | tree-sitter |
| JavaScript | `.js`, `.mjs`, `.cjs`, `.jsx` | tree-sitter |
| TypeScript | `.ts`, `.tsx` | tree-sitter |
| HTML | `.html`, `.htm` | tree-sitter |
| CSS | `.css`, `.scss` | tree-sitter |
| C | `.c`, `.h` | tree-sitter |
| Bash | `.sh`, `.bash`, `.zsh` | tree-sitter |
| PHP | `.php` | tree-sitter |
| JSON | `.json` | tree-sitter |
| Go | `.go` | tree-sitter |
| TOML | `.toml` | tree-sitter |
| YAML | `.yaml`, `.yml` | tree-sitter |
| Lua | `.lua` | tree-sitter |
| Ruby | `.rb`, `.rake`, `.gemspec` | tree-sitter |
| Zig | `.zig` | tree-sitter |
| Makefile | `Makefile`, `makefile`, `.mk` | tree-sitter |
| Markdown | `.md`, `.markdown` | custom (char-based) |

`tree-sitter-md` was removed due to C assertion crashes. Markdown now uses a fully custom char-based highlighter.

## Special Files

| File | Highlighting |
|---|---|
| `.env`, `.env.*` | Custom (KEY=value pattern, comments) |
| `Makefile` / `GNUmakefile` | Detected by filename |

## How It Works

- **Tree-sitter**: grammars are compiled into the binary. On file open, the extension is matched to a grammar. The parser produces an AST, then highlight queries map node types to capture names (`@keyword`, `@string`, `@comment`, etc). Capture names map to theme colors.

- **Markdown**: uses a custom char-based highlighter instead of tree-sitter. Handles headings, code blocks, inline code, bold, links, blockquotes, lists, horizontal rules. Code block state is precomputed per frame to avoid O(n^2).

- **Reparsing**: after every edit, the buffer is reparsed on the next render frame. Tree-sitter's incremental parsing makes this fast.

- **Safety**: `compute_styles()` wraps tree-sitter query iteration in `catch_unwind` to protect against C assertion failures in grammar code. On panic, highlighting silently degrades to plain text for that parse.

## Adding a Language

To add a new statically compiled language:

1. Add the grammar crate to `Cargo.toml`
2. Add a `LangConfig` entry in `src/syntax/languages.rs`
3. Write a highlight query using only named node types (not string literal patterns)
4. If the file is detected by filename (not extension), add a match in `Highlighter::detect_language()` in `src/syntax/highlight.rs`
