//! Simple demonstration of the Editor component
//!
//! Run with: cargo run --example editor_demo --features tui-agent

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use mistralrs_tui::components::{render_editor, EditorMode, EditorState};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create editor with sample content
    let mut editor = EditorState::new()?;
    editor.content = vec![
        "// Welcome to the Editor Component Demo!".to_string(),
        "".to_string(),
        "fn main() {".to_string(),
        "    println!(\"Hello, world!\");".to_string(),
        "    ".to_string(),
        "    // This is a syntax-highlighted Rust editor".to_string(),
        "    let x = 42;".to_string(),
        "    let name = \"mistral.rs\";".to_string(),
        "}".to_string(),
        "".to_string(),
        "// Try it out:".to_string(),
        "// - Press 'i' to enter Insert mode".to_string(),
        "// - Press ESC to return to Normal mode".to_string(),
        "// - Use arrow keys or h/j/k/l to navigate".to_string(),
        "// - Press 'q' in Normal mode to quit".to_string(),
    ];
    editor.language = "rust".to_string();

    // Run event loop
    let result = run_app(&mut terminal, &mut editor);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    editor: &mut EditorState,
) -> Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // Editor
                    Constraint::Length(3), // Help bar
                ])
                .split(f.area());

            // Render editor
            render_editor(f, chunks[0], editor, true);

            // Render help bar
            render_help_bar(f, chunks[1], editor);
        })?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            match editor.mode {
                EditorMode::Normal => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('i') => editor.set_mode(EditorMode::Insert),
                    KeyCode::Char('v') => editor.set_mode(EditorMode::Visual),
                    KeyCode::Char('h') | KeyCode::Left => editor.navigate_left(),
                    KeyCode::Char('j') | KeyCode::Down => editor.navigate_down(),
                    KeyCode::Char('k') | KeyCode::Up => editor.navigate_up(),
                    KeyCode::Char('l') | KeyCode::Right => editor.navigate_right(),
                    KeyCode::Char('0') | KeyCode::Home => editor.navigate_line_start(),
                    KeyCode::Char('$') | KeyCode::End => editor.navigate_line_end(),
                    KeyCode::Char('g') => editor.navigate_top(),
                    KeyCode::Char('G') => editor.navigate_bottom(),
                    _ => {}
                },
                EditorMode::Insert => {
                    match key.code {
                        KeyCode::Esc => editor.set_mode(EditorMode::Normal),
                        KeyCode::Char(c) => {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                if c == 's' {
                                    // Save shortcut (no-op in demo)
                                }
                            } else {
                                editor.insert_char(c);
                            }
                        }
                        KeyCode::Enter => editor.insert_char('\n'),
                        KeyCode::Backspace => editor.delete_char(),
                        KeyCode::Delete => editor.delete_char_forward(),
                        KeyCode::Left => editor.navigate_left(),
                        KeyCode::Right => editor.navigate_right(),
                        KeyCode::Up => editor.navigate_up(),
                        KeyCode::Down => editor.navigate_down(),
                        KeyCode::Home => editor.navigate_line_start(),
                        KeyCode::End => editor.navigate_line_end(),
                        _ => {}
                    }
                }
                EditorMode::Visual => match key.code {
                    KeyCode::Esc => editor.set_mode(EditorMode::Normal),
                    KeyCode::Char('h') | KeyCode::Left => editor.navigate_left(),
                    KeyCode::Char('j') | KeyCode::Down => editor.navigate_down(),
                    KeyCode::Char('k') | KeyCode::Up => editor.navigate_up(),
                    KeyCode::Char('l') | KeyCode::Right => editor.navigate_right(),
                    _ => {}
                },
            }
        }
    }

    Ok(())
}

fn render_help_bar(f: &mut ratatui::Frame, area: Rect, editor: &EditorState) {
    let help_text = match editor.mode {
        EditorMode::Normal => {
            "NORMAL | q: quit | i: insert | v: visual | hjkl/arrows: move | 0/$: line start/end | g/G: top/bottom"
        }
        EditorMode::Insert => {
            "INSERT | ESC: normal mode | Type to insert | Enter: new line | Backspace/Delete: delete"
        }
        EditorMode::Visual => {
            "VISUAL | ESC: normal mode | hjkl/arrows: move | (selection not yet implemented)"
        }
    };

    let help_spans = vec![
        Span::styled(
            format!(" {} ", editor.mode.name()),
            Style::default().fg(Color::Black).bg(editor.mode.color()),
        ),
        Span::raw(" | "),
        Span::raw(help_text),
    ];

    let help_paragraph = Paragraph::new(Line::from(help_spans))
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(help_paragraph, area);
}
