---
description: Force-refresh the shared Rust/WGSL HDC graph pack for the current project after a major refactor or build change.
disable-model-invocation: true
---

Resolve the project root, then force-refresh the shared graph pack.

1. Run:
   - `powershell.exe -ExecutionPolicy Bypass -File "${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\ensure-project-graph.ps1" -ProjectRoot "<root>" -Force`
2. Report whether the graph refreshed successfully and where the updated `graph.json` lives.
3. Recommend rerunning `/rust-hdc-graph:context` or `/rust-hdc-graph:query` after the refresh if the user is about to continue navigation work.
