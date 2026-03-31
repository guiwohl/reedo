# Kilo — AI Instructions

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
| `main.rs` | Entry point, event loop, popup rendering + input routing | — |
| `app.rs` | Central App state struct, Popup enum, autosave, git refresh | — |
| `editor/` | Core text editing: buffer, cursor, keybinds, undo, brackets, clipboard | [src/editor/CLAUDE.md](src/editor/CLAUDE.md) |
| `ui/` | All UI widgets: editor view, statusbar, tree, search, replace, fuzzy, themes, help, welcome | [src/ui/CLAUDE.md](src/ui/CLAUDE.md) |
| `syntax/` | Tree-sitter highlighting (18 languages) + custom markdown/env highlighters | [src/syntax/CLAUDE.md](src/syntax/CLAUDE.md) |
| `config/` | TOML settings + theme system (8 bundled + custom) | [src/config/CLAUDE.md](src/config/CLAUDE.md) |
| `git/` | Git status, gutter marks, statusbar info via CLI | [src/git/CLAUDE.md](src/git/CLAUDE.md) |

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
| `tree-sitter` + 17 grammar crates | AST-based syntax highlighting |
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
- Status bar left-aligned: `[NORMAL]  42/520  main ~3 +1 ↑2 ↓0`
- Cursor always yellow (block in normal, bar in insert)
- Git refreshes every 5 seconds via CLI commands
- Markdown uses custom char-based highlighter (not tree-sitter)
- Tree-sitter queries use only named node types (no string literal patterns)
