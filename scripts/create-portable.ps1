#!/usr/bin/env pwsh
#Requires -Version 7.0

<#
.SYNOPSIS
    Create portable ZIP distribution of mistral.rs

.DESCRIPTION
    Collects release binaries and runtime dependencies into a versioned ZIP archive.
    Suitable for distribution to users who prefer not to install via MSI.

.PARAMETER Version
    Version string for the archive (default: reads from Cargo.toml)

.PARAMETER OutputDir
    Directory for output ZIP file (default: ./dist)

.PARAMETER IncludeModels
    Include sample model files in portable archive

.PARAMETER Features
    Build features to use (default: cuda,flash-attn,cudnn,mkl)

.EXAMPLE
    .\scripts\create-portable.ps1
    Creates portable ZIP with default settings

.EXAMPLE
    .\scripts\create-portable.ps1 -Version "0.6.0" -OutputDir "C:\releases"
    Creates portable ZIP with custom version and output location

.EXAMPLE
    .\scripts\create-portable.ps1 -IncludeModels
    Creates portable ZIP including sample model files

.NOTES
    Author: mistral.rs team
    Requires: PowerShell 7.0+, Rust toolchain
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [string]$Version = "",

    [Parameter(Mandatory = $false)]
    [string]$OutputDir = "dist",

    [Parameter(Mandatory = $false)]
    [switch]$IncludeModels = $false,

    [Parameter(Mandatory = $false)]
    [string]$Features = "cuda,flash-attn,cudnn,mkl"
)

# Error handling
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# Script configuration
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ReleaseDir = Join-Path $ProjectRoot "target\release"
$TempDir = Join-Path $env:TEMP "mistralrs-portable-$(Get-Random)"

# Colors for output
function Write-ColorOutput {
    param([string]$Message, [string]$Color = "White")
    Write-Host $Message -ForegroundColor $Color
}

function Write-Success { param([string]$Message) Write-ColorOutput "✓ $Message" "Green" }
function Write-Info { param([string]$Message) Write-ColorOutput "ℹ $Message" "Cyan" }
function Write-Warning { param([string]$Message) Write-ColorOutput "⚠ $Message" "Yellow" }
function Write-Error { param([string]$Message) Write-ColorOutput "✗ $Message" "Red" }
function Write-Step { param([string]$Message) Write-ColorOutput "→ $Message" "Magenta" }

# Extract version from Cargo.toml if not provided
function Get-ProjectVersion {
    if ($Version) {
        return $Version
    }

    $CargoToml = Join-Path $ProjectRoot "Cargo.toml"
    if (-not (Test-Path $CargoToml)) {
        Write-Error "Cargo.toml not found at: $CargoToml"
        exit 1
    }

    $VersionLine = Get-Content $CargoToml | Select-String -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    if ($VersionLine) {
        return $VersionLine.Matches.Groups[1].Value
    }

    Write-Warning "Could not extract version from Cargo.toml, using default: 0.0.0"
    return "0.0.0"
}

# Validate prerequisites
function Test-Prerequisites {
    Write-Step "Validating prerequisites..."

    # Check cargo
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "Cargo not found. Install Rust toolchain from https://rustup.rs"
        exit 1
    }

    # Check release binaries exist
    if (-not (Test-Path $ReleaseDir)) {
        Write-Error "Release directory not found: $ReleaseDir"
        Write-Info "Run 'make build' or 'cargo build --release' first"
        exit 1
    }

    Write-Success "Prerequisites validated"
}

# Build release binaries
function Invoke-ReleaseBuild {
    Write-Step "Building release binaries with features: $Features..."

    Push-Location $ProjectRoot
    try {
        $BuildArgs = @(
            "build",
            "--release",
            "--features", $Features,
            "--workspace"
        )

        Write-Info "Running: cargo $($BuildArgs -join ' ')"
        & cargo @BuildArgs

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Build failed with exit code: $LASTEXITCODE"
            exit 1
        }

        Write-Success "Build completed successfully"
    }
    finally {
        Pop-Location
    }
}

# Collect binaries
function Get-BinaryList {
    @(
        "mistralrs-server.exe",
        "mistralrs-tui.exe",
        "mistralrs-bench.exe",
        "mistralrs-web-chat.exe",
        "mcp-server.exe"
    )
}

# Collect runtime dependencies
function Get-RuntimeDependencies {
    $Dependencies = @()

    # CUDA DLLs (if available)
    $CudaPath = $env:CUDA_PATH
    if ($CudaPath -and (Test-Path $CudaPath)) {
        $CudaBin = Join-Path $CudaPath "bin"

        $CudaDlls = @(
            "cudart64_*.dll",
            "cublas64_*.dll",
            "cublasLt64_*.dll"
        )

        foreach ($Pattern in $CudaDlls) {
            $Files = Get-ChildItem -Path $CudaBin -Filter $Pattern -ErrorAction SilentlyContinue
            $Dependencies += $Files
        }
    }

    # cuDNN DLLs (if available)
    $CudnnPath = $env:CUDNN_PATH
    if ($CudnnPath -and (Test-Path $CudnnPath)) {
        $CudnnBin = Join-Path $CudnnPath "bin"

        $CudnnDlls = @(
            "cudnn64_*.dll",
            "cudnn_*_infer64_*.dll"
        )

        foreach ($Pattern in $CudnnDlls) {
            $Files = Get-ChildItem -Path $CudnnBin -Filter $Pattern -ErrorAction SilentlyContinue
            $Dependencies += $Files
        }
    }

    return $Dependencies
}

# Create portable structure
function New-PortableStructure {
    param([string]$TempDir, [string]$Version)

    Write-Step "Creating portable structure..."

    # Create directories
    $PortableRoot = Join-Path $TempDir "mistralrs-$Version"
    $BinDir = Join-Path $PortableRoot "bin"
    $DocsDir = Join-Path $PortableRoot "docs"
    $ModelsDir = Join-Path $PortableRoot "models"

    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    New-Item -ItemType Directory -Path $DocsDir -Force | Out-Null

    if ($IncludeModels) {
        New-Item -ItemType Directory -Path $ModelsDir -Force | Out-Null
    }

    Write-Success "Directory structure created"

    return @{
        Root   = $PortableRoot
        Bin    = $BinDir
        Docs   = $DocsDir
        Models = $ModelsDir
    }
}

# Copy binaries
function Copy-Binaries {
    param([string]$BinDir)

    Write-Step "Copying binaries..."

    $Binaries = Get-BinaryList
    $CopiedCount = 0

    foreach ($Binary in $Binaries) {
        $SourcePath = Join-Path $ReleaseDir $Binary
        $DestPath = Join-Path $BinDir $Binary

        if (Test-Path $SourcePath) {
            Copy-Item -Path $SourcePath -Destination $DestPath -Force
            Write-Info "  Copied: $Binary"
            $CopiedCount++
        }
        else {
            Write-Warning "  Skipped (not found): $Binary"
        }
    }

    Write-Success "Copied $CopiedCount binaries"
}

# Copy runtime dependencies
function Copy-Dependencies {
    param([string]$BinDir)

    Write-Step "Copying runtime dependencies..."

    $Dependencies = Get-RuntimeDependencies
    $CopiedCount = 0

    foreach ($Dependency in $Dependencies) {
        $DestPath = Join-Path $BinDir $Dependency.Name
        Copy-Item -Path $Dependency.FullName -Destination $DestPath -Force
        Write-Info "  Copied: $($Dependency.Name)"
        $CopiedCount++
    }

    if ($CopiedCount -eq 0) {
        Write-Warning "No CUDA/cuDNN dependencies found (CPU-only mode)"
    }
    else {
        Write-Success "Copied $CopiedCount runtime dependencies"
    }
}

# Copy documentation
function Copy-Documentation {
    param([string]$DocsDir)

    Write-Step "Copying documentation..."

    $DocFiles = @(
        "README.md",
        "LICENSE",
        "CLAUDE.md"
    )

    $CopiedCount = 0

    foreach ($DocFile in $DocFiles) {
        $SourcePath = Join-Path $ProjectRoot $DocFile
        if (Test-Path $SourcePath) {
            $DestPath = Join-Path $DocsDir $DocFile
            Copy-Item -Path $SourcePath -Destination $DestPath -Force
            Write-Info "  Copied: $DocFile"
            $CopiedCount++
        }
    }

    # Copy docs directory if exists
    $DocsSource = Join-Path $ProjectRoot "docs"
    if (Test-Path $DocsSource) {
        Copy-Item -Path "$DocsSource\*" -Destination $DocsDir -Recurse -Force
        Write-Info "  Copied: docs directory"
    }

    Write-Success "Copied documentation"
}

# Create README for portable version
function New-PortableReadme {
    param([string]$RootDir, [string]$Version)

    $ReadmePath = Join-Path $RootDir "README-PORTABLE.txt"

    $Content = @"
mistral.rs Portable Distribution v$Version
=========================================

This is a portable (no installation required) distribution of mistral.rs.

Quick Start:
-----------
1. Extract this archive to any location
2. Add the 'bin' directory to your PATH, or run binaries directly
3. Run 'mistralrs-tui.exe' for interactive mode
4. Run 'mistralrs-server.exe --help' for server options

Included Binaries:
-----------------
- mistralrs-server.exe  : HTTP server with OpenAI-compatible API
- mistralrs-tui.exe     : Interactive terminal interface
- mistralrs-bench.exe   : Benchmarking tool
- mistralrs-web-chat.exe: Web-based chat interface
- mcp-server.exe        : Model Context Protocol server

GPU Acceleration:
----------------
This build includes CUDA support. Requirements:
- NVIDIA GPU with Compute Capability 6.0+ (Pascal or newer)
- CUDA Toolkit 12.1 or newer
- cuDNN 9.0 or newer
- 16GB+ VRAM recommended for larger models

If CUDA is not available, mistral.rs will automatically fall back to CPU mode.

Documentation:
-------------
See the 'docs' directory for detailed documentation, or visit:
https://ericlbuehler.github.io/mistral.rs/

Examples:
--------
# Interactive TUI
mistralrs-tui.exe

# HTTP server on port 1234
mistralrs-server.exe --port 1234 plain -m meta-llama/Llama-3.2-3B-Instruct

# With local GGUF model
mistralrs-server.exe -i gguf -m /path/to/model -f model.gguf

Support:
-------
- GitHub: https://github.com/EricLBuehler/mistral.rs
- Documentation: https://ericlbuehler.github.io/mistral.rs/
- Discord: https://discord.gg/SZrecqK8qw

License:
-------
MIT License - See LICENSE file for details

"@

    Set-Content -Path $ReadmePath -Value $Content -Encoding UTF8
    Write-Success "Created portable README"
}

# Create ZIP archive
function New-ZipArchive {
    param([string]$TempDir, [string]$OutputDir, [string]$Version)

    Write-Step "Creating ZIP archive..."

    # Ensure output directory exists
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

    # Archive name
    $Platform = "windows-x64"
    if ($Features -match "cuda") {
        $Platform += "-cuda"
    }

    $ArchiveName = "mistralrs-$Version-$Platform-portable.zip"
    $ArchivePath = Join-Path $OutputDir $ArchiveName

    # Remove existing archive if present
    if (Test-Path $ArchivePath) {
        Remove-Item -Path $ArchivePath -Force
        Write-Info "Removed existing archive"
    }

    # Create archive
    $PortableRoot = Join-Path $TempDir "mistralrs-$Version"

    Push-Location $TempDir
    try {
        Compress-Archive -Path "mistralrs-$Version" -DestinationPath $ArchivePath -CompressionLevel Optimal

        if (Test-Path $ArchivePath) {
            $ArchiveSize = (Get-Item $ArchivePath).Length / 1MB
            Write-Success "Archive created: $ArchiveName ($([math]::Round($ArchiveSize, 2)) MB)"
            Write-Info "Location: $ArchivePath"
        }
        else {
            Write-Error "Failed to create archive"
            exit 1
        }
    }
    finally {
        Pop-Location
    }

    return $ArchivePath
}

# Cleanup temporary files
function Remove-TempFiles {
    param([string]$TempDir)

    Write-Step "Cleaning up temporary files..."

    if (Test-Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force
        Write-Success "Cleanup complete"
    }
}

# Generate checksum
function New-Checksum {
    param([string]$ArchivePath)

    Write-Step "Generating SHA256 checksum..."

    $Hash = Get-FileHash -Path $ArchivePath -Algorithm SHA256
    $ChecksumPath = "$ArchivePath.sha256"

    $ChecksumContent = "$($Hash.Hash)  $(Split-Path -Leaf $ArchivePath)"
    Set-Content -Path $ChecksumPath -Value $ChecksumContent -Encoding ASCII

    Write-Success "Checksum: $($Hash.Hash)"
    Write-Info "Saved to: $ChecksumPath"
}

# Main execution
function Main {
    Write-ColorOutput "`n=== mistral.rs Portable Distribution Builder ===" "Cyan"
    Write-ColorOutput "================================================`n" "Cyan"

    # Get version
    $ProjectVersion = Get-ProjectVersion
    Write-Info "Version: $ProjectVersion"
    Write-Info "Features: $Features"
    Write-Info "Output: $OutputDir"
    Write-Info ""

    # Validate
    Test-Prerequisites

    # Build
    Invoke-ReleaseBuild

    # Create structure
    $Dirs = New-PortableStructure -TempDir $TempDir -Version $ProjectVersion

    # Copy files
    Copy-Binaries -BinDir $Dirs.Bin
    Copy-Dependencies -BinDir $Dirs.Bin
    Copy-Documentation -DocsDir $Dirs.Docs
    New-PortableReadme -RootDir $Dirs.Root -Version $ProjectVersion

    # Create archive
    $ArchivePath = New-ZipArchive -TempDir $TempDir -OutputDir $OutputDir -Version $ProjectVersion

    # Generate checksum
    New-Checksum -ArchivePath $ArchivePath

    # Cleanup
    Remove-TempFiles -TempDir $TempDir

    Write-ColorOutput "`n=== Build Complete ===" "Green"
    Write-Success "Portable distribution ready: $ArchivePath"
}

# Run
Main
