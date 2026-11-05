# bindgen_cuda Windows Compatibility Analysis

**Date**: 2026-01-31
**Status**: Partial support available with workarounds

## Executive Summary

The `bindgen_cuda` crate (v0.1.6) **does have Windows (MSVC) support**, but with known issues. Upstream mistral.rs v0.7.0 has moved to `candle 0.9.2` from crates.io, which may improve stability but does not fundamentally change the bindgen_cuda dependency chain.

## Current State

### Dependency Chain

```
mistral.rs
├── mistralrs-paged-attn (build.rs uses bindgen_cuda)
├── candle-flash-attn (build.rs uses bindgen_cuda)
└── candle-kernels (build.rs uses bindgen_cuda)
```

### candle-flash-attn/build.rs Windows Support

```rust
// From candle-flash-attn/build.rs
let target = std::env::var("TARGET").unwrap();
let out_file = if target.contains("msvc") {
    // Windows case
    build_dir.join("libflashattention.lib")
} else {
    build_dir.join("libflashattention.a")
};

// Windows-specific compiler flags
if !target.contains("msvc") {
    builder = builder.arg("-fPIC");  // Skip on Windows
}
if target.contains("msvc") {
    builder = builder.arg("-D_USE_MATH_DEFINES");  // Add for Windows
}
```

### Known Issues

| Issue | Description | Status |
|-------|-------------|--------|
| [bindgen_cuda #14](https://github.com/Narsil/bindgen_cuda/issues/14) | "Can't find Cuda in Windows" | Open (Nov 2025) |
| [bindgen_cuda #8](https://github.com/Narsil/bindgen_cuda/issues/8) | nvidia-smi/nvcc version mismatch | Open |
| [mistral.rs #847](https://github.com/EricLBuehler/mistral.rs/issues/847) | Windows VS + CUDA 12.6 build issues | Workarounds available |

## Upstream Approach

Upstream mistral.rs v0.7.0 takes a **conservative approach**:

```rust
// mistralrs-paged-attn/build.rs (upstream)
#[cfg(all(feature = "cuda", target_family = "unix"))]
fn main() -> Result<()> {
    // Full CUDA paged attention implementation
}

#[cfg(not(any(all(feature = "cuda", target_family = "unix"), feature = "metal")))]
fn main() -> Result<()> {
    // No-op - CUDA paged attention disabled on Windows
    Ok(())
}
```

**This means**: Upstream does NOT support CUDA paged attention on Windows.

## Solutions

### Option 1: Use Upstream Approach (Recommended for Stability)

Disable CUDA paged attention on Windows, use CPU/basic CUDA paths instead.

**Pros**:
- Stable, matches upstream
- Avoids bindgen_cuda issues
- Still gets CUDA acceleration for most operations

**Cons**:
- Loses paged attention performance optimizations
- May impact throughput for long contexts

### Option 2: Keep Local Windows CUDA Support (Current)

Maintain the custom `build.rs` with Windows support.

**Pros**:
- Full CUDA paged attention on Windows
- Better performance for long contexts

**Cons**:
- May break with bindgen_cuda updates
- Requires manual maintenance
- Risk of build failures

### Option 3: Environment Variable Workarounds

Set environment variables to help bindgen_cuda find CUDA:

```powershell
# PowerShell
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
$env:CUDA_COMPUTE_CAP = "89"  # For RTX 50-series
$env:NVCC_CCBIN = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\bin\Hostx64\x64\cl.exe"

# Add to PATH
$env:PATH = "$env:CUDA_PATH\bin;$env:PATH"
```

### Option 4: Pre-compiled CUDA Kernels

Pre-compile CUDA kernels on a Linux system and include them as binary blobs.

**Status**: Not implemented, would require significant build system changes.

## Candle 0.9.2 Analysis

Upstream's switch to crates.io `candle 0.9.2` **does not resolve** the bindgen_cuda Windows issues because:

1. `candle-flash-attn` still uses `bindgen_cuda = "0.1.6"` as a build dependency
2. The build process still invokes NVCC through bindgen_cuda
3. Path detection issues remain in bindgen_cuda itself

**However**, crates.io releases are more stable than git revisions, so fewer breaking changes are expected.

## Recommendations

### For This Fork

1. **Keep current Windows CUDA support** with the custom `build.rs`
2. **Add fallback logic** for when CUDA compilation fails
3. **Document environment variable requirements** clearly
4. **Monitor [bindgen_cuda #14](https://github.com/Narsil/bindgen_cuda/issues/14)** for upstream fixes

### Makefile Updates

Ensure `NVCC_CCBIN` and `CUDA_PATH` are set:

```makefile
# Makefile (already implemented)
export NVCC_CCBIN := C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.40.33807\bin\Hostx64\x64\cl.exe
export CUDA_PATH := C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9
```

### .cargo/config.toml Updates

```toml
[env]
NVCC_CCBIN = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.40.33807\\bin\\Hostx64\\x64\\cl.exe"
CUDA_COMPUTE_CAP = "89"
```

## Future Considerations

### cudaforge Alternative

The [`cudaforge`](https://crates.io/crates/cudaforge) crate advertises "auto-detection" and "incremental builds" but is very new (v0.1.0) and untested.

### Direct NVCC Integration

Some projects bypass bindgen_cuda entirely by calling NVCC directly in build.rs. This is more complex but avoids the path detection issues.

## Testing

To verify Windows CUDA builds work:

```bash
# 1. Check environment
make check-cuda-env

# 2. Build with CUDA
make build-cuda-full

# 3. If fails, check:
#    - NVCC_CCBIN is set correctly
#    - CUDA_PATH points to installed toolkit
#    - Visual Studio C++ workload is installed
```

## References

- [bindgen_cuda repository](https://github.com/Narsil/bindgen_cuda)
- [candle CUDA issues](https://github.com/huggingface/candle/issues/3249)
- [mistral.rs Windows build issue](https://github.com/EricLBuehler/mistral.rs/issues/847)
- [NVIDIA CUDA Windows Installation Guide](https://docs.nvidia.com/cuda/cuda-installation-guide-microsoft-windows/)
