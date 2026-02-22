# TUI Components

Reusable UI components for the mistralrs-tui application.

## File Explorer Component

A fully-featured file browser component with tree view, filtering, and keyboard navigation.

### Features

- **Tree View**: Hierarchical directory structure with expand/collapse
- **File Metadata**: Display file sizes and modification times
- **Filtering**: Glob pattern filtering (e.g., `*.rs`, `test*`)
- **Keyboard Navigation**: Vim-style and arrow key navigation
- **Visual Indicators**: Emoji icons for directories (📁/📂) and files (📄)
- **Focus Management**: Integrates with the TUI focus system

### Quick Start

```rust
use mistralrs_tui::components::{FileExplorerState, render_file_explorer};

// Create explorer at current directory
let mut explorer = FileExplorerState::new(".")?;

// In your render loop
render_file_explorer(frame, area, &explorer, focused);

// Handle keyboard input
explorer.navigate_down();  // Move cursor down
explorer.enter_selection()?;  // Enter directory or select file
explorer.toggle_expand()?;  // Expand/collapse directory
```

### API Documentation

#### `FileEntry`

Represents a single file or directory:

```rust
pub struct FileEntry {
    pub path: PathBuf,           // Full path
    pub name: String,            // Display name (filename)
    pub is_dir: bool,            // True if directory
    pub size: Option<u64>,       // File size (None for dirs)
    pub modified: Option<SystemTime>, // Last modified time
    pub depth: usize,            // Tree depth for indentation
    pub is_expanded: bool,       // Expansion state (dirs only)
}
```

**Methods:**
- `from_path(path: &Path, depth: usize) -> Result<Self>` - Create from path
- `display_line(&self) -> String` - Format for display with icons

#### `FileExplorerState`

Manages the file explorer state:

```rust
pub struct FileExplorerState {
    pub current_dir: PathBuf,              // Current directory
    pub entries: Vec<FileEntry>,           // Visible entries (flattened tree)
    pub cursor: usize,                     // Current selection index
    pub expanded_dirs: HashSet<PathBuf>,   // Set of expanded directories
    pub filter: Option<String>,            // Optional filter pattern
}
```

**Methods:**

**Initialization:**
- `new(path: impl AsRef<Path>) -> Result<Self>` - Create explorer at path
- `refresh() -> Result<()>` - Reload directory contents

**Navigation:**
- `navigate_up()` - Move cursor up one entry
- `navigate_down()` - Move cursor down one entry
- `navigate_top()` - Jump to first entry
- `navigate_bottom()` - Jump to last entry
- `enter_selection() -> Result<Option<PathBuf>>` - Enter directory or return file path
- `go_parent() -> Result<()>` - Navigate to parent directory
- `toggle_expand() -> Result<()>` - Expand/collapse selected directory

**Filtering:**
- `set_filter(pattern: Option<String>) -> Result<()>` - Set glob filter
- `clear_filter() -> Result<()>` - Clear filter

**Query:**
- `current_entry() -> Option<&FileEntry>` - Get selected entry
- `current_dir() -> &Path` - Get current directory path

#### `FileExplorer` / `render_file_explorer`

Render function for the component:

```rust
pub fn render_file_explorer(
    frame: &mut Frame,
    area: Rect,
    state: &FileExplorerState,
    focused: bool,
)
```

**Parameters:**
- `frame` - Ratatui frame to render to
- `area` - Rectangle area to render within
- `state` - File explorer state to display
- `focused` - Whether this component has focus (affects border color)

### Keyboard Shortcuts

| Key         | Action                              |
|-------------|-------------------------------------|
| ↑ / k       | Move cursor up                      |
| ↓ / j       | Move cursor down                    |
| Home / g    | Jump to top                         |
| End / G     | Jump to bottom                      |
| Enter       | Enter directory or select file      |
| Backspace/h | Go to parent directory              |
| Space / l   | Expand/collapse directory           |
| r           | Refresh file list                   |

### Examples

#### Basic Usage

```rust
use mistralrs_tui::components::FileExplorerState;

// Create explorer
let mut explorer = FileExplorerState::new("/home/user")?;

// Navigate
explorer.navigate_down();
explorer.navigate_down();
explorer.enter_selection()?;  // Enter selected directory

// Expand/collapse
explorer.toggle_expand()?;

// Refresh
explorer.refresh()?;
```

#### With Filtering

```rust
// Show only Rust files
explorer.set_filter(Some("*.rs".to_string()))?;

// Show files starting with "test"
explorer.set_filter(Some("test*".to_string()))?;

// Clear filter
explorer.clear_filter()?;
```

#### Integration with TUI

```rust
use ratatui::Frame;
use mistralrs_tui::components::render_file_explorer;

fn render(frame: &mut Frame, app: &App) {
    if let Some(explorer_state) = app.file_explorer_state() {
        let focused = matches!(app.focus(), FocusArea::FileExplorer);
        render_file_explorer(frame, area, explorer_state, focused);
    }
}
```

### Demo Application

Run the interactive demo:

```bash
cargo run --example file_explorer_demo --features tui-agent,terminal
```

Controls in demo:
- ↑/↓ or j/k: Navigate
- Enter: Open directory or select file
- Backspace or h: Go to parent
- Space or l: Expand/collapse directory
- r: Refresh
- q or Ctrl+C: Quit

### Testing

Run unit tests:

```bash
cargo test -p mistralrs-tui --features tui-agent --lib file_explorer
```

Test coverage includes:
- File size formatting
- Glob pattern matching
- Entry display formatting
- Directory icon states

### Performance Considerations

**Large Directories:**
For directories with thousands of files:
1. Use filtering to reduce displayed entries
2. Only expanded directories load their contents
3. Consider implementing pagination for very large lists

**Refresh Strategy:**
- Call `refresh()` only when needed (after file operations)
- Expanded state is preserved across refreshes
- Cursor position is maintained within valid range

**Memory Usage:**
- Only visible entries are loaded (tree flattening)
- Collapsed directories don't load children
- Filter is applied during directory traversal

### Platform Support

The component works on all platforms:
- **Windows**: Full support with proper path handling
- **Linux**: Full support with proper path handling
- **macOS**: Full support with proper path handling

Path canonicalization is used to handle relative paths and symbolic links consistently across platforms.

### Error Handling

All filesystem operations return `Result<()>` or `Result<T>`:

```rust
// Handle errors explicitly
if let Err(e) = explorer.go_parent() {
    eprintln!("Failed to navigate to parent: {}", e);
}

// Or propagate with ?
explorer.refresh()?;
explorer.toggle_expand()?;
```

Common errors:
- Permission denied (inaccessible directories)
- Path not found (deleted while browsing)
- I/O errors (network filesystems, etc.)

### Customization

#### Custom Icons

Modify `FileEntry::display_line()` to change icons:

```rust
let icon = if self.is_dir {
    if self.is_expanded { "[+]" } else { "[-]" }
} else {
    "   "
};
```

#### Custom Sorting

Modify `load_entries()` to change sort order:

```rust
// Sort by modification time (newest first)
entries.sort_by(|a, b| {
    let a_time = a.metadata().and_then(|m| m.modified()).ok();
    let b_time = b.metadata().and_then(|m| m.modified()).ok();
    b_time.cmp(&a_time)
});
```

#### Custom Filtering

Replace `glob_match()` with more sophisticated pattern matching:

```rust
use regex::Regex;

fn regex_match(text: &str, pattern: &str) -> bool {
    Regex::new(pattern)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}
```

### Future Enhancements

Planned features:
- [ ] File preview panel
- [ ] Multi-selection support
- [ ] File operations (copy, move, delete)
- [ ] Git status indicators
- [ ] Custom file type icons
- [ ] Bookmarks/favorites
- [ ] Search within files
- [ ] Multiple sort modes
- [ ] Hidden file toggle

### Contributing

When contributing to the file explorer component:

1. **Maintain backwards compatibility** - Don't change public API
2. **Add tests** - Cover new functionality
3. **Update documentation** - Keep README in sync
4. **Follow conventions** - Match existing code style
5. **Test on all platforms** - Windows, Linux, macOS

### License

MIT License - See project root LICENSE file.
