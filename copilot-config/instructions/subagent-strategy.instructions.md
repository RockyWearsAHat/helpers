---
description: "Cost-efficient subagent model selection strategy. Use when spawning subagents via runSubagent to ensure the right model is matched to each subtask's complexity."
applyTo: "**"
---

# Subagent Model Selection Strategy

When you have access to the `runSubagent` tool and are about to invoke it, you MUST assess the subtask complexity BEFORE choosing a model. Your job as the orchestrating agent is to maximize quality-per-cost, not to use your own model for everything.

## Decision Framework

**Before every subagent call, ask yourself:**

1. What is the cognitive complexity of this subtask?
2. What is the cheapest model that can reliably complete it?
3. Am I delegating because the task is large, or because it's hard?

## Default Model

**`claude-haiku-4.5`** (or `gpt-4o-mini`) for every subagent call. Always specify `model` explicitly.

## Promotion Criteria

Promote a call to `claude-sonnet-4.6` (or `gpt-4o`) only when the **specific prompt you're about to send** meets one of these:

- The prompt requires synthesizing across conflicting or ambiguous sources
- The prompt requires multi-file structural edits where correctness depends on cross-file consistency
- The prompt requires the agent to infer unstated constraints or fill gaps in ambiguous instructions
- A prior haiku call on the same task produced an output you rejected as insufficient

Promote to `claude-opus-4.6` (or `o3`) only when sonnet has already been tried and the output was insufficient.

**Do not promote based on the agent name or phase label.** "Research," "Evaluate," and "Implement" are not complexity tiers — they are workflow phases. A research call that fetches a URL and extracts a section is cheap. An evaluation call that checks a list against yes/no criteria is cheap. An implementation call that makes a templated edit is cheap. Assess the prompt, not the phase.

## Structural Safety Net — Agent File Model Fields

Every agent `.agent.md` file MUST have `model: claude-haiku-4.5` in its frontmatter. This is the **fallback model** used when the orchestrator does not pass `model` in the `runSubagent` call. If the fallback is sonnet or opus, every time the orchestrator forgets to pass model — or misreads the criteria — you pay sonnet/opus prices silently. Make the wrong default cheap.

**Rule:** agent file `model:` = haiku. Orchestrator `runSubagent` call passes `model: claude-sonnet-4.6` only when that specific prompt warrants promotion. This way forgetting to promote costs nothing; forgetting to demote costs nothing either.

## Rules

1. **Default to haiku.** Every `runSubagent` call starts at `claude-haiku-4.5`. Promote only when the specific prompt demands it.
2. **Always specify `model` explicitly.** Omitting it falls back to the agent file's `model:` field — not the caller's model. If agent files all have `model: claude-haiku-4.5`, omitting is safe. But explicit is always better.
3. **Always use the latest model version.** Within a model family, prefer the most recent release (e.g., `claude-sonnet-4.6` over `claude-sonnet-4`, `gpt-4o` over `gpt-4`). Older versions cost the same but perform worse.
4. **You are the brain.** The orchestrating agent handles planning, decomposition, and quality judgment. Subagents handle execution. Execution is usually cheaper than planning.
5. **Batch cheap tasks.** If you have 5 independent file lookups, send them to a fast model rather than doing them yourself.
6. **Promote only with justification.** If you choose sonnet or higher, you must articulate what in the prompt requires it — not the agent name, not the phase label, the actual prompt content.
7. **Split before promoting.** If a task seems to need a more capable model, first try decomposing it into smaller subtasks that haiku can handle individually.

## Anti-Patterns

- Pre-mapping agent names or workflow phases to model tiers (e.g., "Research → sonnet")
- Sending all subagent calls to the same model regardless of prompt content
- Using your own model (the parent model) for every subagent without considering alternatives
- Using Sonnet-class models for file searches, simple grep operations, or checklist evaluations
- Omitting the `model` parameter entirely — this silently inherits the caller's model
