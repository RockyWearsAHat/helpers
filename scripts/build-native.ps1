#!/usr/bin/env pwsh
# Build helpers-native and copy the binary to the project root

$ErrorActionPreference = "Stop"

$ROOT = Split-Path -Path $PSScriptRoot -Parent
$nativeDir = Join-Path $ROOT "native"

Set-Location $nativeDir
Write-Host "[build-native] Compiling helpers-native in release mode..."

# Build in release mode
cargo build --release

# Determine the binary path
$binaryName = "helpers-native.exe"
$binaryPath = Join-Path $nativeDir "target" "release" $binaryName

# For other architectures, try the more specific path
if (-not (Test-Path $binaryPath)) {
    # Try to find it in target directory
    $found = Get-ChildItem -Path (Join-Path $nativeDir "target") -Filter $binaryName -Recurse | Select-Object -First 1
    if ($found) {
        $binaryPath = $found.FullName
    }
}

if (-not (Test-Path $binaryPath)) {
    Write-Error "[build-native] Binary not found at $binaryPath"
    exit 1
}

$destPath = Join-Path $ROOT $binaryName
Write-Host "[build-native] Copying $binaryPath to $destPath..."
Copy-Item -Path $binaryPath -Destination $destPath -Force
Write-Host "[build-native] Binary copied successfully"
