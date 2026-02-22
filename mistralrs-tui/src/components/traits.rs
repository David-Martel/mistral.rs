//! Component trait and infrastructure for IDE components

#[cfg(feature = "tui-agent")]
use crate::input::InputEvent;
#[cfg(feature = "tui-agent")]
use ratatui::{layout::Rect, Frame};
#[cfg(feature = "tui-agent")]
use std::any::Any;
#[cfg(feature = "tui-agent")]
use std::collections::HashMap;

#[cfg(feature = "tui-agent")]
/// Result of handling an event
#[derive(Debug, Clone, PartialEq)]
pub enum EventResult {
    /// Event was consumed by this component
    Consumed,
    /// Event was ignored, try other handlers
    Ignored,
    /// Request focus change
    FocusChange(FocusTarget),
    /// Request application quit
    Quit,
}

#[cfg(feature = "tui-agent")]
/// Target for focus changes
#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Next,
    Previous,
    Specific(&'static str),
}

#[cfg(feature = "tui-agent")]
/// Context provided to components during event handling and updates
pub struct ComponentContext {
    /// Status line message
    status_message: Option<String>,
    /// Shared state accessible to all components
    shared_state: HashMap<String, String>,
}

#[cfg(feature = "tui-agent")]
impl ComponentContext {
    /// Create a new component context
    pub fn new() -> Self {
        Self {
            status_message: None,
            shared_state: HashMap::new(),
        }
    }

    /// Set the status line message
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    /// Get the current status message
    pub fn status(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    /// Clear the status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Set a shared state value
    pub fn set_shared(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.shared_state.insert(key.into(), value.into());
    }

    /// Get a shared state value
    pub fn get_shared(&self, key: &str) -> Option<&str> {
        self.shared_state.get(key).map(|s| s.as_str())
    }

    /// Remove a shared state value
    pub fn remove_shared(&mut self, key: &str) -> Option<String> {
        self.shared_state.remove(key)
    }
}

#[cfg(feature = "tui-agent")]
impl Default for ComponentContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tui-agent")]
/// Trait for TUI components
pub trait Component: Send {
    /// Handle input event, return result
    fn handle_event(&mut self, event: &InputEvent, ctx: &mut ComponentContext) -> EventResult;

    /// Render the component
    fn render(&self, area: Rect, frame: &mut Frame<'_>, focused: bool);

    /// Whether this component can receive focus
    fn focusable(&self) -> bool {
        true
    }

    /// Unique identifier for this component
    fn id(&self) -> &'static str;

    /// Optional tick for animations/updates
    fn tick(&mut self, _ctx: &mut ComponentContext) {}

    /// For downcasting to concrete types
    fn as_any(&self) -> &dyn Any;

    /// For downcasting to concrete types (mutable)
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

#[cfg(feature = "tui-agent")]
/// Manages a collection of components and handles focus/routing
pub struct ComponentManager {
    /// All registered components
    components: Vec<Box<dyn Component>>,
    /// Index of the currently focused component
    focused_index: usize,
    /// Component context shared across all components
    context: ComponentContext,
}

#[cfg(feature = "tui-agent")]
impl ComponentManager {
    /// Create a new component manager
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
            focused_index: 0,
            context: ComponentContext::new(),
        }
    }

    /// Register a new component
    pub fn register(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }

    /// Get the number of registered components
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Check if there are no components
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Handle an input event by routing to the focused component
    pub fn handle_event(&mut self, event: &InputEvent) -> EventResult {
        if self.components.is_empty() {
            return EventResult::Ignored;
        }

        // Route to focused component
        let result = self.components[self.focused_index].handle_event(event, &mut self.context);

        // Handle focus change requests
        match &result {
            EventResult::FocusChange(target) => {
                match target {
                    FocusTarget::Next => self.focus_next(),
                    FocusTarget::Previous => self.focus_previous(),
                    FocusTarget::Specific(id) => {
                        if !self.focus_by_id(id) {
                            // If component not found, stay on current
                            return EventResult::Ignored;
                        }
                    }
                }
                EventResult::Consumed
            }
            _ => result,
        }
    }

    /// Render all components in their respective areas
    pub fn render_all(&self, frame: &mut Frame<'_>, areas: &[Rect]) {
        if areas.len() != self.components.len() {
            // Area count mismatch - log or handle gracefully
            return;
        }

        for (idx, component) in self.components.iter().enumerate() {
            let focused = idx == self.focused_index;
            component.render(areas[idx], frame, focused);
        }
    }

    /// Tick all components for updates
    pub fn tick_all(&mut self) {
        for component in &mut self.components {
            component.tick(&mut self.context);
        }
    }

    /// Move focus to the next focusable component
    pub fn focus_next(&mut self) {
        if self.components.is_empty() {
            return;
        }

        let start = self.focused_index;
        loop {
            self.focused_index = (self.focused_index + 1) % self.components.len();
            if self.components[self.focused_index].focusable() || self.focused_index == start {
                break;
            }
        }
    }

    /// Move focus to the previous focusable component
    pub fn focus_previous(&mut self) {
        if self.components.is_empty() {
            return;
        }

        let start = self.focused_index;
        loop {
            self.focused_index = if self.focused_index == 0 {
                self.components.len() - 1
            } else {
                self.focused_index - 1
            };
            if self.components[self.focused_index].focusable() || self.focused_index == start {
                break;
            }
        }
    }

    /// Focus a component by its ID
    /// Returns true if the component was found and focused
    pub fn focus_by_id(&mut self, id: &str) -> bool {
        for (idx, component) in self.components.iter().enumerate() {
            if component.id() == id && component.focusable() {
                self.focused_index = idx;
                return true;
            }
        }
        false
    }

    /// Get the currently focused component's ID
    pub fn focused_id(&self) -> Option<&'static str> {
        self.components.get(self.focused_index).map(|c| c.id())
    }

    /// Get the current status message from context
    pub fn status(&self) -> Option<&str> {
        self.context.status()
    }

    /// Get a reference to a component by ID
    pub fn get_component(&self, id: &str) -> Option<&dyn Component> {
        self.components.iter().find(|c| c.id() == id).map(|c| &**c)
    }

    /// Get a mutable reference to a component by ID
    pub fn get_component_mut(&mut self, id: &str) -> Option<&mut dyn Component> {
        for component in &mut self.components {
            if component.id() == id {
                return Some(&mut **component);
            }
        }
        None
    }

    /// Get the component context
    pub fn context(&self) -> &ComponentContext {
        &self.context
    }

    /// Get mutable access to the component context
    pub fn context_mut(&mut self) -> &mut ComponentContext {
        &mut self.context
    }
}

#[cfg(feature = "tui-agent")]
impl Default for ComponentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "tui-agent"))]
mod tests {
    use super::*;

    struct TestComponent {
        id: &'static str,
        focusable: bool,
        events_received: usize,
    }

    impl TestComponent {
        fn new(id: &'static str, focusable: bool) -> Self {
            Self {
                id,
                focusable,
                events_received: 0,
            }
        }
    }

    impl Component for TestComponent {
        fn handle_event(
            &mut self,
            _event: &InputEvent,
            _ctx: &mut ComponentContext,
        ) -> EventResult {
            self.events_received += 1;
            EventResult::Consumed
        }

        fn render(&self, _area: Rect, _frame: &mut Frame<'_>, _focused: bool) {
            // No-op for tests
        }

        fn focusable(&self) -> bool {
            self.focusable
        }

        fn id(&self) -> &'static str {
            self.id
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_component_manager_basic() {
        let mut manager = ComponentManager::new();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());

        manager.register(Box::new(TestComponent::new("test1", true)));
        assert_eq!(manager.len(), 1);
        assert!(!manager.is_empty());
    }

    #[test]
    fn test_focus_navigation() {
        let mut manager = ComponentManager::new();
        manager.register(Box::new(TestComponent::new("test1", true)));
        manager.register(Box::new(TestComponent::new("test2", true)));
        manager.register(Box::new(TestComponent::new("test3", true)));

        assert_eq!(manager.focused_id(), Some("test1"));

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some("test2"));

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some("test3"));

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some("test1")); // Wraps around

        manager.focus_previous();
        assert_eq!(manager.focused_id(), Some("test3"));
    }

    #[test]
    fn test_focus_by_id() {
        let mut manager = ComponentManager::new();
        manager.register(Box::new(TestComponent::new("test1", true)));
        manager.register(Box::new(TestComponent::new("test2", true)));
        manager.register(Box::new(TestComponent::new("test3", true)));

        assert!(manager.focus_by_id("test3"));
        assert_eq!(manager.focused_id(), Some("test3"));

        assert!(!manager.focus_by_id("nonexistent"));
        assert_eq!(manager.focused_id(), Some("test3")); // Unchanged
    }

    #[test]
    fn test_skip_non_focusable() {
        let mut manager = ComponentManager::new();
        manager.register(Box::new(TestComponent::new("test1", true)));
        manager.register(Box::new(TestComponent::new("test2", false))); // Not focusable
        manager.register(Box::new(TestComponent::new("test3", true)));

        assert_eq!(manager.focused_id(), Some("test1"));

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some("test3")); // Skips test2

        manager.focus_next();
        assert_eq!(manager.focused_id(), Some("test1"));
    }

    #[test]
    fn test_context_status() {
        let mut ctx = ComponentContext::new();
        assert!(ctx.status().is_none());

        ctx.set_status("Test message");
        assert_eq!(ctx.status(), Some("Test message"));

        ctx.clear_status();
        assert!(ctx.status().is_none());
    }

    #[test]
    fn test_context_shared_state() {
        let mut ctx = ComponentContext::new();

        ctx.set_shared("key1", "value1");
        assert_eq!(ctx.get_shared("key1"), Some("value1"));

        ctx.set_shared("key2", "value2");
        assert_eq!(ctx.get_shared("key2"), Some("value2"));

        let removed = ctx.remove_shared("key1");
        assert_eq!(removed, Some("value1".to_string()));
        assert!(ctx.get_shared("key1").is_none());
    }
}
