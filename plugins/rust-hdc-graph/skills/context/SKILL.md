---
description: Gather task-scoped Rust/WGSL graph context from the shared HDC graph pack for the current project.
disable-model-invocation: true
---

Resolve the project root, then gather task-level context from the shared graph pack before reading files broadly.

1. Use the user-supplied arguments as the task string.
2. Run:
   - `powershell.exe -ExecutionPolicy Bypass -File "${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\context-project-graph.ps1" -ProjectRoot "<root>" -Task "$ARGUMENTS"`
3. Summarize the most relevant graph hits.
4. Read the graph-surfaced files first.
5. Only widen to native search if the graph misses or needs exact-line confirmation.
