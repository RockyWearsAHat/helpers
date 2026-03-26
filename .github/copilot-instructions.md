---
description: "Git Shell Helpers — shell-based git subcommands, DevOps audit system, MCP servers, community cache, and VS Code extension. Product source in copilot-config/, project config in .github/."
---

# Project Identity

Git Shell Helpers (`github-shell-helpers`, v0.3.4) is a suite of shell-based git subcommands, a multi-agent DevOps Audit system for Copilot customization, MCP tool servers, a crowdsourced community cache, and a VS Code extension. The GitHub repo is `RockyWearsAHat/github-shell-helpers`.

# Critical Separation: Product Source vs. Project Config

This distinction is the single most important architectural boundary in the repo:

- **`copilot-config/`** — PRODUCT SOURCE CODE. Contains agents, instructions, skills, and prompts that ship to end users. The `git copilot-devops-audit --update-agent` command installs these globally to `~/.copilot/` and VS Code prompt locations. **Never confuse these with project-level configuration.**
- **`.github/`** — PROJECT DEVELOPMENT CONFIG. Contains CI/CD workflows, the knowledge base, commit guidelines, and this instructions file. These help develop the repo itself; they are not shipped to users.

When modifying Copilot customization files (agents, skills, instructions, prompts), work in `copilot-config/`. When modifying CI, project guidance, or the knowledge base, work in `.github/`.

# Architecture Overview

## Shell Commands (the original product)

| Command                    | Shell | Purpose                                                          |
| -------------------------- | ----- | ---------------------------------------------------------------- |
| `git-upload`               | bash  | Stage, commit, push; optional AI commit messages via Copilot CLI |
| `git-checkpoint`           | bash  | AI-committed local checkpoint (never pushes unless `--push`)     |
| `git-get`                  | zsh   | Clone or pull with smart defaults                                |
| `git-initialize`           | zsh   | Init repo, set origin, initial commit, push                      |
| `git-fucked-the-push`      | zsh   | Undo last pushed commit, keep changes staged                     |
| `git-resolve`              | zsh   | Interactive merge/rebase conflict resolution                     |
| `git-remerge`              | zsh   | Merge a branch back; aborts cleanly on conflicts                 |
| `git-copilot-quickstart`   | zsh   | Scaffold `.github/` Copilot workflow for any repo                |
| `git-scan-for-leaked-envs` | zsh   | Scan repo for leaked secrets using Copilot                       |
| `git-help-i-pushed-an-env` | zsh   | Scrub secrets from git history                                   |
| `git-copilot-devops-audit` | zsh   | Install/run the DevOps Audit system                              |

## DevOps Audit System

A 4-phase pipeline (Context → Research → Evaluation → Implementation) for auditing any workspace's Copilot customization. Source lives in `copilot-config/`:

- **Agents**: `copilot-config/agents/` (DevOpsAudit orchestrator + 5 specialist subagents)
- **Skills**: `copilot-config/skills/` (orchestration, context, research, evaluation, fix, community-submit)
- **Instructions**: `copilot-config/instructions/` (router, MCP tools, shell scripts, checkpoint, safety)
- **Prompt**: `copilot-config/prompts/copilot-devops-audit.prompt.md`

## MCP Servers (Node.js)

| Server                  | File                    | Purpose                                          |
| ----------------------- | ----------------------- | ------------------------------------------------ |
| `git-research-mcp`      | `git-research-mcp`      | Web search via SearXNG + knowledge cache tools   |
| `git-shell-helpers-mcp` | `git-shell-helpers-mcp` | Combined server (research + vision + checkpoint) |
| `vision-tool`           | `vision-tool/`          | Image analysis MCP tools                         |

## Community Cache

`community-cache/` stores crowdsourced Copilot customization best practices. Managed by:

- `scripts/community-cache-submit.sh` / `scripts/community-cache-pull.sh`
- `scripts/community-research-submit.sh`
- `community-cache/manifest.json` defines the recommended snapshot

## VS Code Extension

`vscode-extension/` — settings panel for community cache participation and MCP server auto-registration. Built with `scripts/build-vsix.sh`.

## Knowledge Base

`knowledge/` — ~80 reference files covering languages, patterns, CS topics, and ecosystem state. Used by `git-research-mcp`'s `search_knowledge_cache` tool. Treat as informed starting context, not authoritative truth — always verify volatile details against current docs. See `knowledge-philosophy.md` for interpretation guidelines.

# Installation & Distribution

Two installer methods ship the same commands and man pages:

- **macOS .pkg** (`scripts/build-pkg.sh`): Installs to `/usr/local/bin` + `/usr/local/share/man/man1`, runs `--update-agent` postinstall
- **Script installer** (`Git-Shell-Helpers-Installer.sh`): Fetches from GitHub raw URLs to `~/bin`, updates `~/.zshrc`

# Build, Test, Release

## Validation

- **`./scripts/test.sh`** — syntax checks (bash -n, zsh -n) + test suites. Run before every commit.
- All checks must pass: `TEST_SUMMARY: pass N/N`

## Build

- Script installer: `./scripts/build-dist.sh` → `dist/Git-Shell-Helpers-Installer*.sh`
- macOS package: `./scripts/build-pkg.sh` → `dist/github-shell-helpers-<version>.pkg`
- VS Code extension: `scripts/build-vsix.sh` → `vscode-extension/*.vsix`

## Release Process

1. Bump `VERSION` (semver, must be ≥ previous release)
2. Add `release-notes/v<version>.md` (required by CI)
3. Push to `main` — CI builds both installers, creates GitHub Release with release notes body

## CI Workflows (`.github/workflows/`)

- `build-installer.yml` — builds script + pkg installers, creates releases on main push
- `test.yml` — runs test suite
- `community-cache-consolidate.yml` / `community-cache-validate.yml` — community cache CI
- `knowledge-audit.yml` — automated knowledge base auditing (disabled; replaced by local `@AuditKBCC` agent)
- `knowledge-index-rebuild.yml` — rebuilds TF-IDF search index on knowledge file changes

# Shell Conventions

- **bash** for portable scripts: `git-upload`, `git-checkpoint`, `scripts/*.sh`
- **zsh** for interactive commands: `git-get`, `git-initialize`, `git-resolve`, `git-fucked-the-push`, `git-copilot-devops-audit`, `Git-Shell-Helpers-Installer.sh`
- **Node.js** (≥18) for MCP servers: `git-research-mcp`, `git-shell-helpers-mcp`, `vision-tool/`
- All shell scripts use `set -euo pipefail`

# Alignment Rules

- When changing command flags, names, or help text: update the command, its man page (`man/man1/`), and both installers in the same commit
- When changing audit agents/skills/instructions: work in `copilot-config/`, test with `git copilot-devops-audit --update-agent --force`
- When changing MCP tool interfaces: update the tool handler, the README MCP tools table, and any knowledge notes that reference the tool

# Project Direction

The project is actively evolving along these axes:

1. **Shell helpers** — stable core, incremental improvements
2. **DevOps Audit** — the primary development focus; improving the 4-phase pipeline, community cache integration, and research quality
3. **MCP tooling** — research server + combined server; extending tool capabilities
4. **Community cache** — crowdsourced Copilot customization knowledge; submission/pull infrastructure
5. **VS Code extension** — settings UI, MCP auto-registration

Regressions to watch: audit quality going backward, community cache data integrity, installer breakage, man page drift from command behavior.
