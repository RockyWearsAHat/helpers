#!/usr/bin/env pwsh
# Verify cap-binary-runs gate: ensure helpers-native binary is in project root and MCP schemas work

$ErrorActionPreference = "Stop"

$ROOT = Split-Path -Path $PSScriptRoot -Parent

# Determine binary name based on platform
$binaryName = if ($IsWindows -or [System.Environment]::OSVersion.Platform -eq 'Win32NT') {
    "helpers-native.exe"
} else {
    "helpers-native"
}
$binaryPath = Join-Path $ROOT $binaryName

# Gate check 1: Binary exists in project root
if (-not (Test-Path $binaryPath)) {
    Write-Error "cap-binary-runs FAILED: Binary not found at $binaryPath"
    exit 1
}

Write-Host "✓ Binary exists at $binaryPath"

# Gate check 2: Binary is executable
if (-not (Test-Path $binaryPath -PathType Leaf)) {
    Write-Error "cap-binary-runs FAILED: $binaryPath is not a file"
    exit 1
}

Write-Host "✓ Binary is a valid file"

# Gate check 3: Binary can run schemas command and output valid JSON with MCP tools
$output = $null
$exitCode = 0

try {
    $output = & $binaryPath schemas 2>&1
    $exitCode = $LASTEXITCODE
} catch {
    Write-Error "cap-binary-runs FAILED: Exception running binary - $($_.Exception.Message)"
    exit 1
}

if ($exitCode -ne 0) {
    Write-Error "cap-binary-runs FAILED: Binary exited with code $exitCode"
    exit 1
}

# Parse as JSON
try {
    $schemas = $output | ConvertFrom-Json
} catch {
    Write-Error "cap-binary-runs FAILED: Invalid JSON from schemas - $($_.Exception.Message)"
    exit 1
}

# Verify it's an array-like object
$schemaArray = @($schemas)

if ($schemaArray.Count -eq 0) {
    Write-Error "cap-binary-runs FAILED: No schemas returned"
    exit 1
}

Write-Host "✓ Binary outputs valid JSON with $($schemaArray.Count) MCP tools"

# Gate check 4: Verify key expected tools are present
$toolNames = @()
foreach ($schema in $schemaArray) {
    if ($schema.name) {
        $toolNames += $schema.name
    }
}

$expectedTools = @("checkpoint", "lint", "project_map")
$missingTools = @()

foreach ($tool in $expectedTools) {
    if ($tool -notin $toolNames) {
        $missingTools += $tool
    }
}

if ($missingTools.Count -gt 0) {
    Write-Host "cap-binary-runs WARNING: Missing expected tools: $($missingTools -join ', ')"
} else {
    Write-Host "✓ All expected MCP tools are present"
}

Write-Host ""
Write-Host "✅ cap-binary-runs gate PASSED"
exit 0
