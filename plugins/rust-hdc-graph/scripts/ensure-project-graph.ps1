param(
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$Force,
    [switch]$InstallGuardrails,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

$resolvedProject = (Resolve-Path -LiteralPath $ProjectRoot).Path
$scriptRoot = $PSScriptRoot
$installScript = Join-Path $scriptRoot "install-project.ps1"
$projectEnsure = Join-Path $resolvedProject ".codex\\rust-hdc-graph\\scripts\\ensure_graph.ps1"

& $installScript -ProjectRoot $resolvedProject -Force:$Force -InstallGuardrails:$InstallGuardrails | Out-Null
& $projectEnsure -ProjectRoot $resolvedProject -Force:$Force -Quiet:$Quiet
