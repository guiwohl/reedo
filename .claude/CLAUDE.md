# Reedo — AI Instructions

## Core Beliefs

- **Planning First** — Think before you code. Understand the problem, explore existing code, then implement.
- **Incremental progress** — Small changes that compile and run. Never a big bang.
- **Hierarchical scoped context** — Create CLAUDE.md files in subdirectories when hard-won knowledge is discovered (quirks, gotchas, non-obvious decisions). If it's obvious from reading the code, skip it.
- **Clean Code** — No comments in code. Code reads like prose. Single Responsibility Principle, applied pragmatically.
- **Simplicity** — Boring and obvious beats clever. The right abstraction is the one you actually need, not the one you might need.
- **Concise communication** — Sacrifice grammar for clarity. Use tables/diagrams when they help.

## Project Map

### Source (`src/`)

| Path | Purpose | CLAUDE.md |
|---|---|---|
| `main.rs` | Entry point, event loop, popup rendering + input routing, mouse handling, SidePanelTree/RecentFilesWidget/MarkdownOutlineWidget, extract_markdown_headings() | — |
| `app.rs` | Central App state struct, Popup enum (incl. RecentFiles), SidePanelMode (FileTree/MarkdownOutline), autosave, git refresh, external file change detection, flash notifications, recent_files tracking, relative_line_numbers/show_whitespace toggles | — |
| `editor/` | Core text editing: buffer, cursor, keybinds, undo, brackets, clipboard | [src/editor/CLAUDE.md](src/editor/CLAUDE.md) |
| `ui/` | All UI widgets: editor view (cursorline, indent guides, relative line numbers, minimap, sticky scroll, whitespace viz), statusbar, tree (guide lines, file sizes, reveal), search, replace, fuzzy (file preview), themes, help, welcome | [src/ui/CLAUDE.md](src/ui/CLAUDE.md) |
| `syntax/` | Tree-sitter highlighting (17 grammars) + custom markdown/env highlighters | [src/syntax/CLAUDE.md](src/syntax/CLAUDE.md) |
| `config/` | TOML settings + theme system (9 bundled + custom), cursorline color | [src/config/CLAUDE.md](src/config/CLAUDE.md) |
| `git/` | Git status, gutter marks (▎ add/modify, ▁ delete), statusbar info via CLI | [src/git/CLAUDE.md](src/git/CLAUDE.md) |

### Docs (`docs/`)

| File | Content |
|---|---|
| [architecture.md](docs/architecture.md) | System overview, data flow, rendering pipeline, key design decisions |
| [keybindings.md](docs/keybindings.md) | Complete keybinding reference for all modes and popups |
| [theming.md](docs/theming.md) | Bundled themes, custom theme creation, color field mapping |
| [configuration.md](docs/configuration.md) | Config file location, all options, runtime overrides |
| [syntax-highlighting.md](docs/syntax-highlighting.md) | Supported languages, how highlighting works, adding languages |
| [git-integration.md](docs/git-integration.md) | Statusbar, tree indicators, gutter marks, refresh cycle |
| [file-explorer.md](docs/file-explorer.md) | Navigation, CRUD operations, move flow, sorting, colors |
| [dev-mode.md](docs/dev-mode.md) | Logging, headless mode, key format, debug vs release |

### Root Files

| File | Purpose |
|---|---|
| `mvp.md` | Original MVP spec and design decisions |
| `README.md` | User-facing documentation |
| `Cargo.toml` | Dependencies and package metadata |

## Stack

| Crate | Purpose |
|---|---|
| `crossterm` | Terminal I/O, raw mode, keyboard events |
| `ratatui` | TUI widget rendering |
| `ropey` | Rope data structure for text buffer |
| `tree-sitter` + 17 grammar crates | AST-based syntax highlighting (no tree-sitter-md) |
| `arboard` | System clipboard |
| `serde` + `toml` | Config/theme parsing |
| `ignore` | File walking (respects .gitignore) |
| `dirs` | XDG paths |
| `tracing` + `tracing-appender` | File-based debug logging |
| `clap` | CLI argument parsing |

## Git Usage

- All feature branches from `main`
- Conventional commits, descriptive messages
- No Claude Code references in commits
- Separate commits when logical
- Never merge directly to main — PRs only
- Always rebase, never merge commits

## Key Design Decisions

See `mvp.md` for the original spec. Critical non-obvious decisions:

- Single buffer only, tree remembers open folders + cursor position
- Esc = close popup first, THEN normal mode (layered)
- Normal mode: dd/yy/p/x/o/O only. No vim motions.
- Insert mode via `i` or `Insert` key
- Search ignores binaries, does NOT respect .gitignore
- Folder colors auto-cycle from palette, files inherit parent color
- Status bar: `[NORMAL] line:col /total │ git │ relative/path.rs`
- Cursor always yellow (block in normal, bar in insert)
- Git refreshes every 5 seconds via CLI commands
- Mouse support: click to place cursor/select tree entries, drag to select text, scroll wheel for navigation
- Flash notifications: transient statusbar messages for save, reload, theme switch (2.5s)
- External file reload: detects out-of-process file changes every ~1s, reloads if buffer is clean
- Markdown uses custom char-based highlighter (tree-sitter-md removed — C assertion crashes)
- Tree-sitter queries use only named node types (no string literal patterns)
- Cursorline: subtle bg highlight on cursor's line, controlled by `cursorline` theme color
- Scrolloff = 5: cursor never hits viewport edge
- Indent guides: subtle │ at each indent level inside leading whitespace
- Relative line numbers: toggle with F5, shows distance from cursor
- Whitespace visualization: toggle with F6, · for spaces, → for tabs
- Minimap scrollbar: 1-char right edge showing viewport position + git change markers
- Sticky scroll: shows current fn/heading at top when scrolled past definition
- Tree guide lines: ├── └── │ for visual hierarchy
- File size in tree: right-aligned dim sizes on every file
- Side panel modes: FileTree (default) and MarkdownOutline (Ctrl+M toggle)
- Recent files: Ctrl+R popup, last 20 files, session-only
- Auto-reveal: opening a file auto-expands tree and scrolls to it
- Fuzzy file preview: split pane showing first N lines of selected file
- Gutter marks: ▎ for added/modified, ▁ for deleted lines
- y/Y in tree: yank relative/full path to clipboard
