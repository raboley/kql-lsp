#!/usr/bin/env pwsh
# test-lsp-initialize.ps1 - Test LSP binary directly via stdin/stdout
# Usage: powershell -ExecutionPolicy Bypass -File scripts/test-lsp-initialize.ps1

param(
    [string]$BinaryPath  # Override binary path (default: lsp/target/release/kql-lsp.exe)
)

$ErrorActionPreference = "Stop"
$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName

if (-not $BinaryPath) {
    $BinaryPath = Join-Path $ProjectRoot "lsp\target\release\kql-lsp.exe"
}

if (-not (Test-Path $BinaryPath)) {
    Write-Host "Binary not found: $BinaryPath" -ForegroundColor Red
    Write-Host "Build it first: cd lsp && cargo build --release" -ForegroundColor Yellow
    exit 1
}

Write-Host "Testing LSP binary: $BinaryPath" -ForegroundColor Yellow

# Prepare initialize request
$initRequest = @{
    jsonrpc = "2.0"
    id = 1
    method = "initialize"
    params = @{
        processId = $PID
        clientInfo = @{
            name = "test-script"
            version = "1.0.0"
        }
        capabilities = @{}
        rootUri = "file:///C:/test"
    }
} | ConvertTo-Json -Depth 10 -Compress

$contentLength = [System.Text.Encoding]::UTF8.GetByteCount($initRequest)
$fullMessage = "Content-Length: $contentLength`r`n`r`n$initRequest"

# Send to LSP binary via stdin
$process = New-Object System.Diagnostics.Process
$process.StartInfo.FileName = $BinaryPath
$process.StartInfo.WorkingDirectory = Join-Path $ProjectRoot "lsp"
$process.StartInfo.UseShellExecute = $false
$process.StartInfo.RedirectStandardInput = $true
$process.StartInfo.RedirectStandardOutput = $true
$process.StartInfo.RedirectStandardError = $true
$process.StartInfo.CreateNoWindow = $true

$process.Start() | Out-Null

# Send initialize request
$process.StandardInput.Write($fullMessage)
$process.StandardInput.Flush()

# Wait for response (timeout 10 seconds)
$deadline = (Get-Date).AddSeconds(10)
$response = ""
while ((Get-Date) -lt $deadline) {
    if ($process.StandardOutput.Peek() -ge 0) {
        $buffer = New-Object char[] 4096
        $count = $process.StandardOutput.Read($buffer, 0, $buffer.Length)
        $response += [string]::new($buffer, 0, $count)
        if ($response.Contains('"result"')) { break }
    }
    Start-Sleep -Milliseconds 100
}

# Cleanup
try {
    $process.StandardInput.Close()
    $process.Kill()
} catch {}

if ($response.Contains('"result"') -and $response.Contains('"capabilities"')) {
    Write-Host "LSP initialize test PASSED" -ForegroundColor Green
    Write-Host "Response received:" -ForegroundColor Cyan

    # Extract just the JSON part
    $jsonStart = $response.IndexOf('{')
    if ($jsonStart -ge 0) {
        $json = $response.Substring($jsonStart)
        try {
            $parsed = $json | ConvertFrom-Json
            Write-Host ($parsed | ConvertTo-Json -Depth 5) -ForegroundColor White
        } catch {
            Write-Host $json -ForegroundColor White
        }
    }
    exit 0
} else {
    Write-Host "LSP initialize test FAILED" -ForegroundColor Red
    Write-Host "Response: $response" -ForegroundColor Red
    exit 1
}
