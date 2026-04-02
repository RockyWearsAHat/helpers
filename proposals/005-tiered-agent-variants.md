# Proposal: Tiered Agent Variant Support in VS Code

**Target**: `microsoft/vscode`
**Component**: Chat — Agent System / Built-in Tools
**Type**: Feature Addition (Agent Tier Selection)

## Summary

Formalize a "tiered agent" pattern that allows a single logical agent to run at different model capability levels, selectable at invocation time by the calling orchestrator. This complements Proposal 004 (call-time `model` parameter for `runSubagent`) by providing a named abstraction over model selection for cases where capability tier matters more than specific model identity.

## Background: The Tiered Variant Workaround

The agent `model:` frontmatter field was designed for the agent definition to declare a preferred model. Users discovered that creating multiple named variants of the same agent with different `model:` pins is an effective strategy for giving orchestrators cost-proportional dispatch:

```yaml
# Explore.agent.md — standard analysis
---
model:
  - claude-haiku-4-5 (copilot)
  - gpt-4o-mini (copilot)
---
# ExploreDeep.agent.md — deep analysis
---
model:
  - claude-sonnet-4-6 (copilot)
  - gpt-4o (copilot)
---
# ExploreFast.agent.md — quick read
---
model:
  - claude-haiku-4-5 (copilot)
---
```

The orchestrator then calls `runSubagent(agentName: "ExploreDeep")` when it needs heavyweight analysis and `runSubagent(agentName: "Explore")` for lighter work.

**This works today** and should be documented as a first-class pattern. VS Code already supports the `model:` array in agent frontmatter.

## Motivation for Formalizing

The workaround is effective but has costs:

1. **File proliferation** — 3 agents instead of 1 for the same logical task
2. **Discovery burden** — orchestrators must know the `ExploreDeep` name exists
3. **Maintenance** — instructions must be kept in sync across variants
4. **Naming convention** — `Explore` / `ExploreDeep` / `ExploreFast` is an informal convention, not a platform feature

A formalized tier system would solve all four without requiring separate files.

## Proposed: `tiers` Frontmatter Field

Add a `tiers` field to agent frontmatter that declares per-tier model overrides for a single agent definition:

```yaml
# Explore.agent.md
---
name: Explore
description: "Fast read-only codebase exploration and Q&A subagent."
tools:
  - read
  - search
tiers:
  fast:
    model: claude-haiku-4-5 (copilot)
  standard:
    model: claude-sonnet-4-6 (copilot)
  deep:
    model: claude-opus-4-1 (copilot)
---
```

`runSubagent` tool gains a `tier` parameter:

```json
{
  "agentName": "Explore",
  "tier": "deep",
  "prompt": "Trace every caller of the auth module and identify breaking changes",
  "description": "Trace auth module callers"
}
```

VS Code resolves the model using the `tier` → `model` mapping from the agent definition, falling back to the agent's top-level `model:` field, then to the session model.

## Proposed: `tier` Parameter in `runSubagent`

The `tier` parameter would be added alongside the `model` parameter from Proposal 004:

```diff
  properties: {
    prompt: { type: "string" },
    description: { type: "string" },
+   model: { type: "string", description: "Explicit model override (see Proposal 004)" },
+   tier: {
+     type: "string",
+     enum: ["fast", "standard", "deep"],
+     description: "Capability tier for this invocation. 'fast' = lightweight/cheap, 'standard' = balanced, 'deep' = maximum capability. Maps to the agent's tier model configuration if defined."
+     }
  }
```

**Model resolution order with tiers:**

1. If `model` is provided → use it (Proposal 004, explicit override)
2. Else if `tier` is provided and the agent defines that tier → use that tier's model
3. Else if the agent has a top-level `model:` field → use it (existing behavior)
4. Else inherit from the parent session model (existing behavior)

## Standardized Tier Names

Three tiers cover the practical space:

| Tier       | Intent                                           | Example use                          |
| ---------- | ------------------------------------------------ | ------------------------------------ |
| `fast`     | Low cost, quick turnaround, read-heavy work      | File listing, context gathering      |
| `standard` | Balanced capability and cost                     | Code analysis, moderate reasoning    |
| `deep`     | Maximum capability, complex multi-step reasoning | Architecture analysis, deep research |

These names are stable and model-agnostic. The actual model behind each tier is defined by the agent file owner and can change across provider updates without breaking orchestrator code.

## VS Code Settings Integration

Settings could provide global tier → model fallback mappings for agents that don't define per-tier models:

```json
{
  "chat.agentTiers.fast": "claude-haiku-4-5",
  "chat.agentTiers.standard": "claude-sonnet-4-6",
  "chat.agentTiers.deep": "claude-opus-4-1"
}
```

## Current Workaround: Best Practices

Until this lands, the tiered variant pattern is the recommended approach. Key guidance:

1. **Use a naming suffix**: `Explorer` (standard), `ExploreDeep` (powerful), `ExploreFast` (lightweight)
2. **Share a system prompt template**: Keep instructions in a shared `.prompt.md` or skill, referenced from each variant
3. **Document the tier family in each agent's description**: `"argument-hint: use ExploreDeep for thorough analysis, Explore for quick reads"`
4. **Register in orchestrator agentlists explicitly**: The orchestrator's `.agent.md` `agents:` list should name all tier variants it may use

## Relationship to Proposal 004

These two proposals are complementary:

- **Proposal 004** (`model` param): explicit, model-ID-level control. Better for orchestrators that know exactly which model to use. Breaks if model IDs change.
- **Proposal 005** (`tier` param): semantic, capability-level control. Better for orchestrators that want "more powerful" vs "cheaper" without caring about specific model IDs. Resilient to model renames.

Both should ship. Proposal 004 is the simpler change and the more immediate need. Proposal 005 represents the cleaner long-term UX.

## Current Patch Status

The workaround (tiered agent variant files) works today without any patching. It is a pure authoring convention built on the existing `model:` frontmatter field. No patch is required.

Proposal 004's patch enables call-time model override without variant files — which supersedes the workaround for cases where the orchestrator wants to control the model directly.
