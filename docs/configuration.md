# Configuration

## Location

```
~/.config/kilo/kilo.conf.toml
```

Created automatically on first run with all options commented out (defaults apply).

Open it from kilo with **Ctrl+,**.

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

# color theme name
# bundled: kilo-dark, kilo-light, catppuccin, dracula, gruvbox, nord, rose-pine, solarized-dark
# custom: drop a .toml in ~/.config/kilo/themes/
theme = "kilo-dark"
```

## Directory Structure

```
~/.config/kilo/
├── kilo.conf.toml      # main config
└── themes/             # custom theme files
    └── my-theme.toml
```

## Runtime Overrides

Some settings can be changed without restarting kilo:

| Setting | How |
|---|---|
| Theme | Ctrl+T (theme switcher) |
| Horizontal padding | Ctrl+] (type a number, Enter) |

Other settings require restarting kilo after editing the config file.
