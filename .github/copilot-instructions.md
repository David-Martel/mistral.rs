# Copilot Project Instructions: mistral.rs

Purpose: Enable AI coding agents to contribute effectively to the mistral.rs multimodal inference engine (text, vision, diffusion, speech) with minimal ramp-up.
Always review the workspace-wide [Repository Guidelines](../AGENTS.md) before following these Copilot-specific notes.

## 1. Core Architecture (Know Before Editing)
- Crate layering:
  - `mistralrs-core/`: Engine, pipelines, model loading, device & quant logic.
  - `mistralrs/`: High-level safe Rust API wrapper (builders like `TextModelBuilder`).
  - `mistralrs-server-core/`: HTTP/OpenAI API routes & shared server logic.
  - `mistralrs-server/`: CLI binary (subcommands: `run`/`plain`, `vision-plain`, `diffusion`, `speech`).
  - `mistralrs-vision/`, `mistralrs-audio/`: Modality extensions.
  - `mistralrs-quant/`: Quantization backends (ISQ, GGUF, GPTQ, AWQ, HQQ, FP8, etc.).
  - `mistralrs-paged-attn/`: PagedAttention implementation.
  - `mistralrs-pyo3/`: Python bindings via PyO3.
  - `mistralrs-mcp/`: Model Context Protocol client (tool orchestration).
- Pipelines: Implement the `Pipeline` trait (`mistralrs-core/src/pipeline/`) per model format (plain, gguf, diffusion, speech, vision). Dispatch lives in `normal.rs` and engine logic in `engine/mod.rs`.
- Model integration relies on Candle VarBuilder prefix chaining (`vb.pp("...")`) to mirror original PyTorch `state_dict` key layout; never rename keys manually.

## 2. Mandatory Build Workflow
DO NOT call `cargo build` directly for featureful builds.
Use `Makefile` targets which set env vars, features, caching:
- `make help` (discover) | `make build` (CPU) | `make build-cuda-full` (CUDA + flash-attn + cudnn) | `make build-metal` (Apple GPU)
- Fast feedback: `make check` (compiles), `make dev` (debug build)
- Quality: `make fmt`, `make lint`, `make lint-fix`, `make test`, `make ci` (aggregated pre-commit pipeline)
If you must provide a cargo command in docs/examples, ensure an equivalent Makefile target exists or note why it’s exception-safe.

## 3. Testing & Minimal Repro Strategy
- Prefer smallest model: `Qwen2.5-1.5B-Instruct-Q4_K_M` for fast GGUF tests.
- Targeted tests: `make test-core`, `make test-server`, `make test-vision`, `make test-quant`, `make test-audio` (if present) instead of full workspace when iterating.
- Always run `make check` after edits before committing patches.
- For failing runtime issues, enable verbose logs: `MISTRALRS_DEBUG=1 <binary ...>`.
- MCP / tool interaction tests: use provided PowerShell scripts (`test-mcp-servers.ps1`) or start server with `--mcp-config` sample from `examples/MCP_QUICK_START.md`.

## 4. Adding a New Model Architecture (Checklist)
1. Implement core module under `mistralrs-core/src/models/<new_model>/`.
2. Extend pipeline variant in `mistralrs-core/src/pipeline/` (and register in `normal.rs`).
3. Add enum / type wiring in `mistralrs-core/src/lib.rs` & any architecture discriminant.
4. Update loader or quant hooks if format-specific.
5. Expose CLI support: modify `mistralrs-server/src/main.rs` arg parsing.
6. Provide example usage under `mistralrs/examples/<new_model>/` (mirroring existing style).
7. Update docs under `docs/` (create `<MODEL>.md` or amend related feature doc).
8. Add minimal test (guarded if heavy) validating pipeline init + one inference step (mock or smallest weights).

## 5. Quantization / Performance Extensions
- Implement new quant kernels under `mistralrs-quant/` maintaining existing trait contracts; ensure fallback path when feature flags are absent.
- Update docs: `docs/QUANTS.md` and mention capability in README quick list if user-facing.
- Validate interplay with PagedAttention (see `mistralrs-paged-attn/`) and KV cache quant (docs: `PAGED_ATTENTION.md`).

## 6. MCP Integration Nuances
- Client config loaded and attached via builder (`with_mcp_client`). Auto tool registration uses JSON config schema (see examples).
- Server side: `--mcp-config` (client tools) vs `--mcp-port` (serve model via MCP). Don’t conflate these.
- Tool calling path: chat request -> tool planning (model outputs tool calls) -> execution via registered MCP servers -> tool messages appended -> final model call. Preserve this order when adjusting conversation state handling.

## 7. HTTP / Responses API
- Routes defined in `mistralrs-server-core/src/routes.rs` (OpenAI-compatible). Streaming chunks follow OpenAI event schema; maintain field parity.
- When modifying response structs, update both: Rust type + Python bindings + docs (`docs/HTTP.md`) + examples (`examples/server/`). Avoid breaking existing JSON keys.

## 8. Common Pitfalls (Prevent Before Submitting)
- Direct `cargo build` usage (skips feature coordination) → use Makefile.
- Forgetting VarBuilder `.pp` path alignment → causes silent weight mismatches; cross-check with `model.safetensors.index.json`.
- Changing public struct fields without updating PyO3 wrappers.
- Introducing blocking IO on async server paths (prefer tokio + spawn blocking for heavy CPU). Review existing async patterns in server-core.
- Large model tests checked into CI (avoid); gate with feature or environment variable.

## 9. Debug & Diagnostics Patterns
- Build issues: inspect `.logs/build.log` (created by Makefile) before retrying.
- CUDA environment: `make check-cuda-env` validates nvcc, arch flags.
- Tensor inspection: set `MISTRALRS_DEBUG=1` to emit `mistralrs_gguf_tensors.txt` / `mistralrs_ggml_tensors.txt`.
- Multi-device mapping code: `device_map.rs` (auto vs manual). When adjusting balancing heuristics, add doc comment with complexity rationale.

## 10. Style & Contribution Conventions
- Commit messages: Conventional style `feat(crate): ...`, `fix(crate): ...`.
- Keep modules cohesive—avoid giant feature flags across unrelated crates; prefer per-crate feature gating.
- Add targeted example + doc line for any user-visible feature; absence is a signal feature isn’t ready.
- Prefer small, composable builders (follow existing `TextModelBuilder`, `VisionModelBuilder` patterns) over sprawling config structs.

## 11. When Unsure
Check these in order:
1. `CLAUDE.md` (root) – detailed workflows & examples.
2. `AGENTS.md` – high-level structure & test/build basics.
3. `docs/` – specific feature details (quantization, device mapping, adapters, etc.).
4. Analogous existing model or quant implementation—mirror style/traits.

## 12. Minimal Verification Before PR / Patch
Run sequentially:
```
make check
make lint
make test-core  # broaden only if area touched
make fmt
make ci         # final gate
```
Ensure binary still launches with smallest test model (GGUF) and one chat round-trip completes.

---
Questions or unclear areas? Ask for: clarification on model integration boundaries, adding a new quant backend, or extending MCP tool flow. Provide file paths + intent, and propose a minimal diff plan.

## 13. MCP Servers & Tooling (Project Guidance)
Configured (see `MCP_CONFIG.json` / secure variant `tests/mcp/MCP_CONFIG_SECURE.json`). Auto registration usually on (`auto_register_tools: true`). Use smallest safe subset for CI or automated agents.

| Server Name | Typical Tools (examples) | Primary Use in mistral.rs | Risk / Scope | When To Enable | Notes |
|-------------|--------------------------|---------------------------|--------------|----------------|-------|
| Memory | `get_memory`, `append_memory`, `list_sessions` | Persist lightweight convo / test state across multi-turn tool plans | Low (scoped file) | Long-running agent demos, ReAct planning benchmarks | File path set via `MEMORY_FILE_PATH`; keep size small (<10MB) |
| Filesystem | `list_directory`, `read_file`, `write_file`, `create_directory`, `delete_file` | Code inspection, generating examples, updating docs | High (repo write) unless sandboxed | Only in guarded sessions or with sandbox path | Prefer secure config: restrict to `sandbox/`; never expose root in CI |
| Sequential Thinking | `sequential_thinking.plan`, `sequential_thinking.reflect` | External structured chain-of-thought / multi-step reasoning driver | Low (no IO) | Complex tool planning experiments | Pure reasoning, safe to keep enabled; can be fallback when internal planner insufficient |
| GitHub | `list_repos`, `get_pull_request`, `create_issue`, `comment_issue` | Remote repo introspection / issue automation | Medium–High (network + write) | Manual ops requiring cross-repo context | Ensure PAT masked; disable in offline or deterministic test runs |
| Fetch | `fetch` (GET/POST), headers filtering | Retrieve external docs/specs at runtime | Medium (network egress) | Ad-hoc spec alignment, pulling model cards | Add domain allowlist if expanding; avoid in hermetic CI |
| Time | `get_current_time` | Timestamp insertion, timeout heuristics | Low | Always safe | Use for latency annotations in benchmarks |
| Serena Claude | (custom: likely `analyze`, `summarize`, variant tool set) | Experimental secondary LLM / meta-analysis | Unknown (exec + Python) | Opt-in local experimentation | Runs via `uv`; heavy—exclude from CI |
| Python FileOps Enhanced | Rich FS ops (batch read/write, search, refactor) | High-performance bulk code transforms | Very High (broad FS, execution) | Only supervised refactors / migration scripts | Security policy strongly advised; disable by default |
| RAG Redis | `ingest_document`, `query_context`, `list_collections` | Retrieval-augmented generation with Redis backend | Medium (data exfil via embeddings) | When testing retrieval features | Requires Redis running; validate env vars before enabling |

Practical Recommendations:
1. Default minimal set for development automation: RAG Redis + Desktop-Commander + Memory + Sequential Thinking + Time.
2. Add Filesystem (sandboxed) only when generating or modifying files; switch to secure config variant for restricted path.
3. Enable Fetch temporarily for pulling external metadata; stub in tests.
4. Keep high-risk servers (Python FileOps Enhanced, GitHub, RAG Redis) OFF in CI to maintain determinism and avoid secrets leakage.
5. For regression reproductions, persist exact MCP config snapshot (commit `MCP_CONFIG.lock.json`).

Security Patterns:
- Use `MCP_CONFIG_SECURE.json` as template: restrict `allowed_paths`, disallow symlinks, block executables.
- Rate limit heavy file ops: adjust `max_concurrent_calls` in config for stress tests.
- Never allow write + delete + unrestricted path simultaneously in automation.

Testing MCP Integrations:
- Smoke: `tests/mcp/test-mcp-config.ps1` validates format.
- Phase 2 extended: `tests/mcp/test-phase2-mcp-servers.ps1` (skips absent binaries gracefully).
- Add new server: supply minimal start/health criteria + one functional tool invocation; gate with feature flag if external service required.

Failure Triage:
- Tool timeout: increase `tool_timeout_secs` or optimize server startup; check server stderr in `tests/mcp/logs/`.
- Silent tool absence: confirm `auto_register_tools` true and server announces tools on `initialize` JSON-RPC.
- RAG Redis empty results: verify `REDIS_URL` reachable and ingestion path accessible; inspect server logs for embed/cache misses.

Design Guidance When Adding New Tools:
- Make them idempotent & fast; heavy tasks should stream partial results.
- Provide explicit schema (object with typed fields) to reduce hallucinated argument names.
- Return minimal payload; large blobs should be summarized plus a reference handle.

If uncertain which server to leverage for an agent feature, start with Sequential Thinking (planning) + Memory (state) and add Filesystem sandbox only after confirming planned operations.
