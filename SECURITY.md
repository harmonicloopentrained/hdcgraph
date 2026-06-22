# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| 1.x | Yes |

## Reporting a vulnerability

Open a private security advisory on GitHub when the repository is public, or contact the maintainer through the repository profile.

## Operational notes

This plugin installs PowerShell hooks and scripts that run inside Claude Code workflows. Review all scripts before enabling global or strict guardrail mode in sensitive repositories.

Strict guardrails are optional. They are intended to constrain broad search until graph context/query has been run, not to provide a general sandbox.
