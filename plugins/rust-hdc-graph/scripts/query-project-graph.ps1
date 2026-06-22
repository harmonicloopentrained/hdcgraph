param(
    [Parameter(Mandatory = $true)]
    [string]$Needle,
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$SkipRefresh
)

$ErrorActionPreference = "Stop"

$resolvedProject = (Resolve-Path -LiteralPath $ProjectRoot).Path
$scriptRoot = $PSScriptRoot
$installScript = Join-Path $scriptRoot "install-project.ps1"
$projectQuery = Join-Path $resolvedProject ".codex\\rust-hdc-graph\\scripts\\query_graph.ps1"

& $installScript -ProjectRoot $resolvedProject | Out-Null
& $projectQuery -ProjectRoot $resolvedProject -Needle $Needle -SkipRefresh:$SkipRefresh
