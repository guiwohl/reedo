mod app;
mod config;
mod editor;
mod git;
mod syntax;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Terminal;

use app::{App, Popup};
use config::settings::Settings;
use ui::render::EditorView;
use ui::statusbar::StatusBar;
use ui::search::SearchBar;
use ui::replace::ReplaceBar;
use ui::fuzzy::FuzzyFinderWidget;
use ui::tree::FileTreeWidget;
use ui::search_project::ProjectSearchWidget;
use ui::replace_project::ProjectReplaceWidget;
use ui::theme_switcher::ThemeSwitcherWidget;
use ui::keybind_help::KeybindHelpWidget;
use ui::welcome::WelcomeScreen;

#[derive(Parser)]
#[command(name = "kilo", about = "A minimal terminal text editor")]
struct Cli {
    file: Option<String>,

    #[arg(long, hide = true)]
    headless: bool,

    #[arg(long, hide = true)]
    dump_frames: Option<String>,
}

fn init_logging() {
    if std::env::var("KILO_LOG").is_ok() {
        let file_appender = tracing_appender::rolling::never("/tmp", "kilo-debug.log");
        tracing_subscriber::fmt()
            .with_writer(file_appender)
            .with_env_filter("kilo=debug")
            .with_ansi(false)
            .init();
        tracing::info!("kilo logging initialized");
    }
}

fn main() -> io::Result<()> {
    init_logging();
    let cli = Cli::parse();
    let settings = Settings::load();

    if cli.headless {
        return run_headless(cli, settings);
    }

    run_tui(cli, settings)
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
        app.set_project_root(
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        );
    }

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &mut app);

    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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

            // main layout: editor + status bar
            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(match app.popup {
                    Popup::Search => vec![
                        Constraint::Min(1),
                        Constraint::Length(1), // search bar
                        Constraint::Length(1), // status bar
                    ],
                    Popup::Replace => vec![
                        Constraint::Min(1),
                        Constraint::Length(2), // replace bar (2 lines)
                        Constraint::Length(1),
                    ],
                    _ => vec![
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ],
                })
                .split(full_area);

            // show welcome screen if no file is open, otherwise editor
            if app.buffer.file_path.is_none() && !app.buffer.dirty && app.buffer.rope.len_chars() == 0 {
                frame.render_widget(WelcomeScreen, main_chunks[0]);
            } else {
                frame.render_widget(EditorView { app }, main_chunks[0]);
            }

            match app.popup {
                Popup::Search => {
                    frame.render_widget(SearchBar { state: &app.search_state }, main_chunks[1]);
                    frame.render_widget(StatusBar { app }, main_chunks[2]);
                }
                Popup::Replace => {
                    frame.render_widget(ReplaceBar { state: &app.replace_state }, main_chunks[1]);
                    frame.render_widget(StatusBar { app }, main_chunks[2]);
                }
                _ => {
                    frame.render_widget(StatusBar { app }, main_chunks[1]);
                }
            }

            // overlay popups
            match app.popup {
                Popup::FileTree => {
                    let tree_width = (full_area.width * 35 / 100).max(30).min(60);
                    let tree_area = Rect::new(0, 0, tree_width, full_area.height - 1);
                    frame.render_widget(FileTreeWidget { state: &app.tree_state }, tree_area);
                }
                Popup::FuzzyFinder => {
                    let popup_width = (full_area.width * 60 / 100).max(40);
                    let popup_height = (full_area.height * 60 / 100).max(10);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(FuzzyFinderWidget { state: &app.fuzzy_state }, popup_area);
                }
                Popup::SearchProject => {
                    let popup_width = (full_area.width * 70 / 100).max(50);
                    let popup_height = (full_area.height * 70 / 100).max(15);
                    let x = (full_area.width - popup_width) / 2;
                    let y = (full_area.height - popup_height) / 4;
                    let popup_area = Rect::new(x, y, popup_width, popup_height);
                    frame.render_widget(
                        ProjectSearchWidget { state: &app.project_search_state },
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
                        ProjectReplaceWidget { state: &app.project_replace_state },
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
                        ThemeSwitcherWidget { state: &app.theme_switcher_state },
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
                        KeybindHelpWidget { state: &app.keybind_help_state },
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
                Event::Resize(w, h) => {
                    app.viewport_width = w as usize;
                    app.viewport_height = h as usize;
                }
                _ => {}
            }
        }

        app.check_autosave();

        if !app.running {
            break;
        }
    }

    Ok(())
}

fn handle_popup_input(app: &mut App, key: crossterm::event::KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

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
                                let start =
                                    crate::editor::cursor::Position::new(line, col);
                                let end =
                                    crate::editor::cursor::Position::new(line, end_col);
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
            let visible = app.viewport_height.saturating_sub(2);

            // ctrl+z/ctrl+y work on the buffer even while tree is open
            if ctrl && key.code == KeyCode::Char('z') {
                if let Some(pos) = app.buffer.apply_undo() {
                    app.cursor.move_to(pos.line, pos.col, false);
                    app.cursor.update_desired_col();
                }
                return;
            }
            if ctrl && key.code == KeyCode::Char('y') {
                if let Some(pos) = app.buffer.apply_redo() {
                    app.cursor.move_to(pos.line, pos.col, false);
                    app.cursor.update_desired_col();
                }
                return;
            }

            // handle active action input first
            if app.tree_state.action != TreeAction::None {
                match key.code {
                    KeyCode::Esc => app.tree_state.cancel_action(),
                    KeyCode::Enter => {
                        match app.tree_state.action {
                            TreeAction::NewFile => {
                                if let Some(new_path) = app.tree_state.confirm_new_file() {
                                    let _ = app.open_file(&new_path);
                                    app.popup = Popup::None;
                                }
                            }
                            TreeAction::NewFolder => {
                                app.tree_state.confirm_new_folder();
                            }
                            TreeAction::Rename => {
                                app.tree_state.confirm_rename();
                            }
                            TreeAction::Delete => {
                                app.tree_state.confirm_delete();
                            }
                            TreeAction::None => {}
                        }
                    }
                    KeyCode::Char('y') if app.tree_state.action == TreeAction::Delete => {
                        app.tree_state.confirm_delete();
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
                                app.tree_state.confirm_move();
                            }
                        }
                    } else if let Some(entry) = app.tree_state.selected_entry() {
                        if entry.is_dir {
                            app.tree_state.toggle_dir();
                        } else {
                            let path = entry.path.clone();
                            let _ = app.open_file(&path);
                            app.popup = Popup::None;
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
                        app.theme = selected;
                        // re-apply highlight colors with new theme
                        if app.highlighter.is_active() {
                            if let Some(config) = crate::syntax::highlight::Highlighter::detect_language(
                                app.buffer.file_path.as_deref().unwrap_or(std::path::Path::new("")),
                            ) {
                                app.highlighter.set_language(&config, &app.theme.colors);
                                let source = app.buffer.rope.to_string();
                                app.highlighter.parse(&source);
                                app.highlighter.compute_styles(&source);
                            }
                        }
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

        Popup::None => unreachable!(),
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
    let parsed: HeadlessInput = serde_json::from_str(&input)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

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
