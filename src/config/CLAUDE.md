# config/

Settings and theming system.

## Files

| File | Purpose |
|---|---|
| `settings.rs` | TOML config loading from `~/.config/kilo/kilo.conf.toml`. Creates a commented default on first run. All fields have serde defaults so partial configs work. |
| `theme.rs` | Theme struct with hex color fields. 8 bundled themes. `load_theme()` checks custom dir first, falls back to bundled. `parse_hex_color()` converts hex strings to ratatui Color. |

## Config Path

`~/.config/kilo/kilo.conf.toml` — determined by `dirs::config_dir()`.

## Theme Loading Order

1. Check `~/.config/kilo/themes/<name>.toml`
2. Fall back to bundled themes in `bundled_themes()`
3. Fall back to `Theme::default()` (kilo-dark)

## Gotchas

- The `Settings` struct is consumed during `App::new()` — individual values are extracted into App fields, the struct itself is not stored.
- Theme is stored as `app.theme` and accessed by renderers and the syntax highlighter.
- The `DEFAULT_CONFIG` constant is a hand-written TOML string with all options commented out — not auto-generated from the struct.
