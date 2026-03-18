mod app;
mod ui;
mod managers;
mod cheatsheet;
mod snapshot;

use anyhow::Result;
use app::{App, Panel};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::stdout;

fn main() -> Result<()> {
    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize app
    let mut app = App::new();
    app.load_tools();

    // Run the event loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        app.maybe_clear_status();
        ui::draw(terminal, app)?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Handle search input mode first
                if app.search_active {
                    match key.code {
                        KeyCode::Esc => {
                            app.search_query.clear();
                            app.search_active = false;
                            app.selected_tool = 0;
                        }
                        KeyCode::Backspace => {
                            app.search_query.pop();
                            app.selected_tool = 0;
                        }
                        KeyCode::Char(c) => {
                            app.search_query.push(c);
                            app.selected_tool = 0;
                        }
                        _ => {}
                    }
                    continue;
                }

                // Global keybinds (non-search mode)
                match key.code {
                    // Quit
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }

                    // Navigation down
                    KeyCode::Char('j') | KeyCode::Down => match app.active_panel {
                        Panel::Managers => app.next_manager(),
                        Panel::Tools | Panel::Cheatsheet => app.next_tool(),
                    },

                    // Navigation up
                    KeyCode::Char('k') | KeyCode::Up => match app.active_panel {
                        Panel::Managers => app.prev_manager(),
                        Panel::Tools | Panel::Cheatsheet => app.prev_tool(),
                    },

                    // Navigate left (previous panel)
                    KeyCode::Char('h') | KeyCode::Left => match app.active_panel {
                        Panel::Managers => {} // nothing
                        Panel::Tools => app.active_panel = Panel::Managers,
                        Panel::Cheatsheet => {
                            app.active_panel = Panel::Tools;
                            app.cheatsheet = None;
                        }
                    },

                    // Navigate right / Tab (next panel)
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                        match app.active_panel {
                            Panel::Managers => app.active_panel = Panel::Tools,
                            Panel::Tools => {
                                app.active_panel = Panel::Cheatsheet;
                                app.load_cheatsheet();
                            }
                            Panel::Cheatsheet => {} // nothing
                        }
                    }

                    // Enter: on Tools panel → switch to Cheatsheet
                    KeyCode::Enter => {
                        if app.active_panel == Panel::Tools {
                            app.active_panel = Panel::Cheatsheet;
                            app.load_cheatsheet();
                        }
                    }

                    // Delete selected tool
                    KeyCode::Char('d') => {
                        if !app.show_confirm_delete
                            && app.active_panel == Panel::Tools
                            && app.selected_tool_item().is_some()
                        {
                            app.show_confirm_delete = true;
                        }
                    }

                    // Refresh
                    KeyCode::Char('r') => {
                        if !app.show_confirm_delete {
                            app.refresh();
                        }
                    }

                    // Activate search
                    KeyCode::Char('/') => {
                        app.search_active = true;
                    }

                    // Escape
                    KeyCode::Esc => {
                        if app.show_confirm_delete {
                            app.show_confirm_delete = false;
                        }
                    }

                    // Confirm delete
                    KeyCode::Char('y') => {
                        if app.show_confirm_delete {
                            if let Err(e) = app.delete_selected_tool() {
                                app.status_message = Some(format!("Error: {}", e));
                            }
                            app.show_confirm_delete = false;
                        }
                    }

                    // Dismiss delete
                    KeyCode::Char('n') => {
                        if app.show_confirm_delete {
                            app.show_confirm_delete = false;
                        }
                    }

                    // Export snapshot
                    KeyCode::Char('e') => {
                        match app.export_snapshot() {
                            Ok(_) => {}
                            Err(e) => app.status_message = Some(format!("Export failed: {}", e)),
                        }
                    }

                    // Import snapshot (CLI only in v1)
                    KeyCode::Char('i') => {
                        app.status_message = Some("Use 'lazypkg import' from command line to import".to_string());
                    }

                    // Toggle help
                    KeyCode::Char('?') => {
                        app.show_help = !app.show_help;
                    }

                    _ => {}
                }
            }
        }
    }
}
