# CLAUDE.md

Guidance for Claude Code when working with this repository.
See [AGENTS.md](AGENTS.md) for cross-agent expectations. See `.claude/context/` for detailed context files.

## Project Overview

mistral.rs is a blazing-fast LLM inference engine in Rust supporting text, vision, image generation, and speech models via Rust, Python, OpenAI HTTP, and MCP APIs.

- **Version**: 0.6.0 (upstream has v0.7.0)
- **Rust**: 1.86+ required (1.89.0 installed)
- **Fork of**: `EricLBuehler/mistral.rs` — currently 158 commits behind upstream, 107 local ahead
- **Branch**: `phase2-upstream-integration`

## Build System

**Use `make`, never `cargo` directly.** The Makefile handles platform detection, CUDA env vars, feature flags, and sccache.

```bash
make help              # All targets
make check             # Quick compile check (ALWAYS before committing)
make build             # Release CPU build
make build-cuda-full   # Windows/Linux CUDA + flash-attn + cuDNN + MKL
make build-metal       # macOS Metal
make test              # Run tests
make fmt               # Format Rust + Python + C/CUDA
make lint-fix          # Auto-fix clippy issues
make ci                # Full pipeline: fmt-check + lint + check + test
```

Test specific packages: `make test-core`, `make test-server`, `make test-quant`, `make test-vision`

See `.claude/CLAUDE.md` for detailed build practices, CUDA config, and troubleshooting.

## Architecture

```
mistralrs-core/        Core inference engine, model implementations, pipelines
mistralrs-server/      CLI binary entry point
mistralrs-server-core/ HTTP server, OpenAI API routes
mistralrs-pyo3/        Python bindings (PyO3)
mistralrs/             High-level Rust API + examples
mistralrs-quant/       Quantization (ISQ, GGUF, GPTQ, HQQ, AFQ, FP8)
mistralrs-paged-attn/  PagedAttention
mistralrs-vision/      Vision model support
mistralrs-audio/       Audio processing
mistralrs-mcp/         MCP client
mistralrs-bench/       Benchmarks
```

### Key Files

- `mistralrs-core/src/engine/mod.rs` — Main engine orchestration
- `mistralrs-core/src/pipeline/mod.rs` — Pipeline trait (all models implement this)
- `mistralrs-core/src/pipeline/normal.rs` — Model detection/routing
- `mistralrs-core/src/lib.rs` — Architecture enum, public types
- `mistralrs-server-core/src/routes.rs` — HTTP API endpoints
- `mistralrs-server/src/main.rs` — CLI args and entry point

### Adding Models

1. Implement in `mistralrs-core/src/models/`
2. Add pipeline in `mistralrs-core/src/pipeline/`
3. Update detection in `mistralrs-core/src/pipeline/normal.rs`
4. Add arch enum in `mistralrs-core/src/lib.rs`
5. Update CLI in `mistralrs-server/src/main.rs`

**VarBuilder rule**: All `.pp()` prefix paths must match PyTorch `state_dict` keys exactly. Check `model.safetensors.index.json` for structure.

## Development Workflow

1. `make check` — before any changes
2. Make edits
3. `make check` — verify compilation
4. `make test-core` (or relevant package)
5. `make fmt && make lint`
6. `make ci` — before committing

Always run `make check` before returning. Never return TODOs.

## Environment

- **GPU**: NVIDIA RTX 5060 Ti (16GB VRAM)
- **CUDA**: 12.9 (also 12.1, 12.6, 12.8, 13.0)
- **cuDNN**: 9.8
- **Platform**: Windows 11 + PowerShell
- **Build tools**: Visual Studio 2022

## Available Test Models

See `MODEL_INVENTORY.json`. Use Qwen2.5-1.5B-Instruct-Q4_K_M (940MB) for quick testing.

## Running

```bash
# TUI (interactive)
./target/release/mistralrs-server -i gguf -m /path/to/model -f model.gguf

# HTTP server
./target/release/mistralrs-server --port 1234 gguf -m /path -f model.gguf

# With MCP tools
./target/release/mistralrs-server --port 1234 --mcp-config mcp-config.json gguf -m /path -f model.gguf

# MCP server mode
./target/release/mistralrs-server --mcp-port 4321 plain -m Qwen/Qwen3-4B
```

Subcommands: `run` (text/vision), `diffusion` (image gen), `speech` (audio gen)

## Common Pitfalls

1. Using `cargo` directly — always use `make`
2. Missing feature flags — Makefile handles per-platform
3. NVCC host compiler — Makefile sets `NVCC_CCBIN` automatically
4. Chat templates — check `chat_templates/` directory
5. MCP uses JSON-RPC over stdin/stdout, not HTTP

## Detailed Context

Consolidated context files in `.claude/context/`:
- `upstream-integration-20260222.md` — Upstream divergence, integration strategy
- `build-deployment-20260222.md` — Detailed build config, CI/CD, deployment
- `agent-tools-20260222.md` — Agent tools implementation status
- `code-quality-20260222.md` — Unwrap elimination, error handling, optimization
- `session-archive-20260222.md` — Historical session timeline
- `CONTEXT_INDEX.md` — Full index of all context files

## Resources

- **Rust API docs**: https://ericlbuehler.github.io/mistral.rs/mistralrs/
- **HTTP API**: `docs/HTTP.md`
- **Python API**: `mistralrs-pyo3/API.md`
- **Discord**: https://discord.gg/SZrecqK8qw
