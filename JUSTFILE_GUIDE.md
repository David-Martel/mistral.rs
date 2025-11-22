# Justfile Build System - Complete Usage Guide

## Overview

The mistral.rs justfile provides **isolated build variants** with dedicated target directories. This allows you to maintain multiple build configurations simultaneously without triggering full rebuilds when switching between variants.

## Key Benefit: Isolated Target Directories

**Problem with standard Cargo builds**: When you switch features (e.g., from CUDA to CPU-only), Cargo triggers a full rebuild.

**Solution with justfile**: Each variant uses its own target directory:

```
target-cpu/          # CPU-only builds
target-cuda/         # CUDA 12.9 with all features
target-cuda-13/      # CUDA 13.0 with all features
target-dev/          # Debug builds (faster compilation)
target-release/      # LTO optimized builds
target-metal/        # macOS Metal builds
```

**Result**: Switch between variants instantly without recompilation!

---

## Quick Start

### 1. List All Available Commands

```bash
just --list
```

### 2. Build CPU-Only Variant (Fastest, No GPU)

**Use case**: Quick testing, Phase 2A cherry-pick validation, CI/CD

```bash
just build-cpu
```

**Output location**: `T:\projects\rust-mistral\mistral.rs\target-cpu\release\mistralrs-server.exe`

**Features enabled**: `mkl` (Intel Math Kernel Library for CPU acceleration)

**Build time**: ~25-35 minutes (first build), ~2-5 minutes (incremental)

### 3. Build CUDA 12.9 Variant (Production)

**Use case**: Full GPU acceleration with CUDA 12.9

```bash
just build-cuda
```

**Output location**: `T:\projects\rust-mistral\mistral.rs\target-cuda\release\mistralrs-server.exe`

**Features enabled**: `cuda,flash-attn,cudnn,mkl`

**Build time**: ~35-50 minutes (first build), ~3-7 minutes (incremental)

**Environment variables set**:
- `NVCC_CCBIN` = MSVC compiler path (cl.exe)
- `CUDA_PATH` = CUDA 12.9 toolkit path
- `CUDNN_PATH` = cuDNN 9.8 library path

### 4. Build CUDA 13.0 Variant (Testing New CUDA)

**Use case**: Testing CUDA 13.0 compatibility

```bash
just build-cuda-13
```

**Output location**: `T:\projects\rust-mistral\mistral.rs\target-cuda-13\release\mistralrs-server.exe`

**Features enabled**: `cuda,flash-attn,cudnn,mkl` (same as CUDA 12.9, but with CUDA 13.0 toolkit)

---

## Fast Compilation Checks (No Full Build)

Before committing time to a full build, verify the code compiles:

### Check CPU-Only Variant (30-60 seconds)

```bash
just check-cpu
```

### Check CUDA 12.9 Variant (45-90 seconds)

```bash
just check-cuda
```

### Check All Variants (2-4 minutes)

```bash
just check-all
```

**What this does**: Runs `cargo check` for each variant (type-checking only, no codegen)

---

## Running Built Binaries

### Run CPU-Only Variant (TUI Mode)

```bash
just run-cpu
```

**Default model**: `T:/models/qwen2.5-1.5b-instruct-q4_k_m`

**Custom model**:
```bash
just run-cpu model_path="T:/models/your-model-path"
```

### Run CUDA 12.9 Variant (TUI Mode)

```bash
just run-cuda-tui
```

### Run CUDA 12.9 Variant (HTTP Server)

```bash
just run-cuda-server
```

**Default port**: 1234

**Custom port**:
```bash
just run-cuda-server port=8080
```

**OpenAI-compatible API**: `http://localhost:1234/v1/chat/completions`

---

## Testing Variants

### Test CPU-Only Variant

```bash
just test-cpu
```

**What this does**:
1. Builds CPU-only variant if not already built
2. Runs `cargo test --workspace --release --no-default-features --features mkl`

### Test CUDA 12.9 Variant

```bash
just test-cuda
```

**What this does**:
1. Builds CUDA 12.9 variant if not already built
2. Runs `cargo test --workspace --release --features cuda,flash-attn,cudnn,mkl`

---

## Cleaning Variants

### Clean CPU-Only Variant

```bash
just clean-cpu
```

**Removes**: `target-cpu/` directory (~8-12 GB)

### Clean CUDA 12.9 Variant

```bash
just clean-cuda
```

**Removes**: `target-cuda/` directory (~10-15 GB)

### Clean All Variants

```bash
just clean-all
```

**Removes**: All `target-*` directories (~40-60 GB total if all built)

---

## Development Workflow Variants

### Debug Build (Fast Compilation, Debug Symbols)

**Use case**: Development, debugging with gdb/lldb

```bash
just build-dev
```

**Output location**: `T:\projects\rust-mistral\mistral.rs\target-dev\debug\mistralrs-server.exe`

**Differences from release**:
- No optimizations (faster compilation)
- Debug symbols included
- Assertions enabled
- Faster to build (~15-20 minutes)

**Binary size**: ~450-550 MB (vs. ~350-400 MB for release)

### Release-Optimized Build (LTO, Maximum Performance)

**Use case**: Benchmarking, production deployment

```bash
just build-release
```

**Output location**: `T:\projects\rust-mistral\mistral.rs\target-release\release\mistralrs-server.exe`

**Differences from standard release**:
- Link-Time Optimization (LTO) enabled
- Profile-Guided Optimization (PGO) potential
- Smaller binary size (~280-320 MB)
- 5-15% performance improvement
- Much longer build time (60-90 minutes)

---

## Batch Operations

### Build All Variants (Overnight Build)

**Use case**: Comprehensive testing before release

```bash
just build-all
```

**What this does**:
1. `just build-cpu`
2. `just build-cuda`
3. `just build-cuda-13`
4. `just build-dev`
5. `just sizes` (show binary sizes)

**Total time**: 2-4 hours depending on system

**Disk space required**: ~50-70 GB

### Quick Check All Variants (Fast Validation)

**Use case**: Pre-commit validation, CI/CD

```bash
just check-all
```

**What this does**:
1. `just check-cpu`
2. `just check-cuda`
3. `just check-cuda-13`

**Total time**: 2-4 minutes

**Disk space required**: ~500 MB (check artifacts)

---

## Utility Commands

### Show Platform Information

```bash
just info
```

**Output**:
```
Platform Detection:
  OS: Windows (PowerShell)
  CUDA Features: cuda,flash-attn,cudnn,mkl
  CPU Features: mkl
  MSVC Path: C:\Program Files\...\cl.exe
  CUDA 12.9 Path: C:\Program Files\...\CUDA\v12.9
  CUDA 13.0 Path: C:\Program Files\...\CUDA\v13.0
  cuDNN Path: C:\Program Files\...\CUDNN\v9.8
```

### Show Binary Sizes

```bash
just sizes
```

**Output**:
```
Variant Binary Sizes:
  CPU-only:         320.45 MB
  CUDA 12.9:        387.23 MB
  CUDA 13.0:        389.12 MB
  Debug:            512.87 MB
  Release-opt:      295.34 MB
```

### Show Disk Usage

```bash
just disk
```

**Output**:
```
Variant Disk Usage:
  target-cpu: 9.87 GB
  target-cuda: 12.45 GB
  target-cuda-13: 12.52 GB
  target-dev: 15.23 GB
  target-release: 11.89 GB
```

---

## Phase 2A Cherry-Pick Validation Workflow

**Context**: Validating upstream commits work without CUDA (avoids NVCC failures)

### Quick Validation (Build Only)

```bash
just validate-phase2a
```

**What this does**:
1. `just clean-cpu`
2. `just build-cpu`
3. Displays success message

**Purpose**: Verify cherry-picked commits compile on CPU-only build

### Full Validation (Build + Test)

```bash
just validate-phase2a-full
```

**What this does**:
1. `just validate-phase2a` (clean + build)
2. `just test-cpu` (run test suite)

**Purpose**: Comprehensive validation before merging

---

## Binary Output Locations Reference

| Variant | Binary Location | Features |
|---------|----------------|----------|
| CPU-only | `target-cpu\release\mistralrs-server.exe` | `mkl` |
| CUDA 12.9 | `target-cuda\release\mistralrs-server.exe` | `cuda,flash-attn,cudnn,mkl` |
| CUDA 13.0 | `target-cuda-13\release\mistralrs-server.exe` | `cuda,flash-attn,cudnn,mkl` |
| Debug | `target-dev\debug\mistralrs-server.exe` | `cuda,flash-attn,cudnn,mkl` (debug) |
| Release-opt | `target-release\release\mistralrs-server.exe` | `cuda,flash-attn,cudnn,mkl` (LTO) |

---

## Integration with Makefile

The justfile **complements** the Makefile, not replaces it:

### Use Makefile for:

- **Code coverage**: `make test-coverage`
- **Git workflow**: `make git-auto-commit`
- **CI/CD pipeline**: `make ci`
- **Formatting**: `make fmt`
- **Linting**: `make lint-fix`

### Use Justfile for:

- **Isolated builds**: `just build-cpu`, `just build-cuda`
- **Quick checks**: `just check-all`
- **Parallel testing**: Build multiple variants simultaneously
- **Development workflow**: `just build-dev`, `just run-cuda-tui`

---

## Common Workflows

### 1. Quick Development Iteration

```bash
# Initial setup
just build-dev

# Develop...

# Quick recompile (incremental)
just build-dev

# Test changes
just test-cpu
```

### 2. Pre-Commit Validation

```bash
# Format and lint (Makefile)
make fmt
make lint-fix

# Verify compiles (Justfile - fast)
just check-all

# Run tests on CPU variant (Justfile)
just test-cpu
```

### 3. Release Preparation

```bash
# Build all variants
just build-all

# Run comprehensive tests
just test-cpu
just test-cuda
just test-cuda-13

# Generate coverage report (Makefile)
make test-coverage

# Create release artifacts
just sizes
just disk
```

### 4. Troubleshooting CUDA Issues

```bash
# Build CPU-only variant to isolate CUDA issues
just clean-cpu
just build-cpu

# If CPU build succeeds, CUDA environment is the problem
just info  # Check CUDA paths

# Try CUDA 13.0 if CUDA 12.9 fails
just build-cuda-13
```

---

## Performance Tips

### 1. Use `check` for Quick Validation

```bash
just check-cpu  # 30-60 seconds
```

Instead of:

```bash
just build-cpu  # 25-35 minutes
```

### 2. Build CPU-Only First

When testing upstream cherry-picks:

```bash
just build-cpu  # No NVCC, faster
```

### 3. Use Debug Build for Development

```bash
just build-dev  # 15-20 minutes
```

Instead of:

```bash
just build-cuda  # 35-50 minutes
```

### 4. Parallel Builds (Advanced)

Build multiple variants simultaneously using background jobs:

```bash
# PowerShell
Start-Job { just build-cpu }
Start-Job { just build-cuda }
Get-Job | Wait-Job
Get-Job | Receive-Job
```

---

## Troubleshooting

### "NVCC not found" Error

**Symptom**: CUDA build fails with NVCC compiler error

**Solution**:
```bash
# Verify environment
just info

# Check MSVC path exists
Test-Path "C:\Program Files\...\cl.exe"

# Try CPU-only build instead
just build-cpu
```

### "Out of disk space" Error

**Symptom**: Build fails with disk space error

**Solution**:
```bash
# Check disk usage
just disk

# Clean old variants
just clean-all

# Build only needed variant
just build-cpu  # Smallest
```

### "Incremental build not working"

**Symptom**: Full rebuild every time

**Solution**:

Ensure you're using the **same** variant consistently:

```bash
just build-cpu  # Creates target-cpu/
just build-cpu  # Reuses target-cpu/ (incremental)
```

NOT:

```bash
just build-cpu   # Creates target-cpu/
just build-cuda  # Creates target-cuda/ (unrelated)
```

---

## Advanced Usage

### Custom Model Paths

```bash
just run-cpu model_path="T:/custom/path"
just run-cuda-tui model_path="T:/custom/path"
just run-cuda-server port=8080 model_path="T:/custom/path"
```

### Environment Variable Overrides

```bash
# Override CUDA path temporarily
$env:CUDA_PATH = "C:\Custom\CUDA\Path"
just build-cuda
```

### Building Specific Packages

The justfile builds the entire workspace. For package-specific builds:

```bash
# Use cargo directly with isolated target dir
$env:CARGO_TARGET_DIR = "target-cpu"
cargo build -p mistralrs-core --release --no-default-features --features mkl
```

---

## Summary

| Command | Purpose | Time | Disk Space |
|---------|---------|------|-----------|
| `just build-cpu` | CPU-only build | 25-35 min | ~10 GB |
| `just build-cuda` | CUDA 12.9 build | 35-50 min | ~12 GB |
| `just build-cuda-13` | CUDA 13.0 build | 35-50 min | ~12 GB |
| `just build-dev` | Debug build | 15-20 min | ~15 GB |
| `just build-release` | LTO build | 60-90 min | ~12 GB |
| `just check-cpu` | Quick check | 30-60 sec | ~500 MB |
| `just check-all` | Check all | 2-4 min | ~500 MB |
| `just clean-all` | Clean all | 10-30 sec | Frees ~50 GB |

---

## Next Steps

1. **Start with CPU-only**: `just build-cpu`
2. **Validate it works**: `just run-cpu`
3. **Run tests**: `just test-cpu`
4. **Build CUDA when ready**: `just build-cuda`

For questions, see:
- `just --list` (all commands)
- `just help` (comprehensive help)
- `just info` (platform detection)
