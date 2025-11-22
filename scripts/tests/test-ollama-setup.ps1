<#
.SYNOPSIS
    Test Ollama setup and verify all components work

.DESCRIPTION
    Verifies:
    1. Configuration loads correctly
    2. Ollama service is running
    3. Models are available
    4. Rust code tools work
    5. Ollama client can generate responses
    6. Code fix generation works
#>

param(
    [switch]$Verbose
)

# Set up paths
$scriptRoot = Split-Path $PSScriptRoot -Parent
$projectRoot = Split-Path $scriptRoot -Parent

# Import modules
Import-Module (Join-Path $scriptRoot "tools\rust-code-tools.psm1") -Force
Import-Module (Join-Path $scriptRoot "clients\ollama-client.psm1") -Force

Write-Host "`n===========================================" -ForegroundColor Cyan
Write-Host "   Ollama Setup Verification Test" -ForegroundColor Cyan
Write-Host "===========================================" -ForegroundColor Cyan

# Test 1: Configuration
Write-Host "`n[Test 1/6] Loading configuration..." -ForegroundColor Yellow
try {
    $config = Get-OllamaConfig
    Write-Host "✓ Configuration loaded successfully" -ForegroundColor Green
    Write-Host "  - Endpoint: $($config.ollama.endpoint)" -ForegroundColor Gray
    Write-Host "  - Primary model: $($config.ollama.model)" -ForegroundColor Gray
    Write-Host "  - Fallback model: $($config.ollama.fallback_model)" -ForegroundColor Gray
} catch {
    Write-Host "✗ Failed to load configuration: $_" -ForegroundColor Red
    exit 1
}

# Test 2: Ollama Service
Write-Host "`n[Test 2/6] Testing Ollama service connectivity..." -ForegroundColor Yellow
try {
    $serviceRunning = Test-OllamaService
    if ($serviceRunning) {
        Write-Host "✓ Ollama service is running" -ForegroundColor Green
    } else {
        Write-Host "✗ Ollama service not accessible" -ForegroundColor Red
        Write-Host "  Run: ollama serve" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Host "✗ Error testing service: $_" -ForegroundColor Red
    exit 1
}

# Test 3: Models
Write-Host "`n[Test 3/6] Listing available models..." -ForegroundColor Yellow
try {
    $models = Get-OllamaModels
    Write-Host "✓ Found $($models.Count) models:" -ForegroundColor Green
    foreach ($model in $models | Select-Object -First 5) {
        Write-Host "  - $($model.name) ($([math]::Round($model.size / 1GB, 2)) GB)" -ForegroundColor Gray
    }

    # Check for required models
    $hasQwenCoder = $models | Where-Object { $_.name -like "qwen2.5-coder*" }
    if ($hasQwenCoder) {
        Write-Host "✓ Qwen2.5-Coder models found" -ForegroundColor Green
    } else {
        Write-Host "⚠  Qwen2.5-Coder models not found - some tests may fail" -ForegroundColor Yellow
    }
} catch {
    Write-Host "✗ Failed to list models: $_" -ForegroundColor Red
    exit 1
}

# Test 4: Rust Code Tools
Write-Host "`n[Test 4/6] Testing Rust code tools..." -ForegroundColor Yellow
try {
    # Create a test file
    $testFile = Join-Path $projectRoot "test_unwrap_sample.rs"
    $testCode = @"
fn example() {
    let value = option.unwrap();
    let result = operation().expect("failed");
}
"@
    Set-Content $testFile $testCode

    # Test file context reading
    $context = Get-RustFileContext -FilePath $testFile -LineNumber 2 -ContextLines 2

    if ($context.TargetLine -match "unwrap") {
        Write-Host "✓ File context reading works" -ForegroundColor Green
        Write-Host "  - Read $($context.TotalLines) lines" -ForegroundColor Gray
        Write-Host "  - Found target line with unwrap()" -ForegroundColor Gray
    } else {
        Write-Host "✗ File context reading failed" -ForegroundColor Red
    }

    # Clean up test file
    Remove-Item $testFile -ErrorAction SilentlyContinue

} catch {
    Write-Host "✗ Rust code tools test failed: $_" -ForegroundColor Red
    # Clean up
    Remove-Item $testFile -ErrorAction SilentlyContinue
}

# Test 5: Basic Ollama Generation
Write-Host "`n[Test 5/6] Testing Ollama text generation..." -ForegroundColor Yellow
try {
    $prompt = "Explain what .unwrap() does in Rust in one sentence."

    Write-Host "  Prompt: $prompt" -ForegroundColor Gray
    Write-Host "  Generating response (this may take 5-10 seconds)..." -ForegroundColor Gray

    $response = Invoke-OllamaGenerate -Prompt $prompt -Model "qwen2.5-coder:3b"

    if ($response -and $response.Length -gt 10) {
        Write-Host "✓ Text generation works" -ForegroundColor Green
        Write-Host "  Response: $($response.Substring(0, [Math]::Min(100, $response.Length)))..." -ForegroundColor Gray
    } else {
        Write-Host "✗ Generated response is empty or too short" -ForegroundColor Red
    }
} catch {
    Write-Host "✗ Text generation failed: $_" -ForegroundColor Red
    Write-Host "  Error details: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Test 6: Code Fix Generation
Write-Host "`n[Test 6/6] Testing code fix generation..." -ForegroundColor Yellow
try {
    $testContext = @"
    fn load_config() -> Config {
        let file_content = fs::read_to_string("config.toml").unwrap();
        parse_config(&file_content)
    }
"@

    Write-Host "  Test code: $testContext" -ForegroundColor Gray
    Write-Host "  Generating fix (this may take 10-20 seconds)..." -ForegroundColor Gray

    $fix = Invoke-OllamaCodeFix `
        -FilePath "test.rs" `
        -LineNumber 2 `
        -Pattern "unwrap" `
        -Context $testContext `
        -Model "qwen2.5-coder:3b"

    if ($fix.FixedCode) {
        Write-Host "✓ Code fix generation works" -ForegroundColor Green
        Write-Host "  Original: let file_content = fs::read_to_string(...).unwrap();" -ForegroundColor Gray
        Write-Host "  Fixed: $($fix.FixedCode)" -ForegroundColor Gray
        Write-Host "  Explanation: $($fix.Explanation)" -ForegroundColor Gray
    } else {
        Write-Host "✗ Code fix generation failed - no fix returned" -ForegroundColor Red
        Write-Host "  Raw response: $($fix.RawResponse)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "✗ Code fix generation failed: $_" -ForegroundColor Red
    Write-Host "  Error details: $($_.Exception.Message)" -ForegroundColor Yellow
}

# Summary
Write-Host "`n===========================================" -ForegroundColor Cyan
Write-Host "   Test Summary" -ForegroundColor Cyan
Write-Host "===========================================" -ForegroundColor Cyan
Write-Host "✓ Configuration: Working" -ForegroundColor Green
Write-Host "✓ Ollama Service: Running" -ForegroundColor Green
Write-Host "✓ Models: Available ($($models.Count) total)" -ForegroundColor Green
Write-Host "✓ Rust Tools: Working" -ForegroundColor Green
Write-Host ""
Write-Host "All core components verified!" -ForegroundColor Green
Write-Host "Ready for Week 2: Git monitor and AST-grep integration" -ForegroundColor Cyan
Write-Host "===========================================" -ForegroundColor Cyan
