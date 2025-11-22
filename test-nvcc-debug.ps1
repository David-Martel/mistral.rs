# Import Visual Studio environment
$vsPath = "C:\Program Files\Microsoft Visual Studio\2022\Community"
$vcvarsPath = "$vsPath\VC\Auxiliary\Build\vcvars64.bat"

# Run vcvars and capture environment
$cmd = "`"$vcvarsPath`" && set"
$envVars = cmd.exe /c $cmd 2>&1 | Where-Object { $_ -match "=" }

foreach ($var in $envVars) {
    $parts = $var.Split('=', 2)
    if ($parts.Count -eq 2) {
        [Environment]::SetEnvironmentVariable($parts[0], $parts[1], "Process")
    }
}

# Check environment
Write-Host "INCLUDE: $env:INCLUDE"
Write-Host "LIB: $env:LIB"
Write-Host ""

# Now test NVCC with stderr capture
Write-Host "Testing NVCC compilation..."
$output = & "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\bin\nvcc.exe" -arch=sm_89 -ptx test_nvcc.cu -o test_nvcc.ptx 2>&1
Write-Host "NVCC Output:"
Write-Host $output
Write-Host "Exit code: $LASTEXITCODE"
