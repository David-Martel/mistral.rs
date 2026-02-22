//! Context gathering for ReAct agent
//!
//! This module provides trait and implementations for gathering contextual information
//! to augment agent reasoning. Context gatherers retrieve relevant information from
//! various sources (conversation history, documentation, external APIs) and manage
//! token budgets to fit within model context windows.

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for context providers that gather relevant information for agent reasoning.
///
/// Implementers provide context from different sources (history, documentation, tools)
/// while respecting token budgets and priority constraints.
#[async_trait]
pub trait ContextGatherer: Send + Sync {
    /// Gather context relevant to the query and current iteration.
    ///
    /// # Arguments
    ///
    /// * `query` - The user's query or task description
    /// * `iteration` - Current ReAct iteration number (0-based)
    /// * `token_budget` - Maximum tokens this gatherer may use
    ///
    /// # Returns
    ///
    /// Vector of context chunks, sorted by priority (highest first).
    /// Total tokens across all chunks should not exceed `token_budget`.
    async fn gather(
        &self,
        query: &str,
        iteration: usize,
        token_budget: usize,
    ) -> Result<Vec<ContextChunk>>;

    /// Priority for this gatherer (higher values are gathered first).
    ///
    /// When multiple gatherers compete for a limited token budget,
    /// those with higher priority are invoked first.
    fn priority(&self) -> ContextPriority;

    /// Human-readable name for logging and debugging.
    fn name(&self) -> &str;
}

/// A chunk of context from a specific source with associated metadata.
#[derive(Debug, Clone)]
pub struct ContextChunk {
    /// The actual content to include in the prompt
    pub content: String,
    /// Source identifier (e.g., "conversation_history", "tool_docs")
    pub source: String,
    /// Priority of this chunk
    pub priority: ContextPriority,
    /// Estimated token count for this content
    pub token_count: usize,
    /// Additional metadata (timestamps, relevance scores, etc.)
    pub metadata: HashMap<String, String>,
}

impl ContextChunk {
    /// Create a new context chunk with automatic token estimation.
    ///
    /// # Arguments
    ///
    /// * `content` - The text content
    /// * `source` - Source identifier
    /// * `priority` - Priority level
    ///
    /// # Examples
    ///
    /// ```
    /// use mistralrs_tui::agent::context::{ContextChunk, ContextPriority};
    ///
    /// let chunk = ContextChunk::new(
    ///     "Previous conversation...".to_string(),
    ///     "history".to_string(),
    ///     ContextPriority::High,
    /// );
    /// ```
    pub fn new(content: String, source: String, priority: ContextPriority) -> Self {
        let token_count = estimate_tokens(&content);
        Self {
            content,
            source,
            priority,
            token_count,
            metadata: HashMap::new(),
        }
    }

    /// Create a chunk with custom metadata.
    pub fn with_metadata(
        content: String,
        source: String,
        priority: ContextPriority,
        metadata: HashMap<String, String>,
    ) -> Self {
        let token_count = estimate_tokens(&content);
        Self {
            content,
            source,
            priority,
            token_count,
            metadata,
        }
    }

    /// Add a metadata entry to this chunk.
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}

/// Priority levels for context chunks.
///
/// Higher numeric values indicate higher priority. When token budget is limited,
/// higher priority chunks are included first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContextPriority {
    /// Must be included (system prompts, critical instructions)
    Critical = 100,
    /// Important context (recent conversation, tool results)
    High = 75,
    /// Useful but not essential (general documentation)
    Medium = 50,
    /// Nice to have (examples, suggestions)
    Low = 25,
    /// Only include if budget allows (tangential information)
    Optional = 0,
}

impl ContextPriority {
    /// Get numeric value for priority.
    pub fn value(&self) -> i32 {
        *self as i32
    }
}

/// Aggregated context from multiple gatherers.
#[derive(Debug, Clone)]
pub struct GatheredContext {
    /// All context chunks, sorted by priority
    pub chunks: Vec<ContextChunk>,
    /// Total token count across all chunks
    pub total_tokens: usize,
    /// List of source identifiers included
    pub sources: Vec<String>,
}

impl GatheredContext {
    /// Create an empty context.
    pub fn empty() -> Self {
        Self {
            chunks: Vec::new(),
            total_tokens: 0,
            sources: Vec::new(),
        }
    }

    /// Create from a vector of chunks.
    pub fn from_chunks(mut chunks: Vec<ContextChunk>) -> Self {
        // Sort by priority (highest first)
        chunks.sort_by(|a, b| b.priority.cmp(&a.priority));

        let total_tokens = chunks.iter().map(|c| c.token_count).sum();
        let sources = chunks
            .iter()
            .map(|c| c.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Self {
            chunks,
            total_tokens,
            sources,
        }
    }

    /// Combine all chunks into a single formatted string.
    pub fn format(&self) -> String {
        self.chunks
            .iter()
            .map(|chunk| format!("# Context from {}\n{}\n", chunk.source, chunk.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get chunks from a specific source.
    pub fn from_source(&self, source: &str) -> Vec<&ContextChunk> {
        self.chunks.iter().filter(|c| c.source == source).collect()
    }
}

/// Combines multiple context gatherers with priority-based token budget allocation.
///
/// This composite gatherer invokes multiple gatherers in priority order,
/// allocating the token budget fairly while respecting priorities.
///
/// # Examples
///
/// ```no_run
/// use mistralrs_tui::agent::context::CompositeContextGatherer;
///
/// let mut gatherer = CompositeContextGatherer::new(2048);
/// // gatherer.add_gatherer(Box::new(history_gatherer));
/// // gatherer.add_gatherer(Box::new(docs_gatherer));
/// ```
pub struct CompositeContextGatherer {
    gatherers: Vec<Box<dyn ContextGatherer>>,
    token_budget: usize,
}

impl CompositeContextGatherer {
    /// Create a new composite gatherer with the specified token budget.
    ///
    /// # Arguments
    ///
    /// * `token_budget` - Maximum tokens for all gathered context combined
    pub fn new(token_budget: usize) -> Self {
        Self {
            gatherers: Vec::new(),
            token_budget,
        }
    }

    /// Add a context gatherer to this composite.
    ///
    /// Gatherers are invoked in priority order (highest first).
    pub fn add_gatherer(&mut self, gatherer: Box<dyn ContextGatherer>) {
        self.gatherers.push(gatherer);
        // Keep gatherers sorted by priority (highest first)
        self.gatherers
            .sort_by_key(|b| std::cmp::Reverse(b.priority()));
    }

    /// Gather context from all registered gatherers.
    ///
    /// # Algorithm
    ///
    /// 1. Sort gatherers by priority (highest first)
    /// 2. For each gatherer:
    ///    a. Calculate remaining budget
    ///    b. Call gather() with remaining budget
    ///    c. Add chunks to result
    ///    d. Subtract tokens from remaining budget
    ///    e. Stop if budget exhausted
    /// 3. Return aggregated context
    ///
    /// # Arguments
    ///
    /// * `query` - The user's query
    /// * `iteration` - Current iteration number
    ///
    /// # Returns
    ///
    /// Aggregated context from all gatherers, respecting the token budget.
    pub async fn gather_all(&self, query: &str, iteration: usize) -> Result<GatheredContext> {
        if self.gatherers.is_empty() {
            return Ok(GatheredContext::empty());
        }

        let mut all_chunks = Vec::new();
        let mut remaining_budget = self.token_budget;

        for gatherer in &self.gatherers {
            if remaining_budget == 0 {
                tracing::debug!(
                    "Token budget exhausted, skipping gatherer: {}",
                    gatherer.name()
                );
                break;
            }

            tracing::debug!(
                "Invoking gatherer '{}' with budget {} tokens",
                gatherer.name(),
                remaining_budget
            );

            match gatherer.gather(query, iteration, remaining_budget).await {
                Ok(chunks) => {
                    let tokens_used: usize = chunks.iter().map(|c| c.token_count).sum();

                    tracing::debug!(
                        "Gatherer '{}' returned {} chunks using {} tokens",
                        gatherer.name(),
                        chunks.len(),
                        tokens_used
                    );

                    all_chunks.extend(chunks);

                    // Prevent underflow
                    remaining_budget = remaining_budget.saturating_sub(tokens_used);
                }
                Err(e) => {
                    tracing::warn!("Gatherer '{}' failed: {}", gatherer.name(), e);
                    // Continue with other gatherers even if one fails
                }
            }
        }

        Ok(GatheredContext::from_chunks(all_chunks))
    }

    /// Get the total token budget.
    pub fn token_budget(&self) -> usize {
        self.token_budget
    }

    /// Set a new token budget.
    pub fn set_token_budget(&mut self, budget: usize) {
        self.token_budget = budget;
    }

    /// Get the number of registered gatherers.
    pub fn gatherer_count(&self) -> usize {
        self.gatherers.len()
    }
}

/// Estimate token count for text using a simple heuristic.
///
/// This uses the approximation that 1 token ≈ 4 characters, which is
/// reasonably accurate for English text with common LLM tokenizers.
///
/// # Arguments
///
/// * `text` - The text to estimate
///
/// # Returns
///
/// Estimated number of tokens
///
/// # Examples
///
/// ```
/// use mistralrs_tui::agent::context::estimate_tokens;
///
/// let text = "Hello, world!";
/// let tokens = estimate_tokens(text);
/// assert_eq!(tokens, 4); // 13 chars / 4 = 3.25 ≈ 4 tokens (rounded up)
/// ```
pub fn estimate_tokens(text: &str) -> usize {
    // Rough estimation: ~4 chars per token
    // This is a simple approximation; real tokenization varies by model
    text.len().div_ceil(4) // Round up
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockGatherer {
        name: String,
        priority: ContextPriority,
        chunks: Vec<ContextChunk>,
    }

    impl MockGatherer {
        fn new(name: &str, priority: ContextPriority) -> Self {
            Self {
                name: name.to_string(),
                priority,
                chunks: Vec::new(),
            }
        }

        fn with_chunks(mut self, chunks: Vec<ContextChunk>) -> Self {
            self.chunks = chunks;
            self
        }
    }

    #[async_trait]
    impl ContextGatherer for MockGatherer {
        async fn gather(
            &self,
            _query: &str,
            _iteration: usize,
            token_budget: usize,
        ) -> Result<Vec<ContextChunk>> {
            let mut result = Vec::new();
            let mut used = 0;

            for chunk in &self.chunks {
                if used + chunk.token_count <= token_budget {
                    result.push(chunk.clone());
                    used += chunk.token_count;
                } else {
                    break;
                }
            }

            Ok(result)
        }

        fn priority(&self) -> ContextPriority {
            self.priority
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn test_token_estimation() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("Hello, world!"), 4); // 13 chars = 4 tokens
    }

    #[test]
    fn test_context_chunk_creation() {
        let chunk = ContextChunk::new(
            "Test content".to_string(),
            "test".to_string(),
            ContextPriority::High,
        );

        assert_eq!(chunk.content, "Test content");
        assert_eq!(chunk.source, "test");
        assert_eq!(chunk.priority, ContextPriority::High);
        assert_eq!(chunk.token_count, 3); // "Test content" = 12 chars = 3 tokens
        assert!(chunk.metadata.is_empty());
    }

    #[test]
    fn test_context_chunk_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("timestamp".to_string(), "2025-11-25".to_string());

        let chunk = ContextChunk::with_metadata(
            "Content".to_string(),
            "source".to_string(),
            ContextPriority::Medium,
            metadata,
        );

        assert_eq!(chunk.metadata.len(), 1);
        assert_eq!(chunk.metadata.get("timestamp").unwrap(), "2025-11-25");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(ContextPriority::Critical > ContextPriority::High);
        assert!(ContextPriority::High > ContextPriority::Medium);
        assert!(ContextPriority::Medium > ContextPriority::Low);
        assert!(ContextPriority::Low > ContextPriority::Optional);
    }

    #[test]
    fn test_gathered_context_empty() {
        let ctx = GatheredContext::empty();
        assert_eq!(ctx.chunks.len(), 0);
        assert_eq!(ctx.total_tokens, 0);
        assert_eq!(ctx.sources.len(), 0);
    }

    #[test]
    fn test_gathered_context_from_chunks() {
        let chunks = vec![
            ContextChunk::new(
                "High priority".to_string(),
                "source1".to_string(),
                ContextPriority::High,
            ),
            ContextChunk::new(
                "Low priority".to_string(),
                "source2".to_string(),
                ContextPriority::Low,
            ),
            ContextChunk::new(
                "Critical priority".to_string(),
                "source1".to_string(),
                ContextPriority::Critical,
            ),
        ];

        let ctx = GatheredContext::from_chunks(chunks);

        // Should be sorted by priority
        assert_eq!(ctx.chunks.len(), 3);
        assert_eq!(ctx.chunks[0].priority, ContextPriority::Critical);
        assert_eq!(ctx.chunks[1].priority, ContextPriority::High);
        assert_eq!(ctx.chunks[2].priority, ContextPriority::Low);

        // Should have 2 unique sources
        assert_eq!(ctx.sources.len(), 2);
        assert!(ctx.sources.contains(&"source1".to_string()));
        assert!(ctx.sources.contains(&"source2".to_string()));
    }

    #[test]
    fn test_gathered_context_format() {
        let chunks = vec![
            ContextChunk::new(
                "Content 1".to_string(),
                "src1".to_string(),
                ContextPriority::High,
            ),
            ContextChunk::new(
                "Content 2".to_string(),
                "src2".to_string(),
                ContextPriority::Medium,
            ),
        ];

        let ctx = GatheredContext::from_chunks(chunks);
        let formatted = ctx.format();

        assert!(formatted.contains("# Context from src1"));
        assert!(formatted.contains("Content 1"));
        assert!(formatted.contains("# Context from src2"));
        assert!(formatted.contains("Content 2"));
    }

    #[tokio::test]
    async fn test_composite_gatherer_empty() {
        let gatherer = CompositeContextGatherer::new(100);
        let ctx = gatherer.gather_all("test query", 0).await.unwrap();

        assert_eq!(ctx.chunks.len(), 0);
        assert_eq!(ctx.total_tokens, 0);
    }

    #[tokio::test]
    async fn test_composite_gatherer_single() {
        let mut gatherer = CompositeContextGatherer::new(100);

        let mock =
            MockGatherer::new("mock", ContextPriority::High).with_chunks(vec![ContextChunk::new(
                "Test".to_string(),
                "test".to_string(),
                ContextPriority::High,
            )]);

        gatherer.add_gatherer(Box::new(mock));

        let ctx = gatherer.gather_all("test", 0).await.unwrap();
        assert_eq!(ctx.chunks.len(), 1);
        assert_eq!(ctx.chunks[0].content, "Test");
    }

    #[tokio::test]
    async fn test_composite_gatherer_priority_order() {
        let mut gatherer = CompositeContextGatherer::new(1000);

        // Add low priority gatherer first
        let low =
            MockGatherer::new("low", ContextPriority::Low).with_chunks(vec![ContextChunk::new(
                "Low priority content".to_string(),
                "low".to_string(),
                ContextPriority::Low,
            )]);

        // Add high priority gatherer second
        let high =
            MockGatherer::new("high", ContextPriority::High).with_chunks(vec![ContextChunk::new(
                "High priority content".to_string(),
                "high".to_string(),
                ContextPriority::High,
            )]);

        gatherer.add_gatherer(Box::new(low));
        gatherer.add_gatherer(Box::new(high));

        let ctx = gatherer.gather_all("test", 0).await.unwrap();

        // Both should be included, but high priority should be invoked first
        assert_eq!(ctx.chunks.len(), 2);
        // After sorting in GatheredContext::from_chunks, high priority is first
        assert_eq!(ctx.chunks[0].priority, ContextPriority::High);
        assert_eq!(ctx.chunks[1].priority, ContextPriority::Low);
    }

    #[tokio::test]
    async fn test_composite_gatherer_budget_enforcement() {
        let mut gatherer = CompositeContextGatherer::new(10); // Very small budget

        let mock = MockGatherer::new("mock", ContextPriority::High).with_chunks(vec![
            ContextChunk::new(
                "This is a very long content string that exceeds budget".to_string(),
                "test".to_string(),
                ContextPriority::High,
            ),
            ContextChunk::new(
                "This too".to_string(),
                "test".to_string(),
                ContextPriority::High,
            ),
        ]);

        gatherer.add_gatherer(Box::new(mock));

        let ctx = gatherer.gather_all("test", 0).await.unwrap();

        // Should respect budget - total tokens should not exceed 10
        assert!(ctx.total_tokens <= 10);
    }

    #[tokio::test]
    async fn test_composite_gatherer_budget_allocation() {
        let mut gatherer = CompositeContextGatherer::new(50);

        // First gatherer uses 20 tokens
        let first = MockGatherer::new("first", ContextPriority::Critical).with_chunks(vec![
            ContextChunk::new(
                "a".repeat(80), // 80 chars = 20 tokens
                "first".to_string(),
                ContextPriority::Critical,
            ),
        ]);

        // Second gatherer should get remaining 30 tokens
        let second = MockGatherer::new("second", ContextPriority::High).with_chunks(vec![
            ContextChunk::new(
                "b".repeat(120), // 120 chars = 30 tokens
                "second".to_string(),
                ContextPriority::High,
            ),
        ]);

        gatherer.add_gatherer(Box::new(first));
        gatherer.add_gatherer(Box::new(second));

        let ctx = gatherer.gather_all("test", 0).await.unwrap();

        assert_eq!(ctx.chunks.len(), 2);
        assert_eq!(ctx.total_tokens, 50); // Exactly at budget
    }

    #[test]
    fn test_composite_gatherer_methods() {
        let mut gatherer = CompositeContextGatherer::new(100);
        assert_eq!(gatherer.token_budget(), 100);
        assert_eq!(gatherer.gatherer_count(), 0);

        gatherer.set_token_budget(200);
        assert_eq!(gatherer.token_budget(), 200);

        let mock = MockGatherer::new("test", ContextPriority::High);
        gatherer.add_gatherer(Box::new(mock));
        assert_eq!(gatherer.gatherer_count(), 1);
    }
}
