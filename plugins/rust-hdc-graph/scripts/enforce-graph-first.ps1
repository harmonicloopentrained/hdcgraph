param()

$ErrorActionPreference = "Stop"

function Resolve-ProjectRoot {
    param([object]$Payload)

    if (-not [string]::IsNullOrWhiteSpace($env:CLAUDE_PROJECT_DIR)) {
        return $env:CLAUDE_PROJECT_DIR
    }
    if (($Payload.PSObject.Properties.Name -contains "cwd") -and -not [string]::IsNullOrWhiteSpace($Payload.cwd)) {
        return $Payload.cwd
    }
    return (Get-Location).Path
}

function Test-RustGraphRepo {
    param([string]$ProjectRoot, [string]$PackRoot)

    if (Test-Path -LiteralPath $PackRoot) {
        return $true
    }
    if (Test-Path -LiteralPath (Join-Path $ProjectRoot "Cargo.toml")) {
        return $true
    }
    return $false
}

function Get-GraphStamp {
    param([string]$PackRoot)

    $stateRoot = Join-Path $PackRoot "state\\claude"
    if (-not (Test-Path -LiteralPath $stateRoot)) {
        return $null
    }

    if (-not [string]::IsNullOrWhiteSpace($env:CLAUDE_SESSION_ID)) {
        $sessionPath = Join-Path $stateRoot ($env:CLAUDE_SESSION_ID + ".json")
        if (Test-Path -LiteralPath $sessionPath) {
            return (Get-Content -Raw -LiteralPath $sessionPath | ConvertFrom-Json)
        }
    }

    $lastPath = Join-Path $stateRoot "last.json"
    if (Test-Path -LiteralPath $lastPath) {
        return (Get-Content -Raw -LiteralPath $lastPath | ConvertFrom-Json)
    }

    return $null
}

function Test-GraphBootstrapCommand {
    param([string]$Command)

    if ([string]::IsNullOrWhiteSpace($Command)) {
        return $false
    }

    return $Command -match 'rust-hdc-graph\\scripts\\(context-project-graph|query-project-graph|ensure-project-graph|install-project|context_graph|query_graph|ensure_graph)\.ps1'
}

function Test-BroadSearchCommand {
    param([string]$ToolName, [string]$Command)

    if ($ToolName -in @("Grep", "Glob")) {
        return $true
    }
    if ($ToolName -ne "Bash") {
        return $false
    }
    if ([string]::IsNullOrWhiteSpace($Command)) {
        return $false
    }

    return [regex]::IsMatch($Command, '(?i)(^|\\s)(rg|grep|fd|find|select-string)(\\s|$)')
}

$rawInput = [Console]::In.ReadToEnd()
if ([string]::IsNullOrWhiteSpace($rawInput)) {
    exit 0
}

$payload = $rawInput | ConvertFrom-Json
$projectRoot = Resolve-ProjectRoot -Payload $payload

try {
    $projectRoot = (Resolve-Path -LiteralPath $projectRoot).Path
} catch {
    exit 0
}

$packRoot = Join-Path $projectRoot ".codex\\rust-hdc-graph"
if (-not (Test-RustGraphRepo -ProjectRoot $projectRoot -PackRoot $packRoot)) {
    exit 0
}

$toolName = [string]$payload.tool_name
$command = ""
if (($payload.PSObject.Properties.Name -contains "tool_input") -and ($payload.tool_input -ne $null) -and ($payload.tool_input.PSObject.Properties.Name -contains "command")) {
    $command = [string]$payload.tool_input.command
}

if (Test-GraphBootstrapCommand -Command $command) {
    exit 0
}

$stamp = Get-GraphStamp -PackRoot $packRoot
$graphWarm = $null -ne $stamp -and @("context", "query") -contains $stamp.mode
if ($graphWarm) {
    exit 0
}

if (-not (Test-BroadSearchCommand -ToolName $toolName -Command $command)) {
    exit 0
}

@{
    hookSpecificOutput = @{
        hookEventName = "PreToolUse"
        permissionDecision = "deny"
        permissionDecisionReason = "Use the shared HDC graph first. Run /rust-hdc-graph:context for task-level retrieval or /rust-hdc-graph:query for exact symbol lookups before broad search."
    }
} | ConvertTo-Json -Depth 10 -Compress
