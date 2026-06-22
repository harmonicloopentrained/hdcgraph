param(
    [string]$ProjectRoot = (Get-Location).Path,
    [switch]$Force,
    [switch]$InstallGuardrails,
    [switch]$InstallProjectPlugin,
    [switch]$InstallStrictGuardrails
)

$ErrorActionPreference = "Stop"

$resolvedProject = (Resolve-Path -LiteralPath $ProjectRoot).Path
$pluginRoot = Split-Path -Parent $PSScriptRoot
$templateRoot = Join-Path $pluginRoot "assets\\project-template"
$packRoot = Join-Path $resolvedProject ".codex\\rust-hdc-graph"
$toolSource = Join-Path $templateRoot "tool"
$toolDest = Join-Path $packRoot "tool"
$scriptSource = Join-Path $templateRoot "scripts"
$scriptDest = Join-Path $packRoot "scripts"
$agentBlockPath = Join-Path $templateRoot "agent-block.md"
$claudeBlockPath = Join-Path $templateRoot "claude-block.md"
$projectPluginRoot = Join-Path $resolvedProject ".claude\\skills\\rust-hdc-graph"
$strictSettingsPath = Join-Path $resolvedProject ".claude\\settings.json"

if (-not (Test-Path -LiteralPath $templateRoot)) {
    throw "Missing plugin project template at $templateRoot"
}

New-Item -ItemType Directory -Force -Path $packRoot | Out-Null

function Copy-DirectoryContents {
    param(
        [string]$Source,
        [string]$Destination
    )

    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Set-MarkedBlock {
    param(
        [string]$Path,
        [string]$Block,
        [string]$StartMarker,
        [string]$EndMarker
    )

    if (Test-Path -LiteralPath $Path) {
        $existing = Get-Content -Raw -LiteralPath $Path
    } else {
        $existing = ""
    }

    $legacyPattern = '(?s)\r?\n?# HDC Graph First\s+Use the project-local skill at `\.codex/skills/project-hdc-graph/SKILL\.md` before broad codebase exploration\..*?Never paste the full `graph\.json` into model context\.\s*'
    $existing = [regex]::Replace($existing, $legacyPattern, "")
    $existing = $existing.Trim()

    $newContent = if ($existing.Contains($StartMarker) -and $existing.Contains($EndMarker)) {
        [regex]::Replace(
            $existing,
            "(?s)$([regex]::Escape($StartMarker)).*?$([regex]::Escape($EndMarker))",
            [System.Text.RegularExpressions.MatchEvaluator]{ param($match) $Block.TrimEnd() }
        )
    } elseif ([string]::IsNullOrWhiteSpace($existing)) {
        $Block.TrimEnd() + [Environment]::NewLine
    } else {
        $Block.TrimEnd() + [Environment]::NewLine + [Environment]::NewLine + $existing.TrimStart()
    }

    [System.IO.File]::WriteAllText($Path, $newContent)
}

function Ensure-StrictClaudeSettings {
    param([string]$Path)

    $hookCommand = 'powershell.exe -ExecutionPolicy Bypass -File "$CLAUDE_PROJECT_DIR\\.claude\\skills\\rust-hdc-graph\\scripts\\enforce-graph-first.ps1"'

    function ConvertTo-HashtableCompat {
        param([object]$Value)

        if ($null -eq $Value) {
            return $null
        }
        if ($Value -is [hashtable]) {
            $copy = @{}
            foreach ($key in $Value.Keys) {
                $copy[$key] = ConvertTo-HashtableCompat -Value $Value[$key]
            }
            return $copy
        }
        if ($Value -is [System.Collections.IEnumerable] -and -not ($Value -is [string])) {
            $items = @()
            foreach ($item in $Value) {
                $items += ,(ConvertTo-HashtableCompat -Value $item)
            }
            return $items
        }
        if ($Value.PSObject -and $Value.PSObject.Properties.Count -gt 0) {
            $map = @{}
            foreach ($prop in $Value.PSObject.Properties) {
                $map[$prop.Name] = ConvertTo-HashtableCompat -Value $prop.Value
            }
            return $map
        }
        return $Value
    }

    if (Test-Path -LiteralPath $Path) {
        $raw = Get-Content -Raw -LiteralPath $Path
        $settings = if ([string]::IsNullOrWhiteSpace($raw)) {
            @{}
        } else {
            ConvertTo-HashtableCompat -Value ($raw | ConvertFrom-Json)
        }
    } else {
        $settings = @{}
    }

    if (-not $settings.ContainsKey("hooks")) {
        $settings["hooks"] = @{}
    }
    if (-not $settings["hooks"].ContainsKey("PreToolUse")) {
        $settings["hooks"]["PreToolUse"] = @()
    } elseif (($settings["hooks"]["PreToolUse"] -is [hashtable]) -or ($settings["hooks"]["PreToolUse"] -isnot [System.Array])) {
        $settings["hooks"]["PreToolUse"] = @($settings["hooks"]["PreToolUse"])
    }

    $alreadyPresent = $false
    foreach ($entry in $settings["hooks"]["PreToolUse"]) {
        if ($entry -is [hashtable] -and $entry.ContainsKey("hooks")) {
            foreach ($hook in $entry["hooks"]) {
                if (($hook -is [hashtable]) -and ($hook["command"] -eq $hookCommand)) {
                    $alreadyPresent = $true
                }
            }
        }
    }

    if (-not $alreadyPresent) {
        $settings["hooks"]["PreToolUse"] += @{
            matcher = "Bash|Grep|Glob"
            hooks = @(
                @{
                    type = "command"
                    command = $hookCommand
                }
            )
        }
    }

    $json = $settings | ConvertTo-Json -Depth 30
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null
    [System.IO.File]::WriteAllText($Path, $json)
}

$needsToolSync = $Force -or -not (Test-Path -LiteralPath $toolDest)
$needsScriptSync = $Force -or -not (Test-Path -LiteralPath $scriptDest)

if ($Force -and (Test-Path -LiteralPath $toolDest)) {
    Remove-Item -LiteralPath $toolDest -Recurse -Force
}
if ($Force -and (Test-Path -LiteralPath $scriptDest)) {
    Remove-Item -LiteralPath $scriptDest -Recurse -Force
}

if ($needsToolSync) {
    Copy-DirectoryContents -Source $toolSource -Destination $toolDest
}
if ($needsScriptSync) {
    Copy-DirectoryContents -Source $scriptSource -Destination $scriptDest
}

if ($InstallProjectPlugin) {
    $resolvedPluginRoot = (Resolve-Path -LiteralPath $pluginRoot).Path
    $resolvedProjectPluginRoot = if (Test-Path -LiteralPath $projectPluginRoot) {
        (Resolve-Path -LiteralPath $projectPluginRoot).Path
    } else {
        $null
    }

    if ($resolvedProjectPluginRoot -ne $resolvedPluginRoot) {
        if ($Force -and (Test-Path -LiteralPath $projectPluginRoot)) {
            Remove-Item -LiteralPath $projectPluginRoot -Recurse -Force
        }
        Copy-DirectoryContents -Source $pluginRoot -Destination $projectPluginRoot
    }
}

if ($InstallGuardrails) {
    $agentBlock = Get-Content -Raw -LiteralPath $agentBlockPath
    $claudeBlock = Get-Content -Raw -LiteralPath $claudeBlockPath
    Set-MarkedBlock -Path (Join-Path $resolvedProject "AGENTS.md") -Block $agentBlock -StartMarker "<!-- rust-hdc-graph:start -->" -EndMarker "<!-- rust-hdc-graph:end -->"
    Set-MarkedBlock -Path (Join-Path $resolvedProject "CLAUDE.md") -Block $claudeBlock -StartMarker "<!-- rust-hdc-graph-claude:start -->" -EndMarker "<!-- rust-hdc-graph-claude:end -->"
}

if ($InstallStrictGuardrails) {
    Ensure-StrictClaudeSettings -Path $strictSettingsPath
}

Write-Output "pack_root=$packRoot"
Write-Output "tool_root=$toolDest"
Write-Output "scripts_root=$scriptDest"
if ($InstallProjectPlugin) {
    Write-Output "project_plugin_root=$projectPluginRoot"
}
if ($InstallStrictGuardrails) {
    Write-Output "strict_settings=$strictSettingsPath"
}
