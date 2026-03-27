---
name: copilot-devops-audit
description: "Deterministic manual entrypoint for the Copilot customization audit workflow."
agent: DevOpsAudit
tools:
  - agent
---

Run the deterministic manual entrypoint for the Copilot customization audit workflow.

Everything the user typed after `/copilot-devops-audit` is audit input. It may be empty, short, or a long detailed request. Do not treat all of it as undifferentiated audit scope.

Build a short request contract before doing anything else:

- `primary action`
- `primary object`
- `requested outcome`
- `explicit exclusions`
- `target surface`

- Preserve the user focus verbatim and optimize the audit around it after the request contract is clear.
- Classify the target surface from the user's wording before starting. If the user names `copilot-config/`, internal audit instructions, shipped prompts, agents, skills, or other product-source customization assets, audit that source surface rather than defaulting to `.github/`.
- If the workspace declares a source-of-truth Copilot location that differs from `.github/`, honor that location in the context phase and carry it through the rest of the workflow.
- If the request contract shows the user is really asking to improve or debug the audit workflow itself rather than audit the current workspace, stop and say the request should be handled as a direct customization change, not an audit run.
- If the user asked a question, requested theory/advice/best practices, asked for analysis only, requested recommendations only, asked for read-only output, or otherwise did not ask to change files, run report-only mode.
- Otherwise, run the full audit path.

Load `devops-audit-orchestration`, start the first specialist subagent after the request contract is clear, and keep this prompt as a thin manual entrypoint rather than a second copy of the workflow rules or a second place that performs the audit directly.
