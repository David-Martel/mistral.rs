//! Thought parsing module for ReAct agent
//!
//! This module provides robust parsing of LLM responses into structured thoughts,
//! supporting multiple reasoning patterns and formats.
//!
//! # Supported Patterns
//!
//! - **ReAct Classic**: Traditional "Thought: ...\nAction: ..." format
//! - **JSON Structured**: Structured JSON with thought/action fields
//! - **Tool Calls Inline**: Tool calls embedded directly in responses
//! - **Free Form**: Unstructured text analysis
//!
//! # Example
//!
//! ```rust
//! # #[cfg(feature = "tui-agent")]
//! # use mistralrs_tui::agent::react::thought::{ThoughtParser, Thought};
//! # #[cfg(feature = "tui-agent")]
//! # {
//! let parser = ThoughtParser::new();
//! let response = "Thought: I need to check the weather\nAction: get_weather({\"city\": \"London\"})";
//! let thought = parser.parse(response, &[]).expect("Failed to parse");
//! assert!(!thought.is_final_answer);
//! assert_eq!(thought.planned_actions.len(), 1);
//! # }
//! ```

#![cfg(feature = "tui-agent")]

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::OnceLock;

use super::super::llm_integration::LLMToolCall;

/// Parsed thought from LLM response
///
/// Represents the model's reasoning process, including its analysis,
/// confidence level, planned actions, and whether it represents a final answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// The reasoning or analysis text
    pub reasoning: String,

    /// Optional confidence score (0.0-1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,

    /// Actions the model plans to take
    pub planned_actions: Vec<PlannedAction>,

    /// Whether this is the final answer (terminates agent loop)
    pub is_final_answer: bool,

    /// Raw LLM response content for debugging
    pub raw_content: String,
}

impl Thought {
    /// Create a new thought with just reasoning text
    pub fn new(reasoning: impl Into<String>) -> Self {
        let reasoning = reasoning.into();
        Self {
            reasoning: reasoning.clone(),
            confidence: None,
            planned_actions: Vec::new(),
            is_final_answer: false,
            raw_content: reasoning,
        }
    }

    /// Create a final answer thought
    pub fn final_answer(answer: impl Into<String>) -> Self {
        let answer = answer.into();
        Self {
            reasoning: answer.clone(),
            confidence: Some(1.0),
            planned_actions: Vec::new(),
            is_final_answer: true,
            raw_content: answer,
        }
    }

    /// Add a planned action
    pub fn with_action(mut self, action: PlannedAction) -> Self {
        self.planned_actions.push(action);
        self
    }

    /// Set confidence level
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(confidence.clamp(0.0, 1.0));
        self
    }

    /// Mark as final answer
    pub fn as_final_answer(mut self) -> Self {
        self.is_final_answer = true;
        self
    }
}

/// A planned action extracted from thought
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    /// Tool/function name to call
    pub tool_name: String,

    /// Human-readable description of what this action does
    pub description: String,

    /// Parsed arguments (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<JsonValue>,
}

impl PlannedAction {
    /// Create a new planned action
    pub fn new(tool_name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            description: description.into(),
            arguments: None,
        }
    }

    /// Add arguments to the action
    pub fn with_arguments(mut self, arguments: JsonValue) -> Self {
        self.arguments = Some(arguments);
        self
    }
}

/// Extraction pattern for parsing LLM responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionPattern {
    /// Classic ReAct format: "Thought: ...\nAction: ..."
    ReActClassic,

    /// Structured JSON with thought/action fields
    JsonStructured,

    /// Tool calls embedded in response (OpenAI/Anthropic format)
    ToolCallsInline,

    /// Free-form text analysis
    FreeForm,
}

/// Fallback strategy when parsing fails
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Treat unparsed content as reasoning thought
    TreatAsThought,

    /// Treat unparsed content as final answer
    TreatAsFinalAnswer,

    /// Request clarification from LLM (not implemented, returns error)
    RequestClarification,
}

/// Errors that can occur during thought parsing
#[derive(Debug, thiserror::Error)]
pub enum ThoughtParseError {
    #[error("Failed to parse LLM response: {0}")]
    ParseError(String),

    #[error("Response was empty")]
    EmptyResponse,

    #[error("Invalid tool call format: {0}")]
    InvalidToolCall(String),

    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("No valid pattern matched and clarification requested")]
    ClarificationNeeded,
}

/// Parser for LLM responses into structured thoughts
///
/// The parser attempts multiple extraction patterns in sequence,
/// falling back to simpler strategies if structured parsing fails.
pub struct ThoughtParser {
    /// Ordered list of extraction patterns to try
    patterns: Vec<ExtractionPattern>,

    /// Strategy to use when all patterns fail
    fallback_strategy: FallbackStrategy,
}

impl Default for ThoughtParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ThoughtParser {
    /// Create a new parser with default patterns
    ///
    /// Default pattern order:
    /// 1. ToolCallsInline (highest priority - direct tool calls)
    /// 2. JsonStructured (structured JSON responses)
    /// 3. ReActClassic (traditional format)
    /// 4. FreeForm (fallback for unstructured text)
    pub fn new() -> Self {
        Self {
            patterns: vec![
                ExtractionPattern::ToolCallsInline,
                ExtractionPattern::JsonStructured,
                ExtractionPattern::ReActClassic,
                ExtractionPattern::FreeForm,
            ],
            fallback_strategy: FallbackStrategy::TreatAsThought,
        }
    }

    /// Create parser with custom patterns
    pub fn with_patterns(patterns: Vec<ExtractionPattern>) -> Self {
        Self {
            patterns,
            fallback_strategy: FallbackStrategy::TreatAsThought,
        }
    }

    /// Set fallback strategy
    pub fn with_fallback(mut self, strategy: FallbackStrategy) -> Self {
        self.fallback_strategy = strategy;
        self
    }

    /// Parse LLM response into a structured thought
    ///
    /// # Arguments
    ///
    /// * `llm_response` - Raw text response from the LLM
    /// * `tool_calls` - Structured tool calls extracted by LLM framework
    ///
    /// # Returns
    ///
    /// Parsed `Thought` or error if parsing fails completely
    pub fn parse(
        &self,
        llm_response: &str,
        tool_calls: &[LLMToolCall],
    ) -> Result<Thought, ThoughtParseError> {
        let trimmed = llm_response.trim();

        if trimmed.is_empty() {
            return Err(ThoughtParseError::EmptyResponse);
        }

        // Try each pattern in order
        for pattern in &self.patterns {
            if let Ok(thought) = self.try_pattern(trimmed, tool_calls, *pattern) {
                return Ok(thought);
            }
        }

        // All patterns failed, use fallback
        self.apply_fallback(trimmed)
    }

    /// Try a specific extraction pattern
    fn try_pattern(
        &self,
        text: &str,
        tool_calls: &[LLMToolCall],
        pattern: ExtractionPattern,
    ) -> Result<Thought, ThoughtParseError> {
        match pattern {
            ExtractionPattern::ToolCallsInline => self.parse_tool_calls_inline(text, tool_calls),
            ExtractionPattern::JsonStructured => self.parse_json_structured(text),
            ExtractionPattern::ReActClassic => self.parse_react_classic(text),
            ExtractionPattern::FreeForm => self.parse_free_form(text),
        }
    }

    /// Parse tool calls from LLM framework (OpenAI/Anthropic format)
    fn parse_tool_calls_inline(
        &self,
        text: &str,
        tool_calls: &[LLMToolCall],
    ) -> Result<Thought, ThoughtParseError> {
        if tool_calls.is_empty() {
            return Err(ThoughtParseError::ParseError(
                "No tool calls provided".to_string(),
            ));
        }

        // Extract reasoning (everything before tool calls, or the text itself)
        let reasoning = self.extract_reasoning(text);
        let is_final = self.detect_final_answer(text, !tool_calls.is_empty());

        let planned_actions: Vec<PlannedAction> = tool_calls
            .iter()
            .map(|call| {
                PlannedAction::new(&call.name, format!("Call {} with arguments", call.name))
                    .with_arguments(call.arguments.clone())
            })
            .collect();

        Ok(Thought {
            reasoning,
            confidence: None,
            planned_actions,
            is_final_answer: is_final && tool_calls.is_empty(),
            raw_content: text.to_string(),
        })
    }

    /// Parse structured JSON response
    ///
    /// Expects format like:
    /// ```json
    /// {
    ///   "thought": "I need to check the weather",
    ///   "action": "get_weather",
    ///   "arguments": {"city": "London"},
    ///   "final_answer": false
    /// }
    /// ```
    fn parse_json_structured(&self, text: &str) -> Result<Thought, ThoughtParseError> {
        // Try to find JSON in the text (might be surrounded by markdown code blocks)
        let json_text = self.extract_json(text).unwrap_or(text);

        let value: JsonValue = serde_json::from_str(json_text)?;

        let obj = match value.as_object() {
            Some(obj) => obj,
            None => {
                return Err(ThoughtParseError::ParseError(
                    "JSON is not an object".to_string(),
                ));
            }
        };

        // Extract reasoning
        let reasoning = obj
            .get("thought")
            .or_else(|| obj.get("reasoning"))
            .or_else(|| obj.get("analysis"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if reasoning.is_empty() {
            return Err(ThoughtParseError::ParseError(
                "No thought/reasoning field found".to_string(),
            ));
        }

        // Extract action
        let mut planned_actions = Vec::new();
        if let Some(action) = obj.get("action").and_then(|v| v.as_str()) {
            let arguments = obj.get("arguments").cloned();
            let description = obj
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Execute action");

            let mut planned_action = PlannedAction::new(action, description);
            if let Some(args) = arguments {
                planned_action = planned_action.with_arguments(args);
            }
            planned_actions.push(planned_action);
        }

        // Extract final answer flag
        let is_final_answer = obj
            .get("final_answer")
            .or_else(|| obj.get("is_final"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Extract confidence
        let confidence = obj
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|c| c as f32);

        Ok(Thought {
            reasoning,
            confidence,
            planned_actions,
            is_final_answer,
            raw_content: text.to_string(),
        })
    }

    /// Parse classic ReAct format
    ///
    /// Expects patterns like:
    /// - "Thought: I need to do X"
    /// - "Action: tool_name"
    /// - "Action Input: {args}"
    /// - "Final Answer: The result is Y"
    fn parse_react_classic(&self, text: &str) -> Result<Thought, ThoughtParseError> {
        // Check for final answer first
        if let Some(answer) = self.extract_final_answer(text) {
            return Ok(Thought::final_answer(answer));
        }

        // Extract thought/reasoning
        let reasoning = if let Some(thought) = self.extract_thought_section(text) {
            thought
        } else {
            // No explicit "Thought:" marker, use full text as reasoning
            text.to_string()
        };

        if reasoning.trim().is_empty() {
            return Err(ThoughtParseError::ParseError(
                "No reasoning found".to_string(),
            ));
        }

        // Extract actions
        let planned_actions = self.extract_planned_actions(text);

        Ok(Thought {
            reasoning,
            confidence: None,
            planned_actions,
            is_final_answer: false,
            raw_content: text.to_string(),
        })
    }

    /// Parse free-form text
    fn parse_free_form(&self, text: &str) -> Result<Thought, ThoughtParseError> {
        let is_final = self.detect_final_answer(text, false);

        Ok(Thought {
            reasoning: text.to_string(),
            confidence: None,
            planned_actions: Vec::new(),
            is_final_answer: is_final,
            raw_content: text.to_string(),
        })
    }

    /// Extract reasoning text from response
    pub fn extract_reasoning(&self, text: &str) -> String {
        // Try to extract explicit thought section
        if let Some(thought) = self.extract_thought_section(text) {
            return thought;
        }

        // Otherwise, use text up to first action marker or full text
        static ACTION_MARKER: OnceLock<Regex> = OnceLock::new();
        let re = ACTION_MARKER
            .get_or_init(|| Regex::new(r"(?i)(^|\n)(action|final answer|answer):\s*").unwrap());

        if let Some(mat) = re.find(text) {
            text[..mat.start()].trim().to_string()
        } else {
            text.trim().to_string()
        }
    }

    /// Extract planned actions from text
    pub fn extract_planned_actions(&self, text: &str) -> Vec<PlannedAction> {
        let mut actions = Vec::new();

        // Pattern: "Action: tool_name"
        static ACTION_PATTERN: OnceLock<Regex> = OnceLock::new();
        let action_re =
            ACTION_PATTERN.get_or_init(|| Regex::new(r"(?i)(?:^|\n)action:\s*([^\n]+)").unwrap());

        // Pattern: "Action Input: {json}" or "Arguments: {json}"
        static INPUT_PATTERN: OnceLock<Regex> = OnceLock::new();
        let input_re = INPUT_PATTERN.get_or_init(|| {
            Regex::new(r"(?i)(?:^|\n)(?:action input|arguments):\s*(\{[^}]*\})").unwrap()
        });

        for cap in action_re.captures_iter(text) {
            let action_text = cap[1].trim();

            // Try to parse arguments if they're inline like "tool_name({args})"
            if let Some(paren_idx) = action_text.find('(') {
                let tool_name = action_text[..paren_idx].trim();
                let args_text = &action_text[paren_idx + 1..]; // Skip opening paren

                // Try to extract JSON from parentheses (remove trailing paren if present)
                let json_text = if let Some(stripped) = args_text.strip_suffix(')') {
                    stripped
                } else {
                    args_text
                };

                if let Some(args) = self.try_parse_json(json_text) {
                    actions.push(
                        PlannedAction::new(tool_name, format!("Execute {}", tool_name))
                            .with_arguments(args),
                    );
                    continue;
                }
            }

            // Simple action without args
            actions.push(PlannedAction::new(
                action_text,
                format!("Execute {}", action_text),
            ));
        }

        // Try to match separate "Action Input:" sections
        for cap in input_re.captures_iter(text) {
            if let Some(args) = self.try_parse_json(&cap[1]) {
                // If we have an action without args, add args to it
                if let Some(last_action) = actions.last_mut() {
                    if last_action.arguments.is_none() {
                        last_action.arguments = Some(args);
                    }
                }
            }
        }

        actions
    }

    /// Detect if response contains final answer
    pub fn detect_final_answer(&self, text: &str, has_tool_calls: bool) -> bool {
        // If there are tool calls, it's not a final answer
        if has_tool_calls {
            return false;
        }

        self.extract_final_answer(text).is_some()
    }

    /// Extract thought section from text
    fn extract_thought_section(&self, text: &str) -> Option<String> {
        static THOUGHT_PATTERN: OnceLock<Regex> = OnceLock::new();
        let re = THOUGHT_PATTERN.get_or_init(|| {
            Regex::new(r"(?i)(?:^|\n)(?:thought|reasoning|analysis):\s*(.+?)(?:\n(?:action|final answer):|\z)").unwrap()
        });

        re.captures(text).map(|cap| cap[1].trim().to_string())
    }

    /// Extract final answer from text
    fn extract_final_answer(&self, text: &str) -> Option<String> {
        static FINAL_ANSWER_PATTERN: OnceLock<Regex> = OnceLock::new();
        let re = FINAL_ANSWER_PATTERN.get_or_init(|| {
            Regex::new(r"(?i)(?:^|\n)(?:final answer|answer|FINAL ANSWER):\s*(.+)").unwrap()
        });

        re.captures(text).map(|cap| cap[1].trim().to_string())
    }

    /// Extract JSON from text (handles markdown code blocks)
    fn extract_json<'a>(&self, text: &'a str) -> Option<&'a str> {
        // Try to find JSON in markdown code block
        static JSON_BLOCK: OnceLock<Regex> = OnceLock::new();
        let re =
            JSON_BLOCK.get_or_init(|| Regex::new(r"```(?:json)?\s*(\{[^`]*\})\s*```").unwrap());

        if let Some(cap) = re.captures(text) {
            // Get the matched string slice from the original text
            let mat = cap.get(1)?;
            return Some(&text[mat.start()..mat.end()]);
        }

        // Try to find bare JSON object
        if text.trim_start().starts_with('{') {
            return Some(text.trim());
        }

        None
    }

    /// Try to parse JSON, returning None if it fails
    fn try_parse_json(&self, text: &str) -> Option<JsonValue> {
        serde_json::from_str(text).ok()
    }

    /// Apply fallback strategy when parsing fails
    fn apply_fallback(&self, text: &str) -> Result<Thought, ThoughtParseError> {
        match self.fallback_strategy {
            FallbackStrategy::TreatAsThought => Ok(Thought::new(text)),
            FallbackStrategy::TreatAsFinalAnswer => Ok(Thought::final_answer(text)),
            FallbackStrategy::RequestClarification => Err(ThoughtParseError::ClarificationNeeded),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_react_classic() {
        let parser = ThoughtParser::new();
        let response = "Thought: I need to check the weather in London\nAction: get_weather\nAction Input: {\"city\": \"London\"}";

        let thought = parser.parse(response, &[]).unwrap();

        assert!(!thought.is_final_answer);
        assert!(thought.reasoning.contains("check the weather"));
        assert_eq!(thought.planned_actions.len(), 1);
        assert_eq!(thought.planned_actions[0].tool_name, "get_weather");
    }

    #[test]
    fn test_parse_final_answer() {
        let parser = ThoughtParser::new();
        let response = "Thought: I have all the information\nFinal Answer: The weather in London is sunny, 22°C";

        let thought = parser.parse(response, &[]).unwrap();

        assert!(thought.is_final_answer);
        assert!(thought.reasoning.contains("sunny"));
        assert_eq!(thought.planned_actions.len(), 0);
    }

    #[test]
    fn test_parse_json_structured() {
        let parser = ThoughtParser::new();
        let response = r#"{
            "thought": "I need to get the weather data",
            "action": "get_weather",
            "arguments": {"city": "London", "units": "metric"},
            "confidence": 0.95,
            "final_answer": false
        }"#;

        let thought = parser.parse(response, &[]).unwrap();

        assert!(!thought.is_final_answer);
        assert_eq!(thought.confidence, Some(0.95));
        assert_eq!(thought.planned_actions.len(), 1);
        assert_eq!(thought.planned_actions[0].tool_name, "get_weather");
        assert!(thought.planned_actions[0].arguments.is_some());
    }

    #[test]
    fn test_parse_json_in_markdown() {
        let parser = ThoughtParser::new();
        let response = r#"Here's my analysis:
```json
{
    "thought": "Need to analyze the data",
    "action": "analyze",
    "final_answer": false
}
```
"#;

        let thought = parser.parse(response, &[]).unwrap();

        assert!(!thought.is_final_answer);
        assert!(thought.reasoning.contains("analyze"));
        assert_eq!(thought.planned_actions.len(), 1);
    }

    #[test]
    fn test_parse_tool_calls_inline() {
        let parser = ThoughtParser::new();
        let response = "I'll check the weather now";
        let tool_calls = vec![LLMToolCall {
            name: "get_weather".to_string(),
            arguments: serde_json::json!({"city": "London"}),
            id: Some("call_123".to_string()),
        }];

        let thought = parser.parse(response, &tool_calls).unwrap();

        assert!(!thought.is_final_answer);
        assert_eq!(thought.planned_actions.len(), 1);
        assert_eq!(thought.planned_actions[0].tool_name, "get_weather");
        assert!(thought.planned_actions[0].arguments.is_some());
    }

    #[test]
    fn test_parse_free_form() {
        let parser = ThoughtParser::new();
        let response = "The user wants to know about the weather. I should help them.";

        let thought = parser.parse(response, &[]).unwrap();

        assert!(!thought.is_final_answer);
        assert_eq!(thought.reasoning, response);
        assert_eq!(thought.planned_actions.len(), 0);
    }

    #[test]
    fn test_empty_response() {
        let parser = ThoughtParser::new();
        let result = parser.parse("", &[]);

        assert!(matches!(result, Err(ThoughtParseError::EmptyResponse)));
    }

    #[test]
    fn test_extract_reasoning() {
        let parser = ThoughtParser::new();

        let text = "Thought: I need to check the weather\nAction: get_weather";
        let reasoning = parser.extract_reasoning(text);
        assert!(reasoning.contains("check the weather"));

        let text2 = "Some reasoning here\nAction: do_something";
        let reasoning2 = parser.extract_reasoning(text2);
        assert_eq!(reasoning2, "Some reasoning here");
    }

    #[test]
    fn test_extract_planned_actions() {
        let parser = ThoughtParser::new();

        let text = "Action: get_weather\nAction Input: {\"city\": \"London\"}";
        let actions = parser.extract_planned_actions(text);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].tool_name, "get_weather");
        assert!(actions[0].arguments.is_some());
    }

    #[test]
    fn test_detect_final_answer() {
        let parser = ThoughtParser::new();

        assert!(parser.detect_final_answer("Final Answer: 42", false));
        assert!(parser.detect_final_answer("FINAL ANSWER: The result", false));
        assert!(!parser.detect_final_answer("Thought: Still thinking", false));
        assert!(!parser.detect_final_answer("Final Answer: 42", true)); // has tool calls
    }

    #[test]
    fn test_fallback_strategy() {
        let parser =
            ThoughtParser::with_patterns(vec![]).with_fallback(FallbackStrategy::TreatAsThought);

        let thought = parser.parse("Some random text", &[]).unwrap();
        assert!(!thought.is_final_answer);
        assert_eq!(thought.reasoning, "Some random text");
    }

    #[test]
    fn test_fallback_as_final_answer() {
        let parser = ThoughtParser::with_patterns(vec![])
            .with_fallback(FallbackStrategy::TreatAsFinalAnswer);

        let thought = parser.parse("Some answer", &[]).unwrap();
        assert!(thought.is_final_answer);
    }

    #[test]
    fn test_fallback_clarification_needed() {
        let parser = ThoughtParser::with_patterns(vec![])
            .with_fallback(FallbackStrategy::RequestClarification);

        let result = parser.parse("Some text", &[]);
        assert!(matches!(
            result,
            Err(ThoughtParseError::ClarificationNeeded)
        ));
    }

    #[test]
    fn test_action_with_inline_args() {
        let parser = ThoughtParser::new();
        let text = "Action: calculate({\"expression\": \"2+2\"})";

        let actions = parser.extract_planned_actions(text);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].tool_name, "calculate");
        assert!(actions[0].arguments.is_some());
    }

    #[test]
    fn test_multiple_actions() {
        let parser = ThoughtParser::new();
        let text = "Action: first_tool\nAction: second_tool";

        let actions = parser.extract_planned_actions(text);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].tool_name, "first_tool");
        assert_eq!(actions[1].tool_name, "second_tool");
    }

    #[test]
    fn test_confidence_extraction() {
        let parser = ThoughtParser::new();
        let response = r#"{
            "thought": "Analysis complete",
            "confidence": 0.75,
            "final_answer": true
        }"#;

        let thought = parser.parse(response, &[]).unwrap();
        assert_eq!(thought.confidence, Some(0.75));
    }

    #[test]
    fn test_thought_builder() {
        let thought = Thought::new("Test reasoning")
            .with_confidence(0.8)
            .with_action(PlannedAction::new("test", "Test action"))
            .as_final_answer();

        assert!(thought.is_final_answer);
        assert_eq!(thought.confidence, Some(0.8));
        assert_eq!(thought.planned_actions.len(), 1);
    }

    #[test]
    fn test_case_insensitive_patterns() {
        let parser = ThoughtParser::new();

        // Test uppercase
        let response1 = "THOUGHT: Testing\nACTION: test";
        let thought1 = parser.parse(response1, &[]).unwrap();
        assert!(thought1.reasoning.contains("Testing"));

        // Test mixed case
        let response2 = "ThOuGhT: Testing\nAcTiOn: test";
        let thought2 = parser.parse(response2, &[]).unwrap();
        assert!(thought2.reasoning.contains("Testing"));
    }

    #[test]
    fn test_multiline_reasoning() {
        let parser = ThoughtParser::new();
        let response = "Thought: This is a complex problem.\nI need to think carefully.\nLet me break it down.\nAction: analyze";

        let thought = parser.parse(response, &[]).unwrap();
        assert!(thought.reasoning.contains("complex problem"));
        assert!(thought.reasoning.contains("break it down"));
    }
}
