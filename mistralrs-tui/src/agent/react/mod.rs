//! ReAct (Reasoning + Acting) agent implementation
//!
//! This module provides a full ReAct agent implementation for the TUI,
//! combining reasoning steps with tool actions in an iterative loop.

#[cfg(feature = "tui-agent")]
pub mod engine;
pub mod observation;
pub mod state;
pub mod thought;

#[cfg(feature = "tui-agent")]
pub use engine::{ReActConfig, ReActResponse, TuiReActEngine};

pub use observation::{
    Observation, ObservationKind, ObservationMetadata, ObservationProcessor, ObservationSummary,
    TruncationStrategy,
};

pub use state::{
    Action, ActionResult, ActionStatus, IterationOutcome, ReActIteration, ReActPhase, ReActState,
    StateError, TerminationReason, ThoughtSummary, DEFAULT_REACT_TIMEOUT, MAX_REACT_ITERATIONS,
};

pub use thought::{
    ExtractionPattern, FallbackStrategy, PlannedAction, Thought as ParsedThought,
    ThoughtParseError, ThoughtParser,
};
