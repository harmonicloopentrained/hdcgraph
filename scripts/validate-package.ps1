$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot

Write-Output "Checking JSON manifests..."
Get-Content -Raw -LiteralPath (Join-Path $Root ".claude-plugin\marketplace.json") | ConvertFrom-Json | Out-Null
Get-Content -Raw -LiteralPath (Join-Path $Root "pluginsust-hdc-graph\.claude-plugin\plugin.json") | ConvertFrom-Json | Out-Null
Get-Content -Raw -LiteralPath (Join-Path $Root "pluginsust-hdc-graph\hooks\hooks.json") | ConvertFrom-Json | Out-Null

Write-Output "Checking required files..."
$required = @(
    "README.md",
    "LICENSE",
    "NOTICE",
    "CITATION.cff",
    ".claude-plugin\marketplace.json",
    "pluginsust-hdc-graph\.claude-plugin\plugin.json",
    "pluginsust-hdc-graph\skillsootstrap\SKILL.md",
    "pluginsust-hdc-graph\skills\context\SKILL.md",
    "pluginsust-hdc-graph\skills\query\SKILL.md",
    "pluginsust-hdc-graph\skillsefresh\SKILL.md",
    "pluginsust-hdc-graph\skills\graph-first\SKILL.md"
)
foreach ($item in $required) {
    $path = Join-Path $Root $item
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Missing required file: $item"
    }
}

Write-Output "Package structure OK. Run Claude's own validators locally if available:"
Write-Output "  claude plugin validate ./plugins/rust-hdc-graph"
Write-Output "  claude plugin marketplace validate ."
