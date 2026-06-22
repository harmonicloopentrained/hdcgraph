# Rust HDC Graph for Claude Code

Claude Code plugin and marketplace repo for **Rust HDC Graph**: a graph-first Rust/WGSL codebase navigation workflow that keeps a shared project-local graph pack under `.codex/rust-hdc-graph/` and exposes Claude-native skills/hooks for context gathering, exact symbol lookup, refresh, and project bootstrap.

> Claude-first release. This repository is intentionally formatted for Claude Code. Codex users should ask Codex to convert this Claude plugin into a Codex plugin/skill package first, rather than installing this repo directly as-is.

## Credit / attribution

Created by **Dylan Henry**. Replace `YOUR_GITHUB` in the manifests with your actual GitHub owner before publishing.

Recommended public identity fields to update before release:

- `.claude-plugin/marketplace.json`
- `plugins/rust-hdc-graph/.claude-plugin/plugin.json`
- `CITATION.cff`
- `package-metadata.json`

## Repository layout

```text
.claude-plugin/marketplace.json          # Claude marketplace manifest
plugins/rust-hdc-graph/                  # Installable Claude plugin
  .claude-plugin/plugin.json             # Plugin manifest
  skills/                                # Claude skills
  hooks/hooks.json                       # Soft graph-first reminders
  scripts/                               # Plugin runtime/install scripts
  assets/project-template/               # Shared Rust graph-pack template
docs/                                    # Handoff and conversion notes
scripts/install-into-project.ps1         # Convenience installer from repo root
```

## What the plugin installs into a project

When bootstrapped into a target repository, it installs:

```text
<repo>/.codex/rust-hdc-graph/            # Shared graph pack and Rust extractor
<repo>/.claude/skills/rust-hdc-graph/    # Optional repo-local Claude plugin copy
<repo>/AGENTS.md                         # Graph-first agent block
<repo>/CLAUDE.md                         # Claude-specific graph-first block
<repo>/.claude/settings.json             # Optional strict guardrail hook
```

## Install from a GitHub marketplace repo

After publishing this repo, users can add it as a Claude Code plugin marketplace:

```bash
claude plugin marketplace add YOUR_GITHUB/rust-hdc-graph-claude --scope user
claude plugin install rust-hdc-graph@rust-hdc-graph-claude --scope user
```

Claude Code also supports discovering/installing plugins from marketplaces through `/plugin` in the app.

## Install from a local checkout

```powershell
git clone https://github.com/YOUR_GITHUB/rust-hdc-graph-claude.git
cd rust-hdc-graph-claude
claude plugin marketplace add . --scope user
claude plugin install rust-hdc-graph@rust-hdc-graph-claude --scope user
```

Restart Claude Code after first global install so new sessions load the plugin and hooks.

## Bootstrap a target Rust/WGSL repo

From this repository root:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\scripts\install-into-project.ps1 -ProjectRoot C:\path\to\repo
```

For stricter graph-first enforcement that blocks broad search before graph context/query has been run:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\scripts\install-into-project.ps1 -ProjectRoot C:\path\to\repo -StrictGuardrails
```

Inside Claude Code, after the plugin is enabled, the skills are:

```text
/rust-hdc-graph:bootstrap
/rust-hdc-graph:context
/rust-hdc-graph:query
/rust-hdc-graph:refresh
```

## Publish checklist

Before making the GitHub repo public:

1. Replace every `YOUR_GITHUB` placeholder.
2. Choose and confirm the license. This package currently ships with MIT because it is the cleanest default for public installation and reuse with attribution.
3. Run the validators locally:

   ```bash
   claude plugin validate ./plugins/rust-hdc-graph
   claude plugin marketplace validate .
   ```

4. Run the Rust tool checks from a bootstrapped project:

   ```bash
   cd <repo>/.codex/rust-hdc-graph/tool
   cargo test
   cargo run -- --help
   ```

5. Tag a release:

   ```bash
   git tag v1.0.0
   git push origin main --tags
   ```

6. Optionally archive the GitHub release through Zenodo for a DOI if you want citation-grade credit.

## Platform support

Current scripts target **Windows + PowerShell**. The plugin can live in a public GitHub marketplace repo, but non-Windows users should treat it as a reference package until shell equivalents are added.

## Codex note

This is not packaged as a Codex plugin. Keep it Claude-first so the installed, working Rust HDC Graph plugin is not duplicated or confused with your existing Codex setup. For Codex conversion, point Codex at this repo and ask it to create a native Codex plugin/skill package from the Claude plugin structure.
