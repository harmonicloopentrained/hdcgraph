<!-- rust-hdc-graph:start -->
# Rust HDC Graph First

Use the project-local graph pack in `.codex/rust-hdc-graph/` before broad codebase exploration.

## Required Flow

1. Ensure the graph is current:
   - `.codex/rust-hdc-graph/scripts/ensure_graph.ps1 -ProjectRoot .`
2. For task-level context, start with:
   - `.codex/rust-hdc-graph/scripts/context_graph.ps1 -ProjectRoot . -Task "<task>"`
3. For symbol or subsystem lookup, start with:
   - `.codex/rust-hdc-graph/scripts/query_graph.ps1 -ProjectRoot . -Needle "<needle>"`
4. Read the graph-surfaced files first, then verify exact lines with native search and direct file reads.

## Guardrails

- Use graph context/query first for architecture, shader wiring, data flow, file discovery, and task scoping.
- Retry misses with exact project terms, snake_case and camelCase variants, pipeline names, shader labels, or file stems before widening the search.
- Use native search only after the graph narrows the search space or when the graph misses.
- Refresh the graph after meaningful refactors.
- Never paste the full `graph.json` into model context.
<!-- rust-hdc-graph:end -->
