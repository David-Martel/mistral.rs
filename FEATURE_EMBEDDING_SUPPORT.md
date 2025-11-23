# Feature Branch: Embedding Support

**Branch**: `feature/embedding-support`
**Base**: `phase2-upstream-integration` (Phase 3A complete)
**Status**: Ready for implementation
**Priority**: Medium (Phase 3B)

## Objective

Implement embedding model support infrastructure to enable:

- EmbeddingGemma models
- Qwen 3 Embedding models
- Search functionality with embeddings
- Integration of 2 deferred commits from Phase 3A

## Commits to Integrate

### Priority 2 Features (New Infrastructure)

1. **a3d3d473f** - Support embedding models: EmbeddingGemma (#1684)

   - Implements EmbeddingGemma model support
   - Adds embedding-specific pipeline
   - Status: To be integrated

1. **230e9c7c6** - Implement Qwen 3 Embedding (#1686)

   - Implements Qwen 3 Embedding model
   - Extends embedding infrastructure
   - Status: To be integrated

### Deferred from Phase 3A (Requires Embedding Infrastructure)

3. **b0326ff7a** - Fix auto loader confusion (#1707)

   - Why deferred: References `EmbeddingLoader` and embedding-specific logic
   - Depends on: Commits 1-2 above
   - Status: Ready to integrate after embedding infrastructure

1. **2b7dd90d4** - Fix embedding inputs processor in search + flash attn case (#1705)

   - Why deferred: Requires embedding model pipeline infrastructure
   - Depends on: Commits 1-2 above
   - Status: Ready to integrate after embedding infrastructure

### Priority 3 Refactoring

5. **542aafde8** - Refactor search embedding to use EmbeddingGemma (#1698)
   - Refactors existing search to use new embedding infrastructure
   - Depends on: All commits above
   - Status: To be integrated last

## Implementation Plan

### Phase 1: Base Infrastructure (Commits 1-2)

**Estimated Effort**: 4-6 hours

1. Cherry-pick `a3d3d473f` (EmbeddingGemma)

   - Expected changes:
     - `mistralrs-core/src/models/` - New EmbeddingGemma model
     - `mistralrs-core/src/pipeline/` - Embedding pipeline support
     - `mistralrs-core/src/lib.rs` - New architecture enum variants
   - Test: Build and verify model loads

1. Cherry-pick `230e9c7c6` (Qwen 3 Embedding)

   - Expected changes:
     - `mistralrs-core/src/models/` - New Qwen3Embedding model
     - Extends embedding pipeline
   - Test: Build and verify model loads

1. Validate base infrastructure:

   - Download EmbeddingGemma test model
   - Load and test embeddings generation
   - Verify API compatibility

### Phase 2: Deferred Fixes (Commits 3-4)

**Estimated Effort**: 2-3 hours

4. Cherry-pick `b0326ff7a` (Auto loader fix)

   - Now has required `EmbeddingLoader` infrastructure
   - Test: Auto-loading correctly identifies embedding models

1. Cherry-pick `2b7dd90d4` (Inputs processor fix)

   - Now has embedding pipeline support
   - Test: Search with flash attention works

### Phase 3: Refactoring (Commit 5)

**Estimated Effort**: 1-2 hours

6. Cherry-pick `542aafde8` (Search refactor)
   - Cleanly integrates with new infrastructure
   - Test: Search functionality works with refactored code

### Phase 4: Testing & Validation

**Estimated Effort**: 2-3 hours

- Unit tests for embedding models
- Integration tests for search
- Performance benchmarks
- Documentation updates

## Required Infrastructure

### New Components to Implement

Based on the upstream commits, the following components are expected:

1. **EmbeddingLoader**

   - `EmbeddingLoaderBuilder`
   - `EmbeddingLoaderType` enum
   - `EmbeddingSpecificConfig`

1. **Embedding Pipeline**

   - `EmbeddingPipeline` trait implementation
   - Embedding-specific forward pass
   - Output handling for embeddings vs. text generation

1. **Model Architectures**

   - EmbeddingGemma model implementation
   - Qwen3Embedding model implementation
   - Shared embedding layer utilities

## Testing Strategy

### Unit Tests

- Model loading tests
- Embedding generation tests
- Pipeline tests

### Integration Tests

- End-to-end embedding generation
- Search functionality
- Flash attention with embeddings

### Models for Testing

Recommended test models:

- `google/gemma-embedding-base` (if available)
- `Qwen/Qwen3-Embedding` (if available)
- Any compatible embedding model for validation

## Success Criteria

- ✅ All 5 commits integrated successfully
- ✅ EmbeddingGemma models load and generate embeddings
- ✅ Qwen 3 Embedding models load and generate embeddings
- ✅ Auto loader correctly identifies embedding models
- ✅ Search functionality works with embeddings
- ✅ All tests passing
- ✅ No performance regressions
- ✅ Documentation updated

## Merge Strategy

Once complete:

1. Create pull request: `feature/embedding-support` → `phase2-upstream-integration`
1. Review changes
1. Run full test suite
1. Merge if all tests pass
1. Delete feature branch after merge

## Notes

- This branch was created as part of Phase 3A completion
- 2 commits were deferred from Phase 3A because they required this infrastructure
- Implementing this unblocks those deferred commits
- Priority 2 feature for Phase 3B integration

______________________________________________________________________

**Created**: 2025-11-23
**Last Updated**: 2025-11-23
**Status**: Branch created, ready for implementation
