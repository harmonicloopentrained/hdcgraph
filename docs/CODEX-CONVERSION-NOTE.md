# Codex Conversion Note

This repository is intentionally Claude-first. It should not be treated as a native Codex plugin without conversion.

Suggested prompt for Codex:

> Convert this Claude Code plugin repository into a native Codex plugin/skill package. Preserve the plugin name, author attribution, license, NOTICE, and public credit metadata. Do not duplicate an already-installed Rust HDC Graph package in this environment; create a clean Codex-native package only if one is missing.

Expected conversion work:

- Add Codex-native plugin metadata.
- Convert Claude-specific hook/skill assumptions where needed.
- Preserve shared project graph state under `.codex/rust-hdc-graph/`.
- Preserve `LICENSE`, `NOTICE`, `AUTHORS.md`, and `CITATION.cff`.
