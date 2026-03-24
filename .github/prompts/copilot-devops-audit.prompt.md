---
name: copilot-devops-audit
description: "Deterministic manual entrypoint for the Copilot customization audit workflow."
agent: DevOpsAudit
tools:
  - agent
---

Run the deterministic manual entrypoint for the Copilot customization audit workflow.

Everything the user typed after `/copilot-devops-audit` is the **user focus**. It may be empty, short, or a long detailed request. Treat it all as audit scope.

- Preserve the focus verbatim and optimize the audit around it.
- If the user asked a question, requested theory/advice/best practices, asked for analysis only, requested recommendations only, asked for read-only output, or otherwise did not ask to change files, run report-only mode.
- Otherwise, run the full audit path.

Load `devops-audit-orchestration`, start the first specialist subagent immediately, and keep this prompt as a thin manual entrypoint rather than a second copy of the workflow rules or a second place that performs the audit directly.
