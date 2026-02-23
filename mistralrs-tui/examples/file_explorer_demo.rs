//! Demo application showing the File Explorer component in action
//!
//! Run with: cargo run --example file_explorer_demo --features tui-agent,terminal

#[cfg(all(feature = "tui-agent", feature = "terminal"))]
fn main() -> anyhow::Result<()> {
    use crossterm::{
        event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    };
    use mistralrs_tui::components::{FileExplorer, FileExplorerState};
    use ratatui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Style},
        text::Line,
        widgets::{Block, Borders, Paragraph},
        Terminal,
    };
    use std::io;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create file explorer state
    let start_path = std::env::current_dir()?;
    let mut explorer_state = FileExplorerState::new(&start_path)?;
    let mut status_message = String::from("Use ↑/↓ to navigate, Enter to open, Backspace for parent, Space to expand/collapse, 'q' to quit");

    // Main event loop
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),    // File explorer
                    Constraint::Length(3), // Status bar
                ])
                .split(frame.area());

            // Render file explorer
            FileExplorer::render(frame, chunks[0], &explorer_state, true);

            // Render status bar
            let status_block = Block::default()
                .title("Status")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Gray));

            let status_text = if let Some(entry) = explorer_state.current_entry() {
                format!("{} | Selected: {}", status_message, entry.path.display())
            } else {
                status_message.clone()
            };

            let status_paragraph = Paragraph::new(Line::from(status_text)).block(status_block);

            frame.render_widget(status_paragraph, chunks[1]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            break;
                        }
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                            explorer_state.navigate_up();
                        }
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                            explorer_state.navigate_down();
                        }
                        (KeyCode::Home, _) | (KeyCode::Char('g'), _) => {
                            explorer_state.navigate_top();
                        }
                        (KeyCode::End, _) | (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                            explorer_state.navigate_bottom();
                        }
                        (KeyCode::Enter, _) => {
                            if let Ok(Some(path)) = explorer_state.enter_selection() {
                                status_message = format!("Selected file: {}", path.display());
                            }
                        }
                        (KeyCode::Backspace, _) | (KeyCode::Char('h'), _) => {
                            explorer_state.go_parent()?;
                        }
                        (KeyCode::Char(' '), _) | (KeyCode::Char('l'), _) => {
                            explorer_state.toggle_expand()?;
                        }
                        (KeyCode::Char('/'), _) => {
                            // In a real app, you'd open an input dialog here
                            status_message =
                                "Filter: (demo only - feature not implemented in this example)"
                                    .to_string();
                        }
                        (KeyCode::Char('c'), KeyModifiers::ALT) => {
                            explorer_state.clear_filter()?;
                            status_message = "Filter cleared".to_string();
                        }
                        (KeyCode::Char('r'), _) => {
                            explorer_state.refresh()?;
                            status_message = "Refreshed".to_string();
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    println!("File Explorer Demo exited successfully!");
    Ok(())
}

#[cfg(not(all(feature = "tui-agent", feature = "terminal")))]
fn main() {
    eprintln!("This example requires both 'tui-agent' and 'terminal' features.");
    eprintln!("Run with: cargo run --example file_explorer_demo --features tui-agent,terminal");
    std::process::exit(1);
}
