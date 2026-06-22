param(
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$Force,
    [switch]$StrictGuardrails
)

$ErrorActionPreference = "Stop"

$bundleRoot = Split-Path -Parent $PSScriptRoot
$pluginRoot = Join-Path $bundleRoot "plugins/rust-hdc-graph"
$installScript = Join-Path $pluginRoot "scripts/install-project.ps1"

if (-not (Test-Path -LiteralPath $installScript)) {
    throw "Missing Claude plugin installer at $installScript"
}

& $installScript `
    -ProjectRoot $ProjectRoot `
    -Force:$Force `
    -InstallProjectPlugin `
    -InstallGuardrails `
    -InstallStrictGuardrails:$StrictGuardrails

Write-Output "next_step=run /reload-plugins or restart Claude Code if the repo-local plugin was installed during an active session"
