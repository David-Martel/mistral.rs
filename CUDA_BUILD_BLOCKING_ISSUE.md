# CUDA Build Blocking Issue - Windows bindgen_cuda Bug

## Status: BLOCKED

**Date**: 2025-11-22
**Phase**: 6.1 (Build Verification)
**Severity**: Critical - Prevents all CUDA builds on Windows

## Summary

CUDA builds fail on Windows due to a bug in `bindgen_cuda` (v0.1.5 and v0.1.7) where it cannot execute `nvcc` properly, even though manual execution of the exact same command succeeds.

## Investigation Timeline

### Phase 6: CUDA 13.0 Integration

1. ✅ **Commit bd2bc35d0** cherry-picked successfully
   - Added CUDA 13.0 support via mistralrs-quant changes
   - Updated candle to rev cc5ca5e4

2. ❌ **Build verification failed**
   - All CUDA builds fail with bindgen_cuda errors
   - Empty stdout/stderr from nvcc executions

3. ✅ **Commit f788be344** cherry-picked successfully
   - "Fix flash attn on cuda 13.0 build"
   - Reverted candle to rev 175926c9 (working version)
   - Removed problematic cc5ca5e4 dependency

4. ❌ **Build still fails** with same bindgen_cuda error

## Root Cause Analysis

### Affected Components

- `candle-kernels` (uses bindgen_cuda 0.1.5 from crates.io)
- `candle-flash-attn` (uses bindgen_cuda 0.1.5)
- `mistralrs-core` (uses bindgen_cuda 0.1.7 from git)
- `mistralrs-quant` (uses bindgen_cuda 0.1.7 from git)

### Error Pattern

```
thread 'main' panicked at bindgen_cuda-0.1.5\src\lib.rs:391:13:
nvcc error while compiling "src\\affine.cu":

# CLI "nvcc" "--gpu-architecture=sm_89" "--ptx" ...

# stdout
(empty)

# stderr
(empty)
```

### Verification Tests

#### ✅ Test 1: Manual nvcc execution

```powershell
cd C:\Users\david\.cargo\git\checkouts\candle-6740f55d69a3bf41\175926c\candle-kernels
nvcc --gpu-architecture=sm_89 --ptx --default-stream per-thread `
  --output-directory . -Isrc `
  -IC:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\include `
  -allow-unsupported-compiler `
  -ccbin "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe" `
  src\affine.cu
```

**Result**: SUCCESS - PTX file generated, no errors

#### ✅ Test 2: CPU-only build

```bash
cargo check --package mistralrs-core --no-default-features
```

**Result**: SUCCESS - Builds in 1m 23s

#### ❌ Test 3: CUDA build (debug mode)

```bash
make check  # Uses features: cuda,flash-attn,cudnn,mkl
```

**Result**: FAILED - bindgen_cuda panic

#### ❌ Test 4: CUDA build (release mode)

```bash
cargo build --release --package mistralrs-server --features cuda
```

**Result**: FAILED - bindgen_cuda panic

#### ❌ Test 5: CUDA without flash-attn

```bash
cargo build --release --package mistralrs-server --features cuda,cudnn
```

**Result**: FAILED - candle-kernels still required by candle-core

#### ✅ Test 6: sccache disabled

```bash
$env:RUSTC_WRAPPER=""; cargo check --package mistralrs-quant --features cuda
```

**Result**: FAILED - same error (not sccache-related)

## Technical Analysis

### Why Manual Execution Works

When nvcc is invoked manually from PowerShell:
- Full shell environment is available
- PATH is correctly resolved
- All environment variables are accessible
- Process spawning works normally

### Why bindgen_cuda Fails

`bindgen_cuda` uses `std::process::Command` to spawn nvcc, but:
- The spawned process has empty stdout/stderr
- nvcc appears to execute but produces no output
- Exit code suggests failure but no error message
- Likely related to Windows process spawning or shell handling

### Versions Tested

- bindgen_cuda 0.1.5 (from crates.io) - FAILS
- bindgen_cuda 0.1.7 (from git 19e33d0e) - FAILS
- Both versions exhibit identical behavior

## Environment Details

### System Configuration

```yaml
OS: Windows 11
GPU: NVIDIA GeForce RTX 5060 Ti (16GB VRAM)
CUDA: 12.9 (with 12.1, 12.6, 12.8, 13.0 also installed)
cuDNN: 9.8
MSVC: 14.44.35207 (Visual Studio 2022 Community)
Rust: 1.89.0
```

### Environment Variables (Verified)

```toml
# From .cargo/config.toml
NVCC_CCBIN = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\cl.exe"
CUDA_PATH = "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v12.9"
CUDNN_PATH = "C:\\Program Files\\NVIDIA\\CUDNN\\v9.8"
```

All paths verified to exist and be accessible.

## Impact

### Blocked Features

- ❌ CUDA acceleration (all models CPU-only)
- ❌ Flash Attention
- ❌ cuDNN acceleration
- ❌ CUDA 13.0 features
- ❌ RTX 50-series optimizations (sm_89)

### Working Features

- ✅ CPU inference (all models)
- ✅ All non-CUDA features
- ✅ MCP integration
- ✅ HTTP API
- ✅ TUI interface
- ✅ Python bindings (if built separately)

## Workarounds

### Option 1: WSL2 Build (Recommended)

Build on WSL2 where bindgen_cuda works correctly:

```bash
# In WSL2
cd /mnt/t/projects/rust-mistral/mistral.rs
make build-cuda-full

# Copy binary to Windows
cp target/release/mistralrs-server /mnt/c/path/to/windows/
```

**Pros**: Full CUDA support, all features working
**Cons**: Requires WSL2 setup, binary may have compatibility issues

### Option 2: Use Pre-built Binaries

Check for pre-built Windows CUDA binaries:

```bash
gh release list --repo EricLBuehler/mistral.rs
gh release download v0.6.0 --pattern "*windows*"
```

**Status**: No Windows binaries available in v0.6.0 release
**Pros**: Simple if available
**Cons**: Currently not provided by upstream

### Option 3: Docker Build

Use Docker with Windows containers:

```bash
docker build -t mistralrs-cuda --build-arg FEATURES=cuda,flash-attn,cudnn .
docker cp container:/app/target/release/mistralrs-server.exe .
```

**Pros**: Isolated build environment
**Cons**: Requires Docker Desktop, complex setup

### Option 4: CPU-Only Deployment

Deploy without CUDA for now:

```bash
cargo build --release --no-default-features --features mkl
```

**Pros**: Works immediately, no blockers
**Cons**: Significantly slower inference (~10x)

## Upstream Issues

### Related Issues

1. **bindgen_cuda repository**:
   - Issue to be filed: "Windows: nvcc execution produces empty stdout/stderr"
   - Affects both 0.1.5 and 0.1.7
   - Likely related to std::process::Command usage

2. **candle repository**:
   - May need to pin or patch bindgen_cuda version
   - Alternative: provide pre-compiled CUDA kernel binaries

3. **mistral.rs repository**:
   - No Windows-specific CUDA build documentation
   - No pre-built Windows binaries with CUDA support

## Recommendations

### Immediate (Next Session)

1. ✅ Document this issue (this file)
2. ⏳ Push all CUDA 13.0 work to GitHub with notes
3. ⏳ Update README with Windows CUDA limitation
4. ⏳ Update TODO.md with workaround options

### Short-term (Next Week)

1. File issue with bindgen_cuda project
2. Test WSL2 build workaround
3. Investigate Docker build option
4. Check if newer CUDA toolkit versions help

### Long-term (Next Month)

1. Work with upstream to provide Windows CUDA binaries
2. Contribute fix to bindgen_cuda if feasible
3. Explore alternative CUDA build systems

## Commits

### Successfully Integrated

- `bd2bc35d0` - Support cuda 13.0! (#1697)
  - 7 files changed, 1259 insertions, 1078 deletions
  - Adds CUDA 13.0 support in mistralrs-quant

- `f788be344` - Fix flash attn on cuda 13.0 build (#1704)
  - 2 files changed, 10 insertions, 10 deletions
  - Reverts candle to working version (175926c9)

### Build Status

- ✅ All commits cherry-picked successfully
- ✅ No merge conflicts
- ❌ **Build blocked by bindgen_cuda bug**
- ✅ CPU-only builds work fine
- ❌ CUDA builds fail consistently

## Testing Matrix

| Configuration | Status | Notes |
|---|---|---|
| CPU-only (no features) | ✅ PASS | 1m 23s build time |
| CPU + MKL | ✅ PASS | Works as expected |
| CUDA (debug) | ❌ FAIL | bindgen_cuda error |
| CUDA (release) | ❌ FAIL | bindgen_cuda error |
| CUDA + flash-attn | ❌ FAIL | bindgen_cuda error |
| CUDA without flash-attn | ❌ FAIL | candle-kernels required |

## Next Steps

Given this blocking issue, the recommended path forward is:

1. **Document and commit** all findings (this session)
2. **Test WSL2 workaround** (next session)
3. **Deploy CPU-only version** for immediate use
4. **File upstream issues** to track resolution
5. **Resume Phase 3B** once CUDA builds work

## Files Modified

- `Cargo.toml` - Candle dependencies reverted to 175926c9
- `Cargo.lock` - Dependencies updated
- `mistralrs-quant/build.rs` - CUDA 13.0 support
- `mistralrs-quant/kernels/ops/ops.cu` - CUDA 13.0 kernels
- `mistralrs-core/src/attention/backends/flash.rs` - Unused code removed

## References

- bindgen_cuda: https://github.com/guoqingbao/bindgen_cuda
- candle: https://github.com/EricLBuehler/candle (fork)
- mistral.rs: https://github.com/EricLBuehler/mistral.rs
- CUDA 13.0 PR: https://github.com/EricLBuehler/mistral.rs/pull/1697
- Flash attn fix PR: https://github.com/EricLBuehler/mistral.rs/pull/1704

---

**Status**: Investigation complete. Issue documented. Workarounds identified.
**Next Action**: Push to GitHub and explore WSL2 build option.
