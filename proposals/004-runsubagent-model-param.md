# Proposal: Add `model` Parameter to `runSubagent` Tool

**Target**: `microsoft/vscode`
**Component**: Chat — Built-in Tools / RunSubagentTool
**Type**: API Enhancement (Built-in Tool Schema)

## Summary

Add an optional `model` parameter to the built-in `runSubagent` tool so that an orchestrator can specify which model to use for a subagent invocation at call-time, independent of the agent definition's default.

## Motivation

The `runSubagent` tool currently accepts `agentName`, `prompt`, and `description`. The model used for the subagent is determined statically by the agent's `model:` frontmatter field (if present) or inherited from the parent session. There is no call-time override.

This matters for two real workflows:

### 1. Orchestrators need cost-proportional routing

A powerful model (e.g., Opus) orchestrating a multi-step task should be able to say: "run this lightweight file-reading step with Haiku, then run the deep analysis step with Sonnet." Today, it can't — every `runSubagent` call inherits the same endpoint or uses whatever the agent definition baked in.

The built-in `execution_subagent` and `search_subagent` tools already support model override via VS Code settings (`chat.executionSubagent.model`, `chat.searchSubagent.model`). Custom agent invocations through `runSubagent` have no equivalent.

### 2. Users want to direct model strength via conversation

A user talking to Opus wants to say "run 3 subagents with Sonnet 4.6 for the analysis steps." The orchestrator has no mechanism to honor this — `runSubagent` has no model slot to fill.

### Real-world use case: DevOpsAudit pipeline

Our audit pipeline (`DevOpsAudit` → context → research → evaluate → implement) runs in a sequence where different steps need different model characteristics:

- **Context subagent**: Read-heavy, fast — benefits from Haiku
- **Research subagent**: Web search + reasoning — needs Sonnet or better
- **Evaluate subagent**: Analytical comparison — mid-tier
- **Implement subagent**: Precise file edits — mid-tier, but needs tool-calling reliability

Today all four get whatever model the outer orchestrator runs on. Adding `model` to `runSubagent` lets the orchestrator dispatch each step appropriately.

### Prior art within VS Code itself

`ExecutionSubagentToolCallingLoop.getEndpoint()` already implements exactly this pattern — it reads `chat.executionSubagent.model` from settings and passes it to `endpointProvider.getChatEndpoint()`. The `RunSubagentTool` invoke method has the `userSelectedModelId` variable (`p`) and `resolveSubagentModel()` infrastructure already in place — it just doesn't expose a call-time override slot.

## Proposed Change

```diff
  getToolData() {
    let t = {
      type: "object",
      properties: {
        prompt: { type: "string", description: "A detailed description of the task for the agent to perform" },
        description: { type: "string", description: "A short (3-5 word) description of the task" },
+       model: {
+         type: "string",
+         description: "Optional model identifier for this subagent invocation. Overrides the agent definition's default model. Examples: claude-sonnet-4-6, claude-haiku-4-5, gpt-4o, gpt-4o-mini."
+       }
      },
      required: ["prompt", "description"]
    };
    // existing agentName handling...
  }

  async invoke(e, t, o, n) {
    // ...existing model resolution via resolveSubagentModel()...

+   // Call-time model override
+   if (r.model) {
+     const overrideModel = this.languageModelsService.lookupLanguageModel(r.model);
+     if (overrideModel) {
+       p = r.model;
+       v = overrideModel.name;
+     }
+   }

    const ve = {
      // ...
      userSelectedModelId: p,   // picks up override
      modelConfiguration: p ? this.languageModelsService.getModelConfiguration(p) : void 0,
      // ...
    };
  }
```

## Implementation Notes

**Injection point 1 — tool schema** (`workbench.desktop.main.js`, `RunSubagentTool.getToolData()`):

Add the `model` property to the `properties` object alongside `prompt` and `description`. This is unconditional — the `model` override should work regardless of whether `chat.customAgentInSubagent.enabled` is set.

**Injection point 2 — invoke override** (`workbench.desktop.main.js`, `RunSubagentTool.invoke()`):

After `resolveSubagentModel()` completes and `p`/`v` are set (covering both the `agentName` branch and the fallback branch), apply the `r.model` override if present. The override must run before the `ve` request object is constructed so `userSelectedModelId` and `modelConfiguration` both reflect the caller-specified model.

**Validation**: If the specified model ID is not found via `lookupLanguageModel()`, silently fall through to the agent-resolved model. This is graceful degradation — a typo or unknown model name does not hard-fail the invocation.

**`prepareToolInvocation()`**: The preview message already shows model name via `resolveSubagentModel()`. With the call-time override not yet applied at prepare-time, the preview may show the agent default before the actual invocation uses the override. This is consistent with how multiplier caching already works (`_resolvedModels` map is pre-populated at prepare-time). A follow-up could pass the `model` input through `prepareToolInvocation()` as well if display accuracy is important.

## Multiplier / Cost Guard

The existing `resolveSubagentModel()` logic refuses to use a model with a higher multiplier than the main agent model, falling back to the parent. This guard should _not_ apply to call-time overrides — the calling model is explicitly requesting a specific model, which counts as user intent. The guard still applies to implicit resolution from the agent definition.

Alternatively, a softer approach: apply the guard but emit a `logService.warn` that the override was rejected due to cost policy, so the orchestrator can see it in the output log.

## Backward Compatibility

- Existing `runSubagent` calls without `model` are unaffected — `r.model` is `undefined`, the override block is a no-op
- Agent definitions with `model:` frontmatter continue to work as today
- The parameter is optional in the schema — callers that don't know about it simply don't provide it

## Current Workaround: Tiered Agent Variants

Without call-time override, the only workaround is to create multiple registered variants of the same agent with different `model:` fields in their frontmatter:

```yaml
# Explore.agent.md (fast)
model:
  - claude-haiku-4-5 (copilot)

# ExploreDeep.agent.md (powerful)
model:
  - claude-sonnet-4-6 (copilot)
```

The orchestrator then decides at call-time which named variant to invoke. This works, but it multiplies agent file count and requires the orchestrator to use naming conventions rather than explicit model IDs. See Proposal 005 for formalizing this pattern.
