# ui/

All visual components. Each popup/widget is a self-contained file with a state struct + a ratatui Widget impl.

## Files

| File | Purpose |
|---|---|
| `render.rs` | Main editor viewport. Renders git gutter → line numbers → syntax-highlighted text → cursor. Uses theme colors. Markdown has its own highlighting path. |
| `statusbar.rs` | Bottom bar: mode badge, line/total, git info, filename. Colors from theme. |
| `tree.rs` | File explorer popup. TreeState holds entries, selection, open dirs, move state. TreeEntry has path, icon, color, git status. Renders with nerd font icons. Root entry at index 0 = project root title. |
| `search.rs` | In-file search (Ctrl+F). Floating bar, live match highlighting, Enter/Shift+Enter navigation. |
| `replace.rs` | In-file find & replace (Ctrl+H). Two-line bar, Tab to switch fields, y/n/a for approval. |
| `search_project.rs` | Project-wide search (Ctrl+Shift+F). Modal with results list, walks files ignoring binaries. |
| `replace_project.rs` | Project-wide replace (Ctrl+Shift+H). Same as project search but with replace + one-by-one approval. |
| `fuzzy.rs` | Fuzzy file finder (Ctrl+P). Custom fuzzy scoring (not nucleo — hand-rolled). Respects .gitignore via `ignore` crate. |
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
- The padding input popup (Ctrl+]) is rendered inline in main.rs as a `Paragraph` widget, not a separate file.
