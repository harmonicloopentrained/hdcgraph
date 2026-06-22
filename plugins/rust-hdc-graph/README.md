# Rust HDC Graph Plugin

Installable Claude Code plugin for graph-first Rust/WGSL codebase navigation.

## Skills

- `/rust-hdc-graph:bootstrap`
- `/rust-hdc-graph:context`
- `/rust-hdc-graph:query`
- `/rust-hdc-graph:refresh`

The non-user-invoked `graph-first` skill biases Claude toward the project-local HDC graph before broad file search.

## Runtime assumptions

- Windows + PowerShell scripts.
- Rust toolchain available in target projects when building or refreshing the graph pack.
- Shared graph state lives under `.codex/rust-hdc-graph/` in the target project.
