# Component Trait Architecture for mistralrs-tui IDE Components

## Executive Summary

This document outlines the architectural design for a Component trait system to support IDE-like features (FileExplorer, Editor, GitPanel) in mistralrs-tui. The design balances simplicity with extensibility, avoiding the full complexity of Helix's component system while providing a solid foundation for modular UI development.

## Architectural Impact Assessment

**Impact Level**: Medium-High

| Aspect | Impact | Rationale |
|--------|--------|-----------|
| Module Structure | Medium | New `components/` module, minimal changes to existing code |
| Event Flow | High | New event routing through Component trait |
| Rendering | Medium | Components integrate with existing ratatui patterns |
| State Management | Medium | Components own local state, shared state via Context |
| Testing | Low | Components are independently testable |

---

## Part 1: Existing Pattern Analysis

### Current Architecture Overview

The existing mistralrs-tui follows a **centralized state machine** pattern:

```
                    +------------------+
                    |      App         |
                    |   (State Owner)  |
                    +--------+---------+
                             |
              +--------------+--------------+
              |              |              |
        +-----v----+  +------v-----+  +-----v-----+
        | Sessions |  |    Chat    |  |  Models   |
        |  Panel   |  |   Panel    |  |  Panel    |
        +----------+  +------------+  +-----------+
```

#### Key Patterns Identified:

1. **Focus Management via Enum**
   ```rust
   pub enum FocusArea {
       Sessions,
       Chat,
       Models,
       CommandLine,
       AgentTools,      // feature-gated
       AgentBrowser,    // feature-gated
       AgentHistory,    // feature-gated
   }
   ```

2. **Centralized Event Handling**
   - All keyboard events flow through `App::handle_key()`
   - Focus determines which handler processes the event
   - Event handlers directly mutate `App` state

3. **Stateless Rendering**
   - `ui::render()` takes immutable `&App` reference
   - Rendering functions are pure (no side effects)
   - Layout calculations happen at render time

4. **Existing Input Abstraction**
   ```rust
   pub enum InputEvent {
       Tick,
       Key(KeyEvent),
       Resize(u16, u16),
   }

   pub struct KeyEvent {
       pub code: KeyCode,
       pub modifiers: Modifiers,
   }
   ```

5. **Agent UI State Pattern**
   ```rust
   pub struct AgentUiState {
       pub panel_visible: bool,
       pub browser_visible: bool,
       pub tool_cursor: usize,
       pub active_execution: Option<ActiveExecution>,
       // ...
   }
   ```
   This demonstrates localized UI state within a larger context.

---

## Part 2: Component Trait Design

### Core Traits and Types

```rust
//! Component infrastructure for mistralrs-tui IDE features
//!
//! Location: mistralrs-tui/src/components/mod.rs

use ratatui::{layout::Rect, Frame};
use std::any::Any;

// Re-export existing input types for consistency
pub use crate::input::{InputEvent, KeyCode, KeyEvent, Modifiers};

/// Result of handling an event within a component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// Event was handled, no further processing needed
    Consumed,
    /// Event was not handled, should propagate to parent/siblings
    Ignored,
    /// Event was handled and focus should move to specified area
    FocusChange(FocusTarget),
    /// Event was handled and application should quit
    Quit,
}

/// Target for focus change requests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    /// Return focus to the main chat area
    Chat,
    /// Move to next component in tab order
    Next,
    /// Move to previous component in tab order
    Previous,
    /// Move to a specific named component
    Named(&'static str),
}

/// Shared context passed to components during event handling
///
/// Provides read/write access to application-wide state that
/// components may need to query or modify.
pub struct ComponentContext<'a> {
    /// Status line for user feedback
    status: &'a mut StatusLine,
    /// Async runtime for spawning background tasks
    runtime: &'a tokio::runtime::Runtime,
    /// Event bus for component communication (optional, feature-gated)
    #[cfg(feature = "tui-agent")]
    event_bus: Option<&'a crate::agent::EventBus>,
    /// Session store for persistence operations
    session_store: &'a std::sync::Arc<crate::session::SessionStore>,
    /// Model inventory for model operations
    model_inventory: &'a std::sync::Arc<crate::inventory::ModelInventory>,
}

impl<'a> ComponentContext<'a> {
    /// Set a status message for user feedback
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status.set(message);
    }

    /// Get the tokio runtime for async operations
    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        self.runtime
    }

    /// Emit an event to the event bus (if available)
    #[cfg(feature = "tui-agent")]
    pub fn emit_event(&self, event: crate::agent::ExecutionEvent) {
        if let Some(bus) = self.event_bus {
            bus.emit(event);
        }
    }

    /// Access the session store for persistence
    pub fn session_store(&self) -> &crate::session::SessionStore {
        self.session_store
    }

    /// Access the model inventory
    pub fn model_inventory(&self) -> &crate::inventory::ModelInventory {
        self.model_inventory
    }
}

/// Core trait for UI components
///
/// Components are self-contained UI elements that:
/// - Own their local state
/// - Handle their own input events
/// - Render themselves to a given area
///
/// # Design Principles
///
/// 1. **Single Responsibility**: Each component handles one UI concern
/// 2. **Local State Ownership**: Components own and manage their state
/// 3. **Stateless Rendering**: Render is a pure function of current state
/// 4. **Event Bubbling**: Unhandled events propagate to parent
///
/// # Example
///
/// ```rust
/// struct FileExplorer {
///     current_dir: PathBuf,
///     entries: Vec<DirEntry>,
///     cursor: usize,
///     expanded: HashSet<PathBuf>,
/// }
///
/// impl Component for FileExplorer {
///     fn handle_event(&mut self, event: &InputEvent, ctx: &mut ComponentContext) -> EventResult {
///         match event {
///             InputEvent::Key(key) => self.handle_key(key, ctx),
///             _ => EventResult::Ignored,
///         }
///     }
///
///     fn render(&self, area: Rect, frame: &mut Frame<'_>, focused: bool) {
///         // Render file tree...
///     }
/// }
/// ```
pub trait Component: Send {
    /// Handle an input event
    ///
    /// Returns how the event was processed, which determines
    /// whether it propagates to other components.
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ComponentContext) -> EventResult;

    /// Render the component to the given area
    ///
    /// # Arguments
    /// * `area` - The rectangular area to render into
    /// * `frame` - The ratatui frame for drawing
    /// * `focused` - Whether this component currently has focus
    fn render(&self, area: Rect, frame: &mut Frame<'_>, focused: bool);

    /// Whether this component can receive focus
    ///
    /// Return false for decorative or informational components
    /// that don't need keyboard input.
    fn focusable(&self) -> bool {
        true
    }

    /// Unique identifier for this component instance
    ///
    /// Used for focus management and event routing.
    fn id(&self) -> &'static str;

    /// Called each tick for time-based updates
    ///
    /// Use for animations, polling, or periodic state refresh.
    fn tick(&mut self, _ctx: &mut ComponentContext) {
        // Default: no tick handling
    }

    /// Provide type-erased access for downcasting
    ///
    /// Required for runtime type identification in collections.
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Extension trait for component-specific operations
pub trait ComponentExt: Component {
    /// Attempt to downcast to a concrete component type
    fn downcast_ref<T: Component + 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }

    /// Attempt to downcast to a concrete component type (mutable)
    fn downcast_mut<T: Component + 'static>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

impl<T: Component + ?Sized> ComponentExt for T {}

/// Helper macro to implement Any boilerplate
#[macro_export]
macro_rules! impl_component_any {
    ($type:ty) => {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}
```

### StatusLine Reference

```rust
/// Re-use existing StatusLine from app.rs
/// Location: mistralrs-tui/src/app.rs (already exists)
pub struct StatusLine {
    message: String,
}

impl StatusLine {
    pub fn new(message: impl Into<String>) -> Self { /* ... */ }
    pub fn set(&mut self, message: impl Into<String>) { /* ... */ }
    pub fn message(&self) -> &str { /* ... */ }
}
```

---

## Part 3: Module Organization

### Proposed Directory Structure

```
mistralrs-tui/
├── src/
│   ├── lib.rs                    # Add: pub mod components;
│   ├── app.rs                    # Modify: Add ComponentManager integration
│   ├── ui.rs                     # Modify: Delegate to component rendering
│   ├── input.rs                  # Keep: Already abstracted
│   ├── components/
│   │   ├── mod.rs                # Component trait, Context, EventResult
│   │   ├── manager.rs            # ComponentManager for routing
│   │   ├── file_explorer.rs      # FileExplorer component
│   │   ├── editor.rs             # Editor component (future)
│   │   ├── git_panel.rs          # GitPanel component (future)
│   │   └── terminal.rs           # Terminal component (future)
│   ├── agent/                    # Keep: Existing agent module
│   └── ...
```

### Component Manager

```rust
//! Component manager for routing events and managing focus
//!
//! Location: mistralrs-tui/src/components/manager.rs

use super::{Component, ComponentContext, EventResult, FocusTarget};
use crate::input::InputEvent;
use ratatui::{layout::Rect, Frame};
use std::collections::HashMap;

/// Manages a collection of components and routes events
pub struct ComponentManager {
    /// Registered components by ID
    components: HashMap<&'static str, Box<dyn Component>>,
    /// Current focus (component ID)
    focused: Option<&'static str>,
    /// Tab order for focus cycling
    tab_order: Vec<&'static str>,
    /// Whether manager is active (components visible)
    active: bool,
}

impl ComponentManager {
    /// Create a new component manager
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            focused: None,
            tab_order: Vec::new(),
            active: false,
        }
    }

    /// Register a component with this manager
    pub fn register(&mut self, component: Box<dyn Component>) {
        let id = component.id();
        if component.focusable() {
            self.tab_order.push(id);
        }
        self.components.insert(id, component);
    }

    /// Set which component has focus
    pub fn set_focus(&mut self, id: &'static str) {
        if self.components.contains_key(id) {
            self.focused = Some(id);
        }
    }

    /// Clear focus from all components
    pub fn clear_focus(&mut self) {
        self.focused = None;
    }

    /// Handle an input event
    ///
    /// Routes to focused component first, then tries others.
    pub fn handle_event(&mut self, event: &InputEvent, ctx: &mut ComponentContext) -> EventResult {
        if !self.active {
            return EventResult::Ignored;
        }

        // First, try the focused component
        if let Some(focused_id) = self.focused {
            if let Some(component) = self.components.get_mut(focused_id) {
                let result = component.handle_event(event, ctx);
                match result {
                    EventResult::Consumed => return result,
                    EventResult::FocusChange(target) => {
                        self.handle_focus_change(target);
                        return EventResult::Consumed;
                    }
                    EventResult::Quit => return result,
                    EventResult::Ignored => {}
                }
            }
        }

        EventResult::Ignored
    }

    /// Handle focus change request
    fn handle_focus_change(&mut self, target: FocusTarget) {
        match target {
            FocusTarget::Chat => {
                self.focused = None;
            }
            FocusTarget::Next => {
                self.focus_next();
            }
            FocusTarget::Previous => {
                self.focus_previous();
            }
            FocusTarget::Named(id) => {
                self.set_focus(id);
            }
        }
    }

    /// Move focus to next component in tab order
    fn focus_next(&mut self) {
        if self.tab_order.is_empty() {
            return;
        }

        let current_idx = self.focused
            .and_then(|id| self.tab_order.iter().position(|&x| x == id))
            .unwrap_or(self.tab_order.len());

        let next_idx = (current_idx + 1) % self.tab_order.len();
        self.focused = Some(self.tab_order[next_idx]);
    }

    /// Move focus to previous component in tab order
    fn focus_previous(&mut self) {
        if self.tab_order.is_empty() {
            return;
        }

        let current_idx = self.focused
            .and_then(|id| self.tab_order.iter().position(|&x| x == id))
            .unwrap_or(0);

        let prev_idx = if current_idx == 0 {
            self.tab_order.len() - 1
        } else {
            current_idx - 1
        };
        self.focused = Some(self.tab_order[prev_idx]);
    }

    /// Tick all components
    pub fn tick(&mut self, ctx: &mut ComponentContext) {
        for component in self.components.values_mut() {
            component.tick(ctx);
        }
    }

    /// Render a specific component by ID
    pub fn render_component(&self, id: &str, area: Rect, frame: &mut Frame<'_>) {
        if let Some(component) = self.components.get(id) {
            let focused = self.focused == Some(id);
            component.render(area, frame, focused);
        }
    }

    /// Get a component by ID
    pub fn get<T: Component + 'static>(&self, id: &str) -> Option<&T> {
        self.components.get(id)?.as_any().downcast_ref()
    }

    /// Get a mutable component by ID
    pub fn get_mut<T: Component + 'static>(&mut self, id: &str) -> Option<&mut T> {
        self.components.get_mut(id)?.as_any_mut().downcast_mut()
    }

    /// Activate the component manager
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate the component manager
    pub fn deactivate(&mut self) {
        self.active = false;
        self.focused = None;
    }

    /// Check if manager is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Default for ComponentManager {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## Part 4: Integration Strategy

### Integration with Existing App Struct

The integration follows a **composition over inheritance** approach:

```rust
// Modified App struct (changes highlighted)
// Location: mistralrs-tui/src/app.rs

pub struct App {
    // Existing fields (unchanged)
    session_store: Arc<SessionStore>,
    model_inventory: Arc<ModelInventory>,
    focus: FocusArea,
    metrics: Metrics,
    status: StatusLine,
    should_quit: bool,
    sessions: Vec<SessionSummary>,
    session_cursor: usize,
    active_session: SessionContext,
    model_cursor: usize,
    cli_input: String,
    cli_history: Vec<String>,
    cli_history_cursor: usize,
    cli_output: Vec<String>,
    cli_output_max_lines: usize,

    // Agent fields (existing, feature-gated)
    #[cfg(feature = "tui-agent")]
    agent_toolkit: Option<AgentToolkit>,
    #[cfg(feature = "tui-agent")]
    agent_ui_state: AgentUiState,
    // ... other agent fields

    // NEW: IDE component manager (feature-gated)
    #[cfg(feature = "ide-components")]
    component_manager: ComponentManager,
    #[cfg(feature = "ide-components")]
    ide_mode: bool,
}

// Extended FocusArea enum
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

    // NEW: IDE component areas
    #[cfg(feature = "ide-components")]
    FileExplorer,
    #[cfg(feature = "ide-components")]
    Editor,
    #[cfg(feature = "ide-components")]
    GitPanel,
}
```

### Event Flow Integration

```rust
impl App {
    /// Handle keyboard input with component integration
    fn handle_key(&mut self, key: KeyEvent, runtime: &Runtime) -> Result<()> {
        // Global shortcuts first (Ctrl+C, etc.)
        if key.modifiers.control && matches!(key.code, KeyCode::Char('c')) {
            self.should_quit = true;
            return Ok(());
        }

        // NEW: Toggle IDE mode with Ctrl+E
        #[cfg(feature = "ide-components")]
        if key.modifiers.control && matches!(key.code, KeyCode::Char('e')) {
            self.toggle_ide_mode();
            return Ok(());
        }

        // NEW: Route to IDE components if active
        #[cfg(feature = "ide-components")]
        if self.ide_mode {
            let mut ctx = self.create_component_context(runtime);
            let result = self.component_manager.handle_event(
                &InputEvent::Key(key),
                &mut ctx
            );

            match result {
                EventResult::Consumed => return Ok(()),
                EventResult::FocusChange(FocusTarget::Chat) => {
                    self.ide_mode = false;
                    self.focus = FocusArea::Chat;
                    return Ok(());
                }
                EventResult::Quit => {
                    self.should_quit = true;
                    return Ok(());
                }
                _ => {}
            }
        }

        // Existing handling for non-IDE mode
        // ... (keep existing code)
    }

    #[cfg(feature = "ide-components")]
    fn create_component_context<'a>(&'a mut self, runtime: &'a Runtime) -> ComponentContext<'a> {
        ComponentContext {
            status: &mut self.status,
            runtime,
            #[cfg(feature = "tui-agent")]
            event_bus: self.event_bus.as_ref(),
            session_store: &self.session_store,
            model_inventory: &self.model_inventory,
        }
    }

    #[cfg(feature = "ide-components")]
    fn toggle_ide_mode(&mut self) {
        self.ide_mode = !self.ide_mode;
        if self.ide_mode {
            self.component_manager.activate();
            self.status.set("IDE mode enabled (Ctrl+E to toggle)");
        } else {
            self.component_manager.deactivate();
            self.status.set("IDE mode disabled");
        }
    }
}
```

### Rendering Integration

```rust
// Modified render function
// Location: mistralrs-tui/src/ui.rs

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let size = frame.area();

    // NEW: IDE mode layout
    #[cfg(feature = "ide-components")]
    if app.ide_mode {
        render_ide_layout(frame, size, app);
        return;
    }

    // Existing layouts (agent mode, normal mode)
    // ... (keep existing code)
}

#[cfg(feature = "ide-components")]
fn render_ide_layout(frame: &mut Frame<'_>, area: Rect, app: &App) {
    use ratatui::layout::{Constraint, Direction, Layout};

    // IDE layout: FileExplorer | Editor/Chat | GitPanel
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Main content
            Constraint::Length(3), // CLI panel
            Constraint::Length(1), // Status bar
        ])
        .split(area);

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // FileExplorer
            Constraint::Min(40),    // Editor/Chat
            Constraint::Length(28), // GitPanel
        ])
        .split(main_layout[0]);

    // Render IDE components
    app.component_manager.render_component("file_explorer", content_layout[0], frame);

    // Center panel: Editor or Chat based on context
    if app.component_manager.is_active() {
        app.component_manager.render_component("editor", content_layout[1], frame);
    } else {
        render_chat(frame, content_layout[1], app);
    }

    app.component_manager.render_component("git_panel", content_layout[2], frame);

    // Render CLI and status (reuse existing)
    render_cli_panel(frame, main_layout[1], app);
    render_status(frame, main_layout[2], app);
}
```

---

## Part 5: FileExplorer Component Example

```rust
//! File explorer component for IDE mode
//!
//! Location: mistralrs-tui/src/components/file_explorer.rs

use super::{Component, ComponentContext, EventResult, FocusTarget, impl_component_any};
use crate::input::{InputEvent, KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use std::{
    any::Any,
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

/// A file/directory entry in the explorer
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full path to the entry
    pub path: PathBuf,
    /// Display name (file/directory name)
    pub name: String,
    /// Whether this is a directory
    pub is_dir: bool,
    /// Depth in the tree (for indentation)
    pub depth: usize,
}

/// File explorer component for browsing project files
pub struct FileExplorer {
    /// Root directory being explored
    root: PathBuf,
    /// Flattened list of visible entries
    entries: Vec<FileEntry>,
    /// Current cursor position
    cursor: usize,
    /// Set of expanded directories
    expanded: HashSet<PathBuf>,
    /// Currently selected file (for opening)
    selected: Option<PathBuf>,
    /// Filter text for file search
    filter: String,
    /// Whether in filter mode
    filter_mode: bool,
}

impl FileExplorer {
    /// Create a new file explorer rooted at the given path
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let mut explorer = Self {
            root: root.clone(),
            entries: Vec::new(),
            cursor: 0,
            expanded: HashSet::new(),
            selected: None,
            filter: String::new(),
            filter_mode: false,
        };

        // Expand root by default
        explorer.expanded.insert(root);
        explorer.refresh_entries();
        explorer
    }

    /// Refresh the entries list based on current expanded state
    pub fn refresh_entries(&mut self) {
        self.entries.clear();
        self.collect_entries(&self.root.clone(), 0);

        // Ensure cursor is valid
        if !self.entries.is_empty() && self.cursor >= self.entries.len() {
            self.cursor = self.entries.len() - 1;
        }
    }

    /// Recursively collect entries from a directory
    fn collect_entries(&mut self, dir: &Path, depth: usize) {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };

        let mut entries: Vec<_> = read_dir
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                // Filter hidden files and common ignore patterns
                !name.starts_with('.') &&
                !matches!(name.as_str(), "target" | "node_modules" | "__pycache__")
            })
            .collect();

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);

            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            // Apply filter if active
            if !self.filter.is_empty() {
                let filter_lower = self.filter.to_lowercase();
                if !name.to_lowercase().contains(&filter_lower) && !is_dir {
                    continue;
                }
            }

            self.entries.push(FileEntry {
                path: path.clone(),
                name,
                is_dir,
                depth,
            });

            // Recursively add children if expanded
            if is_dir && self.expanded.contains(&path) {
                self.collect_entries(&path, depth + 1);
            }
        }
    }

    /// Handle key input
    fn handle_key(&mut self, key: &KeyEvent, ctx: &mut ComponentContext) -> EventResult {
        // Filter mode handling
        if self.filter_mode {
            return self.handle_filter_key(key, ctx);
        }

        match key.code {
            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                EventResult::Consumed
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor < self.entries.len().saturating_sub(1) {
                    self.cursor += 1;
                }
                EventResult::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                EventResult::Consumed
            }
            KeyCode::End => {
                self.cursor = self.entries.len().saturating_sub(1);
                EventResult::Consumed
            }

            // Expand/collapse
            KeyCode::Enter | KeyCode::Char('l') => {
                if let Some(entry) = self.entries.get(self.cursor) {
                    if entry.is_dir {
                        // Toggle expansion
                        if self.expanded.contains(&entry.path) {
                            self.expanded.remove(&entry.path);
                        } else {
                            self.expanded.insert(entry.path.clone());
                        }
                        self.refresh_entries();
                    } else {
                        // Select file for opening
                        self.selected = Some(entry.path.clone());
                        ctx.set_status(format!("Selected: {}", entry.name));
                        // Could trigger editor opening here
                    }
                }
                EventResult::Consumed
            }
            KeyCode::Char('h') | KeyCode::Left => {
                // Collapse current or go to parent
                if let Some(entry) = self.entries.get(self.cursor) {
                    if entry.is_dir && self.expanded.contains(&entry.path) {
                        self.expanded.remove(&entry.path);
                        self.refresh_entries();
                    } else if let Some(parent) = entry.path.parent() {
                        // Move cursor to parent directory
                        if let Some(idx) = self.entries.iter().position(|e| e.path == parent) {
                            self.cursor = idx;
                        }
                    }
                }
                EventResult::Consumed
            }

            // Refresh
            KeyCode::Char('r') => {
                self.refresh_entries();
                ctx.set_status("File tree refreshed");
                EventResult::Consumed
            }

            // Search/filter
            KeyCode::Char('/') => {
                self.filter_mode = true;
                self.filter.clear();
                ctx.set_status("Filter: (type to search, Enter to confirm, Esc to cancel)");
                EventResult::Consumed
            }

            // Clear filter
            KeyCode::Esc => {
                if !self.filter.is_empty() {
                    self.filter.clear();
                    self.refresh_entries();
                    EventResult::Consumed
                } else {
                    // Return focus to chat
                    EventResult::FocusChange(FocusTarget::Chat)
                }
            }

            // Tab to next component
            KeyCode::Tab => {
                EventResult::FocusChange(FocusTarget::Next)
            }

            _ => EventResult::Ignored,
        }
    }

    /// Handle key input in filter mode
    fn handle_filter_key(&mut self, key: &KeyEvent, ctx: &mut ComponentContext) -> EventResult {
        match key.code {
            KeyCode::Enter => {
                self.filter_mode = false;
                ctx.set_status(format!("Filter applied: {}", self.filter));
                self.refresh_entries();
                EventResult::Consumed
            }
            KeyCode::Esc => {
                self.filter_mode = false;
                self.filter.clear();
                self.refresh_entries();
                ctx.set_status("Filter cancelled");
                EventResult::Consumed
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.refresh_entries();
                EventResult::Consumed
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.refresh_entries();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }

    /// Get the currently selected entry
    pub fn current_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// Get the selected file path (if any)
    pub fn selected_file(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    /// Set the root directory
    pub fn set_root(&mut self, root: impl Into<PathBuf>) {
        self.root = root.into();
        self.expanded.clear();
        self.expanded.insert(self.root.clone());
        self.cursor = 0;
        self.refresh_entries();
    }
}

impl Component for FileExplorer {
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ComponentContext) -> EventResult {
        match event {
            InputEvent::Key(key) => self.handle_key(key, ctx),
            InputEvent::Tick => {
                // Could implement file watching here
                EventResult::Ignored
            }
            InputEvent::Resize(_, _) => EventResult::Ignored,
        }
    }

    fn render(&self, area: Rect, frame: &mut Frame<'_>, focused: bool) {
        let items: Vec<ListItem> = self.entries
            .iter()
            .map(|entry| {
                let indent = "  ".repeat(entry.depth);
                let icon = if entry.is_dir {
                    if self.expanded.contains(&entry.path) {
                        "v "
                    } else {
                        "> "
                    }
                } else {
                    // File icon based on extension
                    match entry.path.extension().and_then(|e| e.to_str()) {
                        Some("rs") => "  " ,
                        Some("toml") => "  ",
                        Some("md") => "  ",
                        Some("json") => "  ",
                        Some("py") => "  ",
                        _ => "  ",
                    }
                };

                let style = if entry.is_dir {
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(icon, style),
                    Span::styled(entry.name.clone(), style),
                ]))
            })
            .collect();

        let title = if self.filter_mode {
            format!("Files [Filter: {}]", self.filter)
        } else if !self.filter.is_empty() {
            format!("Files (filtered: {})", self.filter)
        } else {
            "Files".to_string()
        };

        let mut block = Block::default()
            .title(title)
            .borders(Borders::ALL);

        if focused {
            block = block.border_style(Style::default().fg(Color::Cyan));
        }

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        if !self.entries.is_empty() {
            state.select(Some(self.cursor));
        }

        frame.render_stateful_widget(list, area, &mut state);
    }

    fn focusable(&self) -> bool {
        true
    }

    fn id(&self) -> &'static str {
        "file_explorer"
    }

    impl_component_any!(FileExplorer);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;

    fn create_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test structure
        fs::create_dir(dir.path().join("src")).unwrap();
        File::create(dir.path().join("src/main.rs")).unwrap();
        File::create(dir.path().join("src/lib.rs")).unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        File::create(dir.path().join("README.md")).unwrap();

        dir
    }

    #[test]
    fn test_file_explorer_creation() {
        let dir = create_test_dir();
        let explorer = FileExplorer::new(dir.path());

        assert!(!explorer.entries.is_empty());
        assert!(explorer.expanded.contains(dir.path()));
    }

    #[test]
    fn test_navigation() {
        let dir = create_test_dir();
        let mut explorer = FileExplorer::new(dir.path());

        // Initial cursor at 0
        assert_eq!(explorer.cursor, 0);

        // Move down
        let key = KeyEvent::new(KeyCode::Down, Modifiers::NONE);
        // Would need context mock to test fully
    }

    #[test]
    fn test_expand_collapse() {
        let dir = create_test_dir();
        let mut explorer = FileExplorer::new(dir.path());

        // Find src directory
        let src_idx = explorer.entries.iter()
            .position(|e| e.name == "src")
            .unwrap();

        explorer.cursor = src_idx;

        // Initially expanded (we expand root)
        let initial_count = explorer.entries.len();

        // Collapse src
        explorer.expanded.remove(&dir.path().join("src"));
        explorer.refresh_entries();

        // Should have fewer entries now
        assert!(explorer.entries.len() < initial_count);
    }
}
```

---

## Part 6: Feature Flag Configuration

### Cargo.toml Updates

```toml
# Add to mistralrs-tui/Cargo.toml

[features]
default = ["terminal"]
terminal = ["ratatui/crossterm", "dep:crossterm"]
gpu = ["dep:wgpu", "dep:winit", "dep:pollster", "dep:raw-window-handle", "dep:glyphon"]
tui-agent = ["dep:mistralrs-agent-tools"]

# NEW: IDE components feature
ide-components = []

# Future features
# syntax-highlighting = ["dep:tree-sitter", "ide-components"]
# git-integration = ["dep:git2", "ide-components"]
```

---

## Part 7: SOLID Principle Compliance

### Single Responsibility Principle (SRP)

Each component has one responsibility:
- `FileExplorer`: Navigate and display file trees
- `Editor`: Display and edit text files
- `GitPanel`: Show git status and operations
- `ComponentManager`: Route events and manage focus

### Open/Closed Principle (OCP)

New components can be added without modifying existing code:
```rust
// Adding a new component requires only:
// 1. Implement Component trait
// 2. Register with ComponentManager
// No changes to App, ui.rs, or other components
```

### Liskov Substitution Principle (LSP)

All components implement the same `Component` trait and can be used interchangeably:
```rust
let components: Vec<Box<dyn Component>> = vec![
    Box::new(FileExplorer::new(".")),
    Box::new(GitPanel::new()),
    // Any Component implementation works here
];
```

### Interface Segregation Principle (ISP)

The `Component` trait is minimal with optional methods:
- Required: `handle_event`, `render`, `id`, `as_any`, `as_any_mut`
- Optional: `focusable` (default true), `tick` (default no-op)

### Dependency Inversion Principle (DIP)

Components depend on abstractions:
- `ComponentContext` instead of concrete `App`
- `InputEvent` abstraction instead of platform-specific events
- No direct dependencies between components

---

## Part 8: Migration Path

### Phase 1: Infrastructure (Low Risk)
1. Add `components/` module with trait definitions
2. Add `ComponentManager`
3. Feature-gate with `ide-components`
4. No changes to existing functionality

### Phase 2: FileExplorer (Medium Risk)
1. Implement `FileExplorer` component
2. Add IDE mode toggle (Ctrl+E)
3. Integrate with existing layout system
4. Test thoroughly

### Phase 3: Editor (Higher Risk)
1. Design editor state management
2. Implement basic text display
3. Add syntax highlighting (optional)
4. Handle file I/O safely

### Phase 4: Git Integration (Medium Risk)
1. Implement `GitPanel` component
2. Add git status display
3. Add basic git operations
4. Handle errors gracefully

---

## Part 9: Long-term Implications

### Positive Implications

1. **Modularity**: Easy to add new IDE features
2. **Testability**: Components are independently testable
3. **Maintainability**: Clear separation of concerns
4. **Extensibility**: Plugin-like architecture possible

### Potential Concerns

1. **Event Routing Complexity**: May need refinement as component count grows
2. **State Synchronization**: Components need clear ownership boundaries
3. **Performance**: Many components could impact render performance
4. **Focus Management**: Complex focus flows may need careful design

### Recommendations

1. Start simple with FileExplorer only
2. Add components incrementally
3. Profile rendering performance early
4. Consider component lifecycle hooks if needed
5. Document component interactions clearly

---

## Appendix: Complete Module Structure

```rust
// mistralrs-tui/src/components/mod.rs

//! Component infrastructure for IDE-like features
//!
//! This module provides:
//! - `Component` trait for building UI components
//! - `ComponentManager` for routing and focus management
//! - `ComponentContext` for shared state access
//!
//! # Feature Gate
//!
//! IDE components are feature-gated behind `ide-components`:
//! ```toml
//! [dependencies]
//! mistralrs-tui = { version = "0.6", features = ["ide-components"] }
//! ```

mod traits;
mod context;
mod manager;

pub use traits::{Component, ComponentExt, EventResult, FocusTarget};
pub use context::ComponentContext;
pub use manager::ComponentManager;

#[cfg(feature = "ide-components")]
mod file_explorer;

#[cfg(feature = "ide-components")]
pub use file_explorer::{FileExplorer, FileEntry};

// Re-export input types for convenience
pub use crate::input::{InputEvent, KeyCode, KeyEvent, Modifiers};

/// Helper macro for implementing Any boilerplate
#[macro_export]
macro_rules! impl_component_any {
    ($type:ty) => {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}
```

---

## Summary

This architecture provides:

1. **Minimal disruption** to existing code through feature-gating
2. **Clean abstractions** that follow SOLID principles
3. **Extensible design** for future IDE features
4. **Compatible integration** with existing patterns
5. **Comprehensive example** with FileExplorer implementation

The design prioritizes simplicity over the full complexity of editors like Helix, while still providing a solid foundation for building IDE-like features in the TUI.
