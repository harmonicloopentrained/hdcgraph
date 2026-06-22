# Claude Handoff

Use this when you want another Claude Code session to install or extend the transferable Rust/WGSL graph workflow.

## Prompt To Give Claude

```text
Use this GitHub repository or a checked-out local clone as the source of truth.

Install its Rust HDC graph workflow into the current repository by:
1. Running scripts/install-into-project.ps1 from the transfer pack against this repo.
2. Verifying that .codex/rust-hdc-graph exists.
3. Verifying that .claude/skills/rust-hdc-graph exists.
4. Verifying that AGENTS.md and CLAUDE.md contain the rust-hdc-graph guardrail blocks.
5. If I ask for strict enforcement, also verify that .claude/settings.json has the PreToolUse graph-first hook.
6. Use the shared graph pack for architecture and symbol navigation work before broad search.

Do not redesign the workflow. Extend it only if you can explain exactly how your change preserves the shared .codex graph pack contract.
```

## Expected End State

- `<repo>\.codex\rust-hdc-graph\`
- `<repo>\.claude\skills\rust-hdc-graph\`
- `AGENTS.md` contains the shared graph-first block
- `CLAUDE.md` imports `@AGENTS.md` through the Claude bridge block
- Optional: `.claude/settings.json` contains the strict `PreToolUse` hook

## Verification Checklist

Ask Claude to verify these paths and behaviors:

1. `graph.json` and `SUMMARY.md` exist under `.codex/rust-hdc-graph\`
2. The plugin manifest exists at `.claude/skills/rust-hdc-graph/.claude-plugin/plugin.json`
3. The manual skills exist under `.claude/skills/rust-hdc-graph/skills/`
4. The plugin hooks file exists at `.claude/skills/rust-hdc-graph/hooks/hooks.json`
5. The tool builds successfully with `cargo test` from `.codex/rust-hdc-graph/tool`
6. A graph query or context call succeeds before Claude starts broad search

## Extension Rules

If Claude needs to evolve the package:

- Keep `.codex/rust-hdc-graph/` as the shared graph authority
- Keep Claude-native behavior in `.claude/skills/rust-hdc-graph/`
- Preserve repo-local scripts so future agents are not blocked on a machine-global install
- Prefer additive docs and scripts over changing the shared graph contract
