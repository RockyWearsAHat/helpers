---
description: "Opt-in routing rules for using paid model-provider subagents when users want to reduce rate-limit pressure."
applyTo: "**"
---

# Paid Provider Subagents (Optional)

Use this only when the user explicitly asks to use a paid provider or says they want to avoid rate limits by subscribing directly to a model provider.

## Default Behavior

- Keep existing local-model and standard Copilot flows as the default path.
- Do not switch users to paid providers automatically.

## When User Requests Paid Provider Routing

- Treat paid-provider usage as explicit opt-in.
- If the user did not name a subagent, ask which provider-backed subagent they want to use.
- If they did name a subagent, delegate the requested work to that subagent directly.
- Prefer provider subagent execution for heavy or repeated tasks likely to hit rate limits.
- Fall back to local/Copilot routes if provider subagent invocation fails.

## Safety and Cost Controls

- Never request or store secrets in chat logs; rely on already-configured provider credentials.
- Confirm user intent before long or expensive multi-step provider runs.
- Keep the user informed when routing changes from local to paid provider (or back).

## Failure Handling

- If paid-provider subagents are unavailable, explain what failed and continue with available local/Copilot paths.
- Do not block the task solely because a paid subagent is unavailable.