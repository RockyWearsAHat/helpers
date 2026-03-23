---
name: devops-audit-router
description: "Use when the user asks in natural language to audit, fix, improve, investigate, or redesign GitHub Copilot customization, agent orchestration, subagent workflows, prompt routing, visual development testing flow, autonomous debugging flow, or related .github AI setup. Route the request through the audit workflow without requiring the slash command."
---

# DevOps Audit Router

When the user's request is about auditing, fixing, improving, redesigning, or debugging GitHub Copilot customization, `.github/` setup, agent orchestration, subagent workflows, prompt routing, or related AI infrastructure:

1. **Keep the current main session as the audit coordinator.** Do not invoke `DevOpsAudit` as a subagent — nested orchestration loses `runSubagent` and contradicts the audit architecture.
2. Load `devops-audit-orchestration` for the full workflow. Start the first specialist subagent immediately.
3. Pass the user's full wording verbatim as the **user focus** into every phase subagent prompt.
4. If the user asked for analysis only, recommendations only, read-only output, or no edits, run report-only mode (Context → Research → Evaluation, no Implementation).
5. Invoke the named audit phase specialists as subagents in order: `DevOpsAuditContext` → `DevOpsAuditResearch` → `DevOpsAuditEvaluate` → `DevOpsAuditImplement` (skip if report-only).
6. Do not ask the user to switch agents or type a slash command. This natural language detection is the trigger.
7. If subagent invocation fails at runtime, report the error and offer `/copilot-devops-audit` as manual fallback.
