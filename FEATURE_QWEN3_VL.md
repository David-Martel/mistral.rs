# Feature Branch: Qwen3 VL Support

**Branch**: `feature/qwen3-vl`
**Base**: `phase2-upstream-integration` (Phase 3A complete)
**Status**: Ready for implementation
**Priority**: High (Priority 2 feature)

## Objective

Implement Qwen 3 Vision-Language model support to enable:

- Qwen3 VL model loading and inference
- Vision pipeline for Qwen3 architecture
- Integration of 2 deferred commits from Phase 3A
- Fix Conv3dNoBias infrastructure issue

## Commits to Integrate

### Priority 2 Feature (New Model)

1. **530463af1** - Implement Qwen 3 VL! (#1657)
   - Large feature: Complete Qwen3 VL implementation
   - Vision model support
   - Status: To be integrated

### Deferred from Phase 3A (Requires Qwen3 VL Infrastructure)

2. **c3d69e0e4** - Correctly handle tied embeddings in qwen3_vl config (#1682)

   - Why deferred: Calls `Conv3dNoBias.weight()` method which doesn't exist
   - Depends on: Commit 1 above
   - **Known Issue**: Requires Conv3dNoBias.weight() implementation
   - Status: Ready to integrate after fixing Conv3dNoBias

1. **bde5f3e67** - Remove restriction on qwen vl batch size (#1673)

   - Why deferred: Targets Qwen3 VL model not yet implemented
   - Depends on: Commits 1-2 above
   - Status: Ready to integrate after Qwen3 VL implementation

## Known Issues

### Conv3dNoBias Missing Method

**Error Location**: `mistralrs-core/src/vision_models/qwen3_vl/vision.rs:123`

**Error Message**:

```rust
error[E0599]: no method named `weight` found for struct `Conv3dNoBias`
  --> mistralrs-core\src\vision_models\qwen3_vl\vision.rs:123:68
   |
123|  ...weight: layer.weight().clone(),
    |                   ^^^^^^ method not found in `Conv3dNoBias`
```

**Root Cause**: The `Conv3dNoBias` struct doesn't implement a `weight()` method.

**Possible Solutions**:

1. Implement `weight()` method for `Conv3dNoBias` struct
1. Use direct field access if weights are public
1. Refactor tied embeddings handling to not require `weight()` accessor

**Priority**: Must fix before integrating commit `c3d69e0e4`

## Implementation Plan

### Phase 1: Qwen3 VL Base Implementation

**Estimated Effort**: 6-8 hours

1. Cherry-pick `530463af1` (Qwen 3 VL implementation)

   - Expected changes:
     - `mistralrs-core/src/models/qwen3_vl/` - New Qwen3 VL model
     - `mistralrs-core/src/vision_models/qwen3_vl/` - Vision components
     - `mistralrs-core/src/pipeline/` - Vision pipeline updates
     - `mistralrs-core/src/lib.rs` - New architecture enum
   - Expected conflicts: None (new model)
   - Test: Build succeeds

1. Validate base implementation:

   - Download Qwen3 VL test model
   - Test model loading
   - Verify compilation

### Phase 2: Fix Conv3dNoBias Issue

**Estimated Effort**: 2-3 hours

3. Investigate Conv3dNoBias structure:

   - Locate `Conv3dNoBias` definition
   - Understand weight storage mechanism
   - Determine best fix approach

1. Implement fix (one of):

   - **Option A**: Add `weight()` method to `Conv3dNoBias`
   - **Option B**: Modify tied embeddings code to use direct access
   - **Option C**: Refactor weight handling in Qwen3 VL

1. Test fix:

   - Verify compilation
   - Test tied embeddings functionality
   - Validate model loading

### Phase 3: Tied Embeddings Fix

**Estimated Effort**: 1-2 hours

6. Cherry-pick `c3d69e0e4` (Tied embeddings fix)
   - Now has Conv3dNoBias.weight() method
   - Test: Tied embeddings work correctly

### Phase 4: Batch Size Optimization

**Estimated Effort**: 1 hour

7. Cherry-pick `bde5f3e67` (Remove batch size restriction)
   - Allows flexible batch sizes for Qwen VL
   - Test: Batch processing works

### Phase 5: Testing & Validation

**Estimated Effort**: 3-4 hours

- Unit tests for Qwen3 VL model
- Integration tests for vision pipeline
- Multi-batch testing
- Performance validation
- Documentation updates

## Required Components

### New Model Files

Expected new files from upstream:

- `mistralrs-core/src/models/qwen3_vl/`
- `mistralrs-core/src/vision_models/qwen3_vl/`
- Vision processor implementation
- Image preprocessing utilities

### Conv3dNoBias Modification

Location: Likely in `mistralrs-core/src/layers/` or `mistralrs-vision/src/`

Required change:

```rust
impl Conv3dNoBias {
    pub fn weight(&self) -> &Tensor {
        // Return reference to weight tensor
        &self.weight  // or appropriate field access
    }
}
```

## Testing Strategy

### Unit Tests

- Conv3dNoBias weight access
- Tied embeddings handling
- Batch size flexibility
- Vision preprocessing

### Integration Tests

- End-to-end Qwen3 VL inference
- Multi-modal input handling
- Vision pipeline integration

### Models for Testing

Recommended test models:

- `Qwen/Qwen3-VL` (official model)
- Any compatible Qwen3 VL variants

## Success Criteria

- ✅ All 3 commits integrated successfully
- ✅ Conv3dNoBias.weight() issue resolved
- ✅ Qwen3 VL models load successfully
- ✅ Vision pipeline functional
- ✅ Tied embeddings work correctly
- ✅ Flexible batch sizes supported
- ✅ All tests passing
- ✅ No performance regressions
- ✅ Documentation updated

## Merge Strategy

Once complete:

1. Create pull request: `feature/qwen3-vl` → `phase2-upstream-integration`
1. Review changes
1. Run full test suite
1. Merge if all tests pass
1. Delete feature branch after merge

## Risk Assessment

### High Risk

- **Conv3dNoBias fix**: Critical blocker, may require significant refactoring
- **Vision pipeline**: Complex integration, potential conflicts

### Medium Risk

- **Tied embeddings**: Depends on Conv3dNoBias fix
- **Batch size changes**: May affect existing vision models

### Low Risk

- **Base Qwen3 VL implementation**: New model, isolated changes

## Notes

- This branch was created as part of Phase 3A completion
- 2 commits were deferred from Phase 3A because they required Qwen3 VL infrastructure
- Conv3dNoBias.weight() issue must be resolved first
- Priority 2 feature for Phase 3B integration
- High value: Adds major new model capability

______________________________________________________________________

**Created**: 2025-11-23
**Last Updated**: 2025-11-23
**Status**: Branch created, ready for implementation
