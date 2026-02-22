//! ReAct engine orchestrator for TUI agent mode
//!
//! This module provides the main ReAct loop coordinator that orchestrates:
//! - Iterative Think-Act-Observe cycles
//! - LLM integration for reasoning
//! - Tool execution with timeout handling
//! - Context gathering and management
//! - Event emission for UI updates
//! - Cancellation and error recovery
//!
//! # Architecture
//!
//! The ReAct engine follows the classic ReAct pattern:
//!
//! ```text
//! User Query
//!     ↓
//! ┌─────────────────────┐
//! │ Gather Context      │ (from history, docs, etc.)
//! └──────────┬──────────┘
//!            ↓
//! ┌─────────────────────┐
//! │ THINK: LLM Reasoning│ (analyze situation, plan actions)
//! └──────────┬──────────┘
//!            ↓
//!     Parse Thought
//!            ↓
//!     ┌──────┴──────┐
//!     │             │
//! Final Answer   Actions?
//!     │             │
//!  COMPLETE    ┌────┴─────┐
//!              │ ACT: Execute Tools │
//!              └────┬─────┘
//!                   ↓
//!              ┌─────────────────────┐
//!              │ OBSERVE: Process Results │
//!              └──────────┬──────────┘
//!                         ↓
//!                 Add to Context
//!                         ↓
//!                 Next Iteration
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! # use mistralrs_tui::agent::execution::ToolExecutor;
//! # use mistralrs_tui::agent::events::EventBus;
//! # use mistralrs_tui::agent::react::{TuiReActEngine, ReActConfig};
//! # use mistralrs_agent_tools::AgentToolkit;
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let toolkit = AgentToolkit::with_defaults();
//! let event_bus = EventBus::new(100);
//! let tool_executor = ToolExecutor::with_events(toolkit, event_bus.clone());
//!
//! let mut engine = TuiReActEngine::new(tool_executor, event_bus);
//!
//! let response = engine.run("What files are in the current directory?").await?;
//! println!("Answer: {}", response.final_answer.unwrap_or_default());
//! # Ok(())
//! # }
//! ```

#![cfg(feature = "tui-agent")]

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::observation::{Observation, ObservationProcessor, ObservationSummary};
use super::state::{
    Action, IterationOutcome, ReActIteration, ReActPhase, ReActState, TerminationReason,
};
use super::thought::{PlannedAction, Thought, ThoughtParser};

use crate::agent::context::gatherer::{CompositeContextGatherer, GatheredContext};
use crate::agent::events::{EventBus, ExecutionEvent};
use crate::agent::execution::ToolExecutor;
use crate::agent::llm_integration::LLMToolCall;
use crate::agent::toolkit::ToolCall;

/// Configuration for ReAct engine behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActConfig {
    /// Maximum number of iterations before forced termination
    pub max_iterations: usize,

    /// Timeout for entire ReAct session (seconds)
    pub session_timeout_secs: u64,

    /// Timeout for single iteration (seconds)
    pub iteration_timeout_secs: u64,

    /// Timeout for individual tool execution (seconds)
    pub tool_timeout_secs: u64,

    /// Maximum tokens to use for context window
    pub context_window_tokens: usize,

    /// Whether to execute tools in parallel when possible
    pub parallel_tool_execution: bool,

    /// Whether to include iteration history in context
    pub include_history_in_context: bool,

    /// Maximum observation tokens before truncation
    pub max_observation_tokens: usize,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            session_timeout_secs: 300,   // 5 minutes
            iteration_timeout_secs: 60,  // 1 minute per iteration
            tool_timeout_secs: 30,       // 30 seconds per tool
            context_window_tokens: 4096, // Standard context window
            parallel_tool_execution: true,
            include_history_in_context: true,
            max_observation_tokens: 500, // ~2000 chars per observation
        }
    }
}

/// Response from a complete ReAct session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActResponse {
    /// Final answer from the agent (if completed successfully)
    pub final_answer: Option<String>,

    /// All iterations executed
    pub iterations: Vec<ReActIteration>,

    /// Why the session terminated
    pub terminated_reason: TerminationReason,

    /// Total duration of the session
    #[serde(with = "duration_serde")]
    pub total_duration: Duration,

    /// Total number of tools executed
    pub total_tools_executed: usize,

    /// Session ID for tracking
    pub session_id: Uuid,
}

/// Main ReAct engine orchestrator
///
/// Coordinates the Think-Act-Observe loop with all supporting components:
/// - State management via `ReActState`
/// - LLM reasoning via `ThoughtParser`
/// - Tool execution via `ToolExecutor`
/// - Observation processing via `ObservationProcessor`
/// - Context gathering via `CompositeContextGatherer`
/// - Event emission via `EventBus`
pub struct TuiReActEngine {
    /// Tool executor for running actions
    tool_executor: ToolExecutor,

    /// Event bus for UI updates
    event_bus: EventBus,

    /// Engine configuration
    config: ReActConfig,

    /// Parser for LLM thoughts
    thought_parser: ThoughtParser,

    /// Processor for tool observations
    observation_processor: ObservationProcessor,

    /// Context gatherer (optional)
    context_gatherer: Option<CompositeContextGatherer>,

    /// Current session state
    state: Arc<Mutex<ReActState>>,

    /// Current user query
    user_query: String,

    /// Cancellation flag
    cancelled: Arc<Mutex<bool>>,

    /// Session ID for tracking
    session_id: Uuid,
}

impl TuiReActEngine {
    /// Create a new ReAct engine with default configuration
    ///
    /// # Arguments
    ///
    /// * `tool_executor` - Executor for running tools
    /// * `event_bus` - Event bus for UI updates
    pub fn new(tool_executor: ToolExecutor, event_bus: EventBus) -> Self {
        let config = ReActConfig::default();
        Self::with_config(tool_executor, event_bus, config)
    }

    /// Create a new ReAct engine with custom configuration
    ///
    /// # Arguments
    ///
    /// * `tool_executor` - Executor for running tools
    /// * `event_bus` - Event bus for UI updates
    /// * `config` - Custom configuration
    pub fn with_config(
        tool_executor: ToolExecutor,
        event_bus: EventBus,
        config: ReActConfig,
    ) -> Self {
        let state = ReActState::with_config(
            config.max_iterations,
            Duration::from_secs(config.session_timeout_secs),
        );

        let observation_processor = ObservationProcessor::new(config.max_observation_tokens);

        Self {
            tool_executor,
            event_bus,
            config,
            thought_parser: ThoughtParser::new(),
            observation_processor,
            context_gatherer: None,
            state: Arc::new(Mutex::new(state)),
            user_query: String::new(),
            cancelled: Arc::new(Mutex::new(false)),
            session_id: Uuid::new_v4(),
        }
    }

    /// Add a context gatherer to this engine (builder pattern)
    ///
    /// # Arguments
    ///
    /// * `gatherer` - Composite context gatherer to use
    pub fn with_context_gatherer(mut self, gatherer: CompositeContextGatherer) -> Self {
        self.context_gatherer = Some(gatherer);
        self
    }

    /// Run the full ReAct loop until completion or termination
    ///
    /// # Arguments
    ///
    /// * `user_query` - The user's query or task
    ///
    /// # Returns
    ///
    /// Complete `ReActResponse` with all iteration data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use mistralrs_tui::agent::react::TuiReActEngine;
    /// # async fn example(mut engine: TuiReActEngine) -> anyhow::Result<()> {
    /// let response = engine.run("List all .rs files").await?;
    /// if let Some(answer) = response.final_answer {
    ///     println!("Answer: {}", answer);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(&mut self, user_query: &str) -> Result<ReActResponse> {
        self.user_query = user_query.to_string();
        self.session_id = Uuid::new_v4();

        // Reset state for new query
        {
            let mut state = self.state.lock().expect("Failed to lock state");
            state.reset();
        }

        // Reset cancellation flag
        *self.cancelled.lock().expect("Failed to lock cancelled") = false;

        info!(
            "Starting ReAct session {} for query: {}",
            self.session_id, user_query
        );

        let session_start = Instant::now();
        let mut final_answer: Option<String> = None;
        let mut termination_reason = TerminationReason::MaxIterationsReached;
        let mut total_tools_executed = 0;

        // Emit session started event
        self.emit_progress(0, "Starting ReAct session...", 0.0);

        // Main ReAct loop
        while self.can_continue() {
            let iteration = self.current_iteration();
            debug!("Starting iteration {}", iteration);

            // Execute one iteration step
            match self.step().await {
                Ok(IterationOutcome::Continue { .. }) => {
                    // Continue to next iteration
                    debug!("Iteration {} continuing", iteration);
                }
                Ok(IterationOutcome::Complete {
                    final_answer: answer,
                }) => {
                    // Task complete
                    info!("Task completed with final answer");
                    final_answer = Some(answer);
                    termination_reason = TerminationReason::TaskComplete;
                    break;
                }
                Ok(IterationOutcome::NeedsInput { prompt }) => {
                    // Need user input (not supported in current implementation)
                    warn!("Agent requested input: {}", prompt);
                    termination_reason = TerminationReason::Error;
                    break;
                }
                Ok(IterationOutcome::Error { message }) => {
                    // Error during iteration
                    warn!("Iteration failed: {}", message);
                    termination_reason = TerminationReason::Error;
                    break;
                }
                Err(e) => {
                    warn!("Iteration error: {}", e);
                    termination_reason = TerminationReason::Error;
                    break;
                }
            }

            // Count tools executed in this iteration
            let state = self.state.lock().expect("Failed to lock state");
            if let Some(last_iteration) = state.iteration_history.last() {
                total_tools_executed += last_iteration.actions_taken.len();
            }
        }

        // Check final termination reason if not already set
        {
            let state = self.state.lock().expect("Failed to lock state");
            if state.has_timed_out() {
                termination_reason = TerminationReason::Timeout;
            } else if *self.cancelled.lock().expect("Failed to lock cancelled") {
                termination_reason = TerminationReason::UserCancelled;
            }
        }

        let total_duration = session_start.elapsed();

        // Emit session completed event
        self.emit_progress(
            100,
            &format!("Session completed: {:?}", termination_reason),
            100.0,
        );

        // Build response
        let state = self.state.lock().expect("Failed to lock state");
        Ok(ReActResponse {
            final_answer,
            iterations: state.iteration_history.clone(),
            terminated_reason: termination_reason,
            total_duration,
            total_tools_executed,
            session_id: self.session_id,
        })
    }

    /// Execute a single ReAct iteration step
    ///
    /// This is the granular API for TUI control, allowing the UI to step through
    /// the reasoning process one iteration at a time.
    ///
    /// # Returns
    ///
    /// `IterationOutcome` indicating what happened in this iteration
    pub async fn step(&mut self) -> Result<IterationOutcome> {
        // Check cancellation
        if self.is_cancelled() {
            return Ok(IterationOutcome::Error {
                message: "Cancelled by user".to_string(),
            });
        }

        let iteration_start = Instant::now();
        let iteration_num = self.current_iteration();

        debug!("Step: iteration {}", iteration_num);

        // Increment iteration counter
        {
            let mut state = self.state.lock().expect("Failed to lock state");
            if let Err(e) = state.increment_iteration() {
                warn!("Failed to increment iteration: {}", e);
                return Ok(IterationOutcome::Error {
                    message: format!("Max iterations reached: {}", e),
                });
            }
        }

        // PHASE 1: THINK
        self.transition_to_phase(ReActPhase::Thinking)?;
        let thought = match self.think().await {
            Ok(t) => t,
            Err(e) => {
                return Ok(IterationOutcome::Error {
                    message: format!("Thinking failed: {}", e),
                });
            }
        };

        // Check if this is a final answer
        if thought.is_final_answer {
            let duration = iteration_start.elapsed();
            self.complete_iteration(thought.reasoning.clone(), vec![], vec![], duration);
            return Ok(IterationOutcome::Complete {
                final_answer: thought.reasoning,
            });
        }

        // Check if there are actions to execute
        if thought.planned_actions.is_empty() {
            // No actions, treat as continuation with just reasoning
            let duration = iteration_start.elapsed();
            self.complete_iteration(thought.reasoning.clone(), vec![], vec![], duration);
            return Ok(IterationOutcome::Continue {
                thought_summary: thought.reasoning,
                action_count: 0,
            });
        }

        // PHASE 2: ACT
        self.transition_to_phase(ReActPhase::Acting)?;
        let observations = match self.act(thought.planned_actions.clone()).await {
            Ok(obs) => obs,
            Err(e) => {
                return Ok(IterationOutcome::Error {
                    message: format!("Action execution failed: {}", e),
                });
            }
        };

        // PHASE 3: OBSERVE
        self.transition_to_phase(ReActPhase::Observing)?;
        let observation_summary = match self.observe(observations).await {
            Ok(summary) => summary,
            Err(e) => {
                return Ok(IterationOutcome::Error {
                    message: format!("Observation processing failed: {}", e),
                });
            }
        };

        // Record iteration in history
        let duration = iteration_start.elapsed();
        let action_names: Vec<String> = thought
            .planned_actions
            .iter()
            .map(|a| a.tool_name.clone())
            .collect();
        let observation_texts: Vec<String> = observation_summary
            .observations
            .iter()
            .map(|o| o.content.clone())
            .collect();

        self.complete_iteration(
            thought.reasoning.clone(),
            action_names.clone(),
            observation_texts,
            duration,
        );

        // Return continue outcome
        Ok(IterationOutcome::Continue {
            thought_summary: thought.reasoning,
            action_count: action_names.len(),
        })
    }

    /// THINK phase: Generate reasoning via LLM
    ///
    /// This phase:
    /// 1. Gathers context from various sources
    /// 2. Builds prompt with query + context + history
    /// 3. Calls LLM for reasoning
    /// 4. Parses response into structured `Thought`
    async fn think(&mut self) -> Result<Thought> {
        let iteration = self.current_iteration();
        self.emit_progress(
            iteration,
            &format!("Iteration {} - Thinking...", iteration),
            (iteration as f64 / self.config.max_iterations as f64) * 100.0,
        );

        // Gather context
        let context = self.gather_context().await?;

        // Build prompt for LLM
        let prompt = self.build_prompt(&context)?;

        debug!("LLM prompt: {} chars", prompt.len());

        // TODO: Call actual LLM here
        // For now, return a mock thought with parsed actions
        // In real implementation, this would call the LLM API and parse the response

        // Mock response for demonstration
        let llm_response = self.mock_llm_call(&prompt).await?;
        let tool_calls: Vec<LLMToolCall> = vec![]; // Would be extracted from LLM response

        // Parse thought from LLM response
        let thought = self
            .thought_parser
            .parse(&llm_response, &tool_calls)
            .context("Failed to parse LLM thought")?;

        debug!(
            "Parsed thought: {} actions planned",
            thought.planned_actions.len()
        );

        Ok(thought)
    }

    /// ACT phase: Execute planned actions
    ///
    /// This phase:
    /// 1. Converts planned actions to tool calls
    /// 2. Executes tools (parallel or sequential based on config)
    /// 3. Collects observations from results
    async fn act(&mut self, actions: Vec<PlannedAction>) -> Result<Vec<Observation>> {
        let iteration = self.current_iteration();
        self.emit_progress(
            iteration,
            &format!(
                "Iteration {} - Acting ({} tools)...",
                iteration,
                actions.len()
            ),
            (iteration as f64 / self.config.max_iterations as f64) * 100.0,
        );

        // Convert planned actions to executable actions
        let mut executable_actions = Vec::new();
        for planned_action in &actions {
            let arguments = planned_action
                .arguments
                .clone()
                .unwrap_or(serde_json::json!({}));

            let action = Action::new(&planned_action.tool_name, arguments);
            executable_actions.push(action);
        }

        // Execute tools
        let observations = if self.config.parallel_tool_execution && executable_actions.len() > 1 {
            // Parallel execution
            self.execute_actions_parallel(executable_actions).await?
        } else {
            // Sequential execution
            self.execute_actions_sequential(executable_actions).await?
        };

        debug!("Collected {} observations", observations.len());

        Ok(observations)
    }

    /// OBSERVE phase: Process tool results
    ///
    /// This phase:
    /// 1. Processes raw tool results into observations
    /// 2. Summarizes observations for context
    /// 3. Adds observations to accumulated context
    async fn observe(&mut self, observations: Vec<Observation>) -> Result<ObservationSummary> {
        let iteration = self.current_iteration();
        self.emit_progress(
            iteration,
            &format!("Iteration {} - Observing results...", iteration),
            (iteration as f64 / self.config.max_iterations as f64) * 100.0,
        );

        // Summarize observations
        let summary = self.observation_processor.summarize(observations);

        // Add summary to context
        {
            let mut state = self.state.lock().expect("Failed to lock state");
            state.add_context(summary.formatted_for_llm.clone());
        }

        debug!(
            "Observation summary: {} observations, all successful: {}",
            summary.observations.len(),
            summary.all_successful
        );

        Ok(summary)
    }

    /// Execute actions sequentially
    async fn execute_actions_sequential(
        &mut self,
        actions: Vec<Action>,
    ) -> Result<Vec<Observation>> {
        let mut observations = Vec::with_capacity(actions.len());

        for mut action in actions {
            action.mark_executing();

            let start = Instant::now();
            let result = self
                .tool_executor
                .execute(
                    &action.tool_name,
                    action.arguments.clone(),
                    Some(self.config.tool_timeout_secs),
                )
                .await?;

            let duration = start.elapsed();

            // Create tool call for observation processing
            let tool_call = ToolCall {
                id: Uuid::new_v4(),
                tool_name: action.tool_name.clone(),
                arguments: action.arguments.clone(),
                result: Some(result.clone()),
                timestamp: Utc::now(),
                session_id: Some(self.session_id),
            };

            // Process into observation
            let observation = self.observation_processor.process(&result, &tool_call);
            observations.push(observation);

            // Update action status
            if result.success {
                action.mark_completed(result.output.to_string(), duration);
            } else {
                action.mark_failed(
                    result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    duration,
                );
            }
        }

        Ok(observations)
    }

    /// Execute actions in parallel
    async fn execute_actions_parallel(&mut self, actions: Vec<Action>) -> Result<Vec<Observation>> {
        let mut handles = Vec::new();

        // Spawn parallel tasks
        for action in actions {
            let executor = self.tool_executor.clone();
            let processor = self.observation_processor.clone();
            let timeout = self.config.tool_timeout_secs;
            let session_id = self.session_id;

            let handle = tokio::spawn(async move {
                let start = Instant::now();
                let result = executor
                    .execute(&action.tool_name, action.arguments.clone(), Some(timeout))
                    .await
                    .unwrap_or_else(|e| {
                        // Convert error to failed result
                        crate::agent::toolkit::ToolCallResult {
                            success: false,
                            output: serde_json::Value::Null,
                            error: Some(e.to_string()),
                            duration: start.elapsed(),
                        }
                    });

                let tool_call = ToolCall {
                    id: Uuid::new_v4(),
                    tool_name: action.tool_name.clone(),
                    arguments: action.arguments.clone(),
                    result: Some(result.clone()),
                    timestamp: Utc::now(),
                    session_id: Some(session_id),
                };

                processor.process(&result, &tool_call)
            });

            handles.push(handle);
        }

        // Wait for all to complete
        let mut observations = Vec::new();
        for handle in handles {
            let observation = handle.await.context("Tool execution task failed")?;
            observations.push(observation);
        }

        Ok(observations)
    }

    /// Gather context for the current iteration
    async fn gather_context(&self) -> Result<GatheredContext> {
        if let Some(ref gatherer) = self.context_gatherer {
            let iteration = self.current_iteration();
            gatherer
                .gather_all(&self.user_query, iteration)
                .await
                .context("Failed to gather context")
        } else {
            Ok(GatheredContext::empty())
        }
    }

    /// Build prompt for LLM including context and history
    fn build_prompt(&self, context: &GatheredContext) -> Result<String> {
        let mut prompt = String::new();

        // System instructions
        prompt.push_str("You are a helpful AI assistant with access to tools.\n\n");

        // Context
        if !context.chunks.is_empty() {
            prompt.push_str("# Context\n");
            prompt.push_str(&context.format());
            prompt.push_str("\n\n");
        }

        // Iteration history
        if self.config.include_history_in_context {
            let state = self.state.lock().expect("Failed to lock state");
            if !state.iteration_history.is_empty() {
                prompt.push_str("# Previous Iterations\n");
                for iteration in &state.iteration_history {
                    prompt.push_str(&format!(
                        "Iteration {}: {}\n",
                        iteration.number, iteration.thought_text
                    ));
                    if !iteration.actions_taken.is_empty() {
                        prompt.push_str(&format!(
                            "Actions: {}\n",
                            iteration.actions_taken.join(", ")
                        ));
                    }
                }
                prompt.push_str("\n");
            }
        }

        // User query
        prompt.push_str("# User Query\n");
        prompt.push_str(&self.user_query);
        prompt.push_str("\n\n");

        // Instructions
        prompt.push_str("Think step by step and use tools when appropriate.\n");
        prompt
            .push_str("If you have a final answer, respond with 'Final Answer: <your answer>'.\n");

        Ok(prompt)
    }

    /// Mock LLM call for testing (replace with real LLM integration)
    async fn mock_llm_call(&self, _prompt: &str) -> Result<String> {
        // TODO: Replace with actual LLM API call
        // This is a placeholder for demonstration

        let iteration = self.current_iteration();
        if iteration == 1 {
            Ok("Thought: I need to list the files to see what's available.\nAction: ls({\"path\": \".\", \"all\": false})".to_string())
        } else {
            Ok("Final Answer: I've listed the files in the current directory.".to_string())
        }
    }

    /// Cancel the current ReAct session
    pub fn cancel(&mut self) {
        *self.cancelled.lock().expect("Failed to lock cancelled") = true;
        info!("ReAct session {} cancelled", self.session_id);
    }

    /// Check if the session has been cancelled
    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.lock().expect("Failed to lock cancelled")
    }

    /// Get the current phase
    pub fn current_phase(&self) -> ReActPhase {
        let state = self.state.lock().expect("Failed to lock state");
        state.phase
    }

    /// Get the current iteration count
    pub fn current_iteration(&self) -> usize {
        let state = self.state.lock().expect("Failed to lock state");
        state.iteration
    }

    /// Check if the session can continue
    fn can_continue(&self) -> bool {
        let state = self.state.lock().expect("Failed to lock state");
        state.can_continue() && !self.is_cancelled()
    }

    /// Transition to a new phase
    fn transition_to_phase(&mut self, phase: ReActPhase) -> Result<()> {
        let mut state = self.state.lock().expect("Failed to lock state");
        state
            .transition_to(phase)
            .context("Failed to transition phase")
    }

    /// Complete the current iteration
    fn complete_iteration(
        &mut self,
        thought_text: String,
        actions_taken: Vec<String>,
        observations: Vec<String>,
        duration: Duration,
    ) {
        let mut state = self.state.lock().expect("Failed to lock state");
        state.complete_iteration(thought_text, actions_taken, observations, duration);
    }

    /// Emit a progress event
    fn emit_progress(&self, _iteration: usize, message: &str, percentage: f64) {
        let call_id = Uuid::new_v4();
        self.event_bus.emit(ExecutionEvent::Progress {
            call_id,
            message: message.to_string(),
            percentage: Some(percentage as f32),
            timestamp: Utc::now(),
        });
    }
}

/// Serde module for Duration serialization
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::execution::ToolExecutor;
    use mistralrs_agent_tools::AgentToolkit;

    #[tokio::test]
    async fn test_engine_creation() {
        let toolkit = AgentToolkit::with_defaults();
        let event_bus = EventBus::new(100);
        let executor = ToolExecutor::with_events(toolkit, event_bus.clone());

        let engine = TuiReActEngine::new(executor, event_bus);

        assert_eq!(engine.current_iteration(), 0);
        assert_eq!(engine.current_phase(), ReActPhase::Idle);
        assert!(!engine.is_cancelled());
    }

    #[tokio::test]
    async fn test_engine_with_custom_config() {
        let toolkit = AgentToolkit::with_defaults();
        let event_bus = EventBus::new(100);
        let executor = ToolExecutor::with_events(toolkit, event_bus.clone());

        let config = ReActConfig {
            max_iterations: 5,
            ..Default::default()
        };

        let engine = TuiReActEngine::with_config(executor, event_bus, config);

        let state = engine.state.lock().unwrap();
        assert_eq!(state.max_iterations, 5);
    }

    #[tokio::test]
    async fn test_cancellation() {
        let toolkit = AgentToolkit::with_defaults();
        let event_bus = EventBus::new(100);
        let executor = ToolExecutor::with_events(toolkit, event_bus.clone());

        let mut engine = TuiReActEngine::new(executor, event_bus);

        assert!(!engine.is_cancelled());

        engine.cancel();

        assert!(engine.is_cancelled());
    }

    #[tokio::test]
    async fn test_build_prompt() {
        let toolkit = AgentToolkit::with_defaults();
        let event_bus = EventBus::new(100);
        let executor = ToolExecutor::with_events(toolkit, event_bus.clone());

        let mut engine = TuiReActEngine::new(executor, event_bus);
        engine.user_query = "What is the weather?".to_string();

        let context = GatheredContext::empty();
        let prompt = engine.build_prompt(&context).unwrap();

        assert!(prompt.contains("What is the weather?"));
        assert!(prompt.contains("User Query"));
    }
}
