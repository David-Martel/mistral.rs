//! ReAct state machine for agent reasoning loops
//!
//! This module implements the state management for the ReAct (Reasoning + Acting) pattern:
//! - State transitions through phases: Idle -> Thinking -> Acting -> Observing
//! - Iteration tracking with configurable limits
//! - Timeout and termination handling
//! - Action status tracking and management
//!
//! The ReAct pattern follows this cycle:
//! 1. **Thinking**: LLM reasons about the current state and decides next steps
//! 2. **Acting**: Execute tools/actions based on the thought process
//! 3. **Observing**: Collect results and observations from actions
//! 4. Loop back to Thinking with accumulated context
//!
//! Termination occurs when:
//! - Task is complete (LLM signals completion)
//! - Maximum iterations reached
//! - User cancels the operation
//! - An error occurs
//! - Timeout is exceeded

#[cfg(feature = "tui-agent")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "tui-agent")]
use std::time::{Duration, Instant};
#[cfg(feature = "tui-agent")]
use thiserror::Error;

/// Maximum number of iterations allowed per ReAct session
#[cfg(feature = "tui-agent")]
pub const MAX_REACT_ITERATIONS: usize = 20;

/// Default timeout for a ReAct session (5 minutes)
#[cfg(feature = "tui-agent")]
pub const DEFAULT_REACT_TIMEOUT: Duration = Duration::from_secs(300);

/// Complete state of a ReAct reasoning session
///
/// Tracks the full lifecycle of a ReAct session including:
/// - Current phase of execution
/// - Iteration count and limits
/// - Timing information
/// - Accumulated context from previous iterations
/// - Pending and completed actions
///
/// # Example
/// ```rust,ignore
/// let mut state = ReActState::new();
/// state.transition_to(ReActPhase::Thinking)?;
/// // ... process thought ...
/// state.increment_iteration()?;
/// ```
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone)]
pub struct ReActState {
    /// Current phase of the ReAct cycle
    pub phase: ReActPhase,
    /// Current iteration number (0-indexed)
    pub iteration: usize,
    /// When this session started
    pub started_at: Instant,
    /// Most recent thought summary from the LLM
    pub last_thought: Option<ThoughtSummary>,
    /// Actions queued for execution
    pub pending_actions: Vec<Action>,
    /// Accumulated observations and context from previous iterations
    pub accumulated_context: Vec<String>,
    /// Maximum iterations allowed for this session
    pub max_iterations: usize,
    /// Timeout for this session
    pub timeout: Duration,
    /// History of all iterations
    pub iteration_history: Vec<ReActIteration>,
}

/// Current phase in the ReAct reasoning cycle
///
/// The state machine follows this flow:
/// ```text
/// Idle -> Thinking -> Acting -> Observing -> Thinking -> ...
///                                              |
///                                              v
///                                         Terminated
/// ```
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum ReActPhase {
    /// Waiting for a query to start
    Idle,
    /// LLM is reasoning about the current state
    Thinking,
    /// Executing actions/tools
    Acting,
    /// Collecting observations from executed actions
    Observing,
    /// Session has ended
    Terminated {
        /// Reason for termination
        reason: TerminationReason,
    },
}

/// Reason why a ReAct session terminated
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    /// LLM determined the task is complete
    TaskComplete,
    /// Hit the maximum iteration limit
    MaxIterationsReached,
    /// User cancelled the session
    UserCancelled,
    /// An error occurred during execution
    Error,
    /// Session exceeded timeout duration
    Timeout,
}

/// Outcome of a single ReAct iteration
///
/// After each iteration, the system evaluates whether to:
/// - Continue to the next iteration
/// - Complete with a final answer
/// - Request additional input from the user
/// - Terminate due to an error
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum IterationOutcome {
    /// Continue to next iteration
    Continue {
        /// Brief summary of the thought process
        thought_summary: String,
        /// Number of actions queued
        action_count: usize,
    },
    /// Task is complete
    Complete {
        /// Final answer from the LLM
        final_answer: String,
    },
    /// Need additional input from user
    NeedsInput {
        /// Prompt for the user
        prompt: String,
    },
    /// Error occurred during iteration
    Error {
        /// Error message
        message: String,
    },
}

/// Record of a complete ReAct iteration
///
/// Captures all information about a single thought-action-observation cycle
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActIteration {
    /// Iteration number (0-indexed)
    pub number: usize,
    /// The thought/reasoning text from the LLM
    pub thought_text: String,
    /// Names of actions that were executed
    pub actions_taken: Vec<String>,
    /// Observations collected from action executions
    pub observations: Vec<String>,
    /// How long this iteration took
    #[serde(with = "duration_serde")]
    pub duration: Duration,
}

/// A single action to be executed by the agent
///
/// Represents a tool call with its arguments and execution status
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Name of the tool to execute
    pub tool_name: String,
    /// Arguments to pass to the tool (JSON format)
    pub arguments: serde_json::Value,
    /// Current execution status
    pub status: ActionStatus,
    /// Result of execution (if completed or failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ActionResult>,
}

/// Execution status of an action
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Queued but not yet executing
    Pending,
    /// Currently executing
    Executing,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
    /// Skipped (e.g., due to previous failure)
    Skipped,
}

/// Result of an action execution
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded
    pub success: bool,
    /// Output from the action
    pub output: String,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Execution duration
    #[serde(with = "duration_serde")]
    pub duration: Duration,
}

/// A summary of a thought generated by the LLM during reasoning
///
/// This is a lightweight representation for state tracking. For the full
/// parsed thought structure with planned actions, see `thought::Thought`.
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtSummary {
    /// The raw thought text from the LLM
    pub text: String,
    /// Parsed intent/plan
    pub intent: String,
    /// Confidence level (0.0 - 1.0)
    pub confidence: Option<f32>,
}

/// Errors that can occur during state transitions
#[cfg(feature = "tui-agent")]
#[derive(Error, Debug)]
pub enum StateError {
    /// Invalid state transition attempted
    #[error("Invalid state transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Source phase
        from: ReActPhase,
        /// Target phase
        to: ReActPhase,
    },

    /// Maximum iterations exceeded
    #[error("Maximum iterations ({max}) exceeded")]
    MaxIterationsExceeded {
        /// Maximum allowed iterations
        max: usize,
    },

    /// Session has timed out
    #[error("Session timeout after {elapsed:?}")]
    Timeout {
        /// How long the session ran
        elapsed: Duration,
    },

    /// Session is already terminated
    #[error("Session already terminated: {reason:?}")]
    AlreadyTerminated {
        /// Termination reason
        reason: TerminationReason,
    },
}

// Serde helper for Duration serialization
#[cfg(feature = "tui-agent")]
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(feature = "tui-agent")]
impl ReActState {
    /// Create a new ReAct state in Idle phase
    ///
    /// Initializes with default configuration:
    /// - Maximum iterations: 20
    /// - Timeout: 5 minutes
    /// - Empty context and action queues
    pub fn new() -> Self {
        Self {
            phase: ReActPhase::Idle,
            iteration: 0,
            started_at: Instant::now(),
            last_thought: None,
            pending_actions: Vec::new(),
            accumulated_context: Vec::new(),
            max_iterations: MAX_REACT_ITERATIONS,
            timeout: DEFAULT_REACT_TIMEOUT,
            iteration_history: Vec::new(),
        }
    }

    /// Create a new state with custom configuration
    pub fn with_config(max_iterations: usize, timeout: Duration) -> Self {
        Self {
            phase: ReActPhase::Idle,
            iteration: 0,
            started_at: Instant::now(),
            last_thought: None,
            pending_actions: Vec::new(),
            accumulated_context: Vec::new(),
            max_iterations,
            timeout,
            iteration_history: Vec::new(),
        }
    }

    /// Reset the state for a new query
    ///
    /// Clears all accumulated context and resets to Idle phase
    /// while preserving configuration (max_iterations, timeout)
    pub fn reset(&mut self) {
        self.phase = ReActPhase::Idle;
        self.iteration = 0;
        self.started_at = Instant::now();
        self.last_thought = None;
        self.pending_actions.clear();
        self.accumulated_context.clear();
        self.iteration_history.clear();
    }

    /// Transition to a new phase
    ///
    /// Validates the transition is legal according to the state machine:
    /// - Idle -> Thinking (start of session)
    /// - Thinking -> Acting (actions queued)
    /// - Acting -> Observing (actions executing)
    /// - Observing -> Thinking (continue loop) or Terminated (done)
    /// - Any -> Terminated (error/cancel)
    ///
    /// # Errors
    /// Returns `StateError::InvalidTransition` if the transition is not allowed
    /// Returns `StateError::AlreadyTerminated` if session is already terminated
    pub fn transition_to(&mut self, new_phase: ReActPhase) -> Result<(), StateError> {
        // Check if already terminated
        if let ReActPhase::Terminated { reason } = self.phase {
            return Err(StateError::AlreadyTerminated { reason });
        }

        // Validate transition
        let valid = match (&self.phase, &new_phase) {
            // Starting the session
            (ReActPhase::Idle, ReActPhase::Thinking) => true,

            // Normal cycle progression
            (ReActPhase::Thinking, ReActPhase::Acting) => true,
            (ReActPhase::Acting, ReActPhase::Observing) => true,
            (ReActPhase::Observing, ReActPhase::Thinking) => true,

            // Can terminate from any state
            (_, ReActPhase::Terminated { .. }) => true,

            // Can reset from any state back to Idle
            (_, ReActPhase::Idle) => true,

            // All other transitions are invalid
            _ => false,
        };

        if !valid {
            return Err(StateError::InvalidTransition {
                from: self.phase,
                to: new_phase,
            });
        }

        self.phase = new_phase;
        Ok(())
    }

    /// Increment the iteration counter
    ///
    /// Should be called at the start of each new reasoning cycle.
    /// Checks against max_iterations limit.
    ///
    /// # Errors
    /// Returns `StateError::MaxIterationsExceeded` if limit is reached
    pub fn increment_iteration(&mut self) -> Result<(), StateError> {
        if self.iteration >= self.max_iterations {
            return Err(StateError::MaxIterationsExceeded {
                max: self.max_iterations,
            });
        }
        self.iteration += 1;
        Ok(())
    }

    /// Get elapsed time since session start
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Check if the session has timed out
    pub fn has_timed_out(&self) -> bool {
        self.elapsed() >= self.timeout
    }

    /// Check if session is in a terminated state
    pub fn is_terminated(&self) -> bool {
        matches!(self.phase, ReActPhase::Terminated { .. })
    }

    /// Check if the session can continue to next iteration
    ///
    /// Returns false if:
    /// - Already terminated
    /// - Max iterations reached
    /// - Timeout exceeded
    pub fn can_continue(&self) -> bool {
        !self.is_terminated() && self.iteration < self.max_iterations && !self.has_timed_out()
    }

    /// Get the current termination reason if terminated
    pub fn termination_reason(&self) -> Option<TerminationReason> {
        match self.phase {
            ReActPhase::Terminated { reason } => Some(reason),
            _ => None,
        }
    }

    /// Add context from an observation
    pub fn add_context(&mut self, context: String) {
        self.accumulated_context.push(context);
    }

    /// Add an action to the pending queue
    pub fn queue_action(&mut self, action: Action) {
        self.pending_actions.push(action);
    }

    /// Clear all pending actions
    pub fn clear_pending_actions(&mut self) {
        self.pending_actions.clear();
    }

    /// Get number of pending actions
    pub fn pending_action_count(&self) -> usize {
        self.pending_actions.len()
    }

    /// Complete the current iteration and record it in history
    pub fn complete_iteration(
        &mut self,
        thought_text: String,
        actions_taken: Vec<String>,
        observations: Vec<String>,
        duration: Duration,
    ) {
        let iteration = ReActIteration {
            number: self.iteration,
            thought_text,
            actions_taken,
            observations,
            duration,
        };
        self.iteration_history.push(iteration);
    }

    /// Get progress as a percentage (0.0 - 1.0)
    pub fn progress(&self) -> f32 {
        if self.max_iterations == 0 {
            return 0.0;
        }
        (self.iteration as f32 / self.max_iterations as f32).min(1.0)
    }

    /// Get total context size in characters
    pub fn context_size(&self) -> usize {
        self.accumulated_context.iter().map(|s| s.len()).sum()
    }

    /// Terminate the session with a reason
    pub fn terminate(&mut self, reason: TerminationReason) -> Result<(), StateError> {
        self.transition_to(ReActPhase::Terminated { reason })
    }
}

#[cfg(feature = "tui-agent")]
impl Default for ReActState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "tui-agent")]
impl Action {
    /// Create a new pending action
    pub fn new(tool_name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments,
            status: ActionStatus::Pending,
            result: None,
        }
    }

    /// Mark action as executing
    pub fn mark_executing(&mut self) {
        self.status = ActionStatus::Executing;
    }

    /// Mark action as completed with result
    pub fn mark_completed(&mut self, output: String, duration: Duration) {
        self.status = ActionStatus::Completed;
        self.result = Some(ActionResult {
            success: true,
            output,
            error: None,
            duration,
        });
    }

    /// Mark action as failed with error
    pub fn mark_failed(&mut self, error: String, duration: Duration) {
        self.status = ActionStatus::Failed;
        self.result = Some(ActionResult {
            success: false,
            output: String::new(),
            error: Some(error),
            duration,
        });
    }

    /// Mark action as skipped
    pub fn mark_skipped(&mut self) {
        self.status = ActionStatus::Skipped;
    }

    /// Check if action is in a terminal state (completed, failed, or skipped)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ActionStatus::Completed | ActionStatus::Failed | ActionStatus::Skipped
        )
    }

    /// Get output text if available
    pub fn output(&self) -> Option<&str> {
        self.result.as_ref().map(|r| r.output.as_str())
    }

    /// Get error text if available
    pub fn error(&self) -> Option<&str> {
        self.result.as_ref().and_then(|r| r.error.as_deref())
    }
}

#[cfg(feature = "tui-agent")]
impl ThoughtSummary {
    /// Create a new thought summary
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            text: text.clone(),
            intent: text, // Default intent is same as text
            confidence: None,
        }
    }

    /// Create a thought summary with explicit intent
    pub fn with_intent(text: impl Into<String>, intent: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            intent: intent.into(),
            confidence: None,
        }
    }

    /// Set confidence level
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }
}

#[cfg(all(test, feature = "tui-agent"))]
mod tests {
    use super::*;

    #[test]
    fn test_state_new() {
        let state = ReActState::new();
        assert_eq!(state.phase, ReActPhase::Idle);
        assert_eq!(state.iteration, 0);
        assert!(state.can_continue());
    }

    #[test]
    fn test_state_reset() {
        let mut state = ReActState::new();
        state.iteration = 5;
        state.phase = ReActPhase::Acting;
        state.accumulated_context.push("test".to_string());

        state.reset();

        assert_eq!(state.phase, ReActPhase::Idle);
        assert_eq!(state.iteration, 0);
        assert!(state.accumulated_context.is_empty());
    }

    #[test]
    fn test_valid_transitions() {
        let mut state = ReActState::new();

        // Idle -> Thinking
        assert!(state.transition_to(ReActPhase::Thinking).is_ok());
        assert_eq!(state.phase, ReActPhase::Thinking);

        // Thinking -> Acting
        assert!(state.transition_to(ReActPhase::Acting).is_ok());
        assert_eq!(state.phase, ReActPhase::Acting);

        // Acting -> Observing
        assert!(state.transition_to(ReActPhase::Observing).is_ok());
        assert_eq!(state.phase, ReActPhase::Observing);

        // Observing -> Thinking (loop back)
        assert!(state.transition_to(ReActPhase::Thinking).is_ok());
        assert_eq!(state.phase, ReActPhase::Thinking);
    }

    #[test]
    fn test_invalid_transition() {
        let mut state = ReActState::new();

        // Can't go directly from Idle to Acting
        let result = state.transition_to(ReActPhase::Acting);
        assert!(result.is_err());
        assert!(matches!(result, Err(StateError::InvalidTransition { .. })));
    }

    #[test]
    fn test_termination() {
        let mut state = ReActState::new();

        // Can terminate from any state
        assert!(state
            .transition_to(ReActPhase::Terminated {
                reason: TerminationReason::TaskComplete
            })
            .is_ok());

        assert!(state.is_terminated());
        assert!(!state.can_continue());
        assert_eq!(
            state.termination_reason(),
            Some(TerminationReason::TaskComplete)
        );

        // Can't transition after termination
        let result = state.transition_to(ReActPhase::Thinking);
        assert!(matches!(result, Err(StateError::AlreadyTerminated { .. })));
    }

    #[test]
    fn test_iteration_increment() {
        let mut state = ReActState::with_config(3, Duration::from_secs(60));

        assert_eq!(state.iteration, 0);
        assert!(state.increment_iteration().is_ok());
        assert_eq!(state.iteration, 1);
        assert!(state.increment_iteration().is_ok());
        assert_eq!(state.iteration, 2);
        assert!(state.increment_iteration().is_ok());
        assert_eq!(state.iteration, 3);

        // Should fail on max
        let result = state.increment_iteration();
        assert!(matches!(
            result,
            Err(StateError::MaxIterationsExceeded { .. })
        ));
    }

    #[test]
    fn test_timeout_check() {
        let state = ReActState::with_config(10, Duration::from_millis(10));
        assert!(!state.has_timed_out());

        std::thread::sleep(Duration::from_millis(20));
        assert!(state.has_timed_out());
        assert!(!state.can_continue());
    }

    #[test]
    fn test_action_lifecycle() {
        let mut action = Action::new("test_tool", serde_json::json!({"arg": "value"}));
        assert_eq!(action.status, ActionStatus::Pending);
        assert!(!action.is_terminal());

        action.mark_executing();
        assert_eq!(action.status, ActionStatus::Executing);
        assert!(!action.is_terminal());

        action.mark_completed("result".to_string(), Duration::from_millis(100));
        assert_eq!(action.status, ActionStatus::Completed);
        assert!(action.is_terminal());
        assert_eq!(action.output(), Some("result"));
        assert_eq!(action.error(), None);
    }

    #[test]
    fn test_action_failure() {
        let mut action = Action::new("test_tool", serde_json::json!({}));
        action.mark_failed("error message".to_string(), Duration::from_millis(50));

        assert_eq!(action.status, ActionStatus::Failed);
        assert!(action.is_terminal());
        assert_eq!(action.error(), Some("error message"));
    }

    #[test]
    fn test_context_management() {
        let mut state = ReActState::new();
        assert_eq!(state.context_size(), 0);

        state.add_context("First observation".to_string());
        state.add_context("Second observation".to_string());

        assert_eq!(state.accumulated_context.len(), 2);
        assert!(state.context_size() > 0);
    }

    #[test]
    fn test_action_queue() {
        let mut state = ReActState::new();
        assert_eq!(state.pending_action_count(), 0);

        state.queue_action(Action::new("tool1", serde_json::json!({})));
        state.queue_action(Action::new("tool2", serde_json::json!({})));

        assert_eq!(state.pending_action_count(), 2);

        state.clear_pending_actions();
        assert_eq!(state.pending_action_count(), 0);
    }

    #[test]
    fn test_progress_calculation() {
        let mut state = ReActState::with_config(10, Duration::from_secs(60));
        assert_eq!(state.progress(), 0.0);

        state.iteration = 5;
        assert_eq!(state.progress(), 0.5);

        state.iteration = 10;
        assert_eq!(state.progress(), 1.0);
    }

    #[test]
    fn test_thought_summary_creation() {
        let thought = ThoughtSummary::new("I need to check the file");
        assert_eq!(thought.text, "I need to check the file");
        assert_eq!(thought.intent, "I need to check the file");
        assert_eq!(thought.confidence, None);

        let thought =
            ThoughtSummary::with_intent("Raw text", "Parsed intent").with_confidence(0.95);
        assert_eq!(thought.text, "Raw text");
        assert_eq!(thought.intent, "Parsed intent");
        assert_eq!(thought.confidence, Some(0.95));
    }

    #[test]
    fn test_complete_iteration() {
        let mut state = ReActState::new();

        state.complete_iteration(
            "I should check the file".to_string(),
            vec!["cat".to_string()],
            vec!["file contents".to_string()],
            Duration::from_millis(250),
        );

        assert_eq!(state.iteration_history.len(), 1);
        assert_eq!(state.iteration_history[0].number, 0);
        assert_eq!(state.iteration_history[0].actions_taken.len(), 1);
        assert_eq!(state.iteration_history[0].observations.len(), 1);
    }
}
