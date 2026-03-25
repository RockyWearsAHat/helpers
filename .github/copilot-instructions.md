---
description: "Repository baseline for shell-based Git helper commands, installer scripts, packaging assets, and man pages."
---

# Repository Baseline

- This repository ships shell-based Git helper commands, installer scripts, packaging assets, and man pages. Most code changes are in shell scripts and release packaging files.
- Use `./scripts/test.sh` for repo sanity checks. When installer behavior changes, build the script installer with `./scripts/build-dist.sh` and the macOS package with `./scripts/build-pkg.sh`.
- Keep user-facing command behavior, installer scripts, and matching man pages aligned when changing flags, command names, or help text.
- Prefer small, repo-specific Copilot guidance. Add prompts, agents, skills, or hooks only when a repeated workflow clearly benefits from them.
- When editing `.github/` customization files, keep repo-wide instructions factual, keep prompts as manual entrypoints, keep agents focused on role and tool boundaries, and keep step-by-step methodology in skills.

# Knowledge Base & Community Cache

The `.github/knowledge/` directory and `community-cache/` directory serve different roles and should be used together.

## Knowledge Base (`.github/knowledge/`)

Searchable reference material — language specifics, systems patterns, CS topics, ecosystem state. Use these as a **starting point** for any research so the model doesn't begin from zero. See `knowledge-philosophy.md` for how to interpret and weight these files. Key rule: treat knowledge files as informed starting context, not authoritative truth — always verify volatile details (library APIs, toolchain versions) against current docs.

## Community Cache (`community-cache/`)

Copilot customization rules — how to structure `.github/` directories, write instructions/agents/skills, configure frontmatter, avoid anti-patterns. The recommended snapshot is defined in `community-cache/manifest.json`. Load the community cache when working on Copilot customization, audit tasks, or `.github/` file design.

## How They Connect

- **Building a project in language X?** Search the knowledge base for that language's reference file first, then research current ecosystem state.
- **Configuring Copilot customization?** Load the community cache snapshot for principles, practices, and anti-patterns.
- **Running an audit?** The community cache provides the evaluation criteria; the knowledge base provides technical grounding for recommendations.
