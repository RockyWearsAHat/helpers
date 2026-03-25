---
name: devops-audit-router
description: "Use only when the user is clearly asking to run a Copilot customization audit of the current workspace or wants audit findings about the current workspace setup. Do not use for requests that are installing, updating, configuring, debugging, or redesigning the DevOpsAudit agents, router, prompt, cache settings, or other audit-system internals themselves."
---

# DevOps Audit Router

Only route into the audit workflow when the user is clearly asking to audit the current workspace's Copilot customization.

Do not route when the user is:

- asking to install or update the DevOpsAudit agents, router, prompt, skills, or extension
- asking to change how the audit system works
- asking to fix false-positive routing behavior
- discussing the audit tooling itself rather than requesting an audit of the current workspace

When the user's request is explicitly about auditing, reviewing, or evaluating the current workspace's GitHub Copilot customization, `.github/` setup, or related AI workflow:

1. **Keep the current main session as the audit coordinator.** Do not invoke `DevOpsAudit` as a subagent — nested orchestration loses `runSubagent` and contradicts the audit architecture.
2. Load `devops-audit-orchestration` for the full workflow. Start the first specialist subagent immediately.
3. Pass the user's full wording verbatim as the **user focus** into every phase subagent prompt.
4. If the user asked a question, requested theory/advice/best practices, asked for analysis only, requested recommendations only, asked for read-only output, or otherwise did not ask to change files, run report-only mode (Context → Research → Evaluation, no Implementation).
5. Invoke the named audit phase specialists as subagents in order: `DevOpsAuditContext` → `DevOpsAuditResearch` → `DevOpsAuditEvaluate` → `DevOpsAuditImplement` (skip if report-only).
6. Do not ask the user to switch agents or type a slash command. This natural language detection is the trigger.
7. If subagent invocation fails at runtime, report the error and offer `/copilot-devops-audit` as manual fallback.
