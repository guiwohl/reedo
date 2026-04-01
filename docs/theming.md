# Theming

## Bundled Themes

Reedo ships with 9 themes:

| Theme | Vibe |
|---|---|
| `Default` | Uses terminal default foreground/background with reedo accents |
| `reedo-dark` | Default. Tokyo Night inspired, soft contrast |
| `reedo-light` | Light theme, One Light inspired |
| `catppuccin` | Catppuccin Mocha palette |
| `dracula` | Classic Dracula |
| `gruvbox` | Retro groove |
| `nord` | Arctic, muted |
| `rose-pine` | Dark, gentle pinks |
| `solarized-dark` | Solarized dark variant |

## Switching Themes

- **Ctrl+T** opens the theme switcher modal at runtime
- Or set `theme = "dracula"` in `~/.config/reedo/reedo.conf.toml`

## Custom Themes

Create a `.toml` file in `~/.config/reedo/themes/`:

```toml
name = "my-theme"

[colors]
bg = "default"
fg = "default"
gutter = "bright-black"
cursor_bg = "#c0caf5"
cursor_fg = "#1a1b26"
selection = "bright-black"
statusbar_bg = "default"
statusbar_fg = "default"
keyword = "#bb9af7"
string = "#9ece6a"
comment = "bright-black"
function = "#7daeF7"
type = "#2ac3de"
number = "#ff9e64"
operator = "#89ddff"
property = "#73bac2"
```

Values can be hex RGB strings, `default` to inherit the terminal default color, or ANSI names like `blue`, `cyan`, and `bright-black`. Every field has a default so you can override just what you want.

## How Theme Colors Map

| Color field | Used for |
|---|---|
| `bg` | Editor background. Use `default` to inherit the terminal background |
| `fg` | Default text color, fallback for unhighlighted code |
| `gutter` | Line numbers |
| `cursor_bg/fg` | Normal-mode block cursor colors |
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

The theme is applied instantly — syntax highlighting colors update in real time. The selected theme is persisted to `reedo.conf.toml` via `Settings::update_theme()`, so it survives restarts.
