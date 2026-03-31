# File Explorer

Open with **Ctrl+E**. Toggle to close.

## Layout

- Title row shows `<folder> - Explorer` (the project root name)
- Selecting the title row = selecting the project root (for move operations)
- Full terminal height, ~35% width, right border
- Nerd font icons for file types and folders (open/closed)

## Navigation

| Key | Action |
|---|---|
| Up / Down | Move selection |
| Enter | Open file / expand folder |
| Right | Expand folder |
| Left | Collapse folder |
| Esc | Close explorer |

## File Operations

| Key | Action |
|---|---|
| `n` | New file — type name, Enter to confirm |
| `f` | New folder — type name, Enter to confirm |
| `r` | Rename — type new name, Enter to confirm |
| `d` | Delete — confirm with `y`, cancel with `n` |

## Moving Files

1. Select the file/folder you want to move
2. Press `m` — it gets marked with `[moving]`
3. Navigation switches to folders-only (Up/Down skip files)
4. Navigate to the destination folder (or the title row for project root)
5. Press `Enter` or `m` to confirm the move
6. Press `Esc` to cancel

## Sorting

- Folders first (alphabetical)
- Files sorted by extension, then alphabetical within same extension

## Colors

- Each folder gets a color from a cycling palette (8 colors)
- Files inherit their parent folder's color
- Icons use the same color as the filename

## Visibility

- All files shown including hidden (dotfiles)
- `.git/` directory is always hidden

## Git Indicators

Files show their git status character after the name (M, A, D, ?, etc) with color coding.

## State Persistence

The tree remembers which folders are open and the cursor position when you toggle it closed and reopen.
