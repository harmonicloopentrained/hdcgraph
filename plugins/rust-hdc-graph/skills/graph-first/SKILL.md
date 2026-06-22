---
description: Graph-first navigation for Rust and WGSL codebases that keep a shared project-local HDC graph pack under `.codex/rust-hdc-graph`. Use when tracing architecture, shader wiring, subsystem boundaries, file relationships, data flow, or task context in Rust-heavy repos, especially after the shared graph workflow has been installed.
user-invocable: false
---

# Rust HDC Graph

1. Resolve the project root.
2. If `.codex/rust-hdc-graph/` is missing, or the user asks to install the workflow, run the shared installer first:
   - `powershell.exe -ExecutionPolicy Bypass -File "${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\install-project.ps1" -ProjectRoot "<root>" -InstallGuardrails`
   - If the repo should carry its own Claude plugin for future sessions and teammates, add `-InstallProjectPlugin`.
   - If the user wants strict enforcement that blocks broad search before graph use, add `-InstallStrictGuardrails`.
3. Before broad codebase exploration, choose the graph entrypoint that matches the task:
   - architecture, onboarding, bug-scoping, or multi-file task context: `context-project-graph.ps1`
   - exact symbol, shader, subsystem, or file lookup: `query-project-graph.ps1`
4. Read the graph-surfaced files first.
5. Use native search, direct file reads, tests, and builds only after the graph narrows the search space or misses.
6. Retry misses with exact project terminology before widening:
   - exact symbols like `ChartGraph`, `fieldGradientSummary`, or `write_chart_metric_identity_buffers`
   - snake_case and camelCase variants
   - pipeline labels, shader names, and file stems
7. Force a refresh after major refactors, subsystem moves, or when the build changed significantly:
   - `powershell.exe -ExecutionPolicy Bypass -File "${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\ensure-project-graph.ps1" -ProjectRoot "<root>" -Force`

## Command Paths

- `context-project-graph.ps1`: `${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\context-project-graph.ps1`
- `query-project-graph.ps1`: `${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\query-project-graph.ps1`
- `ensure-project-graph.ps1`: `${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\ensure-project-graph.ps1`

## Resources

- Read [references/project-pack.md](references/project-pack.md) when adapting the shared pack layout, explaining how Codex and Claude share the same graph state, or installing the repo-local Claude plugin.
- Read [references/query-strategy.md](references/query-strategy.md) when tuning misses, alias retries, or deciding when native search is justified.
