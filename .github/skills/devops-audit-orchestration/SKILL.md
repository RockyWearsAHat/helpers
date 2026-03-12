---
name: devops-audit-orchestration
description: "Operational workflow for running a Copilot audit as one ordered pass."
user-invocable: false
---

# DevOps Audit Orchestration

Use the agent tool to start isolated subagent calls. Start immediately. Do not inspect the workspace yourself before the first subagent call.

This skill describes how the main chat orchestrator should behave. Do not launch a separate "orchestration" subagent to load this skill. The main chat session is the orchestrator.

## Mandatory Full Pipeline — No Exceptions

The four-phase pipeline always runs completely. Every audit, every time, regardless of what the user said.

- User gave a short command with no extra text → run all four phases for the whole codebase.
- User gave a detailed multi-paragraph request → run all four phases, focused on that request.
- User sent follow-up messages after the command → incorporate them and run all four phases.
- Previous audit artifacts exist → run all four phases anyway (reuse artifacts only if they pass the gate).

Do not skip phases. Do not collapse phases. Do not do any phase's work yourself. Do not stop early because the request "seems simple." The pipeline is the mechanism for all audit work.

## User Focus Forwarding

Everything the user typed after the slash command, plus any follow-up messages, is the **user focus**. Pass the user focus verbatim into every phase subagent prompt. Each phase must receive the focus and optimize its work toward it.

When writing the subagent prompt for each phase, include the user focus near the top, not buried at the bottom. Example:

> "The user focus for this audit is: [exact user text]. Optimize your [context/research/evaluation/implementation] for this focus."

If no user focus exists, tell each phase: "No specific focus was provided. The entire codebase is the scope."

The user focus determines what the research targets. If the user said "build a visual testing flow," the research must find evidence about visual testing patterns in Copilot customization, not just generic Copilot docs. If the user said "qt qss," the research must target Qt/QSS-specific Copilot guidance. The focus is not a tag — it drives the research direction.

Use isolated subagents for the runtime workflow. In each subagent prompt, tell the subagent exactly which audit skill to load, what inputs to use, what boundaries to respect, and what output to return.

When invoking a phase subagent, use the exact named audit specialist for that phase. Never intentionally fall back to a generic exploration agent.

Judge routing by observable runtime behavior, not by hidden implementation assumptions. The debug UI may show a generic model turn while still loading the correct audit skill and continuing through the intended workflow. Do not reject a run solely because the UI does not expose the custom agent identity clearly.

Treat routing as valid when the observable signals line up:

- the audit prompt loads the audit orchestration skill or equivalent audit instructions
- the run preserves the four audit phases in order
- the named audit phase specialists are the intended targets for subagent calls
- the main session does not silently collapse into doing the phase work itself

Treat routing as failed only when runtime evidence is concrete, such as:

- an explicit wrong-agent or unknown-agent error
- a clearly named non-audit subagent such as `Explore` being invoked for a phase that should use an audit specialist
- the main session performing context, research, evaluation, or implementation work itself without the audit pipeline

Judge phase outputs by substance, not ceremony. A phase is valid only if it covers its full scope with the required evidence, correct source weighting, current guidance for normative claims, and project-specific detail. If a phase is incomplete, generic, unsupported, anchored on stale guidance, or unable to recover from a weak line of inquiry by trying another path, reject it.

Run four subagents in order. Each one finishes before the next starts. Pass only compact handoff summaries to the next phase, not giant raw transcripts or repeated file contents.

If a phase fails quality control, rerun that same phase once with a short deficiency list that says exactly what was missing and which alternate evidence paths it should pursue next. If the retry still fails, stop the audit. Do not keep marching forward with weak inputs.

## Handoff Format

Every accepted phase output should be reduced to a compact handoff with these fields:

- `Status`: complete / incomplete
- `Coverage`: what was covered and how many files or evidence items were included
- `Key facts`: only the facts the next phase needs
- `Target state`: what the next phase should treat as the clean or correct implementation direction
- `Evidence refs`: doc titles, URLs, repo/file paths, release note months, transcript titles, or explicit tool failures
- `Blockers`: anything unresolved

Never pass full chat transcripts or large copied file bodies unless a later phase explicitly needs one small excerpt.

## Pipeline

1. **Gather Context** → start an isolated subagent. Tell it to load `devops-audit-context`. Give it the user's request and user focus. It returns a compact project profile and a complete `.github/` Copilot file inventory. The context agent must capture the user focus prominently in the project profile.
2. **Research Best Practices** → start a new isolated subagent. Tell it to load `copilot-research`. Give it the accepted context handoff and the user focus. Tell it to optimize research for the user focus — if the user asked about visual testing, research visual testing patterns; if they asked about Qt, research Qt-specific guidance. It returns current Copilot customization guidance for this project type, including an explicit target-state blueprint optimized for the user focus. The research subagent must treat workspace exploration as out of scope, must use `fetch` and `githubRepo` before any terminal fallback, and must return concrete external evidence rather than a workspace-derived opinion.
3. **Evaluate Setup** → start a new isolated subagent. Tell it to load `devops-audit-evaluation`. Give it the accepted context and research handoffs plus the user focus. It returns file-by-file verdict coverage, prioritized problems and gaps, and an implementation-ready change plan that addresses the user focus.
4. **Implement Fixes** → start a new isolated subagent. Tell it to load `devops-audit-fix`. Give it the approved file-by-file change plan plus supporting research evidence. It applies the changes.

## Phase Gates

### Context Gate

Accept context only if all of these are true:

- The project profile includes build, test, and key workflow information when that information exists in the workspace.
- The Copilot inventory covers every relevant file under `.github/`.
- Each inventory entry includes the path, frontmatter, content summary, and notes.
- The output is compact. Do not accept giant pasted file bodies when summaries would suffice.

### Research Gate

Accept research only if all of these are true:

- Every mandatory topic from `copilot-research` is addressed.
- The output includes many concrete evidence references, not generic claims.
- The output shows why the guidance is current for normative claims, how source types were weighted, and which older examples remain useful only as illustrative patterns.
- Model validation is either verified or reported as a specific blocked check with the exact fallback used.
- At least 3 real repositories were inspected at the file level, not just their landing pages.
- Those repository examples were explored beyond the README: the research read actual `.github/` files and at least one other useful project artifact such as build config, CI, tests, or developer docs to understand why the customization fits the project.
- At least 1 recent product-team video transcript was used, and preferably 2 when the topic is broad enough to support it. If transcript tooling or source availability blocks this, the blocker must be explicit and concrete.
- The output clearly states what correct or better target-state customization should look like for this project or user focus.
- The recommendations are specific enough that the evaluator could rewrite the audited `.github/` files without guessing.
- The output identifies concrete opportunities for improvement, simplification, or bug-finding rather than only validating current state.
- The evidence set is broad enough that a result based mostly on overview docs or community meta-guides would fail.
- The phase demonstrates resilience by pursuing alternate sources or tools when one path fails instead of giving up at the first dead end.
- The transcript evidence is actually used to extract practical workflow principles, not just listed as a citation.
- The research explains the relevant system and workflow in simple plain English rather than hiding behind jargon or source dumping.
- The research leaves no blocker-level confusion unresolved for the evaluator or implementer. Minor uncertainty is acceptable only when it does not prevent defining the best-practices target state or producing an implementation-ready plan.

### Evaluation Gate

Accept evaluation only if all of these are true:

- Every inventoried Copilot file receives an explicit verdict such as keep, fix, merge, move, or delete.
- Problems and gaps are tied back to research evidence.
- If few or no problems are found, the evaluator still explains why the remaining files are correct.
- The evaluation is specific to this workspace. If it would fit any repo unchanged, reject it.
- The evaluation includes a concrete file-by-file change plan that an implementation agent could execute without doing more research.

### Implementation Gate

Accept implementation only if all of these are true:

- The changed files match the approved fixes.
- The implementation input was concrete enough that the implementer did not need to infer the target state from raw links or a broad doc packet.
- Verification was performed and reported.
- No blocked audit artifacts were created.
- Any refused changes are explained clearly.

The orchestrator never performs context-gathering, research, evaluation, or implementation itself. It only invokes subagents, reviews results, and gives the final sign-off.

Phase acceptance rule: accept only outputs that clearly pass the gate for that phase. Do not restart a completed phase because the subagent described its own runtime in an unexpected way, but do reject it if the substance is weak.

If any required subagent cannot be invoked, stop immediately. Report the concrete runtime problem and do not fall back to manual execution. A manual fallback defeats the architecture and produces misleading behavior.

After the last agent finishes, review the implementation result against the approved findings, then report to the user what was found, what changed, and whether any phase had to be rejected or retried.
