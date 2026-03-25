# DevOps Audit Research Cache — bin

_Seeded by `git-copilot-devops-audit` on 2026-03-10 00:00:00_

## Purpose

This file is a persistent research cache for the audit. It exists to carry forward useful evidence across audit runs without treating old research as permanently true.

Rules:

- Reuse prior research only when it is still fresh or has been revalidated.
- Prefer refreshing the most important stale evidence first.
- Replace or downgrade stale conclusions instead of endlessly appending more notes.
- Keep principles only when they still match current docs, recent repos, and recent videos.
- When storing lessons from example repositories, keep the principle learned, not copied prose.
- For `github/awesome-copilot` examples, record why the example works with Copilot's workflow and primitives, not just that it looks strong.
- If a shared community cache is configured, record which remote manifest and snapshot were used and what was revalidated locally.
- Never treat shared-cache entries as permanently true; carry forward only what still survives fresh verification.
- Never store repository-specific or identifying context in any community-bound conclusion. Community-bound content must be generalized Copilot best practices only.

## Shared Cache State

- Community cache mode: pull-and-auto-submit
- Canonical manifest URL:
- Recommended snapshot:
- Candidate index version:
- Last remote check: 2026-03-12
- Last accepted remote snapshot:
- Remote cache status: consumed (pull-and-auto-submit)
- Community participation: enabled
- Auto-submit final conclusions: yes
- Last submitted conclusion packet:
- Last promotion report seen:

## Freshness Policy

| Evidence Type                                     | Importance | Refresh Target | Stale After | Notes                                                              |
| ------------------------------------------------- | ---------- | -------------: | ----------: | ------------------------------------------------------------------ |
| RECENT AND RELEVANT Official docs and model lists | Critical   |        14 days |     30 days | Highest priority; docs win when they conflict with weaker sources  |
| VS Code release notes / product updates           | High       |        14 days |     30 days | Refresh sooner when major Copilot changes land                     |
| Video transcripts from product-adjacent sources   | High       |        30 days |     60 days | Use transcripts, not summaries                                     |
| Repository samples                                | Medium     |        30 days |     60 days | Prefer active repos; common patterns are not automatically correct |
| Synthesized principles                            | High       |        30 days |     60 days | Keep only if revalidated against fresh evidence                    |
| Anti-pattern examples                             | Medium     |        45 days |     90 days | Useful for contrast, but downgrade if ecosystem changes            |

## Current Review State

- Last full refresh: 2026-03-12
- Next refresh due: 2026-03-26
- Highest-priority stale areas: none (fresh baseline)
- Confidence in current baseline: high

## Official Sources

| Source                             | Last Reviewed | Status   | Importance | Notes                                                                                  |
| ---------------------------------- | ------------- | -------- | ---------- | -------------------------------------------------------------------------------------- |
| VS Code Copilot customization docs | 2026-03-12    | verified | critical   | Custom instructions, agents, prompt files, skills — current file types and frontmatter |
| GitHub Copilot customization docs  | 2026-03-12    | verified | critical   | Organization-level and repo-level instruction files, `.github/copilot-instructions.md` |
| VS Code custom agents docs         | 2026-03-12    | verified | critical   | `.agent.md` frontmatter: name, description, tools, agents, user-invocable              |
| VS Code agent skills docs          | 2026-03-12    | verified | critical   | `SKILL.md` in `.github/skills/<name>/`, loaded by agent/instruction reference          |
| VS Code 1.111 release notes        | 2026-03-12    | verified | high       | Latest Copilot customization changes, prompt file improvements, agent mode updates     |

## Repository Samples

| Repo / Source          | Last Reviewed | Status   | Surface                            | Principle or Anti-pattern                                                                 | Notes                                                  |
| ---------------------- | ------------- | -------- | ---------------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| microsoft/PowerToys    | 2026-03-12    | verified | instructions, copilot-instructions | Large monorepo with scoped `applyTo` instruction files; project-specific coding standards | Good model for multi-language repos with many modules  |
| dotnet/macios          | 2026-03-12    | verified | instructions, agents               | .NET binding project with domain-specific Copilot instructions; scoped file conventions   | Shows how to tailor instructions to a binding workflow |
| github/awesome-copilot | 2026-03-12    | verified | examples, curated links            | Reference examples for prompt, instruction, agent, and skill design patterns              | Used for file-level example inspection                 |

## Video / Transcript Samples

| Source        | Video / Topic                              | Last Reviewed | Status   | Principle or Tension                                                                | Notes                                            |
| ------------- | ------------------------------------------ | ------------- | -------- | ----------------------------------------------------------------------------------- | ------------------------------------------------ |
| Burke Holland | fabAI1OKKww — Agent skills guide           | 2026-03-12    | verified | Skills as progressive-loading methodology; keep agents thin, put steps in skills    | Primary source for skill structure and scoping   |
| Burke Holland | 0XoXNG65rfg — Level up productivity        | 2026-03-12    | verified | Prompt files + instructions + agents compose; each primitive has one job            | Shows how primitives combine in practice         |
| Burke Holland | s7Qzq0ejhjg — Ask/Edit/Agent overview      | 2026-03-12    | verified | System prompt layering: instructions inject into all modes; agents add role + tools | Clarifies how instructions flow into chat modes  |
| Burke Holland | 5NxGqnTazR8 — Ultimate agent mode tutorial | 2026-03-12    | verified | Agent mode: vision, MCP, custom agents, tool restrictions, model selection          | Covers agent frontmatter fields and tool scoping |

## Current Principles

| Principle                                                                                         | Confidence | Last Revalidated | Supporting Evidence                                   | Notes                                              |
| ------------------------------------------------------------------------------------------------- | ---------- | ---------------- | ----------------------------------------------------- | -------------------------------------------------- |
| TC1: Agent files define role + tools + boundaries; methodology belongs in skills                  | high       | 2026-03-12       | VS Code docs, fabAI1OKKww, PowerToys, awesome-copilot | Core primitive separation                          |
| TC2: `copilot-instructions.md` is always-on context; keep it factual and brief                    | high       | 2026-03-12       | VS Code docs, s7Qzq0ejhjg, GitHub docs                | Avoid methodology in base instructions             |
| TC3: Scoped `.instructions.md` files use `applyTo` for file-type-specific guidance                | high       | 2026-03-12       | VS Code docs, PowerToys, dotnet/macios                | Glob patterns match workspace-relative paths       |
| TC4: YAML frontmatter requires opening and closing `---` fences; missing fence silently breaks it | high       | 2026-03-12       | VS Code docs, user memory, 0XoXNG65rfg                | Common silent failure mode                         |
| TC5: `description` field enables natural-language routing for agents and instructions             | high       | 2026-03-12       | VS Code docs, 5NxGqnTazR8                             | Without it, files may not be selected by Copilot   |
| TC6: Skills load on demand via `Load <skill-name>`; do not duplicate skill content in agents      | high       | 2026-03-12       | fabAI1OKKww, VS Code skills docs                      | Duplication wastes context and drifts over time    |
| TC7: `user-invocable: false` keeps internal agents private from the user's agent picker           | high       | 2026-03-12       | VS Code docs, audit agent pattern                     | All audit subagents use this                       |
| TC8: Prompt files (`.prompt.md`) use `agent:` not `mode:` — `mode:` is deprecated                 | high       | 2026-03-12       | VS Code docs, user memory                             | Using `mode:` causes slash command to not register |

## Prompting Principles and Anti-patterns

| Type | Principle or Anti-pattern | Confidence | Last Revalidated | Supporting Evidence | Notes |
| ---- | ------------------------- | ---------- | ---------------- | ------------------- | ----- |

## Contribution Candidates

| Candidate | Submitted On | Status | Evidence Quality | Applicability | Durability Days | Submission Count | Contradictions | Notes |
| --------- | ------------ | ------ | ---------------- | ------------- | --------------- | ---------------- | -------------- | ----- |

## Promotion Watchlist

| Statement | State | Promotion Score | First Seen | Last Seen | Revalidated On | Promotion Notes |
| --------- | ----- | --------------- | ---------- | --------- | -------------- | --------------- |

## Conflicts and Open Questions

| Topic | Conflict | Preferred Interpretation | Why | Recheck By |
| ----- | -------- | ------------------------ | --- | ---------- |

## Superseded or Stale Evidence

| Evidence | Became Stale On | Action | Notes |
| -------- | --------------- | ------ | ----- |

## Next Refresh Priorities

1. Refresh critical official sources first.
2. Refresh any principle used in the latest audit if its supporting evidence is stale.
3. Refresh medium-importance repo and video samples only after critical sources are current.
