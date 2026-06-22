---
description: Install or refresh the shared Rust/WGSL HDC graph workflow in the current project, including the shared `.codex/rust-hdc-graph` pack, repo-local Claude plugin copy, AGENTS and CLAUDE guardrails, and optional strict graph-first enforcement.
disable-model-invocation: true
---

Resolve the project root and install the shared Rust/WGSL graph workflow there.

1. Run:
   - `powershell.exe -ExecutionPolicy Bypass -File "${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\install-project.ps1" -ProjectRoot "<root>" -InstallProjectPlugin -InstallGuardrails`
2. If the user explicitly wants strict enforcement that blocks broad search until graph `context` or `query` runs, rerun with:
   - `-InstallStrictGuardrails`
3. Report the installed paths:
   - `.codex/rust-hdc-graph/`
   - `.claude/skills/rust-hdc-graph/`
   - `AGENTS.md`
   - `CLAUDE.md`
   - `.claude/settings.json` when strict guardrails were added
4. If the repo-local plugin was just added during the current session, tell the user to run `/reload-plugins` or restart Claude Code so the newly copied project plugin is available inside the repo itself.
