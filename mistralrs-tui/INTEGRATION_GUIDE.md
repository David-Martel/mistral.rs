# File Explorer Integration Guide

This guide explains how to integrate the File Explorer component into the mistralrs-tui application.

## Overview

The File Explorer component (`components/file_explorer.rs`) provides a tree-view file browser with the following features:

- Directory expansion/collapse
- File filtering by glob patterns
- Keyboard navigation
- File metadata display (size, modified time)
- Visual indicators for directories (📁/📂) and files (📄)

## Component Structure

### Core Types

#### `FileEntry`
Represents a single file or directory entry:
```rust
pub struct FileEntry {
    pub path: PathBuf,           // Full path
    pub name: String,            // Display name
    pub is_dir: bool,            // Directory flag
    pub size: Option<u64>,       // File size (None for dirs)
    pub modified: Option<SystemTime>, // Last modified
    pub depth: usize,            // Tree depth for indentation
    pub is_expanded: bool,       // Expansion state
}
```

#### `FileExplorerState`
Manages the file explorer state:
```rust
pub struct FileExplorerState {
    pub current_dir: PathBuf,              // Current directory
    pub entries: Vec<FileEntry>,           // Visible entries (flattened tree)
    pub cursor: usize,                     // Current selection
    pub expanded_dirs: HashSet<PathBuf>,   // Expanded directories
    pub filter: Option<String>,            // Optional filter pattern
}
```

## Integration Steps

### Step 1: Add to FocusArea Enum

In `src/app.rs`, add a new focus area:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    Sessions,
    Chat,
    Models,
    CommandLine,
    #[cfg(feature = "tui-agent")]
    AgentTools,
    #[cfg(feature = "tui-agent")]
    AgentBrowser,
    #[cfg(feature = "tui-agent")]
    AgentHistory,
    #[cfg(feature = "tui-agent")]
    FileExplorer,  // ADD THIS
}
```

### Step 2: Add State to App

In `src/app.rs`, add file explorer state to the `App` struct:

```rust
pub struct App {
    // ... existing fields ...

    #[cfg(feature = "tui-agent")]
    file_explorer_state: Option<FileExplorerState>,
}
```

Initialize in `App::initialise()`:

```rust
#[cfg(feature = "tui-agent")]
let file_explorer_state = if let Some(sandbox_root) = agent_toolkit.as_ref()
    .map(|t| t.config().sandbox_root.clone()) {
    Some(FileExplorerState::new(&sandbox_root).ok()?)
} else {
    None
};

Ok(Self {
    // ... existing fields ...
    #[cfg(feature = "tui-agent")]
    file_explorer_state,
})
```

Add accessor methods:

```rust
#[cfg(feature = "tui-agent")]
pub fn file_explorer_state(&self) -> Option<&FileExplorerState> {
    self.file_explorer_state.as_ref()
}

#[cfg(feature = "tui-agent")]
pub fn file_explorer_state_mut(&mut self) -> Option<&mut FileExplorerState> {
    self.file_explorer_state.as_mut()
}
```

### Step 3: Add Keyboard Handlers

In `src/app.rs`, add keyboard event handlers:

```rust
#[cfg(feature = "tui-agent")]
fn handle_file_explorer_key(&mut self, key: KeyEvent) -> Result<()> {
    if let Some(explorer) = self.file_explorer_state_mut() {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                explorer.navigate_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                explorer.navigate_down();
            }
            KeyCode::Home | KeyCode::Char('g') => {
                explorer.navigate_top();
            }
            KeyCode::End | KeyCode::Char('G') => {
                explorer.navigate_bottom();
            }
            KeyCode::Enter => {
                if let Ok(Some(path)) = explorer.enter_selection() {
                    self.status.set(format!("Selected: {}", path.display()));
                }
            }
            KeyCode::Backspace | KeyCode::Char('h') => {
                explorer.go_parent()?;
            }
            KeyCode::Char(' ') | KeyCode::Char('l') => {
                explorer.toggle_expand()?;
            }
            KeyCode::Char('r') => {
                explorer.refresh()?;
                self.status.set("File list refreshed");
            }
            _ => {}
        }
    }
    Ok(())
}
```

Update `handle_input()` to route to the file explorer handler:

```rust
pub fn handle_input(&mut self, runtime: &Runtime, event: InputEvent) -> Result<()> {
    match event {
        InputEvent::Key(key) => {
            match self.focus {
                // ... existing cases ...

                #[cfg(feature = "tui-agent")]
                FocusArea::FileExplorer => {
                    self.handle_file_explorer_key(key)?;
                }
            }
        }
    }
    Ok(())
}
```

### Step 4: Update UI Layout

In `src/ui.rs`, add the file explorer to the agent layout:

```rust
#[cfg(feature = "tui-agent")]
fn render_agent_layout(frame: &mut Frame<'_>, main_area: Rect, status_area: Rect, app: &App) {
    let agent_ui_state = app.agent_ui_state();

    // Determine if file explorer should be shown
    let show_file_explorer = app.file_explorer_state().is_some();

    // Create layout based on visibility
    let main_chunks = if show_file_explorer {
        // Layout: Sessions | Chat | FileExplorer | ToolPanel
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),   // Sessions
                Constraint::Min(30),      // Chat
                Constraint::Length(32),   // FileExplorer
                Constraint::Length(28),   // ToolPanel
            ])
            .split(main_area)
    } else {
        // Normal agent layout without file explorer
        // ... existing layout ...
    };

    // Render sessions and chat
    render_sessions(frame, main_chunks[0], app);
    render_chat(frame, main_chunks[1], app);

    // Render file explorer if available
    if show_file_explorer {
        if let Some(explorer_state) = app.file_explorer_state() {
            let focused = matches!(app.focus(), FocusArea::FileExplorer);
            render_file_explorer(frame, main_chunks[2], explorer_state, focused);
        }
    }

    // Render remaining panels
    // ... rest of the layout ...
}
```

### Step 5: Add Focus Cycling

Update focus cycling to include the file explorer:

```rust
pub fn cycle_focus(&mut self) {
    #[cfg(feature = "tui-agent")]
    if self.is_agent_mode() {
        self.focus = match self.focus {
            FocusArea::Sessions => FocusArea::Chat,
            FocusArea::Chat => if self.file_explorer_state.is_some() {
                FocusArea::FileExplorer
            } else {
                FocusArea::AgentTools
            },
            FocusArea::FileExplorer => FocusArea::AgentTools,
            FocusArea::AgentTools => FocusArea::AgentHistory,
            FocusArea::AgentHistory => FocusArea::CommandLine,
            FocusArea::CommandLine => FocusArea::Sessions,
            _ => FocusArea::Sessions,
        };
        return;
    }

    // ... rest of normal mode cycling ...
}
```

## Keyboard Shortcuts

When the File Explorer is focused:

| Key         | Action                              |
|-------------|-------------------------------------|
| ↑ / k       | Move cursor up                      |
| ↓ / j       | Move cursor down                    |
| Home / g    | Go to top                           |
| End / G     | Go to bottom                        |
| Enter       | Enter directory or select file      |
| Backspace/h | Go to parent directory              |
| Space / l   | Expand/collapse directory           |
| r           | Refresh file list                   |
| Tab         | Cycle to next panel                 |

## Example Usage

### Basic Initialization

```rust
use mistralrs_tui::components::FileExplorerState;

// Create explorer at current directory
let explorer = FileExplorerState::new(".")?;

// Create explorer at specific path
let explorer = FileExplorerState::new("/path/to/directory")?;
```

### Navigation

```rust
// Move cursor
explorer.navigate_down();
explorer.navigate_up();
explorer.navigate_top();
explorer.navigate_bottom();

// Navigate directories
explorer.enter_selection()?;  // Enter dir or select file
explorer.go_parent()?;         // Go to parent directory

// Expand/collapse
explorer.toggle_expand()?;
```

### Filtering

```rust
// Set filter
explorer.set_filter(Some("*.rs".to_string()))?;

// Clear filter
explorer.clear_filter()?;
```

### Rendering

```rust
use mistralrs_tui::components::render_file_explorer;

// In your render function
render_file_explorer(frame, area, &explorer_state, focused);
```

## Testing

Run the demo application:

```bash
cargo run --example file_explorer_demo --features tui-agent,terminal
```

Run unit tests:

```bash
cargo test -p mistralrs-tui --features tui-agent --lib file_explorer
```

## Advanced Features

### Custom File Filtering

The built-in glob matching supports `*` and `?` wildcards. For more advanced patterns, you can extend the `glob_match` function or integrate the `glob` crate.

### File Icons

The component uses emoji icons by default:
- 📁 Collapsed directory
- 📂 Expanded directory
- 📄 File

To customize, modify the `FileEntry::display_line()` method.

### File Operations

To add file operations (copy, move, delete, etc.), extend the `FileExplorerState` with methods like:

```rust
impl FileExplorerState {
    pub fn delete_selected(&mut self) -> Result<()> {
        if let Some(entry) = self.current_entry() {
            if entry.is_dir {
                fs::remove_dir_all(&entry.path)?;
            } else {
                fs::remove_file(&entry.path)?;
            }
            self.refresh()?;
        }
        Ok(())
    }

    pub fn create_file(&mut self, name: &str) -> Result<()> {
        let path = self.current_dir.join(name);
        fs::File::create(&path)?;
        self.refresh()?;
        Ok(())
    }

    pub fn create_directory(&mut self, name: &str) -> Result<()> {
        let path = self.current_dir.join(name);
        fs::create_dir(&path)?;
        self.refresh()?;
        Ok(())
    }
}
```

### Integration with Agent Tools

The file explorer can be integrated with the agent toolkit to:
- Browse sandbox directories
- Select files for agent operations
- Display agent-generated files
- Navigate between project directories

Example integration:

```rust
// When agent creates a file, navigate to it
if let Some(created_file) = agent_result.created_file {
    if let Some(explorer) = app.file_explorer_state_mut() {
        // Navigate to the directory containing the file
        if let Some(parent) = created_file.parent() {
            explorer.current_dir = parent.to_path_buf();
            explorer.refresh()?;

            // Find and select the created file
            for (idx, entry) in explorer.entries.iter().enumerate() {
                if entry.path == created_file {
                    explorer.cursor = idx;
                    break;
                }
            }
        }
    }
}
```

## Troubleshooting

### File List Not Updating

Call `refresh()` after file system changes:

```rust
explorer.refresh()?;
```

### Permission Errors

Ensure the application has read permissions for the directories being browsed. The component gracefully handles permission errors but won't display inaccessible directories.

### Large Directories

For directories with thousands of files, consider:
1. Using filters to limit displayed files
2. Implementing lazy loading
3. Adding pagination

### Path Canonicalization Issues

On Windows, paths are automatically canonicalized. If you need to preserve symbolic links, modify the `new()` method to not call `canonicalize()`.

## Future Enhancements

Planned improvements:
- [ ] File preview panel
- [ ] Multi-selection with Ctrl+Click
- [ ] Drag-and-drop support (with tui-textarea)
- [ ] File type icons based on extension
- [ ] Git status indicators
- [ ] Bookmarks/favorites
- [ ] Search within files
- [ ] Custom sort orders (name, size, date)
- [ ] Hidden file toggle
