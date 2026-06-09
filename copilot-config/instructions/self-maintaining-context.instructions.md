---
applyTo: "copilot-config/**,.github/**,*.md,scripts/**,vscode-extension/**,git-*"
description: "Keep project context files current when structure changes."
---

# Self-Maintaining Context (Lean)

Goal: maximize Copilot signal while minimizing per-request token overhead.

## Core Rule

When making structural changes, update `.github/copilot-instructions.md` in the same change.

Structural means architecture, build/test commands, top-level layout, core conventions, or instruction/skill routing behavior.

## Context Budget Policy

`copilot-instructions.md` must stay minimal.

- Keep only durable, repo-wide essentials there.
- Move detailed procedures, edge cases, and domain playbooks to focused docs (for example `knowledge/`, `copilot-config/instructions/`, `copilot-config/skills/`, and module READMEs).
- In `copilot-instructions.md`, prefer short pointers to those docs instead of inlining long guidance.
- Remove stale or duplicated guidance immediately when source-of-truth docs already cover it.

## Documentation Maintenance Scope

Maintenance is not "expand as needed". It is continuous organization across ALL project docs:

- prune obsolete content,
- deduplicate overlapping guidance,
- keep one clear source of truth per topic,
- preserve discoverability with concise pointers/indexes.

## Required Outcome Per Structural Change

1. Global instructions remain short and high-signal.
2. Detailed guidance lives in targeted docs.
3. Cross-references are valid and current.
4. No net increase in global instruction noise without clear repo-wide value.
