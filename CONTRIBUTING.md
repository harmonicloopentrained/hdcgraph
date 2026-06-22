# Contributing

This project is Claude-first. Keep changes compatible with the Claude Code plugin/marketplace structure unless a separate branch intentionally targets another agent runtime.

## Expectations

- Preserve the shared graph-pack layout under `.codex/rust-hdc-graph/`.
- Keep Claude plugin files under `plugins/rust-hdc-graph/`.
- Do not add Codex manifests to this repository unless the maintainer explicitly decides to publish a dual-target package.
- Validate plugin and marketplace manifests before release.
- Keep Windows/PowerShell support working until shell equivalents are added.
