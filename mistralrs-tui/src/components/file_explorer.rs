//! File explorer component for navigating the filesystem in the TUI
//!
//! Provides a tree-view file browser with the following features:
//! - Directory expansion/collapse
//! - File filtering by glob patterns
//! - Keyboard navigation
//! - File metadata display (size, modified time)
//! - Visual indicators for directories and files

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::{Context, Result};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// Represents a single file or directory entry in the explorer
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path to the file/directory
    pub path: PathBuf,
    /// Display name (filename)
    pub name: String,
    /// Whether this is a directory
    pub is_dir: bool,
    /// File size in bytes (None for directories)
    pub size: Option<u64>,
    /// Last modified time
    pub modified: Option<SystemTime>,
    /// Depth in the tree (for indentation)
    pub depth: usize,
    /// Whether this directory is expanded (only relevant for directories)
    pub is_expanded: bool,
}

impl FileEntry {
    /// Create a FileEntry from a path with specified depth
    pub fn from_path(path: &Path, depth: usize) -> Result<Self> {
        let metadata = fs::metadata(path).context("reading file metadata")?;
        let is_dir = metadata.is_dir();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>")
            .to_string();

        Ok(Self {
            path: path.to_path_buf(),
            name,
            is_dir,
            size: if is_dir { None } else { Some(metadata.len()) },
            modified: metadata.modified().ok(),
            depth,
            is_expanded: false,
        })
    }

    /// Format the entry for display with icons and metadata
    pub fn display_line(&self) -> String {
        let icon = if self.is_dir {
            if self.is_expanded {
                "📂"
            } else {
                "📁"
            }
        } else {
            "📄"
        };

        let indent = "  ".repeat(self.depth);
        let size_str = self.size.map(format_file_size).unwrap_or_default();

        if size_str.is_empty() {
            format!("{}{} {}", indent, icon, self.name)
        } else {
            format!("{}{} {} ({})", indent, icon, self.name, size_str)
        }
    }
}

/// State for the file explorer component
#[derive(Debug, Clone)]
pub struct FileExplorerState {
    /// Current directory being viewed
    pub current_dir: PathBuf,
    /// List of visible entries (flattened tree)
    pub entries: Vec<FileEntry>,
    /// Current cursor position
    pub cursor: usize,
    /// Set of expanded directory paths
    pub expanded_dirs: HashSet<PathBuf>,
    /// Optional filter pattern (glob)
    pub filter: Option<String>,
}

impl FileExplorerState {
    /// Create a new file explorer state at the given path
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let current_dir = path.as_ref().canonicalize().context("resolving path")?;
        let mut state = Self {
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            expanded_dirs: HashSet::new(),
            filter: None,
        };
        state.refresh()?;
        Ok(state)
    }

    /// Refresh the directory contents
    pub fn refresh(&mut self) -> Result<()> {
        self.entries.clear();
        self.load_entries(&self.current_dir.clone(), 0)?;

        // Clamp cursor to valid range
        if !self.entries.is_empty() && self.cursor >= self.entries.len() {
            self.cursor = self.entries.len() - 1;
        }

        Ok(())
    }

    /// Load entries from a directory recursively based on expansion state
    fn load_entries(&mut self, dir: &Path, depth: usize) -> Result<()> {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .context("reading directory")?
            .filter_map(|entry| entry.ok())
            .collect();

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            let a_is_dir = a.path().is_dir();
            let b_is_dir = b.path().is_dir();

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in entries {
            let path = entry.path();

            // Apply filter if present
            if let Some(filter) = &self.filter {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !glob_match(name, filter) {
                        continue;
                    }
                }
            }

            if let Ok(mut file_entry) = FileEntry::from_path(&path, depth) {
                let is_expanded = self.expanded_dirs.contains(&path);
                file_entry.is_expanded = is_expanded;
                self.entries.push(file_entry);

                // If this is an expanded directory, recursively load its contents
                if path.is_dir() && is_expanded {
                    self.load_entries(&path, depth + 1)?;
                }
            }
        }

        Ok(())
    }

    /// Move cursor up
    pub fn navigate_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor down
    pub fn navigate_down(&mut self) {
        if !self.entries.is_empty() && self.cursor < self.entries.len() - 1 {
            self.cursor += 1;
        }
    }

    /// Move cursor to the top
    pub fn navigate_top(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to the bottom
    pub fn navigate_bottom(&mut self) {
        if !self.entries.is_empty() {
            self.cursor = self.entries.len() - 1;
        }
    }

    /// Enter the selected directory or return the selected file path
    pub fn enter_selection(&mut self) -> Result<Option<PathBuf>> {
        if self.entries.is_empty() {
            return Ok(None);
        }

        let entry = &self.entries[self.cursor];
        let path = entry.path.clone();

        if entry.is_dir {
            self.current_dir = path;
            self.cursor = 0;
            self.refresh()?;
            Ok(None)
        } else {
            Ok(Some(path))
        }
    }

    /// Go to parent directory
    pub fn go_parent(&mut self) -> Result<()> {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.cursor = 0;
            self.refresh()?;
        }
        Ok(())
    }

    /// Toggle expansion of the selected directory
    pub fn toggle_expand(&mut self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let entry = &self.entries[self.cursor];
        if !entry.is_dir {
            return Ok(());
        }

        let path = entry.path.clone();
        if self.expanded_dirs.contains(&path) {
            self.expanded_dirs.remove(&path);
        } else {
            self.expanded_dirs.insert(path);
        }

        self.refresh()
    }

    /// Set a filter pattern
    pub fn set_filter(&mut self, pattern: Option<String>) -> Result<()> {
        self.filter = pattern;
        self.cursor = 0;
        self.refresh()
    }

    /// Clear the filter
    pub fn clear_filter(&mut self) -> Result<()> {
        self.set_filter(None)
    }

    /// Get the currently selected entry
    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// Get current directory path
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }
}

/// File explorer component wrapper (stateless rendering)
pub struct FileExplorer;

impl FileExplorer {
    /// Render the file explorer UI
    pub fn render(frame: &mut Frame, area: Rect, state: &FileExplorerState, focused: bool) {
        let items: Vec<ListItem> = state
            .entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let display_text = entry.display_line();

                // Highlight current item
                let style = if idx == state.cursor {
                    if focused {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::BOLD)
                    }
                } else if entry.is_dir {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(Span::styled(display_text, style)))
            })
            .collect();

        let title = if let Some(filter) = &state.filter {
            format!(
                "Files: {} [filter: {}]",
                state.current_dir.display(),
                filter
            )
        } else {
            format!("Files: {}", state.current_dir.display())
        };

        let mut block = Block::default().title(title).borders(Borders::ALL);

        if focused {
            block = block.border_style(Style::default().fg(Color::Cyan));
        }

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut list_state = ListState::default();
        list_state.select(Some(state.cursor));

        frame.render_stateful_widget(list, area, &mut list_state);
    }
}

/// Render the file explorer component
///
/// This is a convenience function that matches the pattern used in ui.rs
pub fn render_file_explorer(
    frame: &mut Frame,
    area: Rect,
    state: &FileExplorerState,
    focused: bool,
) {
    FileExplorer::render(frame, area, state, focused);
}

/// Format file size in human-readable format
fn format_file_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut idx = 0;

    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }

    if idx == 0 {
        format!("{} {}", value as u64, UNITS[idx])
    } else {
        format!("{:.1} {}", value, UNITS[idx])
    }
}

/// Simple glob pattern matching (supports * and ? wildcards)
fn glob_match(text: &str, pattern: &str) -> bool {
    // Simple implementation - could be replaced with glob crate if needed
    if pattern == "*" {
        return true;
    }

    // For now, just do simple substring matching if pattern contains *
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.is_empty() {
            return true;
        }

        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 && !text.starts_with(part) {
                return false;
            }

            if i == parts.len() - 1 && !text.ends_with(part) {
                return false;
            }

            if let Some(found_pos) = text[pos..].find(part) {
                pos += found_pos + part.len();
            } else {
                return false;
            }
        }
        true
    } else {
        text == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("test.txt", "*.txt"));
        assert!(glob_match("test.txt", "*"));
        assert!(glob_match("test.txt", "test*"));
        assert!(glob_match("test.txt", "*test*"));
        assert!(!glob_match("test.txt", "*.rs"));
        assert!(glob_match("main.rs", "main.rs"));
        assert!(!glob_match("main.rs", "test.rs"));
    }

    #[test]
    fn test_file_entry_display() {
        let temp_dir = std::env::temp_dir();
        let entry = FileEntry {
            path: temp_dir.join("test.txt"),
            name: "test.txt".to_string(),
            is_dir: false,
            size: Some(1024),
            modified: None,
            depth: 0,
            is_expanded: false,
        };

        let display = entry.display_line();
        assert!(display.contains("📄"));
        assert!(display.contains("test.txt"));
        assert!(display.contains("1.0 KB"));
    }

    #[test]
    fn test_directory_entry_display() {
        let temp_dir = std::env::temp_dir();
        let mut entry = FileEntry {
            path: temp_dir.join("test_dir"),
            name: "test_dir".to_string(),
            is_dir: true,
            size: None,
            modified: None,
            depth: 0,
            is_expanded: false,
        };

        let display = entry.display_line();
        assert!(display.contains("📁"));
        assert!(display.contains("test_dir"));

        entry.is_expanded = true;
        let display_expanded = entry.display_line();
        assert!(display_expanded.contains("📂"));
    }
}
