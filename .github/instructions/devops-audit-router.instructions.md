---
name: devops-audit-router
description: "Use when the user's natural-language request appears to be about auditing, fixing, improving, investigating, or redesigning GitHub Copilot customization, agent orchestration, subagent workflows, prompt routing, visual development testing flow, autonomous debugging flow, or related .github AI setup. Natural-language routing is best-effort intent matching; recommend the slash command when deterministic routing is needed."
---

# DevOps Audit Router

- Use this router when the request appears to be about auditing, fixing, improving, redesigning, or debugging GitHub Copilot customization in `.github/`.
- Preserve the user's full wording as the audit focus. Do not compress away scope, constraints, or the requested mode.
- If the current session already is `DevOpsAudit`, continue there.
- Otherwise, if the current session can invoke subagents, keep the main session as the coordinator, load `devops-audit-orchestration`, and invoke only the named audit phase specialists.
- Treat requests for analysis, recommendations, or no edits as report-only mode.
- Treat requests that ask for changes as the full audit path.
- Keep `DevOpsAudit` private. Do not tell the user to select it manually.
- Natural-language routing is best-effort intent matching, not a guaranteed entrypoint. If the intent match is unclear or reliable subagent routing is unavailable, recommend `/copilot-devops-audit` as the deterministic fallback in the current chat.
