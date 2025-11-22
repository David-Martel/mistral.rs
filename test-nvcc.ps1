# Import Visual Studio environment
$vsPath = "C:\Program Files\Microsoft Visual Studio\2022\Community"
$vcvarsPath = "$vsPath\VC\Auxiliary\Build\vcvars64.bat"

# Run vcvars and capture environment
$cmd = "`"$vcvarsPath`" && set"
$envVars = cmd.exe /c $cmd | Where-Object { $_ -match "=" }

foreach ($var in $envVars) {
    $parts = $var.Split('=', 2)
    if ($parts.Count -eq 2) {
        [Environment]::SetEnvironmentVariable($parts[0], $parts[1], "Process")
    }
}

# Now test NVCC
Write-Host "Testing NVCC compilation..."
& "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9\bin\nvcc.exe" -arch=sm_89 -ptx test_nvcc.cu -o test_nvcc.ptx
Write-Host "Exit code: $LASTEXITCODE"

if (Test-Path test_nvcc.ptx) {
    Write-Host "SUCCESS: PTX file created"
    Get-Item test_nvcc.ptx
} else {
    Write-Host "FAILED: No PTX file created"
}
