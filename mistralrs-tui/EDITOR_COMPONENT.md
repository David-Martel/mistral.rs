# Editor Component Implementation

## Overview

A syntax-highlighted code editor component for mistralrs-tui with vim-like modal editing.

**File**: `mistralrs-tui/src/components/editor.rs`

## Features Implemented

### 1. Modal Editing (Vim-like)

- **Normal Mode** - Navigation and commands (cyan highlight)
- **Insert Mode** - Text entry (green highlight, shows cursor)
- **Visual Mode** - Selection (yellow highlight)

### 2. Syntax Highlighting (Regex-based MVP)

**Rust Support**:
- Keywords: `fn`, `let`, `mut`, `struct`, `impl`, etc. (bold magenta)
- Strings: Double/single quoted (green)
- Comments: `//` line comments (dark gray)
- Numbers: Integer/float literals (yellow)
- Macros: `println!`, `vec!`, etc. (cyan)
- Attributes: `#[derive(...)]` (blue)

**Python Support**:
- Keywords: `def`, `class`, `import`, `if`, `for`, etc. (bold magenta)
- Strings: Single/double/triple quoted (green)
- Comments: `#` line comments (dark gray)
- Numbers: Integer/float literals (yellow)
- Decorators: `@decorator` (cyan)

**Fallback (Text)**:
- Strings (green)
- Numbers (yellow)

### 3. Editor Operations

**Text Editing**:
- `insert_char(c)` - Insert character at cursor
- `delete_char()` - Backspace (delete before cursor)
- `delete_char_forward()` - Delete at cursor
- Newline handling with line splitting

**Navigation**:
- `navigate_up/down()` - Move vertically
- `navigate_left/right()` - Move horizontally
- `navigate_line_start/end()` - Jump to line boundaries
- `navigate_top/bottom()` - Jump to file boundaries
- Smart cursor clamping to line length
- Automatic scrolling to keep cursor visible

**File Operations**:
- `new()` - Create empty editor
- `open_file(path)` - Load file with auto language detection
- `save()` - Save to current file path
- `save_as(path)` - Save to new path
- Modified flag tracking

### 4. Visual Features

**Line Numbers**:
- Dynamic width based on line count (min 3 digits)
- Dark gray styling

**Current Line Highlight**:
- Darker background on active line
- High contrast for visibility

**Status Display**:
- File name in title bar
- Language indicator (`[rust]`, `[python]`, etc.)
- Modified marker `[+]` when unsaved changes
- Mode-colored border (cyan/green/yellow)

**Cursor Display**:
- Visible cursor in Insert mode
- Positioned accurately with line numbers accounted for
- Hidden in Normal/Visual modes

### 5. Language Detection

Auto-detects from file extension:
- `.rs` → Rust
- `.py` → Python
- `.js`, `.jsx` → JavaScript
- `.ts`, `.tsx` → TypeScript
- `.c`, `.h` → C
- `.cpp`, `.hpp` → C++
- `.go` → Go
- `.java` → Java
- `.md` → Markdown
- `.json` → JSON
- `.toml` → TOML
- `.yaml`, `.yml` → YAML
- Default: Text

## API Usage

### Basic Usage

```rust
use mistralrs_tui::components::{EditorState, EditorMode, render_editor};

// Create new empty editor
let mut editor = EditorState::new()?;

// Open existing file
let mut editor = EditorState::open_file(Path::new("main.rs"))?;

// Render (in your TUI loop)
render_editor(&mut frame, area, &mut editor, focused);
```

### Editing Operations

```rust
// Switch to Insert mode
editor.set_mode(EditorMode::Insert);

// Insert text
editor.insert_char('h');
editor.insert_char('i');
editor.insert_char('\n');

// Navigate
editor.navigate_down();
editor.navigate_line_end();

// Delete
editor.delete_char();

// Save
editor.save()?;
```

### Key Bindings (Example Integration)

```rust
match event {
    // Normal mode
    KeyCode::Char('i') => editor.set_mode(EditorMode::Insert),
    KeyCode::Char('v') => editor.set_mode(EditorMode::Visual),
    KeyCode::Char('h') => editor.navigate_left(),
    KeyCode::Char('j') => editor.navigate_down(),
    KeyCode::Char('k') => editor.navigate_up(),
    KeyCode::Char('l') => editor.navigate_right(),
    KeyCode::Char('0') => editor.navigate_line_start(),
    KeyCode::Char('$') => editor.navigate_line_end(),
    KeyCode::Char('g') if last_key == 'g' => editor.navigate_top(),
    KeyCode::Char('G') => editor.navigate_bottom(),

    // Insert mode
    KeyCode::Esc => editor.set_mode(EditorMode::Normal),
    KeyCode::Char(c) => editor.insert_char(c),
    KeyCode::Backspace => editor.delete_char(),
    KeyCode::Delete => editor.delete_char_forward(),
    KeyCode::Enter => editor.insert_char('\n'),

    // Universal
    KeyCode::Char('s') if ctrl => editor.save()?,
}
```

## Architecture

### State Management

**EditorState** holds:
- Content as `Vec<String>` (one string per line)
- Cursor position (row, col)
- Scroll offset for viewport
- File path (optional)
- Language name
- Modified flag
- Editor mode
- Syntax patterns

### Rendering

**Stateless rendering function**:
- `render_editor(frame, area, state, focused)`
- Calculates visible lines from scroll offset
- Applies syntax highlighting to visible lines only
- Renders line numbers with proper width
- Highlights current line
- Positions cursor in Insert mode

### Syntax Highlighting

**Pattern-based approach**:
1. Compile regex patterns for each language
2. Find all matches in line
3. Sort matches by position
4. Build styled `Span` vector
5. Handle overlapping matches (first match wins)

**Performance**: Only highlights visible lines, regex compiled once.

## Testing

**9 unit tests** covering:
- Language detection
- Editor initialization
- Character insertion
- Character deletion
- Navigation (up/down/left/right/home/end/top/bottom)
- Newline insertion and line splitting
- Mode switching
- Syntax highlighting (Rust and Python)

All tests pass:
```
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured
```

## Future Enhancements

**Tree-sitter Integration** (Phase 2):
- More accurate syntax highlighting
- Semantic understanding
- Better performance with incremental parsing

**Additional Features** (Future):
- Undo/redo stack
- Visual block selection
- Copy/paste clipboard
- Search and replace
- Multi-cursor editing
- Line wrapping
- Indentation auto-detection
- Tab expansion
- Bracket matching
- Code folding

## Dependencies

- `ratatui` - TUI framework
- `regex` - Syntax pattern matching
- `anyhow` - Error handling
- `crossterm` (indirectly) - Terminal backend

## File Size

- Implementation: ~720 lines
- Tests: 9 test cases
- Documentation: Comprehensive inline docs

## Integration with mistralrs-tui

Exported from `components/mod.rs`:
```rust
pub use editor::{Editor, EditorMode, EditorState, render_editor};
```

Ready to integrate into the main TUI application alongside:
- File Explorer component
- Git Status component
- Component trait system

## Notes

- **MVP Approach**: Regex-based highlighting is simpler than tree-sitter
- **Vim-inspired**: Familiar keybindings for developers
- **Performance**: Only processes visible lines
- **Extensible**: Easy to add new languages with regex patterns
- **Type-safe**: Uses Rust's ownership system correctly (no borrow checker fights)
- **Tested**: Comprehensive test coverage ensures correctness
