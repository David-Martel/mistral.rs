//! Context management for ReAct agent
//!
//! This module provides context gathering capabilities for the mistralrs-tui ReAct agent.
//! Context gatherers retrieve relevant information from various sources (conversation history,
//! documentation, tool results) and manage token budgets to fit within model context windows.
//!
//! # Architecture
//!
//! The context system uses a composite pattern with priority-based budget allocation:
//!
//! - `ContextGatherer` trait: Interface for context providers
//! - `CompositeContextGatherer`: Combines multiple gatherers with budget management
//! - `ContextChunk`: Individual piece of context with metadata
//! - `GatheredContext`: Aggregated result from multiple gatherers
//!
//! # Example
//!
//! ```no_run
//! use mistralrs_tui::agent::context::{CompositeContextGatherer, ContextPriority};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut gatherer = CompositeContextGatherer::new(2048);
//!
//! // Add gatherers in any order; they'll be sorted by priority
//! // gatherer.add_gatherer(Box::new(history_gatherer));
//! // gatherer.add_gatherer(Box::new(docs_gatherer));
//!
//! let context = gatherer.gather_all("What is Rust?", 0).await?;
//! println!("Gathered {} chunks using {} tokens",
//!          context.chunks.len(),
//!          context.total_tokens);
//! # Ok(())
//! # }
//! ```

pub mod gatherer;

pub use gatherer::{
    estimate_tokens, CompositeContextGatherer, ContextChunk, ContextGatherer, ContextPriority,
    GatheredContext,
};
