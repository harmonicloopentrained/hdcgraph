# Publishing Guide

## GitHub setup

1. Create a new public repository, for example `rust-hdc-graph-claude`.
2. Copy this repo folder into it.
3. Replace all `YOUR_GITHUB` placeholders.
4. Commit and push.
5. Add a `v1.0.0` release tag.

## Claude installation command for users

```bash
claude plugin marketplace add YOUR_GITHUB/rust-hdc-graph-claude --scope user
claude plugin install rust-hdc-graph@rust-hdc-graph-claude --scope user
```

## Credit hardening

- Keep `LICENSE`, `NOTICE`, `AUTHORS.md`, and `CITATION.cff` in the repo.
- Put the same author/homepage/repository fields in both marketplace and plugin manifests.
- Make GitHub releases from signed or traceable commits when possible.
- Connect the repo to Zenodo before making a release if you want a DOI.
