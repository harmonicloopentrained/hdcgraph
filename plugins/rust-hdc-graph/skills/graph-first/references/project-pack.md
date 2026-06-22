# Shared Project Pack

The transferable workflow keeps the authoritative graph pack inside the repository at:

```text
<project>/.codex/rust-hdc-graph/
```

Claude-specific automation can also install a repo-local plugin at:

```text
<project>/.claude/skills/rust-hdc-graph/
```

That split is intentional:

- `.codex/rust-hdc-graph/` stores the shared graph data, extractor source, cache, and project-local scripts.
- `.claude/skills/rust-hdc-graph/` stores Claude-native skills and hooks that point back at the shared graph pack.

Shared graph pack layout:

- `tool/`
  - vendored Rust extractor source and build output
- `scripts/`
  - `ensure_graph.ps1`
  - `query_graph.ps1`
  - `context_graph.ps1`
- `graph.json`
- `SUMMARY.md`
- `query-cache.json`
- `state/claude/`
  - Claude session stamps written after `query` or `context`

Claude repo-local plugin layout:

- `.claude-plugin/plugin.json`
- `skills/`
- `hooks/hooks.json`
- `scripts/`
  - reminder and strict-guardrail helpers

Why this layout exists:

- Codex and Claude can share the same graph artifacts instead of generating separate caches.
- The extractor stays vendored into the repo, so future agents can keep using it after bootstrap.
- `AGENTS.md` and `CLAUDE.md` can both point at project-local scripts instead of machine-specific paths.
- Optional strict Claude hooks can verify whether graph `query` or `context` ran earlier in the session before allowing broad search.

Bootstrap rules:

1. Copy `assets/project-template/` into `.codex/rust-hdc-graph/`.
2. Copy the Claude plugin into `.claude/skills/rust-hdc-graph/` when the repo should be self-contained for Claude sessions.
3. Install or refresh the guardrail block in `AGENTS.md`.
4. Install or refresh the Claude bridge block in `CLAUDE.md`.
5. Build the extractor on first use or when the tool source is newer than the binary.
6. Refresh the graph when it is missing, incompatible, or older than any `*.rs` or `*.wgsl` source file outside ignored folders.
