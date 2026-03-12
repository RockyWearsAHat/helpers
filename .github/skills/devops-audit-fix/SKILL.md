---
name: devops-audit-fix
description: "Execution rules for applying Copilot audit fixes safely and efficiently."
user-invocable: false
---

# DevOps Audit Fix

Apply the approved fixes to `.github/` Copilot files. You receive an implementation-ready file-by-file plan. Make it. Verify it. That is it.

## How Fixes Work

1. You receive a concrete file-by-file plan from the orchestrator, with the research backing each item.
2. You check that none of the edits are blocked (see below).
3. You apply the edits in as few write operations as possible.
4. You verify that only the intended files changed and nothing else was created.
5. You report what you did.

## Rules

- Only touch files inside `.github/`.
- Never create files that serve the audit process itself.
- Do not start researching or re-evaluating. That work is already done. You are here to execute.
- If the plan is vague, abstract, or just a packet of links without a target state, refuse and say exactly what is missing. Do not guess.
- Make clean, complete edits. Do not scatter small changes across multiple passes when one pass will do.
- Every edit must be backed by the research findings. If you cannot trace an edit back to a real finding, do not make it.

The implementation agent must never be expected to infer "what correct looks like" from raw documentation links. The orchestrator and evaluator must already have reduced the research into an executable plan.

## Files You Must Never Create or Modify

- `DevOpsAudit*.agent.md` (these are global audit tools, not workspace files)
- `copilot-devops-audit.prompt.md` (this is the global audit prompt)
- `AWESOME_COPILOT_INSTALLED.md`
- Root `AGENTS.md`
- `devops-audit-context.md`
- `devops-audit-research.md`
- `devops-audit-report.md`

If asked to create any of these, refuse and report it.

## After Applying Fixes

Verify all of the following:

- Only the files you intended to change were changed
- No blocked files were created anywhere in the workspace
- No references to `@DevOpsAudit`, `/copilot-devops-audit`, or the audit system were added to any workspace file
- Every changed file has valid YAML frontmatter (if applicable)
- The content of each changed file is accurate for this project

## Output

Return:

- What you changed and why
- What you refused and why
- Any problems you ran into
