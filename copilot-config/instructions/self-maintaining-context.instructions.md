---
applyTo: "**"
description: "Auto-generate and maintain copilot-instructions.md as a side effect of normal work. Zero user effort."
---

# Self-Maintaining Project Context

## On Scaffold

When creating a new project, generate `copilot-instructions.md` alongside the code. Include: project type, languages, frameworks, build command, test command, directory layout, key conventions. This is a build artifact, not a separate task.

## On Structural Change

When a change alters the project's architecture, build process, directory layout, dependencies, or conventions, update `copilot-instructions.md` to reflect the current state. Do this silently as part of the commit — never ask the user to maintain this file.

## What Counts as Structural

- New module, package, or top-level directory
- Changed build or test command
- New or removed dependency that changes how the project works
- New convention (naming, file organization, API pattern)
- Changed framework or language version with behavioral impact

## What Does Not Count

- Adding a file to an existing module
- Bug fixes that don't change architecture
- Content or copy changes
- Dependency version bumps with no API change

## Keep It Short

The file should be readable in 30 seconds. If it's longer than ~60 lines, trim it. Prefer terse facts over explanatory prose. The goal is accurate context, not documentation.

## Knowledge Note Maintenance

When structural changes happen (new modules, changed dataflows, new MCP tools, architecture changes), also update the relevant knowledge notes — not just `copilot-instructions.md`.

- Use `search_knowledge_index` to find architecture notes before diving into raw code reads
- Use `update_knowledge_note` to target specific sections by heading rather than rewriting whole files
- Architecture notes (`architecture-gsh-internals.md`, `architecture-gsh-user-guide.md`) should reflect the current module inventory and tool surface
- Do not update knowledge notes for routine bug fixes or content changes — only for structural shifts

This is the same trigger as `copilot-instructions.md` — if the change is structural, both the project context file and the relevant knowledge notes should be updated in the same commit.
