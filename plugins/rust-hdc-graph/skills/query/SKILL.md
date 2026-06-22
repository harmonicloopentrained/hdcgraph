---
description: Query the shared Rust/WGSL HDC graph pack for exact symbols, shaders, files, or subsystem names in the current project.
disable-model-invocation: true
---

Resolve the project root, then use the shared graph pack for the exact lookup the user provided.

1. Use the user-supplied arguments as the lookup needle.
2. Run:
   - `powershell.exe -ExecutionPolicy Bypass -File "${CLAUDE_SKILL_DIR}\\..\\..\\scripts\\query-project-graph.ps1" -ProjectRoot "<root>" -Needle "$ARGUMENTS"`
3. Summarize the strongest matches.
4. Read the returned files first.
5. Retry misses with exact symbols, snake_case and camelCase variants, pipeline labels, shader names, or file stems before widening search.
