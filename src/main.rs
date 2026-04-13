mod app;
mod config;
mod editor;
mod git;
mod syntax;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::{CommandFactory, Parser, ValueHint};
use clap_complete::Shell;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::MouseEventKind;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Terminal;

use app::{App, Popup};
use config::settings::Settings;
use editor::clipboard;
use ui::fuzzy::FuzzyFinderWidget;
use ui::keybind_help::KeybindHelpWidget;
use ui::render::EditorView;
use ui::replace::ReplaceBar;
use ui::search::SearchBar;
use ui::search_project::ProjectSearchWidget;
use ui::statusbar::StatusBar;
use ui::theme_switcher::ThemeSwitcherWidget;
use ui::tree::FileTreeWidget;
use ui::welcome::WelcomeScreen;

const SIDE_PANEL_WIDTH: u16 = 32;

#[derive(Parser)]
#[command(name = "reedo", about = "A minimal terminal text editor")]
struct Cli {
    #[arg(value_hint = ValueHint::AnyPath)]
    file: Option<String>,

    #[arg(long, value_enum)]
    generate_completion: Option<Shell>,

    #[arg(long, hide = true)]
    headless: bool,

    #[arg(long, hide = true)]
    dump_frames: Option<String>,
}

fn popup_overlay_area(full_area: Rect) -> Rect {
    Rect::new(
        full_area.x,
        full_area.y,
        full_area.width,
        full_area.height.saturating_sub(1),
    )
}

fn centered_popup_area(
    layer: Rect,
    width_pct: u16,
    height_pct: u16,
    min_width: u16,
    min_height: u16,
) -> Rect {
    let popup_width = (layer.width.saturating_mul(width_pct) / 100)
        .max(min_width)
        .min(layer.width.saturating_sub(2).max(1));
    let popup_height = (layer.height.saturating_mul(height_pct) / 100)
        .max(min_height)
        .min(layer.height.saturating_sub(2).max(1));
    let x = layer.x + layer.width.saturating_sub(popup_width) / 2;
    let y = layer.y + layer.height.saturating_sub(popup_height) / 2;
    Rect::new(x, y, popup_width, popup_height)
}

fn file_tree_popup_area(full_area: Rect) -> Rect {
    centered_popup_area(popup_overlay_area(full_area), 32, 70, 50, 12)
}

fn init_logging() {
    if std::env::var("REEDO_LOG").is_ok() {
        let file_appender = tracing_appender::rolling::never("/tmp", "reedo-debug.log");
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_env_filter("reedo=debug")
            .with_ansi(false)
            .init();
        tracing::info!("reedo logging initialized");
    }
}

fn main() -> io::Result<()> {
    init_logging();
    let cli = Cli::parse();

    if let Some(shell) = cli.generate_completion {
        return generate_completion(shell);
    }

    let settings = Settings::load();

    if cli.headless {
        return run_headless(cli, settings);
    }

    run_tui(cli, settings)
}

fn generate_completion(shell: Shell) -> io::Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, bin_name, &mut io::stdout());
    Ok(())
}

fn run_tui(cli: Cli, settings: Settings) -> io::Result<()> {
    let mut app = App::new(settings);

    if let Some(ref file_path) = cli.file {
        let path = std::path::Path::new(file_path);
        if path.exists() {
            app.open_file(path)?;
            if let Some(parent) = path.parent() {
                let root = if parent.to_string_lossy().is_empty() {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                } else {
                    parent.to_path_buf()
                };
                app.set_project_root(root);
            }
        } else {
            app.buffer.file_path = Some(path.to_path_buf());
        }
    }

    if app.project_root.is_none() {
        app.set_project_root(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    }

    // build tree on startup if side panel is persisted open
    if app.side_panel_open {
        if let Some(root) = app.project_root.clone() {
            app.tree_state.build(&root);
            if let Some(ref git) = app.git_info {
                app.tree_state.apply_git_statuses(git);
            }
        }
    }

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        let size = terminal.size()?;
        app.viewport_height = size.height as usize;
        app.viewport_width = size.width as usize;
        app.scroll_to_cursor();
        app.reparse_if_needed();

        terminal.draw(|frame| {
            let full_area = frame.area();

            let panel_width = if app.side_panel_open {
                SIDE_PANEL_WIDTH.min(full_area.width / 3)
            } else {
                0
            };

            let h_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(if panel_width > 0 {
                    vec![Constraint::Length(panel_width), Constraint::Min(1)]
                } else {
                    vec![Constraint::Min(1)]
                })
                .split(full_area);

            let main_area = if panel_width > 0 {
                h_chunks[1]
            } else {
                h_chunks[0]
            };
            let panel_area = if panel_width > 0 {
                Some(h_chunks[0])
            } else {
                None
            };

            if let Some(panel) = panel_area {
                match app.side_panel_mode {
                    app::SidePanelMode::FileTree => {
                        let is_focused = app.popup == Popup::FileTree;
                        frame.render_widget(
                            SidePanelTree {
                                state: &app.tree_state,
                                theme: &app.theme,
                                focused: is_focused,
                                open_file: app.buffer.file_path.as_deref(),
                            },
                            panel,
                        );
                    }
                    app::SidePanelMode::MarkdownOutline => {
                        let headings = extract_markdown_headings(app);
                        frame.render_widget(
                            MarkdownOutlineWidget {
                                headings: &headings,
                                selected: 0,
                                theme: &app.theme,
                            },
                            panel,
                        );
                    }
                }
            }

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(match app.popup {
                    Popup::Search => vec![
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ],
                    Popup::Replace => vec![
                        Constraint::Min(1),
                        Constraint::Length(2),
                        Constraint::Length(1),
                    ],
                    Popup::GotoLine => vec![
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ],
                    _ => vec![Constraint::Min(1), Constraint::Length(1)],
                })
                .split(main_area);

            if app.buffer.file_path.is_none()
                && !app.buffer.dirty
                && app.buffer.rope.len_chars() == 0
            {
                frame.render_widget(WelcomeScreen { theme: &app.theme }, main_chunks[0]);
            } else {
                frame.render_widget(EditorView { app }, main_chunks[0]);
            }

            match app.popup {
                Popup::Search => {
                    frame.render_widget(
                        SearchBar {
                            state: &app.search_state,
                            theme: &app.theme,
                        },
                        main_chunks[1],
                    );
                    frame.render_widget(StatusBar { app }, main_chunks[2]);
                }
                Popup::Replace => {
                    frame.render_widget(
                        ReplaceBar {
                            state: &app.replace_state,
                            theme: &app.theme,
                        },
                        main_chunks[1],
                    );
                    frame.render_widget(StatusBar { app }, main_chunks[2]);
                }
                Popup::GotoLine => {
                    let bar_area = main_chunks[1];
                    let bg = app.theme.statusbar_bg();
                    let fg = app.theme.statusbar_fg();
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(format!(
                            " Go to line: {}█",
                            app.goto_line_input
                        ))
                        .style(ratatui::style::Style::default().fg(fg).bg(bg)),
                        bar_area,
                    );
                    frame.render_widget(StatusBar { app }, main_chunks[2]);
                }
                _ => {
                    frame.render_widget(StatusBar { app }, main_chunks[1]);
                }
            }

            // overlay popups
            match app.popup {
                Popup::FileTree if !app.side_panel_open => {
                    let tree_area = file_tree_popup_area(full_area);
                    frame.render_widget(
                        FileTreeWidget {
                            state: &app.tree_state,
                            theme: &app.theme,
                        },
                        tree_area,
                    );
                }
                Popup::FuzzyFinder => {
                    let popup_width = (full_area.width * 60 / 100).max(40);
                    let popup_height = (full_area.height * 60 / 100).max(10);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        FuzzyFinderWidget {
                            state: &app.fuzzy_state,
                            theme: &app.theme,
                            project_root: app.project_root.as_deref(),
                        },
                        popup_area,
                    );
                }
                Popup::SearchProject => {
                    let popup_width = (full_area.width * 70 / 100).max(50);
                    let popup_height = (full_area.height * 70 / 100).max(15);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        ProjectSearchWidget {
                            state: &app.project_search_state,
                            theme: &app.theme,
                        },
                        popup_area,
                    );
                }
                Popup::ThemeSwitcher => {
                    let popup_width = (full_area.width * 40 / 100).max(35);
                    let popup_height = (full_area.height * 50 / 100).max(10);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        ThemeSwitcherWidget {
                            state: &app.theme_switcher_state,
                            theme: &app.theme,
                        },
                        popup_area,
                    );
                }
                Popup::RecentFiles => {
                    let popup_width = (full_area.width * 50 / 100).max(40);
                    let popup_height = (full_area.height * 40 / 100).max(8);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        RecentFilesWidget {
                            recent: &app.recent_files,
                            selected: app.fuzzy_state.selected,
                            scroll: app.fuzzy_state.scroll_offset,
                            theme: &app.theme,
                        },
                        popup_area,
                    );
                }
                Popup::KeybindHelp => {
                    let popup_width = (full_area.width * 55 / 100).max(50);
                    let popup_height = (full_area.height * 75 / 100).max(20);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        KeybindHelpWidget {
                            state: &app.keybind_help_state,
                            theme: &app.theme,
                        },
                        popup_area,
                    );
                }
                _ => {}
            }
        })?;

        terminal.hide_cursor()?;

        match app.mode {
            crate::editor::mode::Mode::Normal => {
                execute!(terminal.backend_mut(), SetCursorStyle::SteadyBlock)?;
            }
            crate::editor::mode::Mode::Insert => {
                execute!(terminal.backend_mut(), SetCursorStyle::SteadyBar)?;
            }
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.code == KeyCode::Char('q')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        app.running = false;
                    } else if app.popup != Popup::None {
                        handle_popup_input(app, key);
                    } else {
                        editor::input::handle_key(app, key);
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(app, mouse);
                }
                Event::Resize(w, h) => {
                    app.viewport_width = w as usize;
                    app.viewport_height = h as usize;
                }
                _ => {}
            }
        }

        app.check_autosave();
        app.check_git_refresh();
        app.check_external_changes();

        if !app.running {
            break;
        }
    }

    Ok(())
}

fn extract_markdown_headings(app: &App) -> Vec<(usize, usize, String)> {
    let mut headings = Vec::new();
    let is_md = app
        .buffer
        .file_path
        .as_ref()
        .map(|p| syntax::highlight::is_markdown_file(p))
        .unwrap_or(false);
    if !is_md {
        return headings;
    }
    for i in 0..app.buffer.line_count() {
        let text = app.buffer.line_text(i);
        let trimmed = text.trim_start();
        if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            if level <= 6 {
                let heading_text = trimmed[level..].trim().to_string();
                if !heading_text.is_empty() {
                    headings.push((i, level, heading_text));
                }
            }
        }
    }
    headings
}

struct RecentFilesWidget<'a> {
    recent: &'a [PathBuf],
    selected: usize,
    scroll: usize,
    theme: &'a config::theme::Theme,
}

impl<'a> ratatui::widgets::Widget for RecentFilesWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        use ratatui::style::{Modifier, Style};

        let bg = self.theme.popup_bg();
        let fg = self.theme.fg();
        let border_color = self.theme.popup_border();
        let selected_bg = self.theme.popup_selected();
        let accent = self.theme.popup_accent();
        let dim = self.theme.popup_dim();

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        for x in area.x..area.x + area.width {
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char('─');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        if area.height > 1 {
            let title = " Recent Files ";
            let mut x = area.x + 2;
            for ch in title.chars() {
                if x >= area.x + area.width {
                    break;
                }
                buf.cell_mut((x, area.y + 1)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(
                        Style::default()
                            .fg(accent)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    );
                });
                x += 1;
            }
        }

        if area.height > 2 {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, area.y + 2)).map(|cell| {
                    cell.set_char('─');
                    cell.set_style(Style::default().fg(border_color).bg(bg));
                });
            }
        }

        let list_start = 3u16;
        let list_height = area.height.saturating_sub(list_start) as usize;

        for i in 0..list_height {
            let file_idx = self.scroll + i;
            let y = area.y + list_start + i as u16;
            if y >= area.y + area.height || file_idx >= self.recent.len() {
                break;
            }

            let path = &self.recent[file_idx];
            let is_selected = file_idx == self.selected;
            let line_bg = if is_selected { selected_bg } else { bg };

            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_style(Style::default().bg(line_bg));
                });
            }

            let path_str = path.to_string_lossy();
            let display = format!("  {}", path_str);
            let mut x = area.x;
            for ch in display.chars() {
                if x >= area.x + area.width {
                    break;
                }
                let style = if is_selected {
                    Style::default().fg(fg).bg(line_bg).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(dim).bg(line_bg)
                };
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(ch);
                    cell.set_style(style);
                });
                x += 1;
            }
        }
    }
}

struct MarkdownOutlineWidget<'a> {
    headings: &'a [(usize, usize, String)], // (line, level, text)
    selected: usize,
    theme: &'a config::theme::Theme,
}

impl<'a> ratatui::widgets::Widget for MarkdownOutlineWidget<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        use ratatui::style::{Color, Modifier, Style};

        let bg = self.theme.popup_bg();
        let border_color = self.theme.popup_border();
        let accent = self.theme.popup_accent();
        let _dim = self.theme.popup_dim();

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        let right_x = area.x + area.width - 1;
        for y in area.y..area.y + area.height {
            buf.cell_mut((right_x, y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        let inner_width = (area.width - 1) as usize;
        let title = " Outline ";
        let mut x = area.x + 1;
        for ch in title.chars() {
            if (x - area.x) as usize >= inner_width {
                break;
            }
            buf.cell_mut((x, area.y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(
                    Style::default()
                        .fg(accent)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                );
            });
            x += 1;
        }

        let heading_colors = [
            Color::Rgb(255, 158, 100),
            Color::Rgb(187, 154, 247),
            Color::Rgb(137, 180, 250),
            Color::Rgb(148, 226, 213),
            Color::Rgb(166, 227, 161),
            Color::Rgb(203, 166, 247),
        ];

        for (i, (_, level, text)) in self.headings.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }
            let indent = "  ".repeat(level.saturating_sub(1));
            let display = format!(" {}{}", indent, text);
            let color = heading_colors[(*level).saturating_sub(1).min(5)];
            let is_sel = i == self.selected;
            let line_bg = if is_sel {
                self.theme.popup_selected()
            } else {
                bg
            };

            if is_sel {
                for lx in area.x..area.x + inner_width as u16 {
                    buf.cell_mut((lx, y)).map(|cell| {
                        cell.set_style(Style::default().bg(line_bg));
                    });
                }
            }

            let mut cx = area.x;
            for ch in display.chars() {
                if (cx - area.x) as usize >= inner_width {
                    break;
                }
                buf.cell_mut((cx, y)).map(|cell| {
                    cell.set_char(ch);
                    let style = if *level <= 3 {
                        Style::default()
                            .fg(color)
                            .bg(line_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color).bg(line_bg)
                    };
                    cell.set_style(style);
                });
                cx += 1;
            }
        }
    }
}

struct SidePanelTree<'a> {
    state: &'a ui::tree::TreeState,
    theme: &'a config::theme::Theme,
    focused: bool,
    open_file: Option<&'a std::path::Path>,
}

impl<'a> ratatui::widgets::Widget for SidePanelTree<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        use ratatui::style::{Color, Modifier, Style};

        if area.width < 3 || area.height < 2 {
            return;
        }

        let bg = self.theme.popup_bg();
        let border_color = if self.focused {
            self.theme.popup_accent()
        } else {
            self.theme.popup_border()
        };
        let selected_bg = self.theme.popup_selected();
        let dim = self.theme.popup_dim();
        let accent = self.theme.popup_accent();
        let git_colors = |status: char| -> Color {
            match status {
                'M' => Color::Rgb(249, 226, 175),
                'A' => Color::Rgb(166, 227, 161),
                'D' => Color::Rgb(247, 118, 142),
                'U' => Color::Rgb(247, 118, 142),
                '?' => Color::Rgb(86, 95, 137),
                _ => Color::Rgb(192, 202, 245),
            }
        };

        // fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                buf.cell_mut((x, y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }
        }

        // right border separator
        let right_x = area.x + area.width - 1;
        for y in area.y..area.y + area.height {
            buf.cell_mut((right_x, y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        let inner_width = (area.width - 1) as usize;
        let inner_x = area.x;
        let title_y = area.y;

        let root_name = self
            .state
            .entries
            .first()
            .map(|e| e.name.as_str())
            .unwrap_or("Explorer");
        let title = format!(" \u{f015}  {} ", root_name);

        let is_root_selected = self.focused && self.state.selected == 0;
        let title_bg = if is_root_selected { selected_bg } else { bg };

        for lx in inner_x..inner_x + inner_width as u16 {
            buf.cell_mut((lx, title_y)).map(|cell| {
                cell.set_style(Style::default().bg(title_bg));
            });
        }

        let mut x = inner_x;
        for ch in title.chars() {
            if (x - inner_x) as usize >= inner_width {
                break;
            }
            buf.cell_mut((x, title_y)).map(|cell| {
                cell.set_char(ch);
                cell.set_style(
                    Style::default()
                        .fg(accent)
                        .bg(title_bg)
                        .add_modifier(Modifier::BOLD),
                );
            });
            x += 1;
        }

        // entries
        let visible_height = area.height.saturating_sub(if self.focused { 2 } else { 1 }) as usize;
        for i in 0..visible_height {
            let entry_idx = self.state.scroll_offset + i + 1;
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }

            if let Some(entry) = self.state.entries.get(entry_idx) {
                let is_selected = self.focused && entry_idx == self.state.selected;
                let is_open_file = !entry.is_dir
                    && self.open_file.map_or(false, |p| p == entry.path);
                let line_bg = if is_selected { selected_bg } else { bg };

                for lx in inner_x..inner_x + inner_width as u16 {
                    buf.cell_mut((lx, y)).map(|cell| {
                        cell.set_style(Style::default().bg(line_bg));
                    });
                }

                let guide = ui::tree::tree_guide_prefix(&self.state.entries, entry_idx);
                let is_open = entry.is_dir && self.state.open_dirs.contains(&entry.path);
                let icon = ui::tree::file_icon_pub(&entry.name, entry.is_dir, is_open);
                let git_str = entry
                    .git_status
                    .map(|s| format!(" {}", s))
                    .unwrap_or_default();

                let display = format!(" {}{}{}{}", guide, icon, entry.name, git_str);

                let guide_end = 1 + guide.chars().count();
                let icon_start = guide_end;
                let icon_end = icon_start + icon.chars().count();
                let name_start = icon_end;
                let name_end = name_start + entry.name.len();

                let guide_dim = Color::Rgb(50, 50, 60);

                let name_color = if is_open_file { accent } else { entry.color };

                let mut cx = inner_x;
                for (ci, ch) in display.chars().enumerate() {
                    if (cx - inner_x) as usize >= inner_width {
                        break;
                    }
                    let style = if ci > 0 && ci < guide_end {
                        Style::default().fg(guide_dim).bg(line_bg)
                    } else if ci >= icon_start && ci < icon_end {
                        let mut s = Style::default().fg(name_color).bg(line_bg);
                        if entry.is_dir || is_open_file {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        s
                    } else if ci >= name_start && ci < name_end {
                        let mut s = Style::default().fg(name_color).bg(line_bg);
                        if entry.is_dir || is_open_file {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        s
                    } else if ci >= name_end && entry.git_status.is_some() {
                        Style::default()
                            .fg(git_colors(entry.git_status.unwrap()))
                            .bg(line_bg)
                    } else {
                        Style::default().fg(dim).bg(line_bg)
                    };
                    buf.cell_mut((cx, y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(style);
                    });
                    cx += 1;
                }

                // right-aligned: hint key or file size
                let hint_key = self
                    .state
                    .hint_index_for_entry(entry_idx)
                    .map(|n| format!("{}", n + 1));
                let is_scope_parent = {
                    let scope = self.state.hint_scope.as_ref().or(self.state.root.as_ref());
                    let is_root = self.state.root.as_ref() == Some(&entry.path);
                    scope.map_or(false, |s| entry.path == *s) && !is_root
                };

                let right_label = if let Some(ref k) = hint_key {
                    Some((k.as_str(), dim))
                } else if is_scope_parent {
                    Some(("⌫", dim))
                } else if let Some(size) = entry.file_size {
                    // render file size as before, then skip below
                    let size_str = ui::tree::format_size(size);
                    let size_x =
                        inner_x + inner_width as u16 - size_str.len() as u16 - 1;
                    if size_x > cx {
                        let mut sx = size_x;
                        for ch in size_str.chars() {
                            if (sx - inner_x) as usize >= inner_width {
                                break;
                            }
                            buf.cell_mut((sx, y)).map(|cell| {
                                cell.set_char(ch);
                                cell.set_style(Style::default().fg(dim).bg(line_bg));
                            });
                            sx += 1;
                        }
                    }
                    None
                } else {
                    None
                };

                if let Some((label, color)) = right_label {
                    let label_x =
                        inner_x + inner_width as u16 - label.chars().count() as u16 - 1;
                    if label_x > cx {
                        let mut sx = label_x;
                        for ch in label.chars() {
                            if (sx - inner_x) as usize >= inner_width {
                                break;
                            }
                            buf.cell_mut((sx, y)).map(|cell| {
                                cell.set_char(ch);
                                cell.set_style(Style::default().fg(color).bg(line_bg));
                            });
                            sx += 1;
                        }
                    }
                }
            }
        }

        // hint bar at bottom
        if self.focused {
            let hint_y = area.y + area.height - 1;
            for lx in inner_x..inner_x + inner_width as u16 {
                buf.cell_mut((lx, hint_y)).map(|cell| {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(bg));
                });
            }

            let is_at_root = self.state.hint_scope.as_ref() == self.state.root.as_ref()
                || self.state.hint_scope.is_none();
            let max_n = self.state.hint_indices.len();

            let mut parts: Vec<(&str, String)> = Vec::new();
            if max_n > 0 {
                let range = if max_n == 1 {
                    "1".to_string()
                } else {
                    format!("1-{}", max_n)
                };
                parts.push(("jump", range));
            }
            if !is_at_root {
                parts.push(("back", "⌫".to_string()));
            }
            parts.push(("new", "n".to_string()));
            parts.push(("del", "d".to_string()));

            let guide_dim = Color::Rgb(50, 50, 60);
            let key_fg = self.theme.fg();
            let desc_fg = dim;
            let mut hx = inner_x + 1;
            for (i, (desc, key)) in parts.iter().enumerate() {
                if hx >= inner_x + inner_width as u16 - 1 {
                    break;
                }
                for ch in key.chars() {
                    if hx >= inner_x + inner_width as u16 - 1 {
                        break;
                    }
                    buf.cell_mut((hx, hint_y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(key_fg).bg(bg));
                    });
                    hx += 1;
                }
                if hx < inner_x + inner_width as u16 - 1 {
                    hx += 1;
                }
                for ch in desc.chars() {
                    if hx >= inner_x + inner_width as u16 - 1 {
                        break;
                    }
                    buf.cell_mut((hx, hint_y)).map(|cell| {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(desc_fg).bg(bg));
                    });
                    hx += 1;
                }
                if i + 1 < parts.len() && hx + 2 < inner_x + inner_width as u16 - 1 {
                    hx += 1;
                    buf.cell_mut((hx, hint_y)).map(|cell| {
                        cell.set_char('│');
                        cell.set_style(Style::default().fg(guide_dim).bg(bg));
                    });
                    hx += 2;
                }
            }
        }
    }
}

fn handle_popup_input(app: &mut App, key: crossterm::event::KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // side panel tree: Esc unfocuses (doesn't close panel)
    if app.side_panel_open && key.code == KeyCode::Esc && app.popup == Popup::FileTree {
        let has_pending = app.tree_state.marked_for_move.is_some()
            || app.tree_state.action != crate::ui::tree::TreeAction::None;
        if !has_pending {
            app.popup = Popup::None;
            return;
        }
    }

    match app.popup {
        Popup::Search => match key.code {
            KeyCode::Esc => {
                app.popup = Popup::None;
            }
            KeyCode::Enter => {
                if shift {
                    app.search_state.prev_match();
                } else {
                    app.search_state.next_match();
                }
                if let Some((line, col)) = app.search_state.current_pos() {
                    app.cursor.move_to(line, col, false);
                    app.cursor.update_desired_col();
                }
            }
            KeyCode::Backspace => {
                app.search_state.delete_char();
                app.search_state.find_matches(&app.buffer);
            }
            KeyCode::Char(ch) if !ctrl => {
                app.search_state.insert_char(ch);
                app.search_state.find_matches(&app.buffer);
                let count = app.search_state.matches.len();
                if count > 0 {
                    app.flash(format!("{} matches", count));
                }
                if let Some((line, col)) = app.search_state.current_pos() {
                    app.cursor.move_to(line, col, false);
                    app.cursor.update_desired_col();
                }
            }
            _ => {}
        },

        Popup::Replace => {
            if app.replace_state.awaiting_confirm {
                match key.code {
                    KeyCode::Esc => {
                        app.replace_state.reset();
                        app.popup = Popup::None;
                    }
                    KeyCode::Char('y') => {
                        if let Some((line, col)) = app.replace_state.current_pos() {
                            let end_col = col + app.replace_state.search_query.len();
                            let start = crate::editor::cursor::Position::new(line, col);
                            let end = crate::editor::cursor::Position::new(line, end_col);
                            app.buffer.undo_stack.begin_group(app.cursor.pos);
                            app.buffer.delete_range(start, end);
                            app.buffer
                                .insert_text(start, &app.replace_state.replace_query);
                            app.buffer.undo_stack.finish_group();
                            app.mark_edited();
                        }
                        app.replace_state.skip_current();
                        app.replace_state.find_matches(&app.buffer);
                        if let Some((line, col)) = app.replace_state.current_pos() {
                            app.cursor.move_to(line, col, false);
                            app.cursor.update_desired_col();
                        }
                    }
                    KeyCode::Char('n') => {
                        app.replace_state.skip_current();
                        if let Some((line, col)) = app.replace_state.current_pos() {
                            app.cursor.move_to(line, col, false);
                            app.cursor.update_desired_col();
                        }
                    }
                    KeyCode::Char('a') => {
                        while app.replace_state.awaiting_confirm {
                            if let Some((line, col)) = app.replace_state.current_pos() {
                                let end_col = col + app.replace_state.search_query.len();
                                let start = crate::editor::cursor::Position::new(line, col);
                                let end = crate::editor::cursor::Position::new(line, end_col);
                                app.buffer.undo_stack.begin_group(app.cursor.pos);
                                app.buffer.delete_range(start, end);
                                app.buffer
                                    .insert_text(start, &app.replace_state.replace_query);
                                app.buffer.undo_stack.finish_group();
                            }
                            app.replace_state.skip_current();
                            app.replace_state.find_matches(&app.buffer);
                        }
                        app.mark_edited();
                        app.popup = Popup::None;
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc => {
                        app.replace_state.reset();
                        app.popup = Popup::None;
                    }
                    KeyCode::Tab => {
                        app.replace_state.toggle_field();
                    }
                    KeyCode::Enter => {
                        app.replace_state.find_matches(&app.buffer);
                        if let Some((line, col)) = app.replace_state.current_pos() {
                            app.cursor.move_to(line, col, false);
                            app.cursor.update_desired_col();
                        }
                    }
                    KeyCode::Backspace => {
                        app.replace_state.delete_char();
                    }
                    KeyCode::Char(ch) if !ctrl => {
                        app.replace_state.insert_char(ch);
                    }
                    _ => {}
                }
            }
        }

        Popup::FileTree => {
            use crate::ui::tree::TreeAction;
            let visible = if app.side_panel_open {
                app.viewport_height.saturating_sub(3)
            } else {
                let tree_area = file_tree_popup_area(Rect::new(
                    0,
                    0,
                    app.viewport_width as u16,
                    app.viewport_height as u16,
                ));
                crate::ui::tree::tree_list_height(tree_area)
            };

            // ctrl+z / ctrl+y: tree filesystem undo/redo
            if ctrl && key.code == KeyCode::Char('z') {
                if !app.tree_state.fs_undo_stack.is_empty() {
                    if app.tree_state.undo_last_fs_op() {
                        if let Some(ref git) = app.git_info {
                            app.tree_state.apply_git_statuses(git);
                        }
                    }
                } else if let Some(pos) = app.buffer.apply_undo() {
                    app.cursor.move_to(pos.line, pos.col, false);
                    app.cursor.update_desired_col();
                    app.mark_edited();
                }
                return;
            }
            if ctrl && key.code == KeyCode::Char('y') {
                if !app.tree_state.fs_redo_stack.is_empty() {
                    if app.tree_state.redo_last_fs_op() {
                        if let Some(ref git) = app.git_info {
                            app.tree_state.apply_git_statuses(git);
                        }
                    }
                } else if let Some(pos) = app.buffer.apply_redo() {
                    app.cursor.move_to(pos.line, pos.col, false);
                    app.cursor.update_desired_col();
                    app.mark_edited();
                }
                return;
            }

            if app.tree_state.action != TreeAction::None {
                match key.code {
                    KeyCode::Esc => app.tree_state.cancel_action(),
                    KeyCode::Enter => match app.tree_state.action {
                        TreeAction::NewFile => {
                            if let Some(new_path) = app.tree_state.confirm_new_file() {
                                let name = new_path
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                let _ = app.open_file(&new_path);
                                app.flash(format!("created {}", name));
                                app.popup = Popup::None;
                            }
                        }
                        TreeAction::NewFolder => {
                            if let Some(p) = app.tree_state.confirm_new_folder() {
                                let name = p
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                app.flash(format!("created {}/", name));
                            }
                        }
                        TreeAction::Rename => {
                            if let Some(p) = app.tree_state.confirm_rename() {
                                let name = p
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                app.flash(format!("renamed to {}", name));
                            }
                        }
                        TreeAction::Delete => {
                            if app.tree_state.confirm_delete() {
                                app.flash("deleted");
                            }
                        }
                        TreeAction::None => {}
                    },
                    KeyCode::Char('y') if app.tree_state.action == TreeAction::Delete => {
                        if app.tree_state.confirm_delete() {
                            app.flash("deleted");
                        }
                    }
                    KeyCode::Char('n') if app.tree_state.action == TreeAction::Delete => {
                        app.tree_state.cancel_action();
                    }
                    KeyCode::Backspace => {
                        app.tree_state.input_buf.pop();
                    }
                    KeyCode::Char(ch) if !ctrl => {
                        app.tree_state.input_buf.push(ch);
                    }
                    _ => {}
                }
                return;
            }

            match key.code {
                KeyCode::Esc => {
                    if app.tree_state.marked_for_move.is_some() {
                        app.tree_state.cancel_move();
                    } else {
                        app.popup = Popup::None;
                    }
                }
                KeyCode::Up => {
                    if app.tree_state.marked_for_move.is_some() {
                        app.tree_state.move_up_folders_only();
                    } else {
                        app.tree_state.move_up();
                    }
                }
                KeyCode::Down => {
                    if app.tree_state.marked_for_move.is_some() {
                        app.tree_state.move_down_folders_only(visible);
                    } else {
                        app.tree_state.move_down(visible);
                    }
                }
                KeyCode::Enter => {
                    if app.tree_state.marked_for_move.is_some() {
                        if let Some(entry) = app.tree_state.selected_entry() {
                            if entry.is_dir {
                                if let Some(p) = app.tree_state.confirm_move() {
                                    let dest = p
                                        .parent()
                                        .unwrap_or(std::path::Path::new("."))
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy();
                                    app.flash(format!("moved to {}/", dest));
                                }
                            }
                        }
                    } else if let Some(entry) = app.tree_state.selected_entry() {
                        if entry.is_dir {
                            app.tree_state.toggle_dir();
                        } else {
                            let path = entry.path.clone();
                            let _ = app.open_file(&path);
                            if !app.side_panel_open {
                                app.popup = Popup::None;
                            }
                        }
                    }
                }
                KeyCode::Right => {
                    if let Some(entry) = app.tree_state.selected_entry() {
                        if entry.is_dir && !app.tree_state.open_dirs.contains(&entry.path) {
                            app.tree_state.toggle_dir();
                        }
                    }
                }
                KeyCode::Left => {
                    if let Some(entry) = app.tree_state.selected_entry() {
                        if entry.is_dir && app.tree_state.open_dirs.contains(&entry.path) {
                            app.tree_state.toggle_dir();
                        }
                    }
                }
                KeyCode::Char('n') if !ctrl => {
                    app.tree_state.start_action(TreeAction::NewFile);
                }
                KeyCode::Char('c') if !ctrl && !shift => {
                    if let Some(path) = app.tree_state.selected_relative_path() {
                        clipboard::copy_to_clipboard(&path);
                        app.flash(format!("copied: {}", path));
                    }
                }
                KeyCode::Char('C') if !ctrl => {
                    if let Some(path) = app.tree_state.selected_full_path() {
                        clipboard::copy_to_clipboard(&path);
                        app.flash(format!("copied: {}", path));
                    }
                }
                KeyCode::Char('y') if !ctrl && !shift => {
                    if let Some(path) = app.tree_state.selected_relative_path() {
                        clipboard::copy_to_clipboard(&path);
                        app.flash(format!("yanked: {}", path));
                    }
                }
                KeyCode::Char('Y') if !ctrl => {
                    if let Some(path) = app.tree_state.selected_full_path() {
                        clipboard::copy_to_clipboard(&path);
                        app.flash(format!("yanked: {}", path));
                    }
                }
                KeyCode::Char('f') if !ctrl => {
                    app.tree_state.start_action(TreeAction::NewFolder);
                }
                KeyCode::Char('r') if !ctrl => {
                    app.tree_state.start_action(TreeAction::Rename);
                }
                KeyCode::Char('d') if !ctrl => {
                    app.tree_state.start_action(TreeAction::Delete);
                }
                KeyCode::Char('m') if !ctrl => {
                    if app.tree_state.marked_for_move.is_some() {
                        app.tree_state.confirm_move();
                    } else {
                        app.tree_state.mark_for_move();
                    }
                }
                KeyCode::Char('z') if !ctrl => {
                    app.tree_state.open_dirs.clear();
                    if let Some(root) = app.tree_state.root.clone() {
                        app.tree_state.build(&root);
                        if let Some(ref git) = app.git_info {
                            app.tree_state.apply_git_statuses(git);
                        }
                    }
                    app.tree_state.selected = 0;
                    app.tree_state.scroll_offset = 0;
                    app.tree_state.init_hint_scope();
                    app.flash("collapsed all");
                }
                KeyCode::Char(ch @ '1'..='9') if !ctrl => {
                    let n = (ch as usize) - ('1' as usize);
                    if let Some(result) = app.tree_state.hint_enter(n) {
                        match result {
                            crate::ui::tree::HintResult::EnteredFolder => {
                                if let Some(ref git) = app.git_info {
                                    app.tree_state.apply_git_statuses(git);
                                }
                            }
                            crate::ui::tree::HintResult::OpenFile(path) => {
                                let _ = app.open_file(&path);
                                if !app.side_panel_open {
                                    app.popup = Popup::None;
                                }
                            }
                        }
                    }
                }
                KeyCode::Backspace if !ctrl => {
                    app.tree_state.hint_back();
                }
                _ => {}
            }
        }

        Popup::FuzzyFinder => {
            let visible = app.viewport_height.saturating_sub(6);
            match key.code {
                KeyCode::Esc => {
                    app.fuzzy_state.reset();
                    app.popup = Popup::None;
                }
                KeyCode::Up => app.fuzzy_state.move_up(),
                KeyCode::Down => app.fuzzy_state.move_down(visible),
                KeyCode::Enter => {
                    if let Some(rel_path) = app.fuzzy_state.selected_path().cloned() {
                        if let Some(root) = app.project_root.clone() {
                            let full_path = root.join(&rel_path);
                            let _ = app.open_file(&full_path);
                        }
                    }
                    app.fuzzy_state.reset();
                    app.popup = Popup::None;
                }
                KeyCode::Backspace => app.fuzzy_state.delete_char(),
                KeyCode::Char(ch) if !ctrl => app.fuzzy_state.insert_char(ch),
                _ => {}
            }
        }

        Popup::SearchProject => {
            let visible = app.viewport_height.saturating_sub(6);
            match key.code {
                KeyCode::Esc => {
                    app.project_search_state.reset();
                    app.popup = Popup::None;
                }
                KeyCode::Up => app.project_search_state.move_up(),
                KeyCode::Down => app.project_search_state.move_down(visible),
                KeyCode::Enter if !app.project_search_state.query.is_empty() => {
                    if app.project_search_state.results.is_empty() {
                        if let Some(root) = app.project_root.clone() {
                            app.project_search_state.search(&root);
                        }
                    } else {
                        if let Some(result) = app.project_search_state.selected_result().cloned() {
                            if let Some(root) = app.project_root.clone() {
                                let full_path = root.join(&result.path);
                                let _ = app.open_file(&full_path);
                                app.cursor.move_to(result.line, result.col, false);
                                app.cursor.update_desired_col();
                            }
                            app.project_search_state.reset();
                            app.popup = Popup::None;
                        }
                    }
                }
                KeyCode::Backspace => app.project_search_state.delete_char(),
                KeyCode::Char(ch) if !ctrl => app.project_search_state.insert_char(ch),
                _ => {}
            }
        }

        Popup::ThemeSwitcher => match key.code {
            KeyCode::Esc => {
                app.popup = Popup::None;
            }
            KeyCode::Up => app.theme_switcher_state.move_up(),
            KeyCode::Down => app.theme_switcher_state.move_down(),
            KeyCode::Enter => {
                if let Some(selected) = app.theme_switcher_state.selected_theme().cloned() {
                    let name = selected.name.clone();
                    app.theme = selected;
                    if app.highlighter.is_active() {
                        if let Some(config) =
                            crate::syntax::highlight::Highlighter::detect_language(
                                app.buffer
                                    .file_path
                                    .as_deref()
                                    .unwrap_or(std::path::Path::new("")),
                            )
                        {
                            app.highlighter.set_language(&config, &app.theme.colors);
                            let source = app.buffer.rope.to_string();
                            app.highlighter.parse(&source);
                            app.highlighter.compute_styles(&source);
                        }
                    }
                    crate::config::settings::Settings::update_theme(&name);
                    app.flash(format!("theme: {}", name));
                }
                app.popup = Popup::None;
            }
            _ => {}
        },

        Popup::KeybindHelp => {
            let max_scroll = crate::ui::keybind_help::KeybindHelpState::total_lines()
                .saturating_sub(app.viewport_height.saturating_sub(6));
            match key.code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                }
                KeyCode::Up => app.keybind_help_state.scroll_up(),
                KeyCode::Down => app.keybind_help_state.scroll_down(max_scroll),
                _ => {}
            }
        }

        Popup::RecentFiles => {
            let visible = app.viewport_height.saturating_sub(6);
            match key.code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                }
                KeyCode::Up => {
                    if app.fuzzy_state.selected > 0 {
                        app.fuzzy_state.selected -= 1;
                    }
                }
                KeyCode::Down => {
                    if app.fuzzy_state.selected + 1 < app.recent_files.len() {
                        app.fuzzy_state.selected += 1;
                        if app.fuzzy_state.selected >= app.fuzzy_state.scroll_offset + visible {
                            app.fuzzy_state.scroll_offset =
                                app.fuzzy_state.selected - visible + 1;
                        }
                    }
                }
                KeyCode::Enter => {
                    if let Some(path) = app.recent_files.get(app.fuzzy_state.selected).cloned() {
                        let _ = app.open_file(&path);
                    }
                    app.popup = Popup::None;
                }
                _ => {}
            }
        }

        Popup::GotoLine => match key.code {
            KeyCode::Esc => {
                app.popup = Popup::None;
            }
            KeyCode::Enter => {
                if let Ok(line_num) = app.goto_line_input.parse::<usize>() {
                    let target = line_num.saturating_sub(1).min(app.buffer.line_count().saturating_sub(1));
                    app.cursor.move_to(target, 0, false);
                    app.cursor.update_desired_col();
                }
                app.popup = Popup::None;
            }
            KeyCode::Backspace => {
                app.goto_line_input.pop();
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                app.goto_line_input.push(ch);
            }
            _ => {}
        },

        Popup::None => unreachable!(),
    }
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    let line_count = app.buffer.line_count();
    let max_line_num_width = format!("{}", line_count).len().max(3);
    let has_git_gutter = !app.gutter_marks.is_empty();
    let git_gutter_width = if has_git_gutter { 1u16 } else { 0 };
    let gutter_width = git_gutter_width + max_line_num_width as u16 + 1;
    let h_padding = app.horizontal_padding as u16;
    let panel_offset = if app.side_panel_open {
        SIDE_PANEL_WIDTH.min(app.viewport_width as u16 / 3)
    } else {
        0
    };
    let text_area_x = panel_offset + gutter_width + h_padding;
    let full_width = app.viewport_width as u16;
    let full_height = app.viewport_height as u16;
    let full_area = Rect::new(0, 0, full_width, full_height);

    match mouse.kind {
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            let click_x = mouse.column;
            let click_y = mouse.row;

            // side panel click
            if app.side_panel_open && click_x < panel_offset {
                app.popup = Popup::FileTree;
                let _visible_height = full_height.saturating_sub(2) as usize;
                if click_y == 0 {
                    app.tree_state.selected = 0;
                } else {
                    let entry_idx = app.tree_state.scroll_offset + click_y as usize;
                    if entry_idx < app.tree_state.entries.len() {
                        if app.tree_state.selected == entry_idx {
                            if let Some(entry) = app.tree_state.entries.get(entry_idx) {
                                if entry.is_dir {
                                    app.tree_state.toggle_dir();
                                } else {
                                    let path = entry.path.clone();
                                    let _ = app.open_file(&path);
                                }
                            }
                        } else {
                            app.tree_state.selected = entry_idx;
                        }
                    }
                }
                return;
            }

            if app.popup == Popup::FileTree && !app.side_panel_open {
                let tree_area = file_tree_popup_area(full_area);
                let inner = crate::ui::tree::tree_inner_area(tree_area);
                let action_active = app.tree_state.action != crate::ui::tree::TreeAction::None;
                let action_y = inner.y + inner.height.saturating_sub(1);

                if click_x >= inner.x
                    && click_x < inner.x + inner.width
                    && click_y >= inner.y
                    && click_y < inner.y + inner.height
                {
                    let entry_idx = if click_y == inner.y {
                        Some(0)
                    } else if action_active && click_y == action_y {
                        None
                    } else {
                        Some(app.tree_state.scroll_offset + (click_y - inner.y - 1) as usize + 1)
                    };

                    if let Some(entry_idx) =
                        entry_idx.filter(|idx| *idx < app.tree_state.entries.len())
                    {
                        if app.tree_state.selected == entry_idx {
                            if let Some(entry) = app.tree_state.entries.get(entry_idx) {
                                if entry.is_dir {
                                    app.tree_state.toggle_dir();
                                } else {
                                    let path = entry.path.clone();
                                    let _ = app.open_file(&path);
                                    app.popup = Popup::None;
                                }
                            }
                        } else {
                            app.tree_state.selected = entry_idx;
                        }
                    }
                }
                return;
            }

            if app.popup == Popup::FuzzyFinder {
                let popup_width = (full_width * 60 / 100).max(40);
                let popup_height = (full_height * 60 / 100).max(10);
                let px = (full_width - popup_width) / 2;
                let py = (full_height - popup_height) / 4;
                if click_x >= px
                    && click_x < px + popup_width
                    && click_y >= py + 3
                    && click_y < py + popup_height
                {
                    let clicked_idx = app.fuzzy_state.scroll_offset + (click_y - py - 3) as usize;
                    if clicked_idx < app.fuzzy_state.filtered.len() {
                        app.fuzzy_state.selected = clicked_idx;
                    }
                }
                return;
            }

            if app.popup == Popup::ThemeSwitcher {
                let popup_width = (full_width * 40 / 100).max(35);
                let popup_height = (full_height * 50 / 100).max(10);
                let px = (full_width - popup_width) / 2;
                let py = (full_height - popup_height) / 4;
                if click_x >= px
                    && click_x < px + popup_width
                    && click_y >= py + 3
                    && click_y < py + popup_height
                {
                    let clicked_idx = (click_y - py - 3) as usize;
                    if clicked_idx < app.theme_switcher_state.themes.len() {
                        app.theme_switcher_state.selected = clicked_idx;
                    }
                }
                return;
            }

            if app.popup == Popup::SearchProject {
                let popup_width = (full_width * 70 / 100).max(50);
                let popup_height = (full_height * 70 / 100).max(15);
                let px = (full_width - popup_width) / 2;
                let py = (full_height - popup_height) / 4;
                if click_x >= px
                    && click_x < px + popup_width
                    && click_y >= py + 3
                    && click_y < py + popup_height
                {
                    let clicked_idx =
                        app.project_search_state.scroll_offset + (click_y - py - 3) as usize;
                    if clicked_idx < app.project_search_state.results.len() {
                        app.project_search_state.selected = clicked_idx;
                    }
                }
                return;
            }

            // editor text area click
            if click_x >= text_area_x && click_y < full_height.saturating_sub(1) {
                let file_col = (click_x - text_area_x) as usize + app.viewport_left;
                let file_line = click_y as usize + app.viewport_top;
                if file_line < line_count {
                    let line_len = app.buffer.line_len(file_line);
                    let col = file_col.min(line_len);
                    app.cursor.move_to(file_line, col, false);
                    app.cursor.update_desired_col();
                }
            }
        }

        MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
            if app.popup == Popup::None
                || app.popup == Popup::Search
                || app.popup == Popup::Replace
            {
                let click_x = mouse.column;
                let click_y = mouse.row;
                if click_x >= text_area_x && click_y < full_height.saturating_sub(1) {
                    let file_col = (click_x - text_area_x) as usize + app.viewport_left;
                    let file_line = click_y as usize + app.viewport_top;
                    if file_line < line_count {
                        let line_len = app.buffer.line_len(file_line);
                        let col = file_col.min(line_len);
                        app.cursor.move_to(file_line, col, true);
                    }
                }
            }
        }

        MouseEventKind::ScrollUp => match app.popup {
            Popup::FileTree => app.tree_state.move_up(),
            Popup::FuzzyFinder => app.fuzzy_state.move_up(),
            Popup::KeybindHelp => app.keybind_help_state.scroll_up(),
            Popup::ThemeSwitcher => app.theme_switcher_state.move_up(),
            Popup::SearchProject => app.project_search_state.move_up(),
            _ => {
                if app.viewport_top > 0 {
                    app.viewport_top = app.viewport_top.saturating_sub(3);
                    if app.cursor.pos.line
                        >= app.viewport_top + app.viewport_height.saturating_sub(2)
                    {
                        let new_line = app.viewport_top + app.viewport_height.saturating_sub(3);
                        let col = app.cursor.desired_col.min(app.buffer.line_len(new_line));
                        app.cursor.move_to(new_line, col, false);
                    }
                }
            }
        },
        MouseEventKind::ScrollDown => match app.popup {
            Popup::FileTree => {
                let tree_area = file_tree_popup_area(full_area);
                let visible = crate::ui::tree::tree_list_height(tree_area);
                app.tree_state.move_down(visible);
            }
            Popup::FuzzyFinder => {
                let visible = app.viewport_height.saturating_sub(6);
                app.fuzzy_state.move_down(visible);
            }
            Popup::KeybindHelp => {
                let max = crate::ui::keybind_help::KeybindHelpState::total_lines()
                    .saturating_sub(app.viewport_height.saturating_sub(6));
                app.keybind_help_state.scroll_down(max);
            }
            Popup::ThemeSwitcher => app.theme_switcher_state.move_down(),
            Popup::SearchProject => {
                let visible = app.viewport_height.saturating_sub(6);
                app.project_search_state.move_down(visible);
            }
            _ => {
                let max_top = line_count.saturating_sub(1);
                if app.viewport_top < max_top {
                    app.viewport_top = (app.viewport_top + 3).min(max_top);
                    if app.cursor.pos.line < app.viewport_top {
                        let col = app
                            .cursor
                            .desired_col
                            .min(app.buffer.line_len(app.viewport_top));
                        app.cursor.move_to(app.viewport_top, col, false);
                    }
                }
            }
        },
        _ => {}
    }
}

fn run_headless(cli: Cli, settings: Settings) -> io::Result<()> {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct HeadlessInput {
        keys: Vec<String>,
    }

    #[derive(Serialize)]
    struct HeadlessOutput {
        buffer: String,
        cursor_line: usize,
        cursor_col: usize,
        mode: String,
        line_count: usize,
        dirty: bool,
    }

    let mut app = App::new(settings);
    app.viewport_height = 24;
    app.viewport_width = 80;

    if let Some(ref file_path) = cli.file {
        let path = std::path::Path::new(file_path);
        if path.exists() {
            app.open_file(path)?;
        } else {
            app.buffer.file_path = Some(path.to_path_buf());
        }
    }

    let input: String = io::read_to_string(io::stdin())?;
    let parsed: HeadlessInput =
        serde_json::from_str(&input).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    for key_str in &parsed.keys {
        if let Some(key_event) = parse_key_string(key_str) {
            if key_event.code == KeyCode::Char('q')
                && key_event.modifiers.contains(KeyModifiers::CONTROL)
            {
                break;
            }
            editor::input::handle_key(&mut app, key_event);
        }
    }

    let output = HeadlessOutput {
        buffer: app.buffer.rope.to_string(),
        cursor_line: app.cursor.pos.line,
        cursor_col: app.cursor.pos.col,
        mode: app.mode.label().to_string(),
        line_count: app.buffer.line_count(),
        dirty: app.buffer.dirty,
    };

    let json = serde_json::to_string_pretty(&output)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    println!("{}", json);

    Ok(())
}

fn parse_key_string(s: &str) -> Option<crossterm::event::KeyEvent> {
    use crossterm::event::{KeyEvent, KeyModifiers};

    let (mods, key_part) = if s.contains('+') {
        let parts: Vec<&str> = s.rsplitn(2, '+').collect();
        let key = parts[0];
        let mod_str = parts[1].to_lowercase();
        let mut m = KeyModifiers::empty();
        for part in mod_str.split('+') {
            match part.trim() {
                "ctrl" => m |= KeyModifiers::CONTROL,
                "shift" => m |= KeyModifiers::SHIFT,
                "alt" => m |= KeyModifiers::ALT,
                _ => {}
            }
        }
        (m, key.to_string())
    } else {
        (KeyModifiers::empty(), s.to_string())
    };

    let code = match key_part.as_str() {
        "Esc" | "Escape" => KeyCode::Esc,
        "Enter" | "Return" => KeyCode::Enter,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Tab" => KeyCode::Tab,
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        _ => return None,
    };

    Some(KeyEvent::new(code, mods))
}
