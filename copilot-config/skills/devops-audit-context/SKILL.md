---
name: devops-audit-context
description: "Builds a thorough understanding of the workspace for the audit pipeline."
user-invocable: false
---

# DevOps Audit Context

Build a complete picture of this workspace so the rest of the audit pipeline has accurate, detailed project context to work with.

This is the foundation of the entire audit. Every other agent — research, evaluation, implementation — depends on what you produce here. If you miss something or get something wrong, every decision after this will be based on incomplete or incorrect information.

## What You Are Doing

You are reading the workspace to understand two things:

1. **What is this project?** — What does it do, what is it built with, how is it built, how is it tested, what do developers actually do here day to day.
2. **What Copilot files currently exist in scope for this audit?** — Every file in the selected Copilot target surface, read completely, with an accurate summary of what each one contains and does.

You are not judging anything. You are not deciding what is right or wrong. You are just reading and reporting accurately.

## How to Read the Project

Read these files to understand the project:

- `.github/copilot-instructions.md` if it exists — it may declare where the repo's Copilot source of truth lives
- `README.md` — what the project is and does
- Build files (`Makefile`, `CMakeLists.txt`, `package.json`, `Cargo.toml`, `pyproject.toml`, etc.) — what it is built with and how
- Test configuration or test scripts — how tests are run
- CI configuration (`.github/workflows/`) — what the automated pipeline looks like
- `CONTRIBUTING.md` if it exists — how developers are expected to work
- The top-level directory structure — what the project is organized like

Do not read every source file. You just need to understand the shape of the project — its type, languages, frameworks, build process, test process, and developer workflow.

## Determine The Audit Target Surface

Before inventorying files, determine which Copilot surface this audit is actually about.

- If the user explicitly names `.github/`, audit the workspace-runtime surface.
- If the user explicitly names `copilot-config/`, shipped prompts, shipped agents, shipped instructions, or the audit workflow's own source files, audit the product-source surface.
- If the repo's baseline instructions say Copilot product source lives outside `.github/`, treat that as the default source-of-truth surface unless the user explicitly asked for the runtime `.github/` copy instead.
- If truthfulness of the workflow depends on runtime or installed copies in addition to product source, include those as supporting surfaces and say why.

Return the chosen target surface in the project profile, including the path or paths in scope and the reason for the choice.

## How to Read Copilot Files In Scope

Read every single one of these completely:

- If the selected surface is `.github/`:
  - `.github/copilot-instructions.md` — the main Copilot instructions file
  - `.github/instructions/*.instructions.md` — scoped instruction files
  - `.github/agents/*.agent.md` — custom agent definitions
  - `.github/prompts/*.prompt.md` — prompt files / slash commands
  - `.github/skills/*/SKILL.md` — skill definitions
- If the selected surface is `copilot-config/` or another repo-defined source directory, read the equivalent instructions, agents, prompts, and skills there.

Do not expect the global DevOpsAudit audit tools to appear here. `DevOpsAudit`, `DevOpsAuditContext`, `DevOpsAuditResearch`, `DevOpsAuditEvaluate`, `DevOpsAuditImplement`, and `/copilot-devops-audit` may be installed in standard user-level locations on disk rather than inside the workspace. Their absence from the selected workspace surface is normal and must not be reported as missing.

For each file, note:

- The full file path
- The YAML frontmatter (every field and value, exactly as written)
- A summary of the content (what it tells Copilot to do)
- The `applyTo` pattern if present (what files this applies to)
- Anything that looks unusual, broken, or out of place

Do not skip files. Do not skim. Read them all. The evaluator will need every detail.

Keep the returned report compact. Summarize each file clearly, but do not paste full file contents into the output unless a broken frontmatter snippet is needed as evidence.

## Output

Return a structured context report with two sections:

### Project Profile

```
PROJECT PROFILE
- Project name: (name of the project)
- Project type: (CLI tool, web app, library, desktop app, etc.)
- Purpose: (one or two sentences about what it does)
- Languages: (primary and secondary languages)
- Frameworks: (Qt, React, Flask, etc. — or none)
- Build system: (Make, CMake, npm, cargo, etc.)
- Build command: (the actual command to build)
- Test command: (the actual command to test)
- CI/CD: (what CI is configured, if any)
- Directory structure: (brief layout — src/, tests/, docs/, etc.)
- Key developer workflows: (what developers actually do — build, test, deploy, etc. BE SPECIFIC, THESE ARE GREAT JUMP OFFS FOR RESEARCH AND FINDING IMPROVEMENTS)
- User focus: (if the user specified a focus area like "qt qss", note it here)
```

### Copilot File Inventory

For each Copilot-related file found in the selected target surface:

```
FILE: .github/instructions/example.instructions.md
FRONTMATTER:
  applyTo: "**/*.cpp"
  description: "C++ coding standards"
CONTENT SUMMARY: Describes coding style for C++ files, covers naming conventions and error handling patterns.
NOTES: (anything unusual — empty file, broken frontmatter, references to things that don't exist in the project, etc.)
```

List every file. If no Copilot files exist in the selected target surface, say so clearly — that is important information.

End with a short coverage summary that states the total number of Copilot files inventoried.

## Rules

- Do not judge or evaluate. Just read and report.
- Do not edit any files.
- Do not research Copilot best practices. That is the research agent's job.
- Be accurate. If you are not sure about something, say so.
- If a file has broken YAML frontmatter that you cannot parse, report it as-is and note it is broken.
- Do not report global audit tooling as missing just because it is not in the workspace.
