<!-- rust-hdc-graph-claude:start -->
@AGENTS.md

## Claude Code

- The shared Rust/WGSL graph pack lives at `.codex/rust-hdc-graph/`.
- If the repo-local plugin is present under `.claude/skills/rust-hdc-graph/`, prefer `/rust-hdc-graph:context`, `/rust-hdc-graph:query`, `/rust-hdc-graph:refresh`, or the automatically invoked graph-first skill before broad search.
- Retry misses with exact symbols, snake_case and camelCase variants, shader labels, pipeline names, or file stems before widening search.
<!-- rust-hdc-graph-claude:end -->
