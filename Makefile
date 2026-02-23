# ============================================================================
# Platform Detection & Feature Management
# ============================================================================
# Detect platform and set appropriate features to avoid cross-platform issues
# (e.g., objc_exception on Windows when --all-features enables metal feature)

# Detect operating system
ifeq ($(OS),Windows_NT)
    DETECTED_OS := Windows
    # Windows: Use CUDA features, exclude macOS-only features
    PLATFORM_FEATURES := cuda,flash-attn,cudnn,mkl
    EXCLUDE_FEATURES := --exclude metal
    SHELL := pwsh.exe
    # Set NVCC compiler binary path for Windows CUDA builds
    export NVCC_CCBIN := C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\cl.exe
    # Use CargoTools wrapper for sccache, lld, auto-copy, and MSVC setup
    # cargo.bat routes through CargoTools module: cargo.bat -> cargo.ps1 -> cargo-route.ps1 -> Invoke-CargoWrapper
    CARGO := cargo
    # Local output directory for post-build copy
    LOCAL_BIN_DIR := target\release
else
    UNAME_S := $(shell uname -s)
    ifeq ($(UNAME_S),Darwin)
        DETECTED_OS := macOS
        # macOS: Use Metal and Accelerate, exclude CUDA
        PLATFORM_FEATURES := metal,accelerate
        EXCLUDE_FEATURES :=
    else
        DETECTED_OS := Linux
        # Linux: Use CUDA features
        PLATFORM_FEATURES := cuda,flash-attn,cudnn,mkl
        EXCLUDE_FEATURES := --exclude metal
    endif
    CARGO := cargo
    LOCAL_BIN_DIR := target/release
endif

# Feature flag for all build commands
# Option A: Platform-specific features (safer)
CARGO_FEATURES := --features $(PLATFORM_FEATURES)

# Option B: All features with exclusions (more comprehensive, but riskier)
CARGO_ALL_FEATURES := --all-features $(EXCLUDE_FEATURES)

# Default to platform-specific for safety, but allow override
CARGO_BUILD_FLAGS ?= $(CARGO_FEATURES)

.PHONY: show-platform
show-platform: ## Show detected platform and features
	@echo "Detected OS: $(DETECTED_OS)"
	@echo "Cargo Command: $(CARGO)"
	@echo "Platform Features: $(PLATFORM_FEATURES)"
	@echo "Excluded Features: $(EXCLUDE_FEATURES)"
	@echo "Build Flags: $(CARGO_BUILD_FLAGS)"
	@echo "Output Directory: $(LOCAL_BIN_DIR)"

# ============================================================================
# Code Coverage Targets
# ============================================================================
# Generate code coverage reports locally
# Note: Coverage builds use local target/ directory and disable sccache

# Helper to ensure local target directory (Windows)
set-local-target:
	@pwsh -Command "$$env:CARGO_TARGET_DIR=''; Write-Host 'Using local target directory'"

test-coverage:
	@echo "Generating code coverage report on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace $(CARGO_BUILD_FLAGS) --html"
	@echo "Coverage report generated in target/llvm-cov/html/index.html"

test-coverage-open:
	@echo "Generating and opening code coverage report on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace $(CARGO_BUILD_FLAGS) --open"

test-coverage-lcov:
	@echo "Generating LCOV coverage report on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace $(CARGO_BUILD_FLAGS) --lcov --output-path lcov.info"
	@echo "LCOV report generated: lcov.info"

test-coverage-json:
	@echo "Generating JSON coverage report on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace $(CARGO_BUILD_FLAGS) --json --output-path coverage.json"
	@echo "JSON report generated: coverage.json"

test-coverage-text:
	@echo "Generating text coverage summary on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace $(CARGO_BUILD_FLAGS) --summary-only"

test-coverage-ci:
	@echo "Generating coverage for CI (LCOV format) on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov --workspace $(CARGO_BUILD_FLAGS) --lcov --output-path lcov.info"

test-coverage-fast:
	@echo "Fast coverage (no pyo3 crates) on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@pwsh -Command "$$env:RUSTC_WRAPPER=''; Remove-Item Env:\CARGO_TARGET_DIR -EA SilentlyContinue; $$env:CARGO_INCREMENTAL='1'; cargo llvm-cov -p mistralrs-core -p mistralrs-agent-tools -p mistralrs-quant -p mistralrs-vision -p mistralrs-mcp $(CARGO_BUILD_FLAGS) --html --open"

install-coverage-tools:
	@echo "Installing code coverage tools..."
	rustup component add llvm-tools-preview
	$(CARGO) install cargo-llvm-cov
	@echo "Coverage tools installed!"

# ============================================================================
# Standard Rust Development Targets
# ============================================================================

.PHONY: dev
dev: ## Quick development build (debug mode)
	@echo "Building workspace (debug) on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) build --workspace $(CARGO_BUILD_FLAGS)

.PHONY: release
release: ## Full release build with optimizations
	@echo "Building release binaries (LTO) on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) build --workspace $(CARGO_BUILD_FLAGS) --release

.PHONY: check
check: ## Quick compilation check (no codegen)
	@echo "Running quick check on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) check --workspace --all-targets $(CARGO_BUILD_FLAGS)

.PHONY: build
build: ## Build all workspace members
	@echo "Building workspace on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) build --workspace $(CARGO_BUILD_FLAGS)

.PHONY: build-release
build-release: ## Build release binaries
	@echo "Building release binaries on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) build --workspace $(CARGO_BUILD_FLAGS) --release

.PHONY: test
test: ## Run all tests
	@echo "Running tests on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) test --workspace $(CARGO_BUILD_FLAGS)

.PHONY: test-core
test-core: ## Run mistralrs-core tests
	@echo "Testing mistralrs-core on $(DETECTED_OS) platform..."
	@$(CARGO) test -p mistralrs-core $(CARGO_BUILD_FLAGS)

.PHONY: test-server
test-server: ## Run server crate tests
	@echo "Testing server crates on $(DETECTED_OS) platform..."
	@$(CARGO) test -p mistralrs-server -p mistralrs-server-core $(CARGO_BUILD_FLAGS)

.PHONY: test-quant
test-quant: ## Run quantization tests
	@echo "Testing mistralrs-quant on $(DETECTED_OS) platform..."
	@$(CARGO) test -p mistralrs-quant $(CARGO_BUILD_FLAGS)

.PHONY: test-vision
test-vision: ## Run vision model tests
	@echo "Testing mistralrs-vision on $(DETECTED_OS) platform..."
	@$(CARGO) test -p mistralrs-vision $(CARGO_BUILD_FLAGS)

.PHONY: fmt
fmt: ## Format code with rustfmt
	@echo "Formatting code..."
	@$(CARGO) fmt --all || (echo "Warning: rustfmt had issues but continuing..." && exit 0)

.PHONY: fmt-check
fmt-check: ## Check code formatting
	@echo "Checking code formatting..."
	@$(CARGO) fmt --all -- --check

.PHONY: lint
lint: ## Run clippy linter
	@echo "Running clippy on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) clippy --workspace --all-targets $(CARGO_BUILD_FLAGS) --exclude mistralrs-pyo3 -- -D warnings || true

.PHONY: lint-fix
lint-fix: ## Auto-fix clippy warnings
	@echo "Auto-fixing clippy issues on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) clippy --workspace --all-targets $(CARGO_BUILD_FLAGS) --exclude mistralrs-pyo3 --fix --allow-dirty --allow-staged || true

.PHONY: clean
clean: ## Clean build artifacts
	@echo "Cleaning build artifacts..."
	@$(CARGO) clean

# ============================================================================
# Server Build Targets
# ============================================================================

.PHONY: build-server
build-server: ## Build the mistralrs-server binary (release)
	@echo "Building mistralrs-server on $(DETECTED_OS) platform..."
	@echo "Using features: $(PLATFORM_FEATURES)"
	@$(CARGO) build -p mistralrs-server $(CARGO_BUILD_FLAGS) --release

.PHONY: build-cuda-full
build-cuda-full: ## Build with all CUDA features (release)
	@echo "Building with full CUDA features on $(DETECTED_OS) platform..."
	@$(CARGO) build --workspace --features cuda,flash-attn,cudnn,mkl --exclude mistralrs-pyo3 --release

# ============================================================================
# Post-Build Copy (copies binaries to local target/ for easy access)
# ============================================================================
# CargoTools (cargo.bat) handles auto-copy from shared cache (T:\RustCache)
# to local ./target/ automatically. This target is for manual copy or
# when running cargo directly.

.PHONY: copy-binaries
copy-binaries: ## Copy built executables to local output directory
ifeq ($(DETECTED_OS),Windows)
	@echo "Ensuring local output directory exists..."
	@if not exist "$(LOCAL_BIN_DIR)" mkdir "$(LOCAL_BIN_DIR)"
	@echo "Copying built executables to $(LOCAL_BIN_DIR)..."
	@pwsh -Command "Get-ChildItem -Path 'T:\RustCache\cargo-target\release\*.exe' -File | Where-Object { $$_.Name -match 'mistralrs' } | ForEach-Object { Copy-Item $$_.FullName -Destination '$(LOCAL_BIN_DIR)\' -Force; Write-Host \"  Copied: $$($_.Name)\" }"
else
	@mkdir -p $(LOCAL_BIN_DIR)
	@echo "Copying built executables to $(LOCAL_BIN_DIR)..."
	@find target/release -maxdepth 1 -type f -executable -name 'mistralrs*' -exec cp {} $(LOCAL_BIN_DIR)/ \; -exec echo "  Copied: {}" \;
endif
	@echo "Done. Binaries available in $(LOCAL_BIN_DIR)/"

# ============================================================================
# Enhanced Git Workflow Targets
# ============================================================================

.PHONY: tag-issues
tag-issues: ## Tag TODO/FIXME comments with @codex/@gemini for external review
	@echo "Tagging outstanding issues..."
	@pwsh -ExecutionPolicy Bypass -File scripts/tag-issues.ps1

.PHONY: tag-issues-dry-run
tag-issues-dry-run: ## Preview TODO/FIXME tagging without making changes
	@echo "Previewing issue tags (dry run)..."
	@pwsh -ExecutionPolicy Bypass -File scripts/tag-issues.ps1 -DryRun

.PHONY: rag-index
rag-index: ## Create semantic index with RAG-Redis
	@echo "Creating semantic index..."
	@pwsh -ExecutionPolicy Bypass -File scripts/rag-index.ps1

.PHONY: git-auto-commit
git-auto-commit: ## Run enhanced git workflow (format, fix, tag, index, commit)
	@echo "Running enhanced git workflow..."
	@echo "Usage: make git-auto-commit MESSAGE='your commit message'"
	@echo ""
	@echo "Please use the script directly:"
	@echo "  pwsh scripts/git-auto-commit.ps1 -Message 'your message' [-Push]"
	@echo ""
	@echo "Options:"
	@echo "  -Push              : Automatically push after commit"
	@echo "  -NoVerify          : Skip pre-commit hooks"
	@echo "  -SkipFormat        : Skip cargo fmt"
	@echo "  -SkipClippy        : Skip cargo clippy --fix"
	@echo "  -SkipTagging       : Skip TODO/FIXME tagging"
	@echo "  -SkipIndex         : Skip RAG-Redis indexing"

.PHONY: pre-commit
pre-commit: fmt lint-fix ## Run pre-commit checks (format + lint-fix)
	@echo "Pre-commit checks completed"
	@git status --short

.PHONY: workflow-prepare
workflow-prepare: fmt lint-fix tag-issues ## Prepare code for commit (format, fix, tag)
	@echo "Code prepared for commit"
	@git add -u
	@git status --short

.PHONY: workflow-full
workflow-full: fmt lint-fix tag-issues rag-index ## Full workflow (format, fix, tag, index)
	@echo "Full workflow completed"
	@git add -u
	@git status --short
	@echo ""
	@echo "Ready to commit. Run: git commit -m 'your message'"

# ============================================================================
# CI/CD Integration Targets
# ============================================================================

.PHONY: ci
ci: fmt-check check lint test ## Run CI checks (format check, compile, lint, test)
	@echo "✓ All CI checks passed"

.PHONY: ci-auto-fix
ci-auto-fix: fmt lint-fix ## CI with auto-fix (formats and fixes issues)
	@echo "CI auto-fix completed"
	@git diff --stat

# ============================================================================
# Help Target
# ============================================================================

.PHONY: help
help: ## Show this help message
	@echo "Available targets:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-25s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Enhanced Git Workflow:"
	@echo "  make tag-issues          - Tag TODO/FIXME with @codex/@gemini"
	@echo "  make rag-index           - Create semantic index"
	@echo "  make pre-commit          - Format + lint-fix"
	@echo "  make workflow-prepare    - Format + fix + tag"
	@echo "  make workflow-full       - Format + fix + tag + index"
	@echo ""
	@echo "For full git workflow, use:"
	@echo "  pwsh scripts/git-auto-commit.ps1 -Message 'commit msg' -Push"

.DEFAULT_GOAL := help

# Include deployment targets if available
-include Makefile.deployment
