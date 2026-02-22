//! Observation processing for ReAct agent tool execution results
//!
//! This module provides structured observation types and processing logic for
//! formatting tool execution results for LLM consumption. It handles truncation,
//! error recovery hints, and observation summarization.

#![cfg(feature = "tui-agent")]

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::agent::toolkit::{ToolCall, ToolCallResult};

/// Structured observation from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Name of the tool that was executed
    pub tool_name: String,
    /// Unique identifier for this tool call
    pub tool_call_id: Uuid,
    /// Type of observation (success, error, etc.)
    pub kind: ObservationKind,
    /// Formatted content for LLM consumption
    pub content: String,
    /// Optional structured data (preserved for potential reprocessing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_data: Option<serde_json::Value>,
    /// Execution duration
    #[serde(with = "duration_serde")]
    pub duration: Duration,
    /// Metadata about observation processing
    pub metadata: ObservationMetadata,
}

/// Classification of observation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ObservationKind {
    /// Successful tool execution
    Success,
    /// Tool execution failed with error
    Error {
        /// Optional error code for categorization
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        /// Whether the error is recoverable with retry or alternative approach
        recoverable: bool,
    },
    /// Tool execution timed out
    Timeout,
    /// Partial result (e.g., truncated output)
    PartialResult {
        /// Estimated completeness (0.0-1.0)
        completeness: f32,
    },
}

/// Metadata about observation processing
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservationMetadata {
    /// Estimated token count of observation content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_estimated: Option<usize>,
    /// Whether content was truncated
    pub truncated: bool,
    /// Original content length before truncation
    pub original_length: usize,
}

/// Summary of multiple observations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationSummary {
    /// All observations in chronological order
    pub observations: Vec<Observation>,
    /// Whether all observations succeeded
    pub all_successful: bool,
    /// Whether any errors are potentially recoverable
    pub has_recoverable_errors: bool,
    /// Total duration across all tool calls
    #[serde(with = "duration_serde")]
    pub total_duration: Duration,
    /// Formatted summary text for LLM context
    pub formatted_for_llm: String,
}

/// Strategy for truncating long observation content
#[derive(Debug, Clone, Copy)]
pub enum TruncationStrategy {
    /// Keep first N% and last (100-N)% of content
    HeadTail {
        /// Ratio of tokens to keep from head (0.0-1.0)
        head_ratio: f32,
    },
    /// Keep only the beginning of content
    HeadOnly,
    /// Keep only the end of content
    TailOnly,
}

impl Default for TruncationStrategy {
    fn default() -> Self {
        Self::HeadTail { head_ratio: 0.6 }
    }
}

/// Processor for converting tool results into structured observations
#[derive(Debug, Clone)]
pub struct ObservationProcessor {
    /// Maximum tokens per observation before truncation
    max_observation_tokens: usize,
    /// Strategy for truncating long content
    truncation_strategy: TruncationStrategy,
}

impl ObservationProcessor {
    /// Create a new observation processor with default truncation strategy
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum tokens per observation before truncation
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_observation_tokens: max_tokens,
            truncation_strategy: TruncationStrategy::default(),
        }
    }

    /// Create a processor with custom truncation strategy
    ///
    /// # Arguments
    /// * `max_tokens` - Maximum tokens per observation
    /// * `strategy` - Truncation strategy to use
    pub fn with_truncation(max_tokens: usize, strategy: TruncationStrategy) -> Self {
        Self {
            max_observation_tokens: max_tokens,
            truncation_strategy: strategy,
        }
    }

    /// Process a tool call result into a structured observation
    ///
    /// # Arguments
    /// * `result` - The tool execution result
    /// * `call` - The original tool call metadata
    pub fn process(&self, result: &ToolCallResult, call: &ToolCall) -> Observation {
        let kind = self.classify_result(result);
        let raw_content = self.extract_content(result);
        let original_length = raw_content.len();

        let (content, truncated) =
            if self.estimate_tokens(&raw_content) > self.max_observation_tokens {
                (self.truncate_content(&raw_content), true)
            } else {
                (raw_content, false)
            };

        let tokens_estimated = Some(self.estimate_tokens(&content));

        Observation {
            tool_name: call.tool_name.clone(),
            tool_call_id: call.id,
            kind,
            content,
            structured_data: Some(result.output.clone()),
            duration: result.duration,
            metadata: ObservationMetadata {
                tokens_estimated,
                truncated,
                original_length,
            },
        }
    }

    /// Summarize multiple observations into a single summary
    ///
    /// # Arguments
    /// * `observations` - List of observations to summarize
    pub fn summarize(&self, observations: Vec<Observation>) -> ObservationSummary {
        let all_successful = observations
            .iter()
            .all(|o| matches!(o.kind, ObservationKind::Success));

        let has_recoverable_errors = observations.iter().any(|o| {
            matches!(
                o.kind,
                ObservationKind::Error {
                    recoverable: true,
                    ..
                }
            )
        });

        let total_duration = observations
            .iter()
            .fold(Duration::ZERO, |acc, o| acc + o.duration);

        let formatted_for_llm = self.format_for_context(&observations);

        ObservationSummary {
            observations,
            all_successful,
            has_recoverable_errors,
            total_duration,
            formatted_for_llm,
        }
    }

    /// Format observations for LLM context window
    ///
    /// # Arguments
    /// * `observations` - List of observations to format
    pub fn format_for_context(&self, observations: &[Observation]) -> String {
        let mut formatted = String::new();

        for (idx, obs) in observations.iter().enumerate() {
            if idx > 0 {
                formatted.push_str("\n\n");
            }

            formatted.push_str(&format!("Observation from {}:\n", obs.tool_name));

            // Status line
            let status_line = match &obs.kind {
                ObservationKind::Success => {
                    format!("[SUCCESS in {:.2}s]", obs.duration.as_secs_f32())
                }
                ObservationKind::Error { code, recoverable } => {
                    let code_str = code.as_deref().unwrap_or("UNKNOWN");
                    let recover_hint = if *recoverable { " (recoverable)" } else { "" };
                    format!(
                        "[ERROR: {}{}] (took {:.2}s)",
                        code_str,
                        recover_hint,
                        obs.duration.as_secs_f32()
                    )
                }
                ObservationKind::Timeout => {
                    format!("[TIMEOUT after {:.2}s]", obs.duration.as_secs_f32())
                }
                ObservationKind::PartialResult { completeness } => {
                    format!(
                        "[PARTIAL RESULT: {:.0}% complete in {:.2}s]",
                        completeness * 100.0,
                        obs.duration.as_secs_f32()
                    )
                }
            };

            formatted.push_str(&status_line);
            formatted.push('\n');

            // Content
            formatted.push_str(&obs.content);

            // Truncation notice
            if obs.metadata.truncated {
                formatted.push_str(&format!(
                    "\n\n[Note: Output truncated from {} to {} characters for brevity]",
                    obs.metadata.original_length,
                    obs.content.len()
                ));
            }
        }

        formatted
    }

    /// Truncate content according to the configured strategy
    ///
    /// # Arguments
    /// * `content` - Content to truncate
    fn truncate_content(&self, content: &str) -> String {
        let max_chars = self.max_observation_tokens * 4; // Approximate 4 chars per token

        if content.len() <= max_chars {
            return content.to_string();
        }

        match self.truncation_strategy {
            TruncationStrategy::HeadOnly => {
                let mut truncated = content.chars().take(max_chars).collect::<String>();
                truncated.push_str("\n...[truncated]");
                truncated
            }
            TruncationStrategy::TailOnly => {
                let skip_count = content.chars().count().saturating_sub(max_chars);
                let mut truncated = String::from("...[truncated]\n");
                truncated.push_str(&content.chars().skip(skip_count).collect::<String>());
                truncated
            }
            TruncationStrategy::HeadTail { head_ratio } => {
                let head_ratio = head_ratio.clamp(0.0, 1.0);
                let head_chars = (max_chars as f32 * head_ratio) as usize;
                let tail_chars = max_chars.saturating_sub(head_chars);

                let total_chars = content.chars().count();
                let skip_count = total_chars.saturating_sub(tail_chars);

                let mut truncated = content.chars().take(head_chars).collect::<String>();
                truncated.push_str("\n\n...[middle section truncated]...\n\n");
                truncated.push_str(&content.chars().skip(skip_count).collect::<String>());
                truncated
            }
        }
    }

    /// Estimate token count for text (simple heuristic)
    ///
    /// Uses a simple character-based estimation:
    /// - Average ~4 characters per token for English text
    /// - Adjusts for whitespace and punctuation
    ///
    /// # Arguments
    /// * `text` - Text to estimate tokens for
    fn estimate_tokens(&self, text: &str) -> usize {
        // Simple heuristic: ~4 characters per token on average
        // This is a rough estimate; real tokenization would be more accurate
        let char_count = text.chars().count();
        (char_count as f32 / 4.0).ceil() as usize
    }

    /// Classify a tool result into an observation kind
    fn classify_result(&self, result: &ToolCallResult) -> ObservationKind {
        if result.success {
            ObservationKind::Success
        } else {
            // Parse error to determine recoverability
            let error_msg = result.error.as_deref().unwrap_or("");
            let recoverable = self.is_recoverable_error(error_msg);
            let code = self.extract_error_code(error_msg);

            ObservationKind::Error { code, recoverable }
        }
    }

    /// Extract content from tool result for observation
    fn extract_content(&self, result: &ToolCallResult) -> String {
        if result.success {
            // Format successful result
            match &result.output {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => "No output".to_string(),
                other => match serde_json::to_string_pretty(other) {
                    Ok(json_str) => json_str,
                    Err(e) => {
                        // Log serialization failure and provide fallback with context
                        tracing::warn!(
                            "Failed to serialize tool output to JSON: {}. Using fallback.",
                            e
                        );
                        format!("[JSON serialization failed: {}]", e)
                    }
                },
            }
        } else {
            // Format error result
            format!(
                "Error: {}",
                result.error.as_deref().unwrap_or("Unknown error")
            )
        }
    }

    /// Determine if an error is potentially recoverable
    fn is_recoverable_error(&self, error_msg: &str) -> bool {
        let error_lower = error_msg.to_lowercase();

        // Recoverable error patterns
        let recoverable_patterns = [
            "not found",
            "does not exist",
            "no such file",
            "permission denied",
            "timeout",
            "connection refused",
            "network error",
            "rate limit",
        ];

        // Non-recoverable error patterns
        let non_recoverable_patterns = [
            "invalid syntax",
            "parse error",
            "malformed",
            "corrupted",
            "incompatible",
        ];

        // Check non-recoverable first
        if non_recoverable_patterns
            .iter()
            .any(|p| error_lower.contains(p))
        {
            return false;
        }

        // Check recoverable
        recoverable_patterns.iter().any(|p| error_lower.contains(p))
    }

    /// Extract error code from error message if present
    fn extract_error_code(&self, error_msg: &str) -> Option<String> {
        // Try to extract common error code patterns
        // e.g., "Error 404: Not Found" -> Some("404")
        // e.g., "ENOENT: File not found" -> Some("ENOENT")

        // Pattern 1: "Error CODE:"
        if let Some(idx) = error_msg.find("Error ") {
            let after_error = &error_msg[idx + 6..];
            if let Some(colon_idx) = after_error.find(':') {
                let code = after_error[..colon_idx].trim();
                if !code.is_empty() {
                    return Some(code.to_string());
                }
            }
        }

        // Pattern 2: "CODE:" at start
        if let Some(colon_idx) = error_msg.find(':') {
            let potential_code = error_msg[..colon_idx].trim();
            if potential_code.len() <= 20 && !potential_code.contains(' ') {
                return Some(potential_code.to_string());
            }
        }

        None
    }
}

impl Default for ObservationProcessor {
    fn default() -> Self {
        Self::new(500) // 500 tokens default (~2000 chars)
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
    use serde_json::json;

    fn create_test_call() -> ToolCall {
        ToolCall {
            id: Uuid::new_v4(),
            tool_name: "test_tool".to_string(),
            arguments: json!({"arg": "value"}),
            result: None,
            timestamp: chrono::Utc::now(),
            session_id: None,
        }
    }

    fn create_success_result(output: serde_json::Value) -> ToolCallResult {
        ToolCallResult {
            success: true,
            output,
            error: None,
            duration: Duration::from_millis(50),
        }
    }

    fn create_error_result(error: &str) -> ToolCallResult {
        ToolCallResult {
            success: false,
            output: json!(null),
            error: Some(error.to_string()),
            duration: Duration::from_millis(25),
        }
    }

    #[test]
    fn test_process_success() {
        let processor = ObservationProcessor::new(1000);
        let call = create_test_call();
        let result = create_success_result(json!("Output text"));

        let obs = processor.process(&result, &call);

        assert!(matches!(obs.kind, ObservationKind::Success));
        assert_eq!(obs.content, "Output text");
        assert!(!obs.metadata.truncated);
    }

    #[test]
    fn test_process_error_recoverable() {
        let processor = ObservationProcessor::new(1000);
        let call = create_test_call();
        let result = create_error_result("File not found");

        let obs = processor.process(&result, &call);

        match obs.kind {
            ObservationKind::Error { recoverable, .. } => assert!(recoverable),
            _ => panic!("Expected Error kind"),
        }
    }

    #[test]
    fn test_process_error_non_recoverable() {
        let processor = ObservationProcessor::new(1000);
        let call = create_test_call();
        let result = create_error_result("Invalid syntax");

        let obs = processor.process(&result, &call);

        match obs.kind {
            ObservationKind::Error { recoverable, .. } => assert!(!recoverable),
            _ => panic!("Expected Error kind"),
        }
    }

    #[test]
    fn test_truncation_head_only() {
        let processor = ObservationProcessor::with_truncation(10, TruncationStrategy::HeadOnly);
        let long_text = "A".repeat(200);

        let truncated = processor.truncate_content(&long_text);

        assert!(truncated.len() < long_text.len());
        assert!(truncated.starts_with("AAAA"));
        assert!(truncated.contains("[truncated]"));
    }

    #[test]
    fn test_truncation_tail_only() {
        let processor = ObservationProcessor::with_truncation(10, TruncationStrategy::TailOnly);
        let long_text = "A".repeat(200);

        let truncated = processor.truncate_content(&long_text);

        assert!(truncated.len() < long_text.len());
        assert!(truncated.ends_with("AAAA"));
        assert!(truncated.contains("[truncated]"));
    }

    #[test]
    fn test_truncation_head_tail() {
        let processor = ObservationProcessor::with_truncation(
            10,
            TruncationStrategy::HeadTail { head_ratio: 0.5 },
        );
        let long_text = "A".repeat(200);

        let truncated = processor.truncate_content(&long_text);

        assert!(truncated.len() < long_text.len());
        assert!(truncated.starts_with("AAAA"));
        assert!(truncated.ends_with("AAAA"));
        assert!(truncated.contains("[middle section truncated]"));
    }

    #[test]
    fn test_format_for_context() {
        let processor = ObservationProcessor::new(1000);
        let call = create_test_call();

        let obs1 = processor.process(&create_success_result(json!("First output")), &call);
        let obs2 = processor.process(&create_error_result("Not found"), &call);

        let formatted = processor.format_for_context(&[obs1, obs2]);

        assert!(formatted.contains("Observation from test_tool"));
        assert!(formatted.contains("[SUCCESS"));
        assert!(formatted.contains("[ERROR"));
        assert!(formatted.contains("First output"));
        assert!(formatted.contains("Not found"));
    }

    #[test]
    fn test_summarize() {
        let processor = ObservationProcessor::new(1000);
        let call = create_test_call();

        let obs1 = processor.process(&create_success_result(json!("Output 1")), &call);
        let obs2 = processor.process(&create_success_result(json!("Output 2")), &call);

        let summary = processor.summarize(vec![obs1, obs2]);

        assert!(summary.all_successful);
        assert!(!summary.has_recoverable_errors);
        assert_eq!(summary.observations.len(), 2);
        assert!(summary.formatted_for_llm.contains("Output 1"));
        assert!(summary.formatted_for_llm.contains("Output 2"));
    }

    #[test]
    fn test_summarize_with_errors() {
        let processor = ObservationProcessor::new(1000);
        let call = create_test_call();

        let obs1 = processor.process(&create_success_result(json!("Output 1")), &call);
        let obs2 = processor.process(&create_error_result("File not found"), &call);

        let summary = processor.summarize(vec![obs1, obs2]);

        assert!(!summary.all_successful);
        assert!(summary.has_recoverable_errors);
    }

    #[test]
    fn test_estimate_tokens() {
        let processor = ObservationProcessor::new(1000);

        // Estimation is ~4 chars per token (rounded up)
        assert_eq!(processor.estimate_tokens("test"), 1); // 4 chars -> 1 token
        assert_eq!(processor.estimate_tokens("test word"), 3); // 9 chars -> 3 tokens
        assert_eq!(processor.estimate_tokens(&"word ".repeat(100)), 125); // 500 chars -> 125 tokens
    }

    #[test]
    fn test_extract_error_code() {
        let processor = ObservationProcessor::new(1000);

        assert_eq!(
            processor.extract_error_code("Error 404: Not Found"),
            Some("404".to_string())
        );
        assert_eq!(
            processor.extract_error_code("ENOENT: File not found"),
            Some("ENOENT".to_string())
        );
        assert_eq!(processor.extract_error_code("Something went wrong"), None);
    }

    #[test]
    fn test_serde_roundtrip() {
        let obs = Observation {
            tool_name: "test".to_string(),
            tool_call_id: Uuid::new_v4(),
            kind: ObservationKind::Success,
            content: "test content".to_string(),
            structured_data: Some(json!({"key": "value"})),
            duration: Duration::from_millis(100),
            metadata: ObservationMetadata {
                tokens_estimated: Some(50),
                truncated: false,
                original_length: 12,
            },
        };

        let serialized = serde_json::to_string(&obs).expect("Failed to serialize");
        let deserialized: Observation =
            serde_json::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(obs.tool_name, deserialized.tool_name);
        assert_eq!(obs.content, deserialized.content);
        assert_eq!(obs.duration, deserialized.duration);
    }
}
