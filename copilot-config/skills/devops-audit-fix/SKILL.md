---
name: devops-audit-fix
description: "Execution rules for applying Copilot audit fixes safely and efficiently."
user-invocable: false
---

# DevOps Audit Fix

Apply the approved fixes to `.github/` Copilot files. You receive an implementation-ready file-by-file plan. Make it. Verify it. That is it.

If the user explicitly asked for no edits, read-only output, report-only mode, or a concise overview without changes, do not apply any fixes. Refuse the run and report that implementation should have been skipped.

If the user focus is advisory, theoretical, or question-style and does not explicitly ask to change files, do not apply any fixes. Refuse the run and report that implementation should have been skipped.

Existing repo-owned Copilot customization files inside `.github/` are in scope when the approved plan names them explicitly. Do not treat agents, prompts, skills, instructions, or the existing research cache as blocked just because they belong to the audit system.

## How Fixes Work

1. You receive a concrete file-by-file plan from the orchestrator, with the research backing each item.
2. You check that none of the edits are blocked (see below).
3. You apply the edits in as few write operations as possible.
4. You verify that only the intended files changed and nothing else was created.
5. You report what you did.

## Rules

- Only touch files inside `.github/`.
- Never create throwaway audit artifacts or reports unless the approved plan explicitly says to maintain an existing repo-owned file.
- Do not start researching or re-evaluating. That work is already done. You are here to execute.
- If the plan is vague, abstract, or just a packet of links without a target state, refuse and say exactly what is missing. Do not guess.
- Make clean, complete edits. Do not scatter small changes across multiple passes when one pass will do.
- Every edit must be backed by the research findings. If you cannot trace an edit back to a real finding, do not make it.
- If the approved findings include required fixes outside `.github/`, do not silently drop them. Keep your edits inside `.github/`, then report explicit follow-up actions naming the remaining files or surfaces and why they still need work.

The implementation agent must never be expected to infer "what correct looks like" from raw documentation links. The orchestrator and evaluator must already have reduced the research into an executable plan.

## Files You Must Never Create or Modify

- `AWESOME_COPILOT_INSTALLED.md`
- Root `AGENTS.md`
- `devops-audit-context.md`
- `devops-audit-report.md`

If asked to create any of these, refuse and report it.

## After Applying Fixes

Verify all of the following:

- Only the files you intended to change were changed
- No blocked files were created anywhere in the workspace
- No new audit-system references were added outside the files and purposes explicitly approved in the plan
- Every changed file has valid YAML frontmatter (if applicable)
- VS Code diagnostics for the touched `.github/` files or folders were checked after the edits
- Any resulting errors or warnings caused by the implementation were fixed and the diagnostics check was repeated until clean, unless a concrete blocker prevented that
- The content of each changed file is accurate for this project
- Any required non-`.github/` fixes that remain out of scope are listed as concrete follow-up actions

## Output

Return:

- What you changed and why
- What you refused and why
- Concrete follow-up actions for any required non-`.github/` fixes
- Any problems you ran into
