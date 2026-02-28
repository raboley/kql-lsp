#!/usr/bin/env pwsh
# test-intellij.ps1 - Build LSP binary + run IntelliJ UI tests
# Usage: powershell -ExecutionPolicy Bypass -File scripts/test-intellij.ps1
# Prerequisites: IDE sandbox must be running (./gradlew runIdeForUiTests)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName

Write-Host "Building LSP binary..." -ForegroundColor Yellow
Push-Location "$ProjectRoot\lsp"
cargo build --release 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "cargo build failed!" -ForegroundColor Red
    Pop-Location
    exit 1
}
Pop-Location

# Check robot server
Write-Host "Checking robot server on port 8082..." -ForegroundColor Yellow
try {
    Invoke-WebRequest -Uri "http://localhost:8082" -TimeoutSec 3 -ErrorAction SilentlyContinue | Out-Null
} catch {
    Write-Host "Robot server not responding on port 8082!" -ForegroundColor Red
    Write-Host "Start the IDE sandbox first:" -ForegroundColor Yellow
    Write-Host "  cd intellij && ./gradlew runIdeForUiTests" -ForegroundColor White
    exit 1
}

Write-Host "Running IntelliJ UI tests..." -ForegroundColor Yellow
Push-Location "$ProjectRoot\intellij"
.\gradlew.bat uiTest 2>&1
$exitCode = $LASTEXITCODE
Pop-Location

if ($exitCode -eq 0) {
    Write-Host "IntelliJ UI tests PASSED" -ForegroundColor Green
} else {
    Write-Host "IntelliJ UI tests FAILED" -ForegroundColor Red
}
exit $exitCode
