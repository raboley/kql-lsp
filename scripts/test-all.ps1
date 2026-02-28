#!/usr/bin/env pwsh
# test-all.ps1 - Build LSP binary + run all test suites
# Usage: powershell -ExecutionPolicy Bypass -File scripts/test-all.ps1

param(
    [switch]$SkipIntelliJ  # Skip IntelliJ tests (useful for fast iteration)
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $ProjectRoot) { $ProjectRoot = Split-Path -Parent $PSScriptRoot }
# Resolve to the kql-lsp root
$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName

Write-Host "`n========================================" -ForegroundColor Cyan
Write-Host "  KQL LSP - Full Test Suite" -ForegroundColor Cyan
Write-Host "========================================`n" -ForegroundColor Cyan

$failed = $false

# Step 1: Build LSP binary
Write-Host "[1/4] Building LSP binary..." -ForegroundColor Yellow
Push-Location "$ProjectRoot\lsp"
try {
    cargo build --release 2>&1
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    Write-Host "[1/4] Building LSP binary... PASSED" -ForegroundColor Green
} catch {
    Write-Host "[1/4] Building LSP binary... FAILED" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    $failed = $true
} finally {
    Pop-Location
}

# Step 2: Rust unit tests
Write-Host "[2/4] Running Rust unit tests..." -ForegroundColor Yellow
Push-Location "$ProjectRoot\lsp"
try {
    cargo test 2>&1
    if ($LASTEXITCODE -ne 0) { throw "cargo test failed" }
    Write-Host "[2/4] Running Rust unit tests... PASSED" -ForegroundColor Green
} catch {
    Write-Host "[2/4] Running Rust unit tests... FAILED" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    $failed = $true
} finally {
    Pop-Location
}

# Step 3: Neovim integration tests
Write-Host "[3/4] Running Neovim integration tests..." -ForegroundColor Yellow
Push-Location $ProjectRoot
try {
    $nvimPath = "C:\Program Files\Neovim\bin\nvim.exe"
    if (-not (Test-Path $nvimPath)) {
        $nvimPath = (Get-Command nvim -ErrorAction SilentlyContinue).Source
    }
    if (-not $nvimPath) { throw "nvim not found" }

    & $nvimPath --headless --noplugin -u neovim/minimal_init.lua -c "PlenaryBustedFile neovim/test/smoke_spec.lua" 2>&1
    if ($LASTEXITCODE -ne 0) { throw "Neovim tests failed" }
    Write-Host "[3/4] Running Neovim integration tests... PASSED" -ForegroundColor Green
} catch {
    Write-Host "[3/4] Running Neovim integration tests... FAILED" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    $failed = $true
} finally {
    Pop-Location
}

# Step 4: IntelliJ UI tests
if ($SkipIntelliJ) {
    Write-Host "[4/4] Skipping IntelliJ UI tests (-SkipIntelliJ)" -ForegroundColor DarkYellow
} else {
    Write-Host "[4/4] Running IntelliJ UI tests..." -ForegroundColor Yellow
    Push-Location "$ProjectRoot\intellij"
    try {
        # Check if robot server is available
        try {
            $response = Invoke-WebRequest -Uri "http://localhost:8082" -TimeoutSec 3 -ErrorAction SilentlyContinue
        } catch {
            Write-Host "  WARNING: Robot server not responding on port 8082." -ForegroundColor DarkYellow
            Write-Host "  Start the IDE sandbox first: ./gradlew runIdeForUiTests" -ForegroundColor DarkYellow
            throw "IDE sandbox not running (robot server on port 8082 not responding)"
        }

        .\gradlew.bat uiTest 2>&1
        if ($LASTEXITCODE -ne 0) { throw "IntelliJ UI tests failed" }
        Write-Host "[4/4] Running IntelliJ UI tests... PASSED" -ForegroundColor Green
    } catch {
        Write-Host "[4/4] Running IntelliJ UI tests... FAILED" -ForegroundColor Red
        Write-Host $_.Exception.Message -ForegroundColor Red
        $failed = $true
    } finally {
        Pop-Location
    }
}

# Summary
Write-Host "`n========================================" -ForegroundColor Cyan
if ($failed) {
    Write-Host "  SOME TESTS FAILED" -ForegroundColor Red
    exit 1
} else {
    Write-Host "  ALL TESTS PASSED" -ForegroundColor Green
    exit 0
}
