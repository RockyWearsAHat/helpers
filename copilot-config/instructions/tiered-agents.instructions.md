---
description: "Cost-proportional model routing for orchestrators. Use when dispatching subagents and you want to match model capability to task complexity."
applyTo: "**"
---

# Tiered Agent Model Routing

When you are an orchestrator dispatching subagents, match the model to the task. Do not run every step at the most capable (and expensive) model available.

## Tier Definitions

| Tier | When to use | Example models |
|---|---|---|
| **fast** | Read-only, file listing, formatting, mechanical data transforms | `claude-haiku-4.5`, `gpt-4o-mini` |
| **standard** | Reasoning, code analysis, web research, structured evaluation | `claude-sonnet-4.6`, `gpt-4o` |
| **deep** | Ambiguous architecture decisions, multi-step planning, high-stakes edits | `claude-opus-4.6`, `gpt-5` |

## How to Override at Call-Time

Pass `model` to `runSubagent` to override an agent's default:

```
runSubagent(
  agentName: "DevOpsAuditResearch",
  model: "claude-opus-4.6",   // upgrade for a hard research problem
  prompt: "...",
  description: "..."
)
```

Use `list_language_models` (MCP tool) to see all valid model ids.

## Routing by Task Type

**Always use fast tier for:**
- Listing files, reading directories, inventory work
- Extracting structured data from files with clear schemas
- Formatting, sanitizing, or serializing output
- Submitting pre-prepared data to an external store

**Always use standard tier for:**
- Web research and evidence synthesis
- Code analysis and pattern detection
- Comparing findings against criteria
- Writing instructions or explanations
- Making file edits where correctness matters

**Consider deep tier when:**
- The problem domain is poorly documented and the model needs to reason under uncertainty
- Multiple conflicting sources need to be weighed before a conclusion
- A mistake in the output would require significant rework to fix

## Default Models by Pipeline Phase

When no explicit override is needed, expect these defaults from the named audit agents:

| Agent | Default Model | Tier | Rationale |
|---|---|---|---|
| DevOpsAuditContext | `claude-haiku-4.5` | fast | File reading, no synthesis |
| DevOpsAuditResearch | `claude-sonnet-4.6` | standard | Web + reasoning |
| DevOpsAuditEvaluate | `claude-sonnet-4.6` | standard | Structured comparison |
| DevOpsAuditImplement | `claude-sonnet-4.6` | standard | Edit reliability matters |
| DevOpsAuditCommunitySubmit | `claude-haiku-4.5` | fast | Mechanical formatting |

## Natural Language Signals

When the user describes a task and you need to pick a tier, look for these signals:

- "quickly", "scan", "list", "check", "summarize", "format" → **fast**
- "analyze", "research", "evaluate", "compare", "implement", "fix" → **standard**
- "deep dive", "thorough", "architecture", "uncertain", "complex", "decide" → **deep**

If none of these signals are present, default to **standard**. Better to use a slightly more capable model than to produce a shallow result and need to redo the step.

## Multi-Step Orchestration

For pipelines with several phases, you can mix tiers freely. Example for a 4-step audit:

```
step 1 → DevOpsAuditContext    (fast:    model="claude-haiku-4.5")
step 2 → DevOpsAuditResearch   (standard: model="claude-sonnet-4.6")
step 3 → DevOpsAuditEvaluate   (standard: model omitted, uses agent default)
step 4 → DevOpsAuditImplement  (standard: model omitted, uses agent default)
```

The total token cost of a well-routed 4-step pipeline is significantly lower than running all four steps at Sonnet or Opus, with no meaningful quality loss on the cheap steps.
