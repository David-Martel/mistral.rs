<#
.SYNOPSIS
    Ollama HTTP API client for PowerShell

.DESCRIPTION
    Provides functions to interact with Ollama's HTTP API for LLM inference

.NOTES
    Part of Ollama-powered panic-code detection and fixing system
    API Reference: https://github.com/ollama/ollama/blob/main/docs/api.md
#>

# Import configuration
$script:ConfigPath = Join-Path $PSScriptRoot "..\..\configs\ollama-agent-config.json"

function Get-OllamaConfig {
    if (-not (Test-Path $script:ConfigPath)) {
        throw "Configuration file not found: $script:ConfigPath"
    }
    return Get-Content $script:ConfigPath | ConvertFrom-Json
}

<#
.SYNOPSIS
    Test Ollama service connectivity

.OUTPUTS
    Boolean indicating if Ollama is running and accessible
#>
function Test-OllamaService {
    [CmdletBinding()]
    param()

    $config = Get-OllamaConfig
    $endpoint = $config.ollama.endpoint

    try {
        $response = Invoke-RestMethod -Uri "$endpoint/api/tags" -Method Get -TimeoutSec 5
        Write-Verbose "Ollama service is running at $endpoint"
        return $true
    } catch {
        Write-Warning "Ollama service not accessible at $endpoint : $_"
        return $false
    }
}

<#
.SYNOPSIS
    List available Ollama models

.OUTPUTS
    Array of model objects
#>
function Get-OllamaModels {
    [CmdletBinding()]
    param()

    $config = Get-OllamaConfig
    $endpoint = $config.ollama.endpoint

    try {
        $response = Invoke-RestMethod -Uri "$endpoint/api/tags" -Method Get
        return $response.models
    } catch {
        throw "Failed to list Ollama models: $_"
    }
}

<#
.SYNOPSIS
    Invoke Ollama model for text generation

.PARAMETER Model
    Model name to use (e.g., "qwen2.5-coder:7b")

.PARAMETER Prompt
    Text prompt for the model

.PARAMETER System
    System message to set context (optional)

.PARAMETER Temperature
    Sampling temperature (0.0 - 1.0)

.PARAMETER MaxTokens
    Maximum tokens to generate

.PARAMETER Stream
    Whether to stream the response (default: false)

.OUTPUTS
    String with generated text or error message
#>
function Invoke-OllamaGenerate {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$false)]
        [string]$Model,

        [Parameter(Mandatory=$true)]
        [string]$Prompt,

        [Parameter(Mandatory=$false)]
        [string]$System,

        [Parameter(Mandatory=$false)]
        [double]$Temperature,

        [Parameter(Mandatory=$false)]
        [int]$MaxTokens,

        [Parameter(Mandatory=$false)]
        [bool]$Stream = $false
    )

    $config = Get-OllamaConfig
    $endpoint = $config.ollama.endpoint

    # Use config defaults if not specified
    if (-not $Model) {
        $Model = $config.ollama.model
    }
    if (-not $Temperature) {
        $Temperature = $config.ollama.temperature
    }
    if (-not $MaxTokens) {
        $MaxTokens = $config.ollama.max_tokens
    }

    # Build request body
    $body = @{
        model = $Model
        prompt = $Prompt
        stream = $Stream
        options = @{
            temperature = $Temperature
            top_p = $config.ollama.top_p
            num_predict = $MaxTokens
        }
    }

    if ($System) {
        $body.system = $System
    }

    $jsonBody = $body | ConvertTo-Json -Depth 10

    try {
        Write-Verbose "Calling Ollama with model: $Model"
        Write-Verbose "Prompt length: $($Prompt.Length) characters"

        $response = Invoke-RestMethod `
            -Uri "$endpoint/api/generate" `
            -Method Post `
            -Body $jsonBody `
            -ContentType "application/json" `
            -TimeoutSec $config.ollama.timeout_secs

        if ($Stream) {
            # Handle streaming response
            return $response
        } else {
            # Return complete response
            return $response.response
        }

    } catch {
        Write-Error "Ollama API call failed: $_"
        throw
    }
}

<#
.SYNOPSIS
    Invoke Ollama model with retry logic

.PARAMETER Model
    Model name to use

.PARAMETER Prompt
    Text prompt for the model

.PARAMETER System
    System message to set context (optional)

.PARAMETER MaxRetries
    Maximum number of retry attempts (uses config default if not specified)

.OUTPUTS
    String with generated text
#>
function Invoke-OllamaWithRetry {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$false)]
        [string]$Model,

        [Parameter(Mandatory=$true)]
        [string]$Prompt,

        [Parameter(Mandatory=$false)]
        [string]$System,

        [Parameter(Mandatory=$false)]
        [int]$MaxRetries
    )

    $config = Get-OllamaConfig

    if (-not $MaxRetries) {
        $MaxRetries = $config.ollama.retry_attempts
    }

    $attempt = 0
    $lastError = $null

    while ($attempt -lt $MaxRetries) {
        $attempt++

        try {
            Write-Verbose "Attempt $attempt of $MaxRetries"

            $response = Invoke-OllamaGenerate `
                -Model $Model `
                -Prompt $Prompt `
                -System $System

            return $response

        } catch {
            $lastError = $_
            Write-Warning "Attempt $attempt failed: $_"

            if ($attempt -lt $MaxRetries) {
                $delay = $config.ollama.retry_delay_secs
                Write-Verbose "Retrying in $delay seconds..."
                Start-Sleep -Seconds $delay
            }
        }
    }

    # All retries failed
    throw "Ollama API call failed after $MaxRetries attempts. Last error: $lastError"
}

<#
.SYNOPSIS
    Generate a code fix using Ollama

.PARAMETER FilePath
    Path to the file containing the issue

.PARAMETER LineNumber
    Line number with the issue

.PARAMETER Pattern
    Panic pattern detected (e.g., "unwrap", "panic")

.PARAMETER Context
    Surrounding code context

.PARAMETER Model
    Model to use (optional, uses config default)

.OUTPUTS
    HashTable with FixedCode, Explanation
#>
function Invoke-OllamaCodeFix {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$FilePath,

        [Parameter(Mandatory=$true)]
        [int]$LineNumber,

        [Parameter(Mandatory=$true)]
        [string]$Pattern,

        [Parameter(Mandatory=$true)]
        [string]$Context,

        [Parameter(Mandatory=$false)]
        [string]$Model
    )

    $config = Get-OllamaConfig

    # Load appropriate prompt template
    $promptTemplatePath = Join-Path $PSScriptRoot "..\..\prompts"

    $promptFile = switch ($Pattern) {
        "unwrap" { "fix-unwrap.txt" }
        "panic" { "fix-panic.txt" }
        "expect" { "enhance-expect.txt" }
        "unreachable" { "fix-unreachable.txt" }
        default { "fix-unwrap.txt" }
    }

    $promptPath = Join-Path $promptTemplatePath $promptFile

    # Build system message
    $systemMessage = @"
You are a Rust code expert specializing in safe error handling.
Your task is to fix panic-inducing code patterns by providing safer alternatives.

Rules:
1. Replace .unwrap() with .expect() and add descriptive context explaining why the unwrap is safe
2. Replace panic!() with proper Result/Option returns when possible
3. Keep the fix minimal - only change the problematic line
4. Preserve indentation and formatting
5. Provide a brief explanation of the fix

Output format (JSON):
{
  "fixed_code": "the corrected line of code",
  "explanation": "brief explanation of what was changed and why"
}
"@

    # Build user prompt
    $userPrompt = @"
File: $FilePath
Line: $LineNumber
Pattern: $Pattern

Context:
$Context

Please provide a fix for the panic-inducing code on line $LineNumber.
Return ONLY valid JSON with 'fixed_code' and 'explanation' fields.
"@

    try {
        Write-Verbose "Requesting code fix from Ollama..."

        $response = Invoke-OllamaWithRetry `
            -Model $Model `
            -Prompt $userPrompt `
            -System $systemMessage

        # Parse JSON response
        # Try to extract JSON if wrapped in markdown code blocks
        $jsonMatch = $response -match '(?s)\{.*"fixed_code".*\}'
        if ($jsonMatch) {
            $jsonStr = $matches[0]
        } else {
            $jsonStr = $response
        }

        $result = $jsonStr | ConvertFrom-Json

        return @{
            FixedCode = $result.fixed_code
            Explanation = $result.explanation
            RawResponse = $response
        }

    } catch {
        Write-Error "Failed to generate code fix: $_"
        return @{
            FixedCode = $null
            Explanation = "Failed to generate fix: $_"
            RawResponse = $null
        }
    }
}

<#
.SYNOPSIS
    Check if a specific model is available

.PARAMETER ModelName
    Name of the model to check

.OUTPUTS
    Boolean indicating if model is available
#>
function Test-OllamaModel {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory=$true)]
        [string]$ModelName
    )

    $models = Get-OllamaModels
    $modelExists = $models | Where-Object { $_.name -eq $ModelName }

    return ($null -ne $modelExists)
}

<#
.SYNOPSIS
    Get the best available model from config preferences

.OUTPUTS
    String with model name
#>
function Get-BestAvailableModel {
    [CmdletBinding()]
    param()

    $config = Get-OllamaConfig

    # Try primary model
    if (Test-OllamaModel -ModelName $config.ollama.model) {
        Write-Verbose "Using primary model: $($config.ollama.model)"
        return $config.ollama.model
    }

    # Try fallback model
    if (Test-OllamaModel -ModelName $config.ollama.fallback_model) {
        Write-Warning "Primary model not available, using fallback: $($config.ollama.fallback_model)"
        return $config.ollama.fallback_model
    }

    # Try alternative models
    foreach ($model in $config.ollama.alternative_models) {
        if (Test-OllamaModel -ModelName $model) {
            Write-Warning "Using alternative model: $model"
            return $model
        }
    }

    throw "No suitable Ollama models available. Please pull qwen2.5-coder:7b or qwen2.5-coder:3b"
}

# Export module functions
Export-ModuleMember -Function @(
    'Get-OllamaConfig',
    'Test-OllamaService',
    'Get-OllamaModels',
    'Invoke-OllamaGenerate',
    'Invoke-OllamaWithRetry',
    'Invoke-OllamaCodeFix',
    'Test-OllamaModel',
    'Get-BestAvailableModel'
)
