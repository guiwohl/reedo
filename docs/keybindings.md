# Keybindings

## Modes

| Key | Action |
|---|---|
| `i` / `Insert` | Enter insert mode |
| `Esc` | Normal mode / close popup |

## Navigation (both modes)

| Key | Action |
|---|---|
| Arrow keys | Move cursor |
| Ctrl+Left/Right | Jump words |
| Ctrl+Alt+Up/Down | Jump paragraphs |
| PgUp / PgDn | Jump paragraphs |
| Ctrl+Home / Ctrl+End | Top / bottom of file |
| Home / End | Start / end of line |
| Shift+Arrows | Select text |
| Ctrl+Shift+Left/Right | Select words |

## Insert Mode

| Key | Action |
|---|---|
| Any character | Type text |
| Enter | New line (smart indent after `{([:`) |
| Tab | Insert indent (spaces) |
| Backspace | Delete char backward |
| Delete | Delete char forward |
| Ctrl+W / Ctrl+Backspace | Delete word backward |

## Normal Mode

| Key | Action |
|---|---|
| `i` | Enter insert mode |
| `dd` | Delete (cut) entire line |
| `yy` | Yank (copy) entire line |
| `p` | Paste yanked line below |
| `x` | Delete char under cursor |
| `o` | New line below + insert mode |
| `O` | New line above + insert mode |
| `?` | Show keybind help |

## Clipboard

| Key | Action |
|---|---|
| Ctrl+C | Copy selection to system clipboard |
| Ctrl+X | Cut selection to system clipboard |
| Ctrl+V | Paste from system clipboard |

## Undo / Redo

| Key | Action |
|---|---|
| Ctrl+Z | Undo (word-grouped) |
| Ctrl+Y | Redo |

## Selection

| Key | Action |
|---|---|
| Shift+Arrow keys | Select by char/line |
| Ctrl+Shift+Arrow | Select by word |
| Ctrl+A | Select all |

## Search & Replace

| Key | Action |
|---|---|
| Ctrl+F | Search in current file |
| Enter / Shift+Enter | Next / prev match |
| Ctrl+H | Find & replace in file |
| Tab | Switch search/replace field |
| y / n / a | Apply / skip / apply all |
| Ctrl+Shift+F | Search across project |
| Ctrl+Shift+H | Replace across project |

## Files & UI

| Key | Action |
|---|---|
| Ctrl+E | Toggle file explorer |
| Ctrl+P | Fuzzy file finder |
| Ctrl+T | Switch theme (persists to config) |
| F2 / Ctrl+] | Set horizontal padding |
| Ctrl+, | Open config file |
| Ctrl+S | Save file |
| Ctrl+Q | Quit |
| F1 / ? (normal) | Show keybind help |

## File Explorer (Ctrl+E)

| Key | Action |
|---|---|
| Up / Down | Navigate entries |
| Enter / Right | Open file / expand folder |
| Left | Collapse folder |
| n | Create new file |
| f | Create new folder |
| r | Rename selected |
| d | Delete selected |
| m | Mark for move |
| m / Enter on folder | Confirm move to folder |
| Esc | Cancel move / close explorer |

Title row = project root. Moving an item there sends it to root.

## In-File Search (Ctrl+F)

| Key | Action |
|---|---|
| Type text | Live search |
| Enter | Jump to next match |
| Shift+Enter | Jump to prev match |
| Esc | Close search |

## Fuzzy Finder (Ctrl+P)

| Key | Action |
|---|---|
| Type text | Filter files |
| Up / Down | Navigate results |
| Enter | Open selected file |
| Esc | Close finder |

## Auto Behaviors

| Behavior | Details |
|---|---|
| Auto-close | `()` `[]` `{}` `<>` `""` `''` ` `` ` |
| Auto-save | 500ms after last edit (debounced) |
| Smart indent | After `{` `(` `[` `:` |
| Overtype | Typing `)` `]` `}` skips existing closing bracket |

## Mouse

| Action | Effect |
|---|---|
| Left click (editor) | Place cursor at click position |
| Left click (file tree) | Select/open entry |
| Left drag | Select text from click to drag position |
| Scroll up/down | Scroll editor, file tree, or active popup |
