//! Reusable UI components for the TUI application

#[cfg(feature = "tui-agent")]
pub mod traits;

#[cfg(feature = "tui-agent")]
pub mod file_explorer;

#[cfg(feature = "tui-agent")]
pub mod git_status;

#[cfg(feature = "tui-agent")]
pub mod editor;

#[cfg(feature = "tui-agent")]
pub use traits::{Component, ComponentContext, ComponentManager, EventResult, FocusTarget};

#[cfg(feature = "tui-agent")]
pub use file_explorer::{FileEntry, FileExplorer, FileExplorerState};

#[cfg(feature = "tui-agent")]
pub use git_status::{render_git_status, GitStatus, GitStatusProvider};

#[cfg(feature = "tui-agent")]
pub use editor::{render_editor, Editor, EditorMode, EditorState};
