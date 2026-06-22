# Query Strategy

Use this order when the first graph lookup misses:

1. Exact symbol names
   - `ChartGraph`
   - `fieldGradientSummary`
   - `write_chart_metric_identity_buffers`
2. Stable snake_case or camelCase field names
3. Pipeline labels and shader names
4. File stems and subsystem nouns
5. Free-form natural language only after trying the above

Retrieval discipline:

- Use `context` for task framing and `query` for focused lookups.
- Read graph-surfaced files first.
- Use `rg`, `Grep`, `Glob`, and direct file reads to confirm exact lines, not to start broad discovery when the graph is healthy.
- Treat repeated misses as signal to improve aliases or extraction rules.
- When strict Claude guardrails are enabled, broad search should stay blocked until `query` or `context` marks the session as graph-warmed.
