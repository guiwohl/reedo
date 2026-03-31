# Architecture

## Overview

Reedo is a single-binary TUI text editor. The event loop runs in `main.rs`, delegates input to the editor module or popup handlers, and renders via ratatui.

```
main.rs (event loop + rendering)
  ├── app.rs (central state)
  ├── editor/ (text editing core)
  ├── ui/ (all visual components)
  ├── syntax/ (tree-sitter highlighting)
  ├── config/ (settings + theming)
  └── git/ (git integration)
```

## Data Flow

```
Terminal Event → crossterm
  → main.rs event_loop
    → Key event:
      → popup open? → handle_popup_input()
      → no popup   → editor::input::handle_key()
    → Mouse event → handle_mouse() (click/drag/scroll)
  → app state mutated
  → periodic checks:
    → git refresh (every 5s)
    → external file change detection (every 1s)
    → flash message expiry (2.5s)
  → ratatui renders frame
    → EditorView (text + gutter + syntax)
    → StatusBar (mode, line, git, flash notifications)
    → overlay popups (tree, fuzzy, search, etc)
  → crossterm writes to terminal
```

## Key Design Decisions

- **Single buffer** — one file at a time, no tabs/splits. The file tree remembers its state across toggles.
- **Ropey for text** — rope data structure for efficient insert/delete on large files.
- **Tree-sitter for syntax** — 17 grammars compiled in. Markdown uses a custom char-based highlighter (tree-sitter-md removed due to C assertion crashes). `catch_unwind` protects against grammar panics.
- **Popup layering** — `Popup` enum in App. Only one popup at a time. Esc closes popup first, then switches to normal mode.
- **Theme from TOML** — colors are hex strings in ThemeColors, parsed to ratatui Color at render time via `parse_hex_color()`.
- **Git via CLI** — shells out to `git status --porcelain` and `git diff --unified=0`. Refreshes every 5 seconds.

## Rendering Pipeline

1. `terminal.draw()` receives a closure
2. Layout splits into editor area + statusbar (+ optional search/replace bar)
3. `EditorView` widget renders: git gutter → line numbers → syntax-highlighted text → cursor
4. Overlay popups render on top of the editor area (centered modals, including the file tree)
5. Cursor shape set via crossterm (`SteadyBlock` for normal, `SteadyBar` for insert)
