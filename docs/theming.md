# Theming

## Bundled Themes

Kilo ships with 8 themes:

| Theme | Vibe |
|---|---|
| `kilo-dark` | Default. Tokyo Night inspired, soft contrast |
| `kilo-light` | Light theme, One Light inspired |
| `catppuccin` | Catppuccin Mocha palette |
| `dracula` | Classic Dracula |
| `gruvbox` | Retro groove |
| `nord` | Arctic, muted |
| `rose-pine` | Dark, gentle pinks |
| `solarized-dark` | Solarized dark variant |

## Switching Themes

- **Ctrl+T** opens the theme switcher modal at runtime
- Or set `theme = "dracula"` in `~/.config/kilo/kilo.conf.toml`

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

All values are hex RGB strings. Every field has a default so you can override just what you want.

## How Theme Colors Map

| Color field | Used for |
|---|---|
| `bg` | Reserved (terminal default used) |
| `fg` | Default text color, fallback for unhighlighted code |
| `gutter` | Line numbers |
| `cursor_bg/fg` | Reserved (cursor is always yellow) |
| `selection` | Selection highlight background |
| `statusbar_bg/fg` | Status bar background and text |
| `keyword` | Language keywords (`fn`, `if`, `class`, etc) |
| `string` | String literals |
| `comment` | Comments |
| `function` | Function names and calls |
| `type` | Type identifiers |
| `number` | Numeric literals and constants |
| `operator` | Operators |
| `property` | Object properties, field names |

## Theme Switcher (Ctrl+T)

Shows all bundled + custom themes. Each entry has color preview dots showing the theme's keyword/string/function/type/number/comment colors. Arrow keys to navigate, Enter to apply.

The theme is applied instantly — syntax highlighting colors update in real time.
