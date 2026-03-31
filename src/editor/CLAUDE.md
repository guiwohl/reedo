# editor/

Core text editing logic. No UI rendering here — pure state manipulation.

## Files

| File | Purpose |
|---|---|
| `buffer.rs` | Ropey-backed text buffer. Insert/delete chars, lines, ranges. File I/O. Undo integration. `pos_to_char_idx()` converts (line, col) to rope char index. |
| `cursor.rs` | Cursor position + selection. `move_to()` handles both regular movement and selection extension via `selecting` param. `desired_col` tracks column for vertical movement. |
| `input.rs` | Keybind dispatch. Routes all keyboard events to the right action. Handles both modes, all ctrl combos, popup opening. The `pending_key` field on App handles multi-key sequences (dd, yy). |
| `mode.rs` | Insert/Normal mode enum. |
| `undo.rs` | Operation-based undo/redo. Groups operations (typing groups by word boundary). `begin_group` finishes any active group first. |
| `brackets.rs` | Auto-close pairs logic. `closing_pair()` returns the matching closer, `is_closing()` for overtype detection. |
| `clipboard.rs` | System clipboard via `arboard` crate. Fails silently with tracing::warn if clipboard unavailable. |

## Gotchas

- `dd`/`yy` use a pending key system — first `d` sets `app.pending_key = Some('d')`, second `d` triggers the action. Any other key clears the pending state.
- Undo groups: consecutive char inserts share one group. Space, Enter, Esc, and mode switches break the group. This means ctrl+z undoes a word at a time, not a char.
- The `handle_key` function checks `app.popup != None` is handled in main.rs before this code runs — input.rs only handles editor-level keys.
