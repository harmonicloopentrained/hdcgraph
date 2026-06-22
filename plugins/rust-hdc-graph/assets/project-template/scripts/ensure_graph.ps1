param(
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$Force,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

$resolvedProject = (Resolve-Path -LiteralPath $ProjectRoot).Path
$packRoot = Split-Path -Parent $PSScriptRoot
$toolRoot = Join-Path $packRoot "tool"
$toolExe = Join-Path $toolRoot "target\\debug\\rust-hdc-graph-tool.exe"
$graphJson = Join-Path $packRoot "graph.json"
$summaryMd = Join-Path $packRoot "SUMMARY.md"

if (-not (Test-Path -LiteralPath $toolRoot)) {
    throw "Missing project-local HDC tool at $toolRoot"
}

function Get-NewestTimestamp {
    param(
        [string]$RootPath,
        [string[]]$Extensions,
        [string[]]$SkipDirs
    )

    $latest = [datetime]::MinValue
    Get-ChildItem -LiteralPath $RootPath -Recurse -File | ForEach-Object {
        $ext = $_.Extension.ToLowerInvariant()
        if ($Extensions -notcontains $ext) {
            return
        }
        foreach ($skip in $SkipDirs) {
            $needle = [IO.Path]::DirectorySeparatorChar + $skip + [IO.Path]::DirectorySeparatorChar
            if ($_.FullName.Contains($needle)) {
                return
            }
        }
        if ($_.LastWriteTimeUtc -gt $latest) {
            $latest = $_.LastWriteTimeUtc
        }
    }
    return $latest
}

function Test-GraphCompatible {
    param([string]$GraphJsonPath)

    if (-not (Test-Path -LiteralPath $GraphJsonPath)) {
        return $false
    }

    $raw = Get-Content -Raw -LiteralPath $GraphJsonPath
    return $raw.Contains('"stable_hash"') -and $raw.Contains('"context_packs"') -and $raw.Contains('"graph_signature"')
}

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

$toolLatest = Get-NewestTimestamp -RootPath $toolRoot -Extensions @(".rs", ".toml", ".lock") -SkipDirs @("target", ".git")
if (-not (Test-Path -LiteralPath $toolExe) -or ((Get-Item -LiteralPath $toolExe).LastWriteTimeUtc -lt $toolLatest)) {
    Push-Location $toolRoot
    try {
        & cargo build | Out-Host
    }
    finally {
        Pop-Location
    }
}

$refresh = $Force.IsPresent -or -not (Test-Path -LiteralPath $graphJson) -or -not (Test-Path -LiteralPath $summaryMd)
if (-not $refresh -and -not (Test-GraphCompatible -GraphJsonPath $graphJson)) {
    $refresh = $true
}
if (-not $refresh) {
    $sourceLatest = Get-NewestTimestamp -RootPath $resolvedProject -Extensions @(".rs", ".wgsl") -SkipDirs @(".git", ".codex", ".agents", "target", "node_modules", "graphify-out")
    if ((Get-Item -LiteralPath $graphJson).LastWriteTimeUtc -lt $sourceLatest) {
        $refresh = $true
    }
}

if ($refresh) {
    & $toolExe extract $resolvedProject $packRoot | Out-Host
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        exit $exitCode
    }
    if (-not $Quiet) {
        Write-Output "status=refreshed"
    }
} elseif (-not $Quiet) {
    Write-Output "status=current"
}

Write-ClaudeGraphStamp -Mode "ensure"

if (-not $Quiet) {
    Write-Output "graph_json=$graphJson"
    Write-Output "summary_md=$summaryMd"
}
