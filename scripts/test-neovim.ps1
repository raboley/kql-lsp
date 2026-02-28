#!/usr/bin/env pwsh
# test-neovim.ps1 - Build LSP binary + run Neovim integration tests
# Usage: powershell -ExecutionPolicy Bypass -File scripts/test-neovim.ps1
# Usage with specific spec: powershell -ExecutionPolicy Bypass -File scripts/test-neovim.ps1 -Spec "neovim/test/smoke_spec.lua"

param(
    [string]$Spec = "neovim/test/smoke_spec.lua"  # Specific spec file to run
)

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

Write-Host "Running Neovim tests: $Spec" -ForegroundColor Yellow
Push-Location $ProjectRoot

$nvimPath = "C:\Program Files\Neovim\bin\nvim.exe"
if (-not (Test-Path $nvimPath)) {
    $nvimPath = (Get-Command nvim -ErrorAction SilentlyContinue).Source
}
if (-not $nvimPath) {
    Write-Host "nvim not found!" -ForegroundColor Red
    Pop-Location
    exit 1
}

& $nvimPath --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile $Spec" 2>&1
$exitCode = $LASTEXITCODE
Pop-Location

if ($exitCode -eq 0) {
    Write-Host "Neovim tests PASSED" -ForegroundColor Green
} else {
    Write-Host "Neovim tests FAILED" -ForegroundColor Red
}
exit $exitCode
