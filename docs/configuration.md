# Configuration

## Location

```
~/.config/reedo/reedo.conf.toml
```

Created automatically on first run with all options commented out (defaults apply).

Open it from reedo with **Ctrl+,**.

## Options

```toml
# number of spaces per indent level
indent_size = 4

# use spaces instead of tabs
use_spaces = true

# delay in ms after last edit before autosaving (0 to disable)
autosave_delay_ms = 500

# horizontal padding (chars) between gutter and text / text and edge
# also adjustable at runtime with Ctrl+]
horizontal_padding = 4

# wrap long lines to the editor width
line_wrapping = true

# color theme name
# bundled: Default, reedo-dark, reedo-light, catppuccin, dracula, gruvbox, nord, rose-pine, solarized-dark
# custom: drop a .toml in ~/.config/reedo/themes/
theme = "reedo-dark"
```

## Directory Structure

```
~/.config/reedo/
├── reedo.conf.toml     # main config
└── themes/             # custom theme files
    └── my-theme.toml
```

## Runtime Overrides

Some settings can be changed without restarting reedo:

| Setting | How |
|---|---|
| Theme | Ctrl+T (theme switcher) — persists to config file |
| Horizontal padding | F2 or Ctrl+] (type a number, Enter) |
| Line wrapping | F3 (runtime only, does not persist) |

Other settings require restarting reedo after editing the config file.

## External File Reload

If the currently open file is modified outside reedo (e.g. by git, another editor), reedo detects the change every ~1 second and reloads automatically. If the buffer has unsaved local edits, the external change is skipped to avoid data loss. A flash notification confirms the reload.
