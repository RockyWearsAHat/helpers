---
description: "Git Shell Helpers workspace baseline: architecture boundaries, fast orientation, and where to find detailed implementation rules."
---

# Workspace Baseline

Git Shell Helpers (`github-shell-helpers`) combines shell-based git helpers, Copilot customization automation, MCP servers, and a community-backed research cache.

## Non-Negotiable Boundary

- `copilot-config/` is product source shipped to users.
- `.github/` is repository development config and supporting project assets.

When updating shipped Copilot customization behavior, edit `copilot-config/`. When updating repository workflows, docs, and governance assets, edit `.github/`.

## Fast Orientation

- Shell commands: repo root scripts (`git-upload`, `git-checkpoint`, etc.)
- MCP servers: `git-research-mcp`, `git-shell-helpers-mcp`, `vision-tool/`
- VS Code extension: `vscode-extension/`
- Community cache data plane: `community-cache/`
- Repo-only local reference cache: `.github/devops-audit-community-cache/`

## Detailed Sources of Truth

- Copilot system assets: `copilot-config/agents/`, `copilot-config/skills/`, `copilot-config/instructions/`, `copilot-config/prompts/`
- Build/test/release workflows: `.github/workflows/`
- Knowledge corpus and philosophy: `knowledge/`, `knowledge/knowledge-philosophy.md`
- Community cache automation scripts: `scripts/community-cache-*.sh`, `scripts/community-research-submit.sh`

## Engineering Defaults

- Prefer MCP tools over shell emulation when tool coverage exists.
- Run strict diagnostics before and after edits.
- Keep command/man page/installer behavior aligned in the same change.
- Treat remote repository state as canonical for community-driven data; local cache copies are accelerators, not source of truth.
