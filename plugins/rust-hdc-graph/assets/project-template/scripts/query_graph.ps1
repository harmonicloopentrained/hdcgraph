param(
    [Parameter(Mandatory = $true)]
    [string]$Needle,
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$SkipRefresh
)

$ErrorActionPreference = "Stop"

$packRoot = Split-Path -Parent $PSScriptRoot
$toolExe = Join-Path $packRoot "tool\\target\\debug\\rust-hdc-graph-tool.exe"
$graphJson = Join-Path $packRoot "graph.json"
$ensureScript = Join-Path $PSScriptRoot "ensure_graph.ps1"

function Write-ClaudeGraphStamp {
    param([string]$Mode)

    $stateRoot = Join-Path $packRoot "state\\claude"
    New-Item -ItemType Directory -Force -Path $stateRoot | Out-Null

    $payload = [ordered]@{
        mode = $Mode
        session_id = $env:CLAUDE_SESSION_ID
        updated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    }
    $json = $payload | ConvertTo-Json -Depth 5
    [System.IO.File]::WriteAllText((Join-Path $stateRoot "last.json"), $json)

    if (-not [string]::IsNullOrWhiteSpace($env:CLAUDE_SESSION_ID)) {
        [System.IO.File]::WriteAllText((Join-Path $stateRoot ($env:CLAUDE_SESSION_ID + ".json")), $json)
    }
}

if (-not $SkipRefresh) {
    & $ensureScript -ProjectRoot $ProjectRoot -Quiet | Out-Null
}

& $toolExe query $graphJson $Needle
$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    exit $exitCode
}

Write-ClaudeGraphStamp -Mode "query"
