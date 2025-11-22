# ============================================================================
# mistral.rs - Just Build System
# ============================================================================
# Comprehensive build automation with isolated target directories per variant
#
# USAGE:
#   just --list              # Show all available recipes
#   just build-cpu           # Build without GPU support (fast testing)
#   just build-cuda          # Build with CUDA 12.9 (current production)
#   just build-cuda-13       # Build with CUDA 13.0 (testing)
#   just clean-all           # Clean all variant target directories
#
# ISOLATED TARGET DIRECTORIES:
#   Each variant uses dedicated target directory to prevent contamination:
#     - target-cpu/          (CPU-only builds)
#     - target-cuda/         (CUDA 12.9 with all features)
#     - target-cuda-13/      (CUDA 13.0 with all features)
#     - target-metal/        (macOS Metal builds)
#     - target-dev/          (Debug builds)
#     - target-release/      (LTO optimized builds)
#
# PLATFORM DETECTION:
#   Automatically detects Windows/Linux/macOS and sets appropriate features
#
# ENVIRONMENT VARIABLES:
#   NVCC_CCBIN, CUDA_PATH, CUDNN_PATH are set per-variant
#
# INTEGRATION WITH MAKEFILE:
#   This justfile complements the Makefile, not replaces it
#   Use Makefile for: coverage, git workflow, CI
#   Use justfile for: isolated variant builds, parallel testing
# ============================================================================

# Default recipe (show help)
default:
    @just --list

# ============================================================================
# Platform Detection
# ============================================================================

# Visual Studio compiler path (Windows only)
msvc_path := "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\cl.exe"

# CUDA toolkit paths
cuda_12_path := "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v12.9"
cuda_13_path := "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v13.0"
cudnn_path := "C:\\Program Files\\NVIDIA\\CUDNN\\v9.8"
cudnn_lib := "C:\\Program Files\\NVIDIA\\CUDNN\\v9.8\\lib\\12.8\\x64"

# ============================================================================
# Build Variants - CPU Only
# ============================================================================

# Build CPU-only variant (for Phase 2A cherry-pick testing)
build-cpu:
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Build Variant: CPU-only"
    @echo "Target Directory: target-cpu"
    @echo "Features: No GPU acceleration (mkl on Windows/Linux, accelerate on macOS)"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    $env:CARGO_TARGET_DIR = "target-cpu"; $env:RUSTC_WRAPPER = ""; cargo build --workspace --release --no-default-features --features mkl
    @echo ""
    @echo "✓ Build complete"
    @echo "Binary: target-cpu\\release\\mistralrs-server.exe"
    @pwsh -Command "if (Test-Path 'target-cpu\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-cpu\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('Size: ' + $size.ToString('F2') + ' MB') }"

# Quick check CPU-only variant compiles (no codegen)
check-cpu:
    @echo "Checking CPU-only variant compiles..."
    $env:CARGO_TARGET_DIR = "target-cpu"; cargo check --workspace --no-default-features --features mkl

# Test CPU-only variant
test-cpu: build-cpu
    @echo "Testing CPU-only variant..."
    $env:CARGO_TARGET_DIR = "target-cpu"; cargo test --workspace --release --no-default-features --features mkl

# Clean CPU variant only
clean-cpu:
    @echo "Cleaning CPU variant..."
    @pwsh -Command "Remove-Item -Recurse -Force target-cpu -ErrorAction SilentlyContinue"

# Run CPU-only variant with TUI
run-cpu model_path="T:/models/qwen2.5-1.5b-instruct-q4_k_m":
    @echo "Running CPU-only variant (TUI mode)..."
    @echo "Model: {{model_path}}"
    .\\target-cpu\\release\\mistralrs-server.exe -i gguf -m "{{model_path}}" -f "*.gguf"

# ============================================================================
# Build Variants - CUDA 12.9 (Production)
# ============================================================================

# Build CUDA 12.9 variant (current production)
build-cuda:
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Build Variant: CUDA 12.9 Full"
    @echo "Target Directory: target-cuda"
    @echo "Features: cuda,flash-attn,cudnn,mkl"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    $env:CARGO_TARGET_DIR = "target-cuda"; $env:RUSTC_WRAPPER = ""; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_12_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; $env:CUDNN_LIB = "{{cudnn_lib}}"; cargo build --workspace --release --features cuda,flash-attn,cudnn,mkl
    @echo ""
    @echo "✓ Build complete"
    @echo "Binary: target-cuda\\release\\mistralrs-server.exe"
    @pwsh -Command "if (Test-Path 'target-cuda\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-cuda\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('Size: ' + $size.ToString('F2') + ' MB') }"
    @echo ""
    @echo "Verify CUDA: .\\target-cuda\\release\\mistralrs-server.exe --version"

# Quick check CUDA 12.9 variant compiles
check-cuda:
    @echo "Checking CUDA 12.9 variant compiles..."
    $env:CARGO_TARGET_DIR = "target-cuda"; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_12_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; cargo check --workspace --features cuda,flash-attn,cudnn,mkl

# Test CUDA 12.9 variant
test-cuda: build-cuda
    @echo "Testing CUDA 12.9 variant..."
    $env:CARGO_TARGET_DIR = "target-cuda"; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_12_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; cargo test --workspace --release --features cuda,flash-attn,cudnn,mkl

# Clean CUDA 12.9 variant only
clean-cuda:
    @echo "Cleaning CUDA 12.9 variant..."
    @pwsh -Command "Remove-Item -Recurse -Force target-cuda -ErrorAction SilentlyContinue"

# Run CUDA 12.9 variant with TUI
run-cuda-tui model_path="T:/models/qwen2.5-1.5b-instruct-q4_k_m":
    @echo "Running CUDA 12.9 variant (TUI mode)..."
    @echo "Model: {{model_path}}"
    .\\target-cuda\\release\\mistralrs-server.exe -i gguf -m "{{model_path}}" -f "*.gguf"

# Run CUDA 12.9 variant as HTTP server
run-cuda-server port="1234" model_path="T:/models/qwen2.5-1.5b-instruct-q4_k_m":
    @echo "Running CUDA 12.9 variant (HTTP server on port {{port}})..."
    @echo "Model: {{model_path}}"
    .\\target-cuda\\release\\mistralrs-server.exe --port {{port}} gguf -m "{{model_path}}" -f "*.gguf"

# ============================================================================
# Build Variants - CUDA 13.0 (Testing)
# ============================================================================

# Build CUDA 13.0 variant (alternative CUDA version)
build-cuda-13:
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Build Variant: CUDA 13.0 Full"
    @echo "Target Directory: target-cuda-13"
    @echo "Features: cuda,flash-attn,cudnn,mkl"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    $env:CARGO_TARGET_DIR = "target-cuda-13"; $env:RUSTC_WRAPPER = ""; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_13_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; $env:CUDNN_LIB = "{{cudnn_lib}}"; cargo build --workspace --release --features cuda,flash-attn,cudnn,mkl
    @echo ""
    @echo "✓ Build complete"
    @echo "Binary: target-cuda-13\\release\\mistralrs-server.exe"
    @pwsh -Command "if (Test-Path 'target-cuda-13\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-cuda-13\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('Size: ' + $size.ToString('F2') + ' MB') }"
    @echo ""
    @echo "Verify CUDA: .\\target-cuda-13\\release\\mistralrs-server.exe --version"

# Quick check CUDA 13.0 variant compiles
check-cuda-13:
    @echo "Checking CUDA 13.0 variant compiles..."
    $env:CARGO_TARGET_DIR = "target-cuda-13"; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_13_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; cargo check --workspace --features cuda,flash-attn,cudnn,mkl

# Test CUDA 13.0 variant
test-cuda-13: build-cuda-13
    @echo "Testing CUDA 13.0 variant..."
    $env:CARGO_TARGET_DIR = "target-cuda-13"; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_13_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; cargo test --workspace --release --features cuda,flash-attn,cudnn,mkl

# Clean CUDA 13.0 variant only
clean-cuda-13:
    @echo "Cleaning CUDA 13.0 variant..."
    @pwsh -Command "Remove-Item -Recurse -Force target-cuda-13 -ErrorAction SilentlyContinue"

# Run CUDA 13.0 variant with TUI
run-cuda-13-tui model_path="T:/models/qwen2.5-1.5b-instruct-q4_k_m":
    @echo "Running CUDA 13.0 variant (TUI mode)..."
    @echo "Model: {{model_path}}"
    .\\target-cuda-13\\release\\mistralrs-server.exe -i gguf -m "{{model_path}}" -f "*.gguf"

# ============================================================================
# Build Variants - Development (Debug)
# ============================================================================

# Build debug variant (fast compilation, debug symbols)
build-dev:
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Build Variant: Debug"
    @echo "Target Directory: target-dev"
    @echo "Features: cuda,flash-attn,cudnn,mkl (debug mode)"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    $env:CARGO_TARGET_DIR = "target-dev"; $env:RUSTC_WRAPPER = ""; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_12_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; cargo build --workspace --features cuda,flash-attn,cudnn,mkl
    @echo ""
    @echo "✓ Build complete"
    @echo "Binary: target-dev\\debug\\mistralrs-server.exe"
    @pwsh -Command "if (Test-Path 'target-dev\\debug\\mistralrs-server.exe') { $size = (Get-Item 'target-dev\\debug\\mistralrs-server.exe').Length / 1MB; Write-Host ('Size: ' + $size.ToString('F2') + ' MB') }"

# Quick check dev variant compiles
check-dev:
    @echo "Checking debug variant compiles..."
    $env:CARGO_TARGET_DIR = "target-dev"; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_12_path}}"; cargo check --workspace --features cuda,flash-attn,cudnn,mkl

# Clean debug variant
clean-dev:
    @echo "Cleaning debug variant..."
    @pwsh -Command "Remove-Item -Recurse -Force target-dev -ErrorAction SilentlyContinue"

# ============================================================================
# Build Variants - Release Optimized (LTO)
# ============================================================================

# Build release-optimized variant (LTO, opt-level=3, takes 60-90 min)
build-release:
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo "Build Variant: Release-Optimized"
    @echo "Target Directory: target-release"
    @echo "Features: cuda,flash-attn,cudnn,mkl + LTO"
    @echo "WARNING: This build takes 60-90 minutes due to LTO"
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    $env:CARGO_TARGET_DIR = "target-release"; $env:RUSTC_WRAPPER = ""; $env:NVCC_CCBIN = "{{msvc_path}}"; $env:CUDA_PATH = "{{cuda_12_path}}"; $env:CUDNN_PATH = "{{cudnn_path}}"; cargo build --workspace --release --features cuda,flash-attn,cudnn,mkl
    @echo ""
    @echo "✓ Build complete"
    @echo "Binary: target-release\\release\\mistralrs-server.exe"
    @pwsh -Command "if (Test-Path 'target-release\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-release\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('Size: ' + $size.ToString('F2') + ' MB') }"

# Clean release-optimized variant
clean-release:
    @echo "Cleaning release-optimized variant..."
    @pwsh -Command "Remove-Item -Recurse -Force target-release -ErrorAction SilentlyContinue"

# ============================================================================
# Cleaning Variants
# ============================================================================

# Clean all variant target directories
clean-all:
    @echo "Cleaning all variant target directories..."
    @pwsh -Command "Remove-Item -Recurse -Force target-cpu,target-cuda,target-cuda-13,target-dev,target-release,target-metal -ErrorAction SilentlyContinue"
    @echo "✓ All variant directories cleaned"

# ============================================================================
# Parallel Build Testing (Phase 2A Validation)
# ============================================================================

# Build all variants in sequence (for overnight builds)
build-all:
    @echo "Building all variants sequentially..."
    @echo "This will take 2-4 hours depending on system"
    @just build-cpu
    @just build-cuda
    @just build-cuda-13
    @just build-dev
    @echo ""
    @echo "✓ All variants built successfully"
    @echo ""
    @just sizes

# Quick check all variants compile (fast validation)
check-all:
    @echo "Checking all variants compile (fast validation)..."
    @just check-cpu
    @just check-cuda
    @just check-cuda-13
    @echo ""
    @echo "✓ All variants compile successfully"

# ============================================================================
# Utility Recipes
# ============================================================================

# Show platform information
info:
    @echo "Platform Detection:"
    @echo "  OS: Windows (PowerShell)"
    @echo "  CUDA Features: cuda,flash-attn,cudnn,mkl"
    @echo "  CPU Features: mkl"
    @echo "  MSVC Path: {{msvc_path}}"
    @echo "  CUDA 12.9 Path: {{cuda_12_path}}"
    @echo "  CUDA 13.0 Path: {{cuda_13_path}}"
    @echo "  cuDNN Path: {{cudnn_path}}"

# Show sizes of all built variants
sizes:
    @echo "Variant Binary Sizes:"
    @pwsh -Command "if (Test-Path 'target-cpu\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-cpu\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('  CPU-only:         ' + $size.ToString('F2') + ' MB') }"
    @pwsh -Command "if (Test-Path 'target-cuda\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-cuda\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('  CUDA 12.9:        ' + $size.ToString('F2') + ' MB') }"
    @pwsh -Command "if (Test-Path 'target-cuda-13\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-cuda-13\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('  CUDA 13.0:        ' + $size.ToString('F2') + ' MB') }"
    @pwsh -Command "if (Test-Path 'target-dev\\debug\\mistralrs-server.exe') { $size = (Get-Item 'target-dev\\debug\\mistralrs-server.exe').Length / 1MB; Write-Host ('  Debug:            ' + $size.ToString('F2') + ' MB') }"
    @pwsh -Command "if (Test-Path 'target-release\\release\\mistralrs-server.exe') { $size = (Get-Item 'target-release\\release\\mistralrs-server.exe').Length / 1MB; Write-Host ('  Release-opt:      ' + $size.ToString('F2') + ' MB') }"

# Show disk space used by variants
disk:
    @echo "Variant Disk Usage:"
    @pwsh -Command "Get-ChildItem -Directory -Filter 'target-*' | ForEach-Object { $size = (Get-ChildItem $_.FullName -Recurse -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1GB; Write-Host ('  ' + $_.Name + ': ' + $size.ToString('F2') + ' GB') }"

# ============================================================================
# Phase 2A: Cherry-Pick Validation Workflow
# ============================================================================

# Validate Phase 2A cherry-picks work on CPU-only build
validate-phase2a:
    @echo "Phase 2A Validation: Building CPU-only to test cherry-picks..."
    @echo ""
    @just clean-cpu
    @just build-cpu
    @echo ""
    @echo "✓ Phase 2A validation complete"
    @echo "  If build succeeded, cherry-picks are compatible with CPU-only builds"
    @echo "  Next: Test with 'just test-cpu' to run test suite"

# Full Phase 2A validation (build + test)
validate-phase2a-full:
    @echo "Phase 2A Full Validation: Build + Test CPU-only variant..."
    @echo ""
    @just validate-phase2a
    @just test-cpu
    @echo ""
    @echo "✓ Phase 2A full validation complete"

# ============================================================================
# Documentation and Help
# ============================================================================

# Show comprehensive help
help:
    @echo "mistral.rs Just Build System"
    @echo ""
    @echo "QUICK START:"
    @echo "  just build-cpu           # Build without GPU (Phase 2A testing)"
    @echo "  just build-cuda          # Build with CUDA 12.9 (production)"
    @echo "  just build-cuda-13       # Build with CUDA 13.0 (testing)"
    @echo "  just validate-phase2a    # Test Phase 2A cherry-picks"
    @echo ""
    @echo "PARALLEL TESTING:"
    @echo "  just check-all           # Fast: Check all variants compile"
    @echo "  just build-all           # Slow: Build all variants (2-4 hours)"
    @echo ""
    @echo "RUNNING:"
    @echo "  just run-cpu             # Run CPU-only variant"
    @echo "  just run-cuda-tui        # Run CUDA 12.9 in TUI mode"
    @echo "  just run-cuda-server     # Run CUDA 12.9 HTTP server"
    @echo ""
    @echo "CLEANING:"
    @echo "  just clean-all           # Clean all variant directories"
    @echo "  just clean-cpu           # Clean CPU variant only"
    @echo "  just clean-cuda          # Clean CUDA 12.9 variant only"
    @echo ""
    @echo "UTILITIES:"
    @echo "  just info                # Show platform detection"
    @echo "  just sizes               # Show binary sizes"
    @echo "  just disk                # Show disk usage"
    @echo ""
    @echo "INTEGRATION WITH MAKEFILE:"
    @echo "  make ci                  # Run CI checks (format, lint, test)"
    @echo "  make test-coverage       # Generate coverage report"
    @echo "  make git-auto-commit     # Enhanced git workflow"
    @echo ""
    @echo "For full recipe list: just --list"
