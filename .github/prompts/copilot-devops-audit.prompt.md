---
name: copilot-devops-audit
description: "Run a full Copilot customization audit for this workspace."
agent: DevOpsAudit
tools:
  - agent
  - fetch
  - githubRepo
  - readFile
  - fileSearch
  - textSearch
  - editFiles
  - runInTerminal
---

Run a full Copilot customization audit for this workspace.

Everything the user typed after `/copilot-devops-audit` is the **user focus**. It may be empty, short, or a long detailed request. Treat it all as audit scope.

- If the user provided focus text, the entire audit (context, research, evaluation, implementation) must optimize for that focus. The focus is not a hint — it defines what the audit should prioritize and what the research should target.
- If the user provided no focus text, the entire codebase is the context. Research broadly for this project type.

You are the DevOpsAudit orchestrator. Load `devops-audit-orchestration` and run the full four-phase pipeline now. Do not skip phases. Do not do the phase work yourself. Start the first subagent call immediately.

If the user sends follow-up messages during or after the audit, those messages are additional audit context. Do not abandon the pipeline. If the pipeline has not started, start it with the new context included. If a phase is in progress, fold the new context into the current or next phase. If the pipeline finished, re-run it with the combined context as a new audit pass.

Never do context-gathering, research, evaluation, or implementation yourself. Always delegate through the subagent pipeline defined in `devops-audit-orchestration`.
