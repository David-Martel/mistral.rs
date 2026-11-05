# Build Reference - mistral.rs

Quick reference for build system. **Always use `make`, never `cargo` directly.**

## Make Targets

### Core Workflow
| Target | Description |
|--------|-------------|
| `make check` | Compile check (run before every commit) |
| `make dev` | Debug build |
| `make build` | Release CPU build |
| `make build-cuda-full` | CUDA + flash-attn + cuDNN + MKL |
| `make build-metal` | macOS Metal + Accelerate |
| `make build-server` | Server only (skips PyO3) |
| `make test` | All tests |
| `make test-core` | Core engine tests |
| `make test-server` | Server tests |
| `make test-quant` | Quantization tests |
| `make test-vision` | Vision model tests |
| `make fmt` | Format Rust + Python + C/CUDA |
| `make fmt-check` | Verify formatting |
| `make lint` | Clippy lints |
| `make lint-fix` | Auto-fix lints |
| `make ci` | Full pipeline: fmt-check + lint + check + test |
| `make clean` | Clean artifacts |
| `make clean-all` | Deep clean including caches |

### Running
| Target | Description |
|--------|-------------|
| `make run-tui` | TUI with smallest test model |
| `make run-server MODEL=/path` | HTTP server on port 8080 |
| `make run-with-mcp MODEL_DIR=/path MODEL_FILE=model.gguf` | With MCP |

### Environment
| Target | Description |
|--------|-------------|
| `make help` | List all targets |
| `make check-env` | Validate build environment |
| `make check-cuda-env` | Validate CUDA setup |

## Environment Variables

Set automatically by Makefile:

| Variable | Value | Purpose |
|----------|-------|---------|
| `NVCC_CCBIN` | `cl.exe` full path | NVCC host compiler |
| `CUDA_PATH` | `C:\Program Files\...\CUDA\v12.9` | CUDA toolkit |
| `CUDNN_PATH` | `C:\Program Files\NVIDIA\CUDNN\v9.8` | cuDNN |
| `RUSTC_WRAPPER` | `sccache` | Build cache |
| `CARGO_INCREMENTAL` | `1` | Incremental builds |

## Feature Flags by Platform

- **Windows/Linux**: `cuda,flash-attn,cudnn,mkl`
- **macOS**: `metal,accelerate`
- **CPU-only**: (no feature flags needed)

Never use `--all-features` — it pulls platform-incompatible deps (e.g., `metal` on Windows causes `objc_exception` link errors).

## Build Times

| Scenario | Time |
|----------|------|
| Cold cache (first build) | 30-45 min |
| With sccache (incremental) | 2-5 min |
| Single crate change | 30-90 sec |

## Troubleshooting

**Build fails**: Check `.logs/build.log`, run `make check-env`
**NVCC fails**: Makefile sets `NVCC_CCBIN` — run `make check-cuda-env`
**PyO3 fails**: Use `make build-server` to skip Python bindings
**Out of memory**: `make build JOBS=4` to limit parallelism
**Tests fail**: `make test-core VERBOSE=1`, check `.testlogs/*.log`
**Stale cache**: `make clean-all && make build`

## Binary Sizes

- `mistralrs-server.exe` (Windows): ~380 MB
- With LTO + strip: ~280 MB

## VRAM Usage

- Qwen2.5-1.5B-Q4: ~1.2 GB
- Gemma 2 2B-Q4: ~2.0 GB
- Qwen2.5-7B-Q4: ~5.5 GB
- Gemma 3 4B-full: ~9.2 GB
