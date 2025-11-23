# mistral.rs TUI Release Standard

**Version**: 1.0.0
**Last Updated**: 2025-11-23
**Status**: Production Ready

______________________________________________________________________

## Overview

This document defines the standardized release process for the mistral.rs TUI (Terminal User Interface) with full feature set support across all platforms.

## Feature Sets

### Available Features

The TUI supports multiple feature combinations:

| Feature     | Description                          | Default | Dependencies          |
| ----------- | ------------------------------------ | ------- | --------------------- |
| `terminal`  | Terminal mode with ratatui/crossterm | ✅ Yes  | ratatui, crossterm    |
| `gpu`       | GPU rendering support                | ❌ No   | wgpu, winit, pollster |
| `tui-agent` | Agent tools integration              | ❌ No   | mistralrs-agent-tools |

### Build Configurations

#### Production Build (All Features)

```bash
# Full feature set (recommended for releases)
cargo build --release --package mistralrs-tui --features "terminal,gpu,tui-agent"
```

**Binary Location**: `target/release/mistralrs-tui` or `target/release/mistralrs-tui.exe`

**Features Enabled**:

- ✅ Terminal mode (default UI)
- ✅ GPU rendering support
- ✅ Agent tools integration
- ✅ Full functionality

#### Minimal Build (Terminal Only)

```bash
# Minimal build for testing
cargo build --release --package mistralrs-tui --features "terminal"
```

**Use Case**: Quick testing, minimal dependencies, CI/CD validation

#### Development Build

```bash
# Fast development iteration
cargo build --package mistralrs-tui --features "terminal,tui-agent"
```

**Features**: Terminal + agent tools (no GPU for faster builds)

______________________________________________________________________

## Testing Matrix

### Available Test Models

From `models.json`, 5 models available for testing:

| Model                    | Size (GB) | Use Case          | VRAM  | Speed        |
| ------------------------ | --------- | ----------------- | ----- | ------------ |
| Qwen2.5-1.5B-Instruct-Q4 | 0.94      | Quick responses   | ~2GB  | 80-100 tok/s |
| Qwen2.5-Coder-3B-Q4      | 1.93      | Code analysis     | ~3GB  | 60-80 tok/s  |
| Gemma 2 2B-it-Q4         | 1.67      | General purpose   | ~3GB  | 70-90 tok/s  |
| Qwen2.5-7B-Instruct-Q4   | 4.37      | Complex reasoning | ~6GB  | 40-60 tok/s  |
| Gemma 3 4B-it-hf         | 8.50      | General purpose   | ~10GB | 30-50 tok/s  |

### Testing Checklist

#### Phase 1: Build Validation

- [ ] **Full Feature Build**

  ```bash
  cargo build --release --package mistralrs-tui --features "terminal,gpu,tui-agent"
  ```

  - Expected: 0 errors, 0 warnings
  - Binary size: ~50-100 MB

- [ ] **Terminal Only Build**

  ```bash
  cargo build --release --package mistralrs-tui --features "terminal"
  ```

  - Expected: Smaller binary, faster build

- [ ] **Clippy Validation**

  ```bash
  cargo clippy --package mistralrs-tui --all-features
  ```

  - Expected: 0 warnings

#### Phase 2: Functional Testing

Test with smallest model (Qwen2.5-1.5B-Instruct-Q4_K_M):

- [ ] **TUI Launch**

  ```bash
  ./target/release/mistralrs-tui gguf -m C:\codedev\llm\.models\qwen2.5-1.5b-it-gguf\Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
  ```

  - Expected: TUI starts without errors
  - VRAM usage: ~2 GB
  - Interface responsive

- [ ] **Chat Interaction**

  - Send test message: "Hello!"
  - Expected: Response within 1-2 seconds
  - Verify: Text renders correctly

- [ ] **Model Switching**

  - Load different model from TUI
  - Expected: Clean model unload/reload
  - No memory leaks

- [ ] **Agent Tools (if enabled)**

  - Test agent command execution
  - Expected: Tools accessible and functional

#### Phase 3: Performance Testing

- [ ] **Memory Profiling**

  - Monitor VRAM usage over time
  - Expected: Stable, no gradual increase

- [ ] **Response Speed**

  - Test with different model sizes
  - Verify tokens/second matches expectations

- [ ] **Long Session Stability**

  - Run for 10+ minutes with multiple queries
  - Expected: No crashes, stable performance

#### Phase 4: Cross-Platform Testing

- [ ] **Windows (Primary Platform)**

  - Binary: `mistralrs-tui.exe`
  - CUDA support verified
  - Launch scripts functional

- [ ] **Linux** (if applicable)

  - Binary: `mistralrs-tui`
  - Build configuration compatible

- [ ] **macOS** (if applicable)

  - Metal support enabled
  - Binary functional

______________________________________________________________________

## Release Process

### Pre-Release Checklist

1. **Code Quality**

   - [ ] All tests passing (`cargo test --package mistralrs-tui`)
   - [ ] No clippy warnings (`cargo clippy --package mistralrs-tui --all-features`)
   - [ ] Code formatted (`cargo fmt --package mistralrs-tui`)

1. **Documentation**

   - [ ] README updated with new features
   - [ ] CHANGELOG.md entry created
   - [ ] API changes documented

1. **Version Bump**

   - [ ] Update `Cargo.toml` version
   - [ ] Update version in this document
   - [ ] Tag release in git

1. **Build Artifacts**

   - [ ] Full feature build created
   - [ ] Binary tested on target platform
   - [ ] Binary stripped (if applicable): `cargo build --release && strip target/release/mistralrs-tui`

### Release Steps

#### Step 1: Final Testing

```bash
# Run comprehensive tests
cargo test --package mistralrs-tui --all-features

# Build release binary
cargo build --release --package mistralrs-tui --features "terminal,gpu,tui-agent"

# Test binary with smallest model
./target/release/mistralrs-tui gguf -m /path/to/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
```

#### Step 2: Create Release Build

```bash
# Clean build
cargo clean

# Full release build
cargo build --release --package mistralrs-tui --features "terminal,gpu,tui-agent"

# Verify binary
file target/release/mistralrs-tui  # or mistralrs-tui.exe on Windows

# Optional: Strip debug symbols (Linux/macOS)
strip target/release/mistralrs-tui
```

#### Step 3: Package Release

```bash
# Create release directory
mkdir -p releases/mistralrs-tui-v0.6.0

# Copy binary
cp target/release/mistralrs-tui releases/mistralrs-tui-v0.6.0/

# Copy documentation
cp README.md releases/mistralrs-tui-v0.6.0/
cp TUI_RELEASE_STANDARD.md releases/mistralrs-tui-v0.6.0/

# Copy launch scripts
cp scripts/launch/*.ps1 releases/mistralrs-tui-v0.6.0/

# Create archive
cd releases && tar -czf mistralrs-tui-v0.6.0.tar.gz mistralrs-tui-v0.6.0/
```

#### Step 4: Git Tagging

```bash
# Create annotated tag
git tag -a tui-v0.6.0 -m "TUI Release v0.6.0

Features:
- Terminal mode with ratatui
- GPU rendering support
- Agent tools integration
- Full CUDA/cuDNN support

Tested with:
- Qwen2.5-1.5B-Instruct-Q4_K_M (0.94 GB)
- Gemma 2 2B-it-Q4_K_M (1.67 GB)
- All features enabled"

# Push tag
git push origin tui-v0.6.0
```

#### Step 5: Create GitHub Release

```bash
# Using GitHub CLI
gh release create tui-v0.6.0 \
  --title "mistralrs-tui v0.6.0" \
  --notes "Full-featured TUI release with terminal, GPU, and agent support" \
  releases/mistralrs-tui-v0.6.0.tar.gz
```

______________________________________________________________________

## Deployment

### Standard Deployment

#### Windows

```powershell
# Copy binary to shared cargo target
$binaryPath = "C:\Users\david\.cargo\shared-target\release\mistralrs-tui.exe"
Copy-Item target\release\mistralrs-tui.exe $binaryPath

# Verify deployment
& $binaryPath --version
```

#### Linux/macOS

```bash
# Copy binary to local bin
sudo cp target/release/mistralrs-tui /usr/local/bin/

# Verify deployment
mistralrs-tui --version
```

### Launch Scripts Deployment

```bash
# Copy launch scripts to user directory
mkdir -p ~/.mistralrs/scripts
cp scripts/launch/*.ps1 ~/.mistralrs/scripts/  # Windows
cp scripts/launch/*.sh ~/.mistralrs/scripts/   # Linux/macOS

# Make executable
chmod +x ~/.mistralrs/scripts/*.sh
```

______________________________________________________________________

## Validation Checklist

### Pre-Production Validation

Before deploying to production, verify:

- [ ] **Binary Integrity**

  - SHA256 checksum verified
  - No corruption during build
  - File size reasonable (~50-100 MB)

- [ ] **Functionality**

  - TUI launches without errors
  - Model loading works
  - Chat interaction functional
  - Agent tools accessible (if enabled)

- [ ] **Performance**

  - Response times acceptable
  - Memory usage stable
  - No performance degradation over time

- [ ] **Compatibility**

  - CUDA version compatible (12.9+)
  - cuDNN version compatible (9.8+)
  - Model formats supported (GGUF, safetensors)

### Post-Deployment Validation

After deployment:

- [ ] **Smoke Test**

  ```bash
  # Quick functional test
  ./mistralrs-tui gguf -m /path/to/small-model.gguf
  ```

- [ ] **Integration Test**

  - Load multiple models
  - Test agent commands
  - Verify MCP integration (if enabled)

- [ ] **User Acceptance**

  - Collect user feedback
  - Monitor error logs
  - Track performance metrics

______________________________________________________________________

## Common Issues and Solutions

### Build Issues

**Issue**: `error: could not compile mistralrs-tui`
**Solution**: Ensure all dependencies installed:

```bash
cargo clean
cargo update
cargo build --release --package mistralrs-tui --features "terminal,gpu,tui-agent"
```

**Issue**: GPU features not working
**Solution**: Verify CUDA environment:

```bash
nvcc --version
echo $CUDA_PATH  # or $env:CUDA_PATH on Windows
```

### Runtime Issues

**Issue**: TUI won't launch
**Solution**: Check model path and permissions:

```bash
# Verify model exists
ls -lh /path/to/model.gguf

# Check file permissions
chmod +r /path/to/model.gguf
```

**Issue**: Out of VRAM errors
**Solution**: Use smaller model or reduce batch size:

```bash
# Use smallest model
./mistralrs-tui gguf -m /path/to/Qwen2.5-1.5B-Instruct-Q4_K_M.gguf
```

### Performance Issues

**Issue**: Slow response times
**Solution**: Check CUDA device utilization:

```bash
nvidia-smi  # Monitor GPU usage
```

**Issue**: Memory leaks over time
**Solution**: Update to latest version, report issue if persistent

______________________________________________________________________

## Version History

| Version | Date       | Changes                           |
| ------- | ---------- | --------------------------------- |
| 1.0.0   | 2025-11-23 | Initial release standard document |

______________________________________________________________________

## Appendix: Feature Matrix

### Complete Feature Combinations

| Build    | terminal | gpu | tui-agent | Use Case                 |
| -------- | -------- | --- | --------- | ------------------------ |
| Minimal  | ✅       | ❌  | ❌        | Testing, CI/CD           |
| Standard | ✅       | ✅  | ❌        | General use              |
| Agent    | ✅       | ❌  | ✅        | Development, automation  |
| Full     | ✅       | ✅  | ✅        | Production (recommended) |

### Command Examples

```bash
# Minimal build
cargo build --release --package mistralrs-tui

# Standard build
cargo build --release --package mistralrs-tui --features "terminal,gpu"

# Agent build
cargo build --release --package mistralrs-tui --features "terminal,tui-agent"

# Full build (recommended)
cargo build --release --package mistralrs-tui --features "terminal,gpu,tui-agent"
```

______________________________________________________________________

**Maintained by**: mistral.rs core team
**Contact**: See project README for support channels
**License**: MIT (see LICENSE file)
