param(
    [Parameter(Mandatory = $true)]
    [string]$Task,
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$SkipRefresh
)

$ErrorActionPreference = "Stop"

$resolvedProject = (Resolve-Path -LiteralPath $ProjectRoot).Path
$scriptRoot = $PSScriptRoot
$installScript = Join-Path $scriptRoot "install-project.ps1"
$projectContext = Join-Path $resolvedProject ".codex\\rust-hdc-graph\\scripts\\context_graph.ps1"

& $installScript -ProjectRoot $resolvedProject | Out-Null
& $projectContext -ProjectRoot $resolvedProject -Task $Task -SkipRefresh:$SkipRefresh
