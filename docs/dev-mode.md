# Dev Mode & Debugging

## File Logging

```bash
REEDO_LOG=1 cargo run -- myfile.txt
# logs go to /tmp/reedo-debug.log
tail -f /tmp/reedo-debug.log  # in another terminal
```

Logs every keypress, mode change, buffer mutation, file open, syntax loading, git refresh, and errors.

## Headless Mode

Run the full editor logic without terminal rendering. Feed keypresses as JSON, get state back as JSON.

```bash
echo '{"keys":["i","h","e","l","l","o","Esc"]}' | cargo run -- --headless test.txt
```

Output:
```json
{
  "buffer": "hello",
  "cursor_line": 0,
  "cursor_col": 5,
  "mode": "NORMAL",
  "line_count": 1,
  "dirty": true
}
```

### Key format

- Single chars: `"a"`, `"1"`, `" "`
- Special keys: `"Esc"`, `"Enter"`, `"Backspace"`, `"Delete"`, `"Tab"`, `"Insert"`, `"Home"`, `"End"`, `"Up"`, `"Down"`, `"Left"`, `"Right"`
- Modifiers: `"ctrl+z"`, `"shift+Enter"`, `"ctrl+alt+Up"`

### Use cases

- Automated testing: feed keypresses → assert buffer/cursor/mode
- CI: verify keybinds don't regress
- Debugging: reproduce a bug without a terminal

## Debug vs Release

- `cargo run` — debug build, logging available, assertions enabled
- `cargo build --release` / `cargo install --path .` — optimized, no debug overhead
