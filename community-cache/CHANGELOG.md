# Community Cache Changelog

## 2026-03-26-deep-research

- Deep-research snapshot built from comprehensive cross-referencing of official VS Code Copilot documentation (12 pages), GitHub Copilot docs, awesome-copilot community collection (40+ patterns), and real-world open-source repo examples (PowerToys, desktop/desktop, dotnet/macios).
- Expanded from 5 to 11 data packs: prompting-principles (25), application-practices (20), anti-patterns (18), workflow-patterns (10), official-sources (12), public-example-sources (13), frontmatter-reference, hooks-reference, community-resources, deprecations (5), and search-index.
- All evidence references now include verified URLs pointing to real documentation pages and repository files.
- Covers all six Copilot customization primitives: instructions, agents, skills, prompts, hooks, and subagents.
- Added search-index.json for indexed lookups by topic, applicability, kind, and cross-reference.
- Added frontmatter-reference.json documenting all file formats, required fields, file locations, and alternate formats (Claude, agentskills.io).
- Added hooks-reference.json documenting all 8 lifecycle events, input/output formats, exit codes, command properties, and security considerations.
- Added community-resources.json documenting awesome-copilot, anthropics/skills, agentskills.io, official docs, VS Code commands, and usage guidelines.
- Added deprecations.json tracking 5 known deprecated/fragile patterns with severity and replacement info.
- Updated submit flow to extract multiple conclusions per audit (3-10) instead of one.
- Added rebuild-snapshot mode to consolidation pipeline for auto-merging promoted candidates.
- Updated GitHub Actions workflow to auto-rebuild snapshots when promoted candidates exist.
- Updated research skill with cache consumption quick-path and indexed lookup instructions.

## 2026-03-12-initial

- Seeded the first public snapshot from March 2026 external research across official GitHub Copilot and VS Code guidance, public example repositories, and larger public pattern scans.
- Promoted only generalized Copilot customization principles and general application advice that stayed privacy-safe outside the source workspaces.
- Published the first bootstrap packs for official sources, public example sources, prompting principles, application practices, and anti-patterns.
