# Kilo — AI Instructions

## Core Beliefs

- **Planning First** — Think before you code. Understand the problem, explore existing code, then implement.
- **Incremental progress** — Small changes that compile and run. Never a big bang.
- **Hierarchical scoped context** — Create CLAUDE.md files in subdirectories when hard-won knowledge is discovered (quirks, gotchas, non-obvious decisions). If it's obvious from reading the code, skip it.
- **Clean Code** — No comments in code. Code reads like prose. Single Responsibility Principle, applied pragmatically.
- **Simplicity** — Boring and obvious beats clever. The right abstraction is the one you actually need, not the one you might need.
- **Concise communication** — Sacrifice grammar for clarity. Use tables/diagrams when they help. English in the simplest/most didactic way.

## Stack

| Component | Crate | Purpose |
|---|---|---|
| Terminal I/O | `crossterm` | Raw mode, keyboard events, terminal ops |
| TUI rendering | `ratatui` | Widget-based frame rendering |
| Text buffer | `ropey` | Rope data structure for efficient text editing |
| Syntax | `tree-sitter` + grammars | AST-based syntax highlighting |
| Clipboard | `arboard` | System clipboard access |
| Config | `serde` + `toml` | TOML config parsing |
| Git | `git2` or `gix` | Git status, branch info |
| Fuzzy search | `nucleo` | Fuzzy file finder |
| File search | `ignore` | Walk dirs, skip binaries |
| Paths | `dirs` | XDG dirs (~/.config/kilo/) |
| Logging | `tracing` + `tracing-appender` | File-based logging for dev mode |

## Project Structure

```
kilo/
├── Cargo.toml
├── CLAUDE.md
├── mvp.md                    # Original spec
├── src/
│   ├── main.rs               # Entry, event loop
│   ├── app.rs                # App state, mode mgmt
│   ├── editor/               # Core editing logic
│   │   ├── buffer.rs         # Rope-backed text buffer
│   │   ├── cursor.rs         # Cursor + selection
│   │   ├── input.rs          # Keybind dispatch
│   │   ├── mode.rs           # Insert/Normal modes
│   │   ├── undo.rs           # Undo/redo stack
│   │   ├── autosave.rs       # Debounced 500ms autosave
│   │   ├── clipboard.rs      # System clipboard ops
│   │   └── brackets.rs       # Auto-close pairs
│   ├── syntax/               # Tree-sitter integration
│   ├── ui/                   # All UI components
│   │   ├── render.rs         # Main viewport
│   │   ├── statusbar.rs      # Bottom bar
│   │   ├── tree.rs           # File tree popup
│   │   ├── search.rs         # In-file search
│   │   ├── search_project.rs # Project-wide search
│   │   ├── replace.rs        # In-file replace
│   │   ├── replace_project.rs# Project replace
│   │   ├── fuzzy.rs          # ctrl+p fuzzy finder
│   │   └── welcome.rs        # ASCII welcome screen
│   ├── config/               # Settings + theming
│   └── git/                  # Git integration
```

## Dev Mode & Debugging Strategy

Since this is a TUI and we can't visually interact with it during automated development, we use a **dev mode** with these capabilities:

### 1. File-based logging (`KILO_LOG=1`)
- Uses `tracing` with `tracing-appender` writing to `/tmp/kilo-debug.log`
- Every keypress, mode change, buffer mutation, render cycle gets logged
- Enabled via `KILO_LOG=1 cargo run` or `--dev` flag
- In release builds, logging is compiled out (zero overhead)

### 2. Headless test mode (`--headless`)
- Runs the full app logic WITHOUT terminal rendering
- Accepts a script of simulated keypresses (from stdin or a file)
- Outputs final buffer state, cursor position, and mode to stdout as JSON
- Used for automated integration testing: feed keystrokes → assert buffer state
- Example: `echo '{"keys":["i","h","e","l","l","o","Esc"]}' | cargo run -- --headless test.txt`

### 3. Frame dump (`--dump-frames`)
- In dev mode, each rendered frame can be dumped as plain text to a file
- Lets us verify UI layout without seeing the actual terminal
- Captures: line numbers, text content, statusbar, any open popups

### 4. Integration test harness
- Tests in `tests/` that spin up the app in headless mode
- Feed keypresses programmatically → assert on buffer, cursor, mode, file state
- Covers: all keybinds, auto-brackets, undo/redo, search/replace, tree ops

### How to debug during development:
```bash
# Run with logging
KILO_LOG=1 cargo run -- myfile.txt 2>/dev/null
# Then in another terminal:
tail -f /tmp/kilo-debug.log

# Run headless test
echo '{"keys":["i","t","e","s","t","Esc",":","w"]}' | cargo run -- --headless test.txt
# Check output JSON for buffer state

# Dump frames
KILO_LOG=1 cargo run -- --dump-frames /tmp/frames/ myfile.txt
```

### Dev-only code convention:
- All dev-mode code gated behind `#[cfg(debug_assertions)]` or `--dev` flag
- Release build (`cargo build --release`) strips all debug tooling automatically
- No performance penalty in shipped binary

## Testing

- `cargo test` for unit tests
- `cargo test --test integration` for headless integration tests
- Always test after implementing a feature. Feed keypresses → assert results.

## Git Usage

- All feature branches from `main`
- Conventional commits, descriptive messages
- No Claude Code references in commits
- Separate commits when logical
- Never merge directly to main — PRs only
- Always rebase, never merge commits

## Key Design Decisions

See `mvp.md` for full spec. Critical non-obvious decisions:
- Single buffer only, but tree remembers open folders + cursor position
- Esc = close popup first, THEN normal mode (layered)
- Normal mode: dd/yy/p/x/o/O only. No vim motions.
- Insert mode via `i` or `Insert` key
- Search ignores binaries but does NOT respect .gitignore
- Folder colors auto-cycle from palette, files inherit parent color
- Status bar left-aligned: `[NORMAL]  148/520  main +3 ~2 ↑1 ↓0`
