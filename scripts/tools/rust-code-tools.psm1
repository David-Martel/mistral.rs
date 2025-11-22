<#
.SYNOPSIS
    PowerShell tools for Rust code analysis and manipulation

.DESCRIPTION
    Provides functions for reading file context, checking compilation,
    applying fixes, and validating Rust code syntax.

.NOTES
    Part of Ollama-powered panic-code detection and fixing system
#>

# Load configuration
function Get-OllamaConfig {
    $configPath = Join-Path $PSScriptRoot "..\..\configs\ollama-agent-config.json"
    if (-not (Test-Path $configPath)) {
        throw "Configuration file not found: $configPath"
    }
    return Get-Content $configPath | ConvertFrom-Json
}

<#
.SYNOPSIS
    Get Rust file context with surrounding lines

.PARAMETER FilePath
    Path to the Rust file

.PARAMETER LineNumber
    Line number of interest (1-indexed)

.PARAMETER ContextLines
    Number of lines before and after to include (default: 10)

.OUTPUTS
    HashTable with FilePath, Line, Context, FullFile
#>
function Get-RustFileContext {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$FilePath,

        [Parameter(Mandatory=$true)]
        [int]$LineNumber,

        [Parameter(Mandatory=$false)]
        [int]$ContextLines = 10
    )

    if (-not (Test-Path $FilePath)) {
        throw "File not found: $FilePath"
    }

    $content = Get-Content $FilePath
    $totalLines = $content.Count

    # Calculate context window
    $start = [Math]::Max(0, $LineNumber - $ContextLines - 1)  # 0-indexed
    $end = [Math]::Min($totalLines - 1, $LineNumber + $ContextLines - 1)

    # Get context with line numbers
    $contextLines = @()
    for ($i = $start; $i -le $end; $i++) {
        $lineNum = $i + 1
        $prefix = if ($lineNum -eq $LineNumber) { ">>> " } else { "    " }
        $contextLines += "${prefix}$lineNum: $($content[$i])"
    }

    return @{
        FilePath = $FilePath
        Line = $LineNumber
        TotalLines = $totalLines
        ContextWindow = ($start + 1, $end + 1)
        Context = ($contextLines -join "`n")
        FullFile = ($content -join "`n")
        TargetLine = $content[$LineNumber - 1]
    }
}

<#
.SYNOPSIS
    Check if Rust code compiles successfully

.PARAMETER FilePath
    Path to the Rust file to check (optional, checks workspace if not provided)

.PARAMETER Package
    Specific package to check (optional)

.OUTPUTS
    HashTable with Success (bool), Errors (array), Warnings (array)
#>
function Invoke-CargoCheck {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$false)]
        [string]$FilePath,

        [Parameter(Mandatory=$false)]
        [string]$Package
    )

    $originalLocation = Get-Location
    try {
        # Navigate to project root
        $projectRoot = Split-Path $PSScriptRoot -Parent | Split-Path -Parent
        Set-Location $projectRoot

        # Build cargo command
        $cargoArgs = @("check", "--message-format=json")
        if ($Package) {
            $cargoArgs += @("-p", $Package)
        }

        # Run cargo check
        $output = & cargo $cargoArgs 2>&1 | Where-Object { $_ -match "^\{" }

        $errors = @()
        $warnings = @()

        foreach ($line in $output) {
            try {
                $json = $line | ConvertFrom-Json
                if ($json.reason -eq "compiler-message") {
                    $message = $json.message

                    # Filter by file if specified
                    if ($FilePath) {
                        $fileMatches = $message.spans | Where-Object {
                            $_.file_name -like "*$([System.IO.Path]::GetFileName($FilePath))"
                        }
                        if (-not $fileMatches) {
                            continue
                        }
                    }

                    if ($message.level -eq "error") {
                        $errors += @{
                            Message = $message.message
                            Code = $message.code.code
                            File = ($message.spans | Select-Object -First 1).file_name
                            Line = ($message.spans | Select-Object -First 1).line_start
                            Column = ($message.spans | Select-Object -First 1).column_start
                        }
                    } elseif ($message.level -eq "warning") {
                        $warnings += @{
                            Message = $message.message
                            Code = $message.code.code
                            File = ($message.spans | Select-Object -First 1).file_name
                            Line = ($message.spans | Select-Object -First 1).line_start
                        }
                    }
                }
            } catch {
                # Skip non-JSON lines
                continue
            }
        }

        return @{
            Success = ($errors.Count -eq 0)
            Errors = $errors
            Warnings = $warnings
        }
    }
    finally {
        Set-Location $originalLocation
    }
}

<#
.SYNOPSIS
    Validate Rust syntax using rustfmt

.PARAMETER FilePath
    Path to the Rust file

.OUTPUTS
    HashTable with Valid (bool), Issues (array)
#>
function Test-RustSyntax {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$FilePath
    )

    if (-not (Test-Path $FilePath)) {
        throw "File not found: $FilePath"
    }

    # Run rustfmt in check mode
    $result = & rustfmt --check --edition 2021 $FilePath 2>&1
    $valid = $LASTEXITCODE -eq 0

    $issues = @()
    if (-not $valid) {
        $issues = $result | Where-Object { $_ -match "Diff in" -or $_ -match "error" }
    }

    return @{
        Valid = $valid
        Issues = $issues
    }
}

<#
.SYNOPSIS
    Apply a code fix to a Rust file with automatic backup and rollback

.PARAMETER FilePath
    Path to the Rust file

.PARAMETER LineNumber
    Line number to replace (1-indexed)

.PARAMETER OriginalCode
    Original code (for verification)

.PARAMETER FixedCode
    Fixed code to apply

.PARAMETER VerifyCompilation
    Whether to verify compilation after fix (default: true)

.OUTPUTS
    HashTable with Success (bool), BackupPath (string), Error (string)
#>
function Apply-CodeFix {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$FilePath,

        [Parameter(Mandatory=$true)]
        [int]$LineNumber,

        [Parameter(Mandatory=$true)]
        [string]$OriginalCode,

        [Parameter(Mandatory=$true)]
        [string]$FixedCode,

        [Parameter(Mandatory=$false)]
        [bool]$VerifyCompilation = $true
    )

    if (-not (Test-Path $FilePath)) {
        throw "File not found: $FilePath"
    }

    # Create backup
    $config = Get-OllamaConfig
    $backupPath = "$FilePath$($config.safety.backup_suffix)"
    Copy-Item $FilePath $backupPath -Force

    Write-Verbose "Created backup: $backupPath"

    try {
        # Read file content
        $content = Get-Content $FilePath

        # Verify original code matches
        $actualLine = $content[$LineNumber - 1].Trim()
        $expectedLine = $OriginalCode.Trim()

        if ($actualLine -ne $expectedLine) {
            throw "Line $LineNumber does not match expected content.`nExpected: $expectedLine`nActual: $actualLine"
        }

        # Apply fix
        $content[$LineNumber - 1] = $FixedCode
        Set-Content $FilePath $content -NoNewline

        Write-Verbose "Applied fix to line $LineNumber"

        # Validate syntax
        $syntaxCheck = Test-RustSyntax -FilePath $FilePath
        if (-not $syntaxCheck.Valid) {
            throw "Syntax validation failed after fix: $($syntaxCheck.Issues -join ', ')"
        }

        # Verify compilation if requested
        if ($VerifyCompilation) {
            $compileCheck = Invoke-CargoCheck -FilePath $FilePath
            if (-not $compileCheck.Success) {
                $errorMsg = $compileCheck.Errors | ForEach-Object { "$($_.File):$($_.Line) - $($_.Message)" }
                throw "Compilation failed after fix:`n$($errorMsg -join "`n")"
            }
            Write-Verbose "Compilation check passed"
        }

        # Success - remove backup if configured
        if (-not $config.safety.preserve_backups) {
            Remove-Item $backupPath -Force
            $backupPath = $null
        }

        return @{
            Success = $true
            BackupPath = $backupPath
            Error = $null
        }

    } catch {
        # Rollback on error
        Write-Warning "Fix failed, rolling back: $_"

        if (Test-Path $backupPath) {
            Copy-Item $backupPath $FilePath -Force
            Write-Verbose "Restored from backup"
        }

        return @{
            Success = $false
            BackupPath = $backupPath
            Error = $_.Exception.Message
        }
    }
}

<#
.SYNOPSIS
    Rollback a file from backup

.PARAMETER FilePath
    Path to the file to restore

.PARAMETER BackupPath
    Path to the backup file

.PARAMETER Reason
    Reason for rollback (for logging)
#>
function Invoke-Rollback {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$FilePath,

        [Parameter(Mandatory=$true)]
        [string]$BackupPath,

        [Parameter(Mandatory=$false)]
        [string]$Reason = "Unspecified"
    )

    Write-Warning "⚠️  Rollback triggered: $Reason"

    if (-not (Test-Path $BackupPath)) {
        throw "Backup not found: $BackupPath - Cannot rollback!"
    }

    Copy-Item $BackupPath $FilePath -Force
    Write-Host "✓ Restored from backup: $FilePath" -ForegroundColor Green

    Remove-Item $BackupPath -Force
    Write-Verbose "Removed backup file"
}

<#
.SYNOPSIS
    Get type information for a Rust expression

.PARAMETER FilePath
    Path to the Rust file

.PARAMETER LineNumber
    Line number containing the expression

.PARAMETER ColumnNumber
    Column number of the expression

.OUTPUTS
    String with type information (if available via rust-analyzer)
#>
function Get-RustTypeInfo {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$FilePath,

        [Parameter(Mandatory=$true)]
        [int]$LineNumber,

        [Parameter(Mandatory=$false)]
        [int]$ColumnNumber = 0
    )

    # This would require rust-analyzer LSP integration
    # For now, return placeholder
    Write-Verbose "Type information requires rust-analyzer integration (future enhancement)"
    return "Type information not available (requires LSP)"
}

# Export module functions
Export-ModuleMember -Function @(
    'Get-OllamaConfig',
    'Get-RustFileContext',
    'Invoke-CargoCheck',
    'Test-RustSyntax',
    'Apply-CodeFix',
    'Invoke-Rollback',
    'Get-RustTypeInfo'
)
