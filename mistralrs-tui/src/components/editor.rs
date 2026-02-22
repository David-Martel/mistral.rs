//! Code editor component with syntax highlighting
//!
//! Provides a basic text editor with the following features:
//! - Vim-like modes (Normal, Insert, Visual)
//! - Syntax highlighting (regex-based for MVP)
//! - Line numbers
//! - Cursor navigation
//! - File loading/saving
//! - Language detection

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use regex::Regex;

/// Editor mode (vim-like)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Navigation mode
    Normal,
    /// Text entry mode
    Insert,
    /// Selection mode
    Visual,
}

impl EditorMode {
    /// Get mode display name
    pub fn name(&self) -> &str {
        match self {
            EditorMode::Normal => "NORMAL",
            EditorMode::Insert => "INSERT",
            EditorMode::Visual => "VISUAL",
        }
    }

    /// Get mode color
    pub fn color(&self) -> Color {
        match self {
            EditorMode::Normal => Color::Cyan,
            EditorMode::Insert => Color::Green,
            EditorMode::Visual => Color::Yellow,
        }
    }
}

/// Syntax highlighting pattern
#[derive(Debug, Clone)]
struct SyntaxPattern {
    regex: Regex,
    style: Style,
}

impl SyntaxPattern {
    fn new(pattern: &str, style: Style) -> Result<Self> {
        Ok(Self {
            regex: Regex::new(pattern).context("compiling regex pattern")?,
            style,
        })
    }
}

/// Language-specific syntax patterns
#[derive(Debug, Clone)]
struct LanguageSyntax {
    patterns: Vec<SyntaxPattern>,
}

impl LanguageSyntax {
    /// Create syntax patterns for Rust
    fn rust() -> Result<Self> {
        Ok(Self {
            patterns: vec![
                // Keywords (bold magenta)
                SyntaxPattern::new(
                    r"\b(fn|let|mut|const|static|struct|enum|impl|trait|pub|use|mod|crate|self|super|as|where|for|while|loop|if|else|match|return|break|continue|async|await|dyn|Box|Vec|String|Result|Option|Some|None|Ok|Err)\b",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )?,
                // Strings (green)
                SyntaxPattern::new(
                    r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"#,
                    Style::default().fg(Color::Green),
                )?,
                // Comments (dark gray)
                SyntaxPattern::new(r"//.*$", Style::default().fg(Color::DarkGray))?,
                // Numbers (yellow)
                SyntaxPattern::new(
                    r"\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b",
                    Style::default().fg(Color::Yellow),
                )?,
                // Macros (cyan)
                SyntaxPattern::new(r"\b\w+!", Style::default().fg(Color::Cyan))?,
                // Attributes (blue)
                SyntaxPattern::new(r"#\[.*?\]", Style::default().fg(Color::Blue))?,
            ],
        })
    }

    /// Create syntax patterns for Python
    fn python() -> Result<Self> {
        Ok(Self {
            patterns: vec![
                // Keywords (bold magenta)
                SyntaxPattern::new(
                    r"\b(def|class|import|from|as|if|elif|else|for|while|return|yield|break|continue|pass|try|except|finally|with|async|await|lambda|and|or|not|is|in|True|False|None)\b",
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )?,
                // Strings (green) - including triple quotes
                SyntaxPattern::new(
                    r#"(?:"""(?:[^"\\]|\\.)*"""|'''(?:[^'\\]|\\.)*'''|"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')"#,
                    Style::default().fg(Color::Green),
                )?,
                // Comments (dark gray)
                SyntaxPattern::new(r"#.*$", Style::default().fg(Color::DarkGray))?,
                // Numbers (yellow)
                SyntaxPattern::new(
                    r"\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b",
                    Style::default().fg(Color::Yellow),
                )?,
                // Decorators (cyan)
                SyntaxPattern::new(r"@\w+", Style::default().fg(Color::Cyan))?,
            ],
        })
    }

    /// Create default syntax (minimal highlighting)
    fn default() -> Result<Self> {
        Ok(Self {
            patterns: vec![
                // Strings (green)
                SyntaxPattern::new(
                    r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"#,
                    Style::default().fg(Color::Green),
                )?,
                // Numbers (yellow)
                SyntaxPattern::new(
                    r"\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b",
                    Style::default().fg(Color::Yellow),
                )?,
            ],
        })
    }

    /// Apply syntax highlighting to a line
    fn highlight_line(&self, line: &str) -> Vec<Span<'_>> {
        let mut spans = Vec::new();
        let mut last_end = 0;

        // Collect all matches with their positions
        let mut matches: Vec<(usize, usize, Style)> = Vec::new();
        for pattern in &self.patterns {
            for cap in pattern.regex.find_iter(line) {
                matches.push((cap.start(), cap.end(), pattern.style));
            }
        }

        // Sort by start position
        matches.sort_by_key(|(start, _, _)| *start);

        // Build spans, handling overlaps by taking first match
        for (start, end, style) in matches {
            if start < last_end {
                continue; // Skip overlapping matches
            }

            // Add unstyled text before this match
            if start > last_end {
                spans.push(Span::raw(line[last_end..start].to_string()));
            }

            // Add styled match
            spans.push(Span::styled(line[start..end].to_string(), style));
            last_end = end;
        }

        // Add remaining unstyled text
        if last_end < line.len() {
            spans.push(Span::raw(line[last_end..].to_string()));
        }

        if spans.is_empty() {
            spans.push(Span::raw(line.to_string()));
        }

        spans
    }
}

/// Editor state
#[derive(Debug, Clone)]
pub struct EditorState {
    /// Lines of content
    pub content: Vec<String>,
    /// Cursor row (0-indexed)
    pub cursor_row: usize,
    /// Cursor column (0-indexed)
    pub cursor_col: usize,
    /// Vertical scroll offset
    pub scroll_offset: usize,
    /// File path (if loaded from file)
    pub file_path: Option<PathBuf>,
    /// Detected language
    pub language: String,
    /// Whether content has been modified
    pub modified: bool,
    /// Current editor mode
    pub mode: EditorMode,
    /// Syntax highlighting patterns
    syntax: LanguageSyntax,
}

impl EditorState {
    /// Create a new empty editor
    pub fn new() -> Result<Self> {
        Ok(Self {
            content: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            file_path: None,
            language: "text".to_string(),
            modified: false,
            mode: EditorMode::Normal,
            syntax: LanguageSyntax::default()?,
        })
    }

    /// Open a file in the editor
    pub fn open_file(path: &Path) -> Result<Self> {
        let content_str = fs::read_to_string(path)
            .with_context(|| format!("reading file: {}", path.display()))?;

        let content: Vec<String> = if content_str.is_empty() {
            vec![String::new()]
        } else {
            content_str.lines().map(|s| s.to_string()).collect()
        };

        let language = detect_language(path);
        let syntax = match language.as_str() {
            "rust" => LanguageSyntax::rust()?,
            "python" => LanguageSyntax::python()?,
            _ => LanguageSyntax::default()?,
        };

        Ok(Self {
            content,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            file_path: Some(path.to_path_buf()),
            language,
            modified: false,
            mode: EditorMode::Normal,
            syntax,
        })
    }

    /// Save the current content to file
    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let content_str = self.content.join("\n");
            fs::write(path, content_str)
                .with_context(|| format!("writing file: {}", path.display()))?;
            self.modified = false;
            Ok(())
        } else {
            anyhow::bail!("No file path set");
        }
    }

    /// Save to a specific path
    pub fn save_as(&mut self, path: &Path) -> Result<()> {
        let content_str = self.content.join("\n");
        fs::write(path, content_str)
            .with_context(|| format!("writing file: {}", path.display()))?;

        self.file_path = Some(path.to_path_buf());
        self.modified = false;

        // Update language detection
        self.language = detect_language(path);
        self.syntax = match self.language.as_str() {
            "rust" => LanguageSyntax::rust()?,
            "python" => LanguageSyntax::python()?,
            _ => LanguageSyntax::default()?,
        };

        Ok(())
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, c: char) {
        if c == '\n' {
            self.insert_newline();
            return;
        }

        if self.cursor_row >= self.content.len() {
            self.content.push(String::new());
        }

        let line = &mut self.content[self.cursor_row];
        if self.cursor_col >= line.len() {
            line.push(c);
            self.cursor_col = line.len();
        } else {
            line.insert(self.cursor_col, c);
            self.cursor_col += 1;
        }

        self.modified = true;
    }

    /// Insert a newline at the cursor position
    fn insert_newline(&mut self) {
        if self.cursor_row >= self.content.len() {
            self.content.push(String::new());
            self.cursor_row = self.content.len() - 1;
            self.cursor_col = 0;
            return;
        }

        // Clone the line to avoid borrowing issues
        let line = self.content[self.cursor_row].clone();
        let (before, after) = line.split_at(self.cursor_col);

        self.content[self.cursor_row] = before.to_string();
        self.content.insert(self.cursor_row + 1, after.to_string());

        self.cursor_row += 1;
        self.cursor_col = 0;
        self.modified = true;
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_char(&mut self) {
        if self.cursor_col == 0 {
            // At start of line - join with previous line
            if self.cursor_row > 0 {
                let current_line = self.content.remove(self.cursor_row);
                self.cursor_row -= 1;
                self.cursor_col = self.content[self.cursor_row].len();
                self.content[self.cursor_row].push_str(&current_line);
                self.modified = true;
            }
        } else {
            // Delete character before cursor
            let line = &mut self.content[self.cursor_row];
            if self.cursor_col <= line.len() {
                line.remove(self.cursor_col - 1);
                self.cursor_col -= 1;
                self.modified = true;
            }
        }
    }

    /// Delete the character at the cursor (delete key)
    pub fn delete_char_forward(&mut self) {
        if self.cursor_row >= self.content.len() {
            return;
        }

        let line = &mut self.content[self.cursor_row];
        if self.cursor_col < line.len() {
            line.remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_row < self.content.len() - 1 {
            // At end of line - join with next line
            let next_line = self.content.remove(self.cursor_row + 1);
            self.content[self.cursor_row].push_str(&next_line);
            self.modified = true;
        }
    }

    /// Move cursor up
    pub fn navigate_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_cursor_col();
            self.scroll_to_cursor();
        }
    }

    /// Move cursor down
    pub fn navigate_down(&mut self) {
        if self.cursor_row < self.content.len().saturating_sub(1) {
            self.cursor_row += 1;
            self.clamp_cursor_col();
            self.scroll_to_cursor();
        }
    }

    /// Move cursor left
    pub fn navigate_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.content[self.cursor_row].len();
            self.scroll_to_cursor();
        }
    }

    /// Move cursor right
    pub fn navigate_right(&mut self) {
        if self.cursor_row >= self.content.len() {
            return;
        }

        let line_len = self.content[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row < self.content.len() - 1 {
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.scroll_to_cursor();
        }
    }

    /// Move cursor to start of line
    pub fn navigate_line_start(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line
    pub fn navigate_line_end(&mut self) {
        if self.cursor_row < self.content.len() {
            self.cursor_col = self.content[self.cursor_row].len();
        }
    }

    /// Move cursor to top of file
    pub fn navigate_top(&mut self) {
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Move cursor to bottom of file
    pub fn navigate_bottom(&mut self) {
        self.cursor_row = self.content.len().saturating_sub(1);
        self.clamp_cursor_col();
        self.scroll_to_cursor();
    }

    /// Clamp cursor column to valid range for current line
    fn clamp_cursor_col(&mut self) {
        if self.cursor_row < self.content.len() {
            let line_len = self.content[self.cursor_row].len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }

    /// Scroll view to ensure cursor is visible
    fn scroll_to_cursor(&mut self) {
        // This will be used during rendering to adjust scroll_offset
        // The actual adjustment happens in render_editor based on visible height
    }

    /// Adjust scroll offset based on visible height
    pub fn adjust_scroll(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        // Scroll down if cursor is below visible area
        if self.cursor_row >= self.scroll_offset + visible_height {
            self.scroll_offset = self.cursor_row - visible_height + 1;
        }

        // Scroll up if cursor is above visible area
        if self.cursor_row < self.scroll_offset {
            self.scroll_offset = self.cursor_row;
        }
    }

    /// Set editor mode
    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

    /// Get current line content
    pub fn current_line(&self) -> &str {
        if self.cursor_row < self.content.len() {
            &self.content[self.cursor_row]
        } else {
            ""
        }
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.content.len()
    }

    /// Get file name for display
    pub fn file_name(&self) -> String {
        self.file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[No Name]")
            .to_string()
    }
}

impl Default for EditorState {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            content: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            file_path: None,
            language: "text".to_string(),
            modified: false,
            mode: EditorMode::Normal,
            syntax: LanguageSyntax {
                patterns: Vec::new(),
            },
        })
    }
}

/// Editor component wrapper (stateless rendering)
pub struct Editor;

impl Editor {
    /// Render the editor UI
    pub fn render(frame: &mut Frame, area: Rect, state: &mut EditorState, focused: bool) {
        // Calculate visible area (subtract borders)
        let visible_height = area.height.saturating_sub(2) as usize;

        // Adjust scroll offset to keep cursor visible
        state.adjust_scroll(visible_height);

        // Build title with file info
        let title = format!(
            "{} [{}] {}",
            state.file_name(),
            state.language,
            if state.modified { "[+]" } else { "" }
        );

        let mut block = Block::default().title(title).borders(Borders::ALL);

        if focused {
            block = block.border_style(Style::default().fg(state.mode.color()));
        }

        // Calculate line number width (for display)
        let line_num_width = format!("{}", state.content.len()).len().max(3);

        // Build visible lines with syntax highlighting
        let visible_lines: Vec<Line> = state
            .content
            .iter()
            .enumerate()
            .skip(state.scroll_offset)
            .take(visible_height)
            .map(|(idx, line)| {
                let line_num = format!("{:>width$} ", idx + 1, width = line_num_width);
                let mut spans = vec![Span::styled(line_num, Style::default().fg(Color::DarkGray))];

                // Apply syntax highlighting
                let mut highlighted = state.syntax.highlight_line(line);
                spans.append(&mut highlighted);

                // Highlight current line
                if idx == state.cursor_row {
                    Line::from(spans)
                        .style(Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White))
                } else {
                    Line::from(spans)
                }
            })
            .collect();

        let paragraph = Paragraph::new(visible_lines).block(block);
        frame.render_widget(paragraph, area);

        // Calculate cursor position for display (accounting for line numbers and borders)
        let cursor_x = area.x + 1 + line_num_width as u16 + 1 + state.cursor_col as u16;
        let cursor_y = area.y + 1 + (state.cursor_row - state.scroll_offset) as u16;

        // Set cursor position if in Insert mode and focused
        if state.mode == EditorMode::Insert && focused {
            if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

/// Render the editor component
///
/// This is a convenience function that matches the pattern used in other components
pub fn render_editor(frame: &mut Frame, area: Rect, state: &mut EditorState, focused: bool) {
    Editor::render(frame, area, state, focused);
}

/// Detect programming language from file extension
fn detect_language(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "py" => "python",
            "js" | "jsx" => "javascript",
            "ts" | "tsx" => "typescript",
            "c" | "h" => "c",
            "cpp" | "cc" | "cxx" | "hpp" => "cpp",
            "go" => "go",
            "java" => "java",
            "md" => "markdown",
            "json" => "json",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            _ => "text",
        })
        .unwrap_or("text")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.rs")), "rust");
        assert_eq!(detect_language(Path::new("script.py")), "python");
        assert_eq!(detect_language(Path::new("app.js")), "javascript");
        assert_eq!(detect_language(Path::new("README.md")), "markdown");
        assert_eq!(detect_language(Path::new("config.toml")), "toml");
        assert_eq!(detect_language(Path::new("data.txt")), "text");
    }

    #[test]
    fn test_editor_new() {
        let editor = EditorState::new().unwrap();
        assert_eq!(editor.content.len(), 1);
        assert_eq!(editor.cursor_row, 0);
        assert_eq!(editor.cursor_col, 0);
        assert_eq!(editor.mode, EditorMode::Normal);
        assert!(!editor.modified);
    }

    #[test]
    fn test_insert_char() {
        let mut editor = EditorState::new().unwrap();
        editor.set_mode(EditorMode::Insert);

        editor.insert_char('h');
        editor.insert_char('i');
        assert_eq!(editor.content[0], "hi");
        assert_eq!(editor.cursor_col, 2);
        assert!(editor.modified);
    }

    #[test]
    fn test_delete_char() {
        let mut editor = EditorState::new().unwrap();
        editor.set_mode(EditorMode::Insert);

        editor.insert_char('h');
        editor.insert_char('i');
        editor.delete_char();
        assert_eq!(editor.content[0], "h");
        assert_eq!(editor.cursor_col, 1);
    }

    #[test]
    fn test_navigation() {
        let mut editor = EditorState::new().unwrap();
        editor.content = vec![
            "line1".to_string(),
            "line2".to_string(),
            "line3".to_string(),
        ];

        editor.navigate_down();
        assert_eq!(editor.cursor_row, 1);

        editor.navigate_down();
        assert_eq!(editor.cursor_row, 2);

        editor.navigate_up();
        assert_eq!(editor.cursor_row, 1);

        editor.navigate_line_end();
        assert_eq!(editor.cursor_col, 5);

        editor.navigate_line_start();
        assert_eq!(editor.cursor_col, 0);
    }

    #[test]
    fn test_newline_insertion() {
        let mut editor = EditorState::new().unwrap();
        editor.set_mode(EditorMode::Insert);

        editor.insert_char('h');
        editor.insert_char('i');
        editor.insert_char('\n');
        editor.insert_char('b');
        editor.insert_char('y');

        assert_eq!(editor.content.len(), 2);
        assert_eq!(editor.content[0], "hi");
        assert_eq!(editor.content[1], "by");
        assert_eq!(editor.cursor_row, 1);
        assert_eq!(editor.cursor_col, 2);
    }

    #[test]
    fn test_mode_switching() {
        let mut editor = EditorState::new().unwrap();
        assert_eq!(editor.mode, EditorMode::Normal);

        editor.set_mode(EditorMode::Insert);
        assert_eq!(editor.mode, EditorMode::Insert);

        editor.set_mode(EditorMode::Visual);
        assert_eq!(editor.mode, EditorMode::Visual);
    }

    #[test]
    fn test_syntax_rust_keywords() {
        let syntax = LanguageSyntax::rust().unwrap();
        let line = "fn main() { let x = 42; }";
        let spans = syntax.highlight_line(line);

        // Should have highlighted spans for keywords
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_syntax_python_keywords() {
        let syntax = LanguageSyntax::python().unwrap();
        let line = "def hello(): print('world')";
        let spans = syntax.highlight_line(line);

        // Should have highlighted spans for keywords and strings
        assert!(!spans.is_empty());
    }
}
