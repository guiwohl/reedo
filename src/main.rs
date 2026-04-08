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

use app::{App, AppMode, Popup};
use config::settings::Settings;
use editor::clipboard;
use ui::fuzzy::FuzzyFinderWidget;
use ui::keybind_help::KeybindHelpWidget;
use ui::render::EditorView;
use ui::replace::ReplaceBar;
use ui::replace_project::ProjectReplaceWidget;
use ui::search::SearchBar;
use ui::search_project::ProjectSearchWidget;
use ui::statusbar::StatusBar;
use ui::theme_switcher::ThemeSwitcherWidget;
use ui::tree::FileTreeWidget;
use ui::welcome::WelcomeScreen;

const SIDE_PANEL_WIDTH: u16 = 30;

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
            // set project root to file's parent or cwd
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

    // set project root if not already set
    if app.project_root.is_none() {
        app.set_project_root(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
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

            // horizontal split: side panel + main content
            let panel_width = if app.side_panel_open {
                SIDE_PANEL_WIDTH.min(full_area.width / 3)
            } else {
                0
            };

            let h_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(if panel_width > 0 {
                    vec![
                        Constraint::Length(panel_width),
                        Constraint::Min(1),
                    ]
                } else {
                    vec![Constraint::Min(1)]
                })
                .split(full_area);

            let main_area = if panel_width > 0 { h_chunks[1] } else { h_chunks[0] };
            let panel_area = if panel_width > 0 { Some(h_chunks[0]) } else { None };

            // render side panel tree (always visible when panel is open)
            if let Some(panel) = panel_area {
                let tree_state = match app.app_mode {
                    AppMode::Editor => &app.tree_state,
                    AppMode::Git => &app.git_tree_state,
                };
                let is_focused = matches!(
                    (&app.app_mode, &app.popup),
                    (AppMode::Editor, Popup::FileTree) | (AppMode::Git, Popup::GitTree)
                );
                frame.render_widget(
                    SidePanelTree {
                        state: tree_state,
                        theme: &app.theme,
                        focused: is_focused,
                        git_mode: app.app_mode == AppMode::Git,
                    },
                    panel,
                );
            }

            // main layout: editor + status bar
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
                    Popup::PaddingInput => vec![
                        Constraint::Min(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ],
                    _ => vec![Constraint::Min(1), Constraint::Length(1)],
                })
                .split(main_area);

            // show welcome screen if no file is open, otherwise editor
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
                Popup::PaddingInput => {
                    let bar_area = main_chunks[1];
                    let bg = app.theme.statusbar_bg();
                    let fg = app.theme.statusbar_fg();
                    frame.render_widget(
                        ratatui::widgets::Paragraph::new(format!(
                            " Horizontal padding: {}█",
                            app.padding_input
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

            // overlay popups (only when NOT using side panel for that tree)
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
                Popup::GitTree if !app.side_panel_open => {
                    let tree_area = file_tree_popup_area(full_area);
                    frame.render_widget(
                        FileTreeWidget {
                            state: &app.git_tree_state,
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
                        },
                        popup_area,
                    );
                }
                Popup::GitFuzzyFinder => {
                    let popup_width = (full_area.width * 60 / 100).max(40);
                    let popup_height = (full_area.height * 60 / 100).max(10);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        FuzzyFinderWidget {
                            state: &app.git_fuzzy_state,
                            theme: &app.theme,
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
                Popup::ReplaceProject => {
                    let popup_width = (full_area.width * 70 / 100).max(50);
                    let popup_height = (full_area.height * 50 / 100).max(10);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        ProjectReplaceWidget {
                            state: &app.project_replace_state,
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

        // set terminal cursor shape based on mode
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
                    // ctrl+q always quits
                    if key.code == KeyCode::Char('q')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        app.running = false;
                    }
                    // route input based on active popup
                    else if app.popup != Popup::None {
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

struct SidePanelTree<'a> {
    state: &'a ui::tree::TreeState,
    theme: &'a config::theme::Theme,
    focused: bool,
    git_mode: bool,
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

        // right border only (separator between panel and editor)
        let right_x = area.x + area.width - 1;
        for y in area.y..area.y + area.height {
            buf.cell_mut((right_x, y)).map(|cell| {
                cell.set_char('│');
                cell.set_style(Style::default().fg(border_color).bg(bg));
            });
        }

        let inner_width = (area.width - 1) as usize; // -1 for border
        let inner_x = area.x;
        let title_y = area.y;

        // title row
        let title_label = if self.git_mode { "Git Changes" } else { "Explorer" };
        let root_name = self
            .state
            .entries
            .first()
            .map(|e| e.name.as_str())
            .unwrap_or(title_label);
        let title = if self.git_mode {
            format!(" \u{e702}  {} ", root_name)
        } else {
            format!(" \u{f015}  {} ", root_name)
        };

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
        let visible_height = area.height.saturating_sub(1) as usize;
        for i in 0..visible_height {
            let entry_idx = self.state.scroll_offset + i + 1;
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.height {
                break;
            }

            if let Some(entry) = self.state.entries.get(entry_idx) {
                let is_selected = self.focused && entry_idx == self.state.selected;
                let line_bg = if is_selected { selected_bg } else { bg };

                for lx in inner_x..inner_x + inner_width as u16 {
                    buf.cell_mut((lx, y)).map(|cell| {
                        cell.set_style(Style::default().bg(line_bg));
                    });
                }

                let indent = "  ".repeat(entry.depth);
                let is_open = entry.is_dir && self.state.open_dirs.contains(&entry.path);
                let icon = ui::tree::file_icon_pub(&entry.name, entry.is_dir, is_open);
                let git_str = entry
                    .git_status
                    .map(|s| format!(" {}", s))
                    .unwrap_or_default();

                let display = format!(" {}{}{}{}", indent, icon, entry.name, git_str);

                let icon_start = 1 + indent.len();
                let icon_end = icon_start + icon.chars().count();
                let name_start = icon_end;
                let name_end = name_start + entry.name.len();

                let mut cx = inner_x;
                for (ci, ch) in display.chars().enumerate() {
                    if (cx - inner_x) as usize >= inner_width {
                        break;
                    }
                    let style = if ci >= icon_start && ci < icon_end {
                        let mut s = Style::default().fg(entry.color).bg(line_bg);
                        if entry.is_dir {
                            s = s.add_modifier(Modifier::BOLD);
                        }
                        s
                    } else if ci >= name_start && ci < name_end {
                        let mut s = Style::default().fg(entry.color).bg(line_bg);
                        if entry.is_dir {
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
            }
        }
    }
}

fn handle_popup_input(app: &mut App, key: crossterm::event::KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // when side panel is open and tree is focused, Esc unfocuses (doesn't close panel)
    // but only if no tree action is pending
    if app.side_panel_open
        && key.code == KeyCode::Esc
        && matches!(app.popup, Popup::FileTree | Popup::GitTree)
    {
        let has_pending = match app.popup {
            Popup::FileTree => {
                app.tree_state.marked_for_move.is_some()
                    || app.tree_state.action != crate::ui::tree::TreeAction::None
            }
            Popup::GitTree => false,
            _ => false,
        };
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
                        // apply replacement at current match
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
                        // replace all remaining
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
                app.viewport_height.saturating_sub(2)
            } else {
                let tree_area = file_tree_popup_area(Rect::new(
                    0,
                    0,
                    app.viewport_width as u16,
                    app.viewport_height as u16,
                ));
                crate::ui::tree::tree_list_height(tree_area, false)
            };

            // ctrl+z / ctrl+y: tree filesystem undo/redo first, then buffer history
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

            // handle active action input first
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
                        app.flash(format!("copied relative path: {}", path));
                    }
                }
                KeyCode::Char('C') if !ctrl => {
                    if let Some(path) = app.tree_state.selected_full_path() {
                        clipboard::copy_to_clipboard(&path);
                        app.flash(format!("copied full path: {}", path));
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
                        // second m press: move to selected folder
                        app.tree_state.confirm_move();
                    } else {
                        app.tree_state.mark_for_move();
                    }
                }
                _ => {}
            }
        }

        Popup::GitTree => {
            let tree_area = file_tree_popup_area(Rect::new(
                0,
                0,
                app.viewport_width as u16,
                app.viewport_height as u16,
            ));
            let visible = if app.side_panel_open {
                app.viewport_height.saturating_sub(2)
            } else {
                crate::ui::tree::tree_list_height(tree_area, false)
            };

            match key.code {
                KeyCode::Esc => {
                    app.popup = Popup::None;
                }
                KeyCode::Up => {
                    app.git_tree_state.move_up();
                }
                KeyCode::Down => {
                    app.git_tree_state.move_down(visible);
                }
                KeyCode::Enter => {
                    if let Some(entry) = app.git_tree_state.selected_entry() {
                        if !entry.is_dir {
                            let path = entry.path.clone();
                            let _ = app.open_file(&path);
                            if !app.side_panel_open {
                                app.popup = Popup::None;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Popup::GitFuzzyFinder => {
            let visible = app.viewport_height.saturating_sub(6);
            match key.code {
                KeyCode::Esc => {
                    app.git_fuzzy_state.reset();
                    app.popup = Popup::None;
                }
                KeyCode::Up => app.git_fuzzy_state.move_up(),
                KeyCode::Down => app.git_fuzzy_state.move_down(visible),
                KeyCode::Enter => {
                    if let Some(rel_path) = app.git_fuzzy_state.selected_path().cloned() {
                        if let Some(root) = app.project_root.clone() {
                            let full_path = root.join(&rel_path);
                            let _ = app.open_file(&full_path);
                        }
                    }
                    app.git_fuzzy_state.reset();
                    app.popup = Popup::None;
                }
                KeyCode::Backspace => app.git_fuzzy_state.delete_char(),
                KeyCode::Char(ch) if !ctrl => app.git_fuzzy_state.insert_char(ch),
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
                        // open selected result
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

        Popup::ReplaceProject => {
            if app.project_replace_state.awaiting_confirm {
                match key.code {
                    KeyCode::Esc => {
                        app.project_replace_state.reset();
                        app.popup = Popup::None;
                    }
                    KeyCode::Char('y') => {
                        app.project_replace_state.apply_current();
                    }
                    KeyCode::Char('n') => {
                        app.project_replace_state.skip_current();
                    }
                    KeyCode::Char('a') => {
                        while app.project_replace_state.awaiting_confirm {
                            app.project_replace_state.apply_current();
                        }
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc => {
                        app.project_replace_state.reset();
                        app.popup = Popup::None;
                    }
                    KeyCode::Tab => app.project_replace_state.toggle_field(),
                    KeyCode::Enter => {
                        if let Some(root) = app.project_root.clone() {
                            app.project_replace_state.search(&root);
                        }
                    }
                    KeyCode::Backspace => app.project_replace_state.delete_char(),
                    KeyCode::Char(ch) if !ctrl => app.project_replace_state.insert_char(ch),
                    _ => {}
                }
            }
        }

        Popup::ThemeSwitcher => {
            match key.code {
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
                        // persist to config
                        crate::config::settings::Settings::update_theme(&name);
                        app.flash(format!("theme: {}", name));
                    }
                    app.popup = Popup::None;
                }
                _ => {}
            }
        }

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

        Popup::PaddingInput => match key.code {
            KeyCode::Esc => {
                app.popup = Popup::None;
            }
            KeyCode::Enter => {
                if let Ok(val) = app.padding_input.parse::<usize>() {
                    app.horizontal_padding = val;
                }
                app.popup = Popup::None;
            }
            KeyCode::Backspace => {
                app.padding_input.pop();
            }
            KeyCode::Char(ch) if ch.is_ascii_digit() => {
                app.padding_input.push(ch);
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
                let tree_state = match app.app_mode {
                    AppMode::Editor => &mut app.tree_state,
                    AppMode::Git => &mut app.git_tree_state,
                };
                let popup_type = match app.app_mode {
                    AppMode::Editor => Popup::FileTree,
                    AppMode::Git => Popup::GitTree,
                };
                // focus the panel
                app.popup = popup_type;
                let visible_height = full_height.saturating_sub(2) as usize;
                if click_y == 0 {
                    tree_state.selected = 0;
                } else {
                    let entry_idx = tree_state.scroll_offset + click_y as usize;
                    if entry_idx < tree_state.entries.len() {
                        if tree_state.selected == entry_idx {
                            // double click behavior
                            if let Some(entry) = tree_state.entries.get(entry_idx) {
                                if entry.is_dir {
                                    tree_state.toggle_dir();
                                } else {
                                    let path = entry.path.clone();
                                    let _ = app.open_file(&path);
                                }
                            }
                        } else {
                            tree_state.selected = entry_idx;
                        }
                    }
                }
                let _ = visible_height;
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

        // drag = text selection
        MouseEventKind::Drag(crossterm::event::MouseButton::Left) => {
            if app.popup == Popup::None || app.popup == Popup::Search || app.popup == Popup::Replace
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
            Popup::GitTree => app.git_tree_state.move_up(),
            Popup::FuzzyFinder => app.fuzzy_state.move_up(),
            Popup::GitFuzzyFinder => app.git_fuzzy_state.move_up(),
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
                let visible = crate::ui::tree::tree_list_height(
                    tree_area,
                    app.tree_state.action != crate::ui::tree::TreeAction::None,
                );
                app.tree_state.move_down(visible);
            }
            Popup::GitTree => {
                let visible = app.viewport_height.saturating_sub(2);
                app.git_tree_state.move_down(visible);
            }
            Popup::FuzzyFinder => {
                let visible = app.viewport_height.saturating_sub(6);
                app.fuzzy_state.move_down(visible);
            }
            Popup::GitFuzzyFinder => {
                let visible = app.viewport_height.saturating_sub(6);
                app.git_fuzzy_state.move_down(visible);
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
