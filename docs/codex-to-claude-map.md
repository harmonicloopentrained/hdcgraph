# Codex To Claude Map

This is the one-to-one concept map between the Codex implementation and the Claude implementation.

## Shared Pieces

- Shared graph pack path: `.codex/rust-hdc-graph/`
- Shared extractor: vendored Rust tool under `.codex/rust-hdc-graph/tool/`
- Shared project scripts:
  - `ensure_graph.ps1`
  - `query_graph.ps1`
  - `context_graph.ps1`
- Shared repo guardrail block in `AGENTS.md`

## Codex Side

- Global personal plugin
- Codex skill metadata in `agents/openai.yaml`
- Codex skill invocation through the Codex skill system
- Codex-first wrapper scripts under the plugin `scripts/` directory

## Claude Side

- Claude plugin manifest at `.claude-plugin/plugin.json`
- Repo-local or personal plugin loading through Claude Code plugins
- Claude skills under `skills/<name>/SKILL.md`
- Claude hook automation under `hooks/hooks.json`
- Claude bridge instructions in `CLAUDE.md`
- Optional strict enforcement merged into `.claude/settings.json`

## Why The Graph Pack Stays In `.codex`

The transferable requirement is easiest to satisfy when both tools share one graph location instead of generating separate caches.

That means:

- Codex can keep using the existing graph output without migration
- Claude can read the same graph output immediately after installation
- The extractor source, build output, and graph cache remain versioned together in the repo

## Guardrail Strategy

Soft guardrails:

- `AGENTS.md` tells all agents to use graph context or query first
- `CLAUDE.md` imports `AGENTS.md` and adds Claude-specific guidance
- Claude plugin hooks inject a reminder into user prompts, subagents, and post-compaction startup

Strict guardrails:

- `.claude/settings.json` adds a `PreToolUse` hook
- The hook blocks `Grep`, `Glob`, and Bash search commands until Claude runs graph `context` or `query`
- Graph session stamps are written under `.codex/rust-hdc-graph/state/claude/`
