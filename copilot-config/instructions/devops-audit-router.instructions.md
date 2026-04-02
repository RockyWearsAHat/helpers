---
name: devops-audit-router
description: "Use only when the user is clearly asking to run a Copilot customization audit of the current workspace or wants audit findings about the current workspace setup. Do not use for requests that are installing, updating, configuring, debugging, or redesigning the DevOpsAudit agents, router, prompt, cache settings, or other audit-system internals themselves."
---

# DevOps Audit Router

Only route into the audit workflow when the user is clearly asking to audit the current workspace's Copilot customization.

Before routing, do a short request-interpretation pass. Separate these fields from the user's wording:

- `primary action`
- `primary object`
- `requested outcome`
- `explicit exclusions`
- `target surface`

Route only when that contract still clearly describes an audit of the current workspace or a request for audit findings.

Do not route when the user is:

- asking to install or update the DevOpsAudit agents, router, prompt, skills, or extension
- asking to change how the audit system works
- asking to fix false-positive routing behavior
- discussing the audit tooling itself rather than requesting an audit of the current workspace
- asking for the caller to interpret directions better, classify intent better, or redesign delegation behavior

When the user's request is explicitly about auditing, reviewing, or evaluating the current workspace's GitHub Copilot customization, repo-defined Copilot source surfaces such as `copilot-config/`, `.github/` setup, or related AI workflow:

1. **Keep the current main session as the audit coordinator.** Do not invoke `DevOpsAudit` as a subagent — nested orchestration loses `runSubagent` and contradicts the audit architecture.
2. Load `devops-audit-orchestration` for the full workflow. Start the first specialist subagent only after the request contract is clear.
3. Pass the user's full wording verbatim as the **user focus** into every phase subagent prompt.
4. Pass the parsed request contract into every phase prompt. Tell the context phase to preserve the contract and determine the correct audit target surface from the user's wording and any repo-declared source-of-truth instructions. Do not collapse that scope to `.github/` unless the user or repo context actually points there.
5. If the user asked a question, requested theory/advice/best practices, asked for analysis only, requested recommendations only, asked for read-only output, or otherwise did not ask to change files, run report-only mode (Context → Research → Evaluation, no Implementation).
6. Invoke the named audit phase specialists as subagents in order: `DevOpsAuditContext` → `DevOpsAuditResearch` → `DevOpsAuditEvaluate` → `DevOpsAuditImplement` (skip if report-only).
7. Do not ask the user to switch agents or type a slash command. This natural language detection is the trigger.
8. If subagent invocation fails at runtime, report the error and offer `/copilot-devops-audit` as manual fallback.
