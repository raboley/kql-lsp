#!/usr/bin/env pwsh
# kill-lsp-zombies.ps1 - Kill orphaned kql-lsp processes and related IDE instances
# Usage: powershell -ExecutionPolicy Bypass -File scripts/kill-lsp-zombies.ps1

Write-Host "Looking for kql-lsp zombie processes..." -ForegroundColor Yellow

$killed = 0

# Kill kql-lsp binaries
$lspProcs = Get-Process -Name "kql-lsp" -ErrorAction SilentlyContinue
foreach ($proc in $lspProcs) {
    Write-Host "  Killing kql-lsp process PID=$($proc.Id)" -ForegroundColor DarkYellow
    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    $killed++
}

if ($killed -eq 0) {
    Write-Host "No kql-lsp zombie processes found." -ForegroundColor Green
} else {
    Write-Host "Killed $killed kql-lsp process(es)." -ForegroundColor Green
}

# Also stop Gradle daemons if requested
if ($args -contains "--gradle") {
    Write-Host "`nStopping Gradle daemons..." -ForegroundColor Yellow
    $intellijDir = Join-Path (Get-Item $PSScriptRoot).Parent.FullName "intellij"
    if (Test-Path "$intellijDir\gradlew.bat") {
        Push-Location $intellijDir
        .\gradlew.bat --stop 2>&1
        Pop-Location
    }
}
