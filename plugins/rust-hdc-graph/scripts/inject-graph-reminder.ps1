param(
    [ValidateSet("prompt", "subagent", "compact")]
    [string]$Mode = "prompt"
)

$ErrorActionPreference = "Stop"

function Resolve-ProjectRoot {
    param([string]$RawJson)

    if (-not [string]::IsNullOrWhiteSpace($env:CLAUDE_PROJECT_DIR)) {
        return $env:CLAUDE_PROJECT_DIR
    }

    if (-not [string]::IsNullOrWhiteSpace($RawJson)) {
        try {
            $parsed = $RawJson | ConvertFrom-Json
            foreach ($key in @("cwd", "project_dir")) {
                if (($parsed.PSObject.Properties.Name -contains $key) -and -not [string]::IsNullOrWhiteSpace($parsed.$key)) {
                    return $parsed.$key
                }
            }
        } catch {
        }
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
    foreach ($dir in @("src", "shaders")) {
        $candidate = Join-Path $ProjectRoot $dir
        if (Test-Path -LiteralPath $candidate) {
            $match = Get-ChildItem -LiteralPath $candidate -Recurse -File -Include *.rs,*.wgsl -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($null -ne $match) {
                return $true
            }
        }
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

$rawInput = [Console]::In.ReadToEnd()
$projectRoot = Resolve-ProjectRoot -RawJson $rawInput

try {
    $projectRoot = (Resolve-Path -LiteralPath $projectRoot).Path
} catch {
    exit 0
}

$packRoot = Join-Path $projectRoot ".codex\\rust-hdc-graph"
if (-not (Test-RustGraphRepo -ProjectRoot $projectRoot -PackRoot $packRoot)) {
    exit 0
}

$stamp = Get-GraphStamp -PackRoot $packRoot
$graphWarm = $null -ne $stamp -and @("context", "query") -contains $stamp.mode

$eventName = switch ($Mode) {
    "subagent" { "SubagentStart" }
    "compact" { "SessionStart" }
    default { "UserPromptSubmit" }
}

$message = if ($graphWarm) {
    "Shared Rust/WGSL graph pack already consulted this session via '$($stamp.mode)'. Keep reading graph-surfaced files first and widen to native search only after the graph narrows the space or misses."
} else {
    "Rust/WGSL repo detected. Use the shared graph pack at `.codex/rust-hdc-graph` before broad search: run `context` for task-scoped retrieval, `query` for exact symbol lookup, then read graph-surfaced files before Grep, Glob, or Bash search."
}

@{
    hookSpecificOutput = @{
        hookEventName = $eventName
        additionalContext = $message
    }
} | ConvertTo-Json -Depth 10 -Compress
