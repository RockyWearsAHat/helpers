# Copilot Customization Studybase

_Last verified: 2026-03-25_

Verified baseline for the research phase. Use as a starting point — verify freshness of normative claims against live sources before recommending.

## Six Primitives

| Primitive    | File                                           | Purpose                                            | Boundary                                              |
| ------------ | ---------------------------------------------- | -------------------------------------------------- | ----------------------------------------------------- |
| Instructions | `.instructions.md` / `copilot-instructions.md` | Ambient project context injected into every prompt | Facts about the project, NOT step-by-step methodology |
| Agents       | `.agent.md`                                    | Role definition + tool boundaries                  | Identity and constraints, NOT detailed how-to         |
| Skills       | `SKILL.md` in a named folder                   | Step-by-step methodology, loaded on demand         | Procedures and templates, NOT role or identity        |
| Prompts      | `.prompt.md`                                   | Thin slash-command entrypoints                     | Route intent to an agent, NOT carry the methodology   |
| Hooks        | `.json` in hooks folder                        | Deterministic lifecycle automation                 | Guaranteed execution, NOT guidance                    |
| Handoffs     | `handoffs:` in agent frontmatter               | Structured agent-to-agent transfer                 | Carry plan + context across agents                    |

## Current Frontmatter Syntax

### .instructions.md

```yaml
---
description: "Human-readable purpose"
applyTo: "**/*.ts"
---
```

No required fields. `copilot-instructions.md` needs no frontmatter.

### .agent.md

```yaml
---
name: "AgentName"
description: "What this agent does"
tools:
  - read/readFile
  - edit/editFiles
  - execute/runInTerminal
agents:
  - TargetSubagent
user-invocable: true
---
```

Key fields: `description` (essential for discovery), `tools` (least-privilege), `agents` (subagent allowlist). `model:` optional — avoid hard-coding.

### SKILL.md

```yaml
---
name: skill-name
description: "When to use this skill and when not to"
user-invocable: false
---
```

Both `name` and `description` required. Discovery depends on description quality.

### .prompt.md

```yaml
---
description: "What this command does"
agent: TargetAgent
tools:
  - read/readFile
---
```

**`agent:` is correct. `mode:` is DEPRECATED and silently prevents registration.**

## Tool Name Format

Tools use `category/tool` format: `read/readFile`, `search/textSearch`, `search/fileSearch`, `web/fetch`, `edit/editFiles`, `execute/runInTerminal`, `agent/runSubagent`.

Category shorthand (`read`, `search`, `edit`, etc.) enables all tools in that category. Use specific names for least-privilege; categories for broad access.

## Native vs Custom Boundary

**Copilot handles natively:**

- Chat completions in any language
- Language-aware code generation
- Open-file context awareness
- Common conventions the model already knows

**Requires custom files when:**

- Enforcing project-specific patterns → Instructions
- Restricting tool access by role → Agents
- Packaging reusable methodology → Skills
- Creating thin user entrypoints → Prompts
- Guaranteeing deterministic execution → Hooks
- Sequencing multi-phase workflows → Agents + Handoffs/Subagents

## File-Type Purpose Violations (Top Anti-Pattern)

The #1 architectural mistake: putting task methodology in agent files. The model reads the instructions, sees it has the tools, and does the work itself instead of loading the skill.

- **Agent with task instructions** → model bypasses skill, does work inline
- **Skill with identity shaping** → confused role boundaries
- **Instruction with step-by-step procedures** → token drag on every prompt
- **Prompt with full methodology** → same as agent stuffing

Fix: agents carry role + tool boundaries only. Skills carry methodology. Instructions carry project facts. Prompts route intent.

## Key Deprecations

| Deprecated                   | Replacement                 | Impact                                                  |
| ---------------------------- | --------------------------- | ------------------------------------------------------- |
| `mode:` in .prompt.md        | `agent:`                    | Breaking — slash command silently fails                 |
| Settings-based instructions  | File-based .instructions.md | Deprecated — still works, will be removed               |
| Hard-coded model names       | Omit or use verified names  | Fragile — model names change                            |
| Complex workflows in prompts | Skills (SKILL.md)           | Soft deprecation — skills preferred for complex methods |

## Discovery Locations

VS Code scans automatically:

- `.github/instructions/`, `.github/agents/`, `.github/skills/`, `.github/prompts/` (workspace)
- `~/.copilot/instructions/`, `~/.copilot/agents/`, `~/.copilot/skills/`, `~/.copilot/prompts/` (user)
- `.claude/agents/`, `.claude/skills/` (Claude-compatible, read natively)

## Verified Repository Patterns

- **desktop/desktop** `deskocat.agent.md` — 5-phase workflow (Understand→Plan→Implement→Verify→Deliver) with risk classification and explicit build verification
- **microsoft/PowerToys** `PlanIssue.agent.md` — Plan extraction + structured handoff to `FixIssue` agent
- **dotnet/macios** `copilot-instructions.md` — Effective always-on instructions: project context without token bloat

## Prompting Quality Principles

Strong prompts define: clear goals, explicit success criteria, truthful boundaries, evidence expectations, and collaboration shape.

Weak prompts optimize for: tone, confidence, polish, or making output "seem right" without improving the workflow.

Evaluate by: does it give Copilot a clear job and honest constraints? Not: does it sound authoritative?

## Subagent Architecture

- `runSubagent` is disabled for subagents (VS Code hardcoded). Nested invocation impossible.
- Each subagent gets its own context window — use for isolation in long workflows.
- Orchestrators should be top-level, not nested under other agents.
- Pass compact handoffs between phases, not full transcripts.

## Community Cache

When available, the community cache at `community-cache/` provides pre-verified evidence. Load after this studybase, before open web research. Treat as trust tier 4 — revalidate normative claims against stronger sources.

Key packs: `prompting-principles.json`, `anti-patterns.json`, `frontmatter-reference.json`, `deprecations.json`, `workflow-patterns.json`, `official-sources.json`.
