use ratatui::buffer::Buffer as RatBuffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

const SECTIONS: &[(&str, &[(&str, &str)])] = &[
    ("Modes", &[
        ("i / Insert", "enter insert mode"),
        ("Esc", "normal mode / close popup"),
    ]),
    ("Navigation (both modes)", &[
        ("Arrow keys", "move cursor"),
        ("Ctrl+Left/Right", "jump words"),
        ("Ctrl+Alt+Up/Down", "jump paragraphs"),
        ("Home / End", "start / end of line"),
        ("Shift+Arrows", "select text"),
        ("Ctrl+Shift+Left/Right", "select words"),
    ]),
    ("Insert Mode", &[
        ("Any character", "type text"),
        ("Enter", "new line (smart indent)"),
        ("Tab", "insert indent (spaces)"),
        ("Backspace", "delete char backward"),
        ("Delete", "delete char forward"),
        ("Ctrl+W / Ctrl+Bksp", "delete word backward"),
    ]),
    ("Normal Mode", &[
        ("i", "enter insert mode"),
        ("dd", "delete (cut) entire line"),
        ("yy", "yank (copy) entire line"),
        ("p", "paste yanked line below"),
        ("x", "delete char under cursor"),
        ("o", "new line below + insert mode"),
        ("O", "new line above + insert mode"),
        ("?", "show this help"),
    ]),
    ("Clipboard (both modes)", &[
        ("Ctrl+C", "copy selection to clipboard"),
        ("Ctrl+X", "cut selection to clipboard"),
        ("Ctrl+V", "paste from clipboard"),
    ]),
    ("Undo / Redo", &[
        ("Ctrl+Z", "undo"),
        ("Ctrl+Y", "redo"),
    ]),
    ("Selection", &[
        ("Shift+Arrow keys", "select by char/line"),
        ("Ctrl+Shift+Arrow", "select by word"),
        ("Ctrl+A", "select all"),
    ]),
    ("Search & Replace", &[
        ("Ctrl+F", "search in current file"),
        ("  Enter / Shift+Enter", "next / prev match"),
        ("Ctrl+H", "find & replace in file"),
        ("  Tab", "switch search/replace field"),
        ("  y / n / a", "apply / skip / all"),
        ("Ctrl+Shift+F", "search across project"),
        ("Ctrl+Shift+H", "replace across project"),
    ]),
    ("Files & UI", &[
        ("Ctrl+E", "toggle file explorer"),
        ("Ctrl+P", "fuzzy file finder"),
        ("Ctrl+T", "switch theme"),
        ("Ctrl+,", "open config file"),
        ("Ctrl+S", "save file"),
        ("Ctrl+Q", "quit kilo"),
        ("F1 / ?", "show this help"),
    ]),
    ("File Explorer (Ctrl+E)", &[
        ("Up / Down", "navigate entries"),
        ("Enter / Right", "open file / expand folder"),
        ("Left", "collapse folder"),
        ("n", "create new file"),
        ("f", "create new folder"),
        ("r", "rename selected"),
        ("d", "delete selected"),
        ("m", "mark for move"),
        ("m / Enter on folder", "confirm move to folder"),
        ("Esc", "cancel move / close explorer"),
    ]),
    ("In-File Search (Ctrl+F)", &[
        ("Type text", "live search"),
        ("Enter", "jump to next match"),
        ("Shift+Enter", "jump to prev match"),
        ("Esc", "close search"),
    ]),
    ("Fuzzy Finder (Ctrl+P)", &[
        ("Type text", "filter files"),
        ("Up / Down", "navigate results"),
        ("Enter", "open selected file"),
        ("Esc", "close finder"),
    ]),
    ("Auto Behaviors", &[
        ("Auto-close", "() [] {} <> \"\" '' ``"),
        ("Auto-save", "500ms after last edit"),
        ("Smart indent", "after { ( [ :"),
        ("Overtype", "typing ) ] } skips existing"),
    ]),
];

#[derive(Debug, Clone, Default)]
pub struct KeybindHelpState {
    pub scroll: usize,
}

impl KeybindHelpState {
    pub fn reset(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self, max: usize) {
        if self.scroll < max {
            self.scroll += 1;
        }
    }

    pub fn total_lines() -> usize {
        let mut count = 0;
        for (_, binds) in SECTIONS {
            count += 1; // section header
            count += binds.len();
            count += 1; // blank line
        }
        count
    }
}

pub struct KeybindHelpWidget<'a> {
    pub state: &'a KeybindHelpState,
    pub theme: &'a crate::config::theme::Theme,
}

impl<'a> Widget for KeybindHelpWidget<'a> {
    fn render(self, area: Rect, buf: &mut RatBuffer) {
        let bg = self.theme.popup_bg();
        let border_color = self.theme.popup_border();
        let section_color = Color::Rgb(187, 154, 247);
        let key_color = Color::Rgb(249, 226, 175);
        let desc_color = Color::Rgb(166, 173, 200);

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // top border
        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        // title
        if area.height > 1 {
            let title = " Keybindings ";
            let mut x = area.x + 2;
            for ch in title.chars() {
                if x >= area.x + area.width { break; }
                buf.cell_mut((x, area.y + 1)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(
                        Style::default()
                            .fg(self.theme.popup_accent())
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    );
                });
                x += 1;
            }
        }

        // separator
        if area.height > 2 {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, area.y + 2)).map(|cell| {
                    cell.set_char('─');
                    cell.set_style(Style::default().fg(border_color).bg(bg));
                });
            }
        }

        // content
        let content_start = 3u16;
        let content_height = area.height.saturating_sub(content_start) as usize;
        let key_col_width = 22;

        let mut lines: Vec<Line> = Vec::new();
        for (section_name, binds) in SECTIONS {
            lines.push(Line::Section(section_name));
            for (key, desc) in *binds {
                lines.push(Line::Bind(key, desc));
            }
            lines.push(Line::Blank);
        }

        for i in 0..content_height {
            let line_idx = self.state.scroll + i;
            let y = area.y + content_start + i as u16;
            if y >= area.y + area.height { break; }
            if line_idx >= lines.len() { break; }

            match lines[line_idx] {
                Line::Section(name) => {
                    let display = format!("  {}", name);
                    let mut x = area.x;
                    for ch in display.chars() {
                        if x >= area.x + area.width { break; }
                        buf.cell_mut((x, y)).map(|cell| {
                            cell.set_char(ch);
                            cell.set_style(
                                Style::default()
                                    .fg(section_color)
                                    .bg(bg)
                                    .add_modifier(Modifier::BOLD),
                            );
                        });
                        x += 1;
                    }
                }
                Line::Bind(key, desc) => {
                    let key_display = format!("    {:<width$}", key, width = key_col_width);
                    let mut x = area.x;
                    for ch in key_display.chars() {
                        if x >= area.x + area.width { break; }
                        buf.cell_mut((x, y)).map(|cell| {
                            cell.set_char(ch);
                            cell.set_style(Style::default().fg(key_color).bg(bg));
                        });
                        x += 1;
                    }
                    for ch in desc.chars() {
                        if x >= area.x + area.width { break; }
                        buf.cell_mut((x, y)).map(|cell| {
                            cell.set_char(ch);
                            cell.set_style(Style::default().fg(desc_color).bg(bg));
                        });
                        x += 1;
                    }
                }
                Line::Blank => {}
            }
        }
    }
}

enum Line<'a> {
    Section(&'a str),
    Bind(&'a str, &'a str),
    Blank,
}
