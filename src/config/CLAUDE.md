# config/

Settings and theming system.

## Files

| File | Purpose |
|---|---|
| `settings.rs` | TOML config loading from `~/.config/reedo/reedo.conf.toml`. Creates a commented default on first run. All fields have serde defaults so partial configs work. |
| `theme.rs` | Theme struct with string color fields including `cursorline`. 9 bundled themes including `Default`. `load_theme()` checks custom dir first, falls back to bundled. `parse_theme_color()` converts hex strings, terminal defaults, and ANSI names to ratatui Color. All 9 bundled themes define a cursorline value. |

## Config Path

`~/.config/reedo/reedo.conf.toml` — determined by `dirs::config_dir()`.

## Theme Loading Order

1. Check `~/.config/reedo/themes/<name>.toml`
2. Fall back to bundled themes in `bundled_themes()`
3. Fall back to `Theme::default()` (reedo-dark)

## Theme Color Fields

ThemeColors includes `cursorline` — used for the current line highlight background in the editor viewport. Every bundled theme must define this.

## Gotchas

- The `Settings` struct is consumed during `App::new()` — individual values are extracted into App fields, the struct itself is not stored.
- Theme is stored as `app.theme` and accessed by renderers and the syntax highlighter.
- The `DEFAULT_CONFIG` constant is a hand-written TOML string with all options commented out — not auto-generated from the struct.
- `Settings::update_theme(name)` writes the selected theme back to `reedo.conf.toml` — called by the theme switcher (Ctrl+T) so the choice persists across restarts. It replaces the existing `theme = ` line or appends one.
