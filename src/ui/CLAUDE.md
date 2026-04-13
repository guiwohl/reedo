# ui/

All visual components. Each popup/widget is a self-contained file with a state struct + a ratatui Widget impl.

## Files

| File | Purpose |
|---|---|
| `render.rs` | Main editor viewport. Renders git gutter (▎ add/modify, ▁ deleted) → line numbers (with relative line number support) → syntax-highlighted text → cursor. Cursorline highlight, active line number bold+bright, indent guides (│), markdown code block bg tinting, whitespace visualization (· spaces, → tabs), minimap scrollbar (1-char right edge with git marks), sticky scroll breadcrumbs via `find_breadcrumb()`. |
| `statusbar.rs` | Bottom bar: mode badge, line:col /total, relative file path, git info, flash notification rendering (right-aligned, 2.5s fadeout). Colors from theme. |
| `tree.rs` | File explorer side panel. TreeState holds entries, selection, open dirs, move state. TreeEntry has path, icon, color, git status, is_last_sibling, file_size. Renders with nerd font icons and tree guide lines (├── └── │) via `tree_guide_prefix()`. `format_file_size()` for human-readable sizes. `reveal_path()` auto-expands tree to a given file. Root entry at index 0 = project root title. |
| `search.rs` | In-file search (Ctrl+F). Floating bar, live match highlighting, Enter/Shift+Enter navigation. |
| `replace.rs` | In-file find & replace (Ctrl+H). Two-line bar, Tab to switch fields, y/n/a for approval. |
| `search_project.rs` | Project-wide search (Ctrl+Shift+F). Modal with results list, walks files ignoring binaries. |
| `fuzzy.rs` | Fuzzy file finder (Ctrl+P). Custom fuzzy scoring (hand-rolled). Respects .gitignore via `ignore` crate. File preview pane on right side showing first N lines of selected file. `project_root` field for resolving preview paths. |
| `theme_switcher.rs` | Theme picker modal (Ctrl+T). Lists bundled + custom themes with color preview dots. |
| `keybind_help.rs` | Keybind reference modal (F1 / ?). Scrollable, organized by section. |
| `welcome.rs` | ASCII art welcome screen shown when no file is open. |

## Widget Pattern

Every popup follows the same pattern:
1. `FooState` struct with data + input methods (insert_char, delete_char, move_up, etc)
2. `FooWidget<'a>` struct with `&'a FooState` reference
3. `impl Widget for FooWidget` — custom render using ratatui Buffer API
4. Popup enum variant in `app.rs`
5. State field in App struct, initialized with `::default()`
6. Render arm in `main.rs` terminal.draw closure
7. Input arm in `handle_popup_input()` in main.rs

## Gotchas

- Tree root entry is at index 0. The render skips it and uses it as the title row. `scroll_offset + i + 1` to map visible row to entry index.
- Markdown highlighting is in `syntax/highlight.rs`, not here — but the render.rs code has a special `is_md` branch that calls it.
- Mouse events (click, drag, scroll) are handled by `handle_mouse()` in main.rs — routes to editor cursor, tree selection, or popup scroll depending on context.
- SidePanelTree in main.rs now has an `open_file` field for highlighting the current file in the tree.
- y/Y keybinds in the tree yank file/directory paths.
- `replace_project.rs` was removed — no more ProjectReplace popup.
- PaddingInput popup was removed.
