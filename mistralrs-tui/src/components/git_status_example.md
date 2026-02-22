# Git Status Integration Example

## Basic Usage

To integrate git status into the TUI status bar, add the `GitStatusProvider` to your `App` struct:

```rust
use crate::components::GitStatusProvider;
use std::path::PathBuf;

pub struct App {
    // ... existing fields
    git_status: GitStatusProvider,
}

impl App {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            // ... existing initialization
            git_status: GitStatusProvider::new(&current_dir),
        }
    }

    pub fn refresh_git_status(&mut self) {
        self.git_status.refresh();
    }

    pub fn git_status(&self) -> &GitStatus {
        self.git_status.status()
    }
}
```

## Update Status Bar Rendering

Modify `render_status()` in `ui.rs` to include git status:

```rust
fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App) {
    use crate::components::render_git_status;

    let metrics = app.metrics();

    // Build status line with git info
    let mut status_spans = vec![];

    // Add git status if available
    let git_span = render_git_status(app.git_status());
    if !git_span.content.is_empty() {
        status_spans.push(git_span);
        status_spans.push(Span::raw(" | "));
    }

    // Add existing status info
    status_spans.push(Span::raw(app.status_line()));
    status_spans.push(Span::raw(" | total tokens: "));
    status_spans.push(Span::raw(metrics.total_tokens.to_string()));

    let paragraph = Paragraph::new(Line::from(status_spans))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}
```

## Refresh Strategy

You can refresh git status:

1. **On a timer** - Add a periodic refresh every 5-10 seconds:

```rust
impl App {
    pub fn tick(&mut self) {
        // ... existing tick logic

        // Refresh git status periodically
        static mut TICK_COUNT: usize = 0;
        unsafe {
            TICK_COUNT += 1;
            if TICK_COUNT % 50 == 0 {  // Every ~5 seconds at 100ms ticks
                self.refresh_git_status();
            }
        }
    }
}
```

2. **On user action** - Refresh when focus changes or commands execute:

```rust
impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => {
                self.next_focus();
                self.refresh_git_status();
            }
            // ...
        }
    }
}
```

3. **Manual refresh** - Add a keyboard shortcut (e.g., F5):

```rust
KeyCode::F(5) => {
    self.refresh_git_status();
}
```

## Status Format Examples

The status bar will display:

- **Clean repo**: `main ✓`
- **Uncommitted changes**: `main *3 +2 ?1` (3 modified, 2 staged, 1 untracked)
- **Ahead/behind**: `main ↑2 ↓1` (2 commits ahead, 1 behind)
- **Full example**: `main ↑2 ↓1 *3 +2 ?1`
- **Not a repo**: (empty string, nothing displayed)

## Color Coding

The git status is color-coded:

- **Green**: Clean and synced (`main ✓`)
- **Yellow**: Has modified or untracked files
- **Cyan**: Ahead or behind remote
- **White**: Staged changes only

## Performance Considerations

Git commands are executed via shell, which is fast but not instant:

- `git rev-parse --abbrev-ref HEAD` - ~10-20ms
- `git status --porcelain` - ~20-50ms
- `git rev-list --left-right --count HEAD...@{u}` - ~30-100ms

Total refresh time: **~60-170ms** depending on repo size.

**Recommendations**:

1. Don't refresh on every frame (use timer or manual trigger)
2. Cache results between refreshes
3. Consider async refresh if repo is very large
4. Skip refresh if app is in background

## Error Handling

The implementation gracefully handles:

- Not in a git repository (displays nothing)
- Detached HEAD state (shows "(detached)")
- No upstream branch (shows 0 ahead/behind)
- Git not installed (displays nothing)
- Permission errors (displays nothing)

No errors are surfaced to the user - failures result in empty/default status.
