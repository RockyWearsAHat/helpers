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

- Community cache mode: pull-only
- Canonical manifest URL:
- Recommended snapshot:
- Candidate index version:
- Last remote check:
- Last accepted remote snapshot:
- Remote cache status: unknown
- Community participation: disabled
- Auto-submit final conclusions: no
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

- Last full refresh:
- Next refresh due:
- Highest-priority stale areas:
- Confidence in current baseline:

## Official Sources

| Source                             | Last Reviewed | Status  | Importance | Notes |
| ---------------------------------- | ------------- | ------- | ---------- | ----- |
| VS Code Copilot customization docs |               | pending | critical   |       |
| GitHub Copilot customization docs  |               | pending | critical   |       |
| Latest VS Code update notes        |               | pending | high       |       |
| Copilot model list / availability  |               | pending | high       |       |

## Repository Samples

| Repo / Source | Last Reviewed | Status | Surface | Principle or Anti-pattern | Notes |
| ------------- | ------------- | ------ | ------- | ------------------------- | ----- |

## Curated Example Samples

| Source File | Last Reviewed | Status | Primitive | Principle Learned | What Not To Copy Literally | Notes |
| ----------- | ------------- | ------ | --------- | ----------------- | -------------------------- | ----- |

## Shared Cache Snapshot History

| Snapshot | Checked On | Status | Accepted Items | Revalidated Items | Rejected or Downgraded Items | Notes |
| -------- | ---------- | ------ | -------------- | ----------------- | ---------------------------- | ----- |

## Video / Transcript Samples

| Source | Video / Topic | Last Reviewed | Status | Principle or Tension | Notes |
| ------ | ------------- | ------------- | ------ | -------------------- | ----- |

## Current Principles

| Principle | Confidence | Last Revalidated | Supporting Evidence | Notes |
| --------- | ---------- | ---------------- | ------------------- | ----- |

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
