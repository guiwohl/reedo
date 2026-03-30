# kilo

A minimal terminal text editor built in Rust. Son of fresh and neovim, but simpler than both.

## Install

```bash
cargo install --path .
```

## Usage

```bash
kilo                    # open with welcome screen
kilo file.rs            # open a file
kilo src/main.rs        # open with project context
```

## Features

- **Vim-inspired modes** — Normal and Insert, no complexity
- **Tree-sitter syntax highlighting** — Rust, Python, JS, TS, HTML, CSS, C, Bash, PHP, JSON, Markdown
- **File explorer** (Ctrl+E) — tree view with nerd font icons, git indicators, folder colors, CRUD operations, move files across folders
- **Fuzzy file finder** (Ctrl+P) — type to search, instant open
- **Search & replace** — in-file (Ctrl+F / Ctrl+H) and project-wide (Ctrl+Shift+F / Ctrl+Shift+H) with one-by-one approval
- **8 bundled themes** — kilo-dark, kilo-light, catppuccin, dracula, gruvbox, nord, rose-pine, solarized-dark
- **Theme switcher** (Ctrl+T) — live preview with color dots
- **Custom themes** — drop a `.toml` in `~/.config/kilo/themes/`
- **Git integration** — branch, changed/staged/ahead/behind in statusbar, file status in tree, gutter marks for additions/modifications/deletions
- **Auto-close brackets** — `()` `[]` `{}` `<>` `""` `''` ` `` `
- **Smart indent** — auto-indent after `{`, `(`, `[`, `:`
- **Autosave** — debounced, 500ms after last edit
- **System clipboard** — Ctrl+C/V/X
- **TOML config** — `~/.config/kilo/kilo.conf.toml`

## Keybindings

Press **F1** or **?** in normal mode for the full keybind reference.

### Modes

| Key | Action |
|---|---|
| `i` / `Insert` | Enter insert mode |
| `Esc` | Normal mode / close popup |

### Navigation

| Key | Action |
|---|---|
| Arrow keys | Move cursor |
| Ctrl+Left/Right | Jump words |
| Ctrl+Alt+Up/Down | Jump paragraphs |
| Shift+Arrows | Select text |
| Home / End | Start / end of line |

### Normal Mode

| Key | Action |
|---|---|
| `dd` | Delete (cut) line |
| `yy` | Yank (copy) line |
| `p` | Paste below |
| `x` | Delete char |
| `o` / `O` | New line below / above |

### Files & UI

| Key | Action |
|---|---|
| Ctrl+E | File explorer |
| Ctrl+P | Fuzzy finder |
| Ctrl+F | Search in file |
| Ctrl+H | Find & replace |
| Ctrl+Shift+F | Project search |
| Ctrl+Shift+H | Project replace |
| Ctrl+T | Switch theme |
| Ctrl+, | Open config |
| Ctrl+S | Save |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+Q | Quit |

### File Explorer

| Key | Action |
|---|---|
| `n` | New file |
| `f` | New folder |
| `r` | Rename |
| `d` | Delete |
| `m` | Mark for move, then navigate to a folder and press Enter |

## Config

Config lives at `~/.config/kilo/kilo.conf.toml`. Created automatically on first run.

```toml
indent_size = 4
use_spaces = true
autosave_delay_ms = 500
horizontal_padding = 0
theme = "kilo-dark"
```

## Custom Themes

Create a `.toml` file in `~/.config/kilo/themes/`:

```toml
name = "my-theme"

[colors]
bg = "#1a1b26"
fg = "#c0caf5"
gutter = "#3b4261"
cursor_bg = "#c0caf5"
cursor_fg = "#1a1b26"
selection = "#283457"
statusbar_bg = "#1e1e2e"
statusbar_fg = "#a6adc8"
keyword = "#bb9af7"
string = "#9ece6a"
comment = "#565f89"
function = "#7daeF7"
type = "#2ac3de"
number = "#ff9e64"
operator = "#89ddff"
property = "#73bac2"
```

## Stack

| Crate | Purpose |
|---|---|
| crossterm | Terminal I/O |
| ratatui | TUI rendering |
| ropey | Rope-based text buffer |
| tree-sitter | Syntax highlighting |
| arboard | System clipboard |
| serde + toml | Config |
| ignore | File walking |

## License

MIT
