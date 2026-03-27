---
name: devops-audit-orchestration
description: "Operational workflow for running a Copilot audit as one ordered pass."
user-invocable: false
---

# DevOps Audit Orchestration

Use the agent tool to start isolated subagent calls. Start immediately. Do not inspect the workspace yourself before the first subagent call.

This skill describes how the main chat orchestrator should behave. Do not launch a separate "orchestration" subagent to load this skill. The main chat session is the orchestrator.

This skill can be loaded in two valid ways:

- `DevOpsAudit` is the active responding custom agent.
- A natural-language-routed main chat session is temporarily acting as the audit coordinator.

In both cases, the main chat session is the orchestrator. Do not invoke `DevOpsAudit` as a nested subagent and then try to run the pipeline from inside that nested agent.

The orchestrator must never perform phase work itself. If `runSubagent` is unavailable or phase delegation fails, stop and report the runtime failure.

## Request Contract

Before the first phase, the orchestrator must derive a `request contract` from the user's wording. This is a mandatory interpretation step, not optional polish.

The request contract must contain:

- `Primary action` — what the user is actually asking for: audit, evaluate, explain, improve, debug, redesign, install, update, or something else specific
- `Primary object` — what that action applies to: workspace customization, audit workflow internals, shipped product source, installed user assets, or another named surface
- `Requested outcome` — report, findings, code changes, parser improvement, behavior redesign, root-cause explanation, or another concrete deliverable
- `Explicit exclusions` — hard boundaries such as no edits, no run, no install, no `.github` fallback, or no command execution
- `Target surface` — workspace-runtime, product-source, user-install, or mixed

Rules:

- Do not treat every sentence after an entrypoint as equal scope. Separate the main ask from examples, rationale, and constraints.
- Constraints override defaults. If the user says not to run the audit or not to inspect a particular surface, do not do it.
- If the contract shows the user is asking to improve, debug, or redesign the audit workflow itself, that is not an audit run. Do not start the audit pipeline. Report the classification and handle it as a direct customization task instead.
- Pass the accepted request contract into every subagent prompt so later phases do not drift away from the original ask.

## Mandatory Full Pipeline

The audit always runs the full pipeline required by the user's requested outcome.

- User gave a short command with no extra text → run the default four phases for the whole codebase.
- User asked a question, requested explanation, asked how to design or improve a setup, or otherwise framed the request as advisory/theoretical without asking for file changes → run Context, Research, and Evaluation, then stop and report. Do not run Implementation.
- User gave a detailed multi-paragraph request → run the default four phases, focused on that request.
- User explicitly asked for no edits, read-only output, report-only mode, or a concise overview without changes → run Context, Research, and Evaluation, then stop and report. Do not run Implementation.
- User explicitly asked to apply, fix, update, rewrite, create, delete, or otherwise change files → run the default four phases for that scope.
- User sent follow-up messages after the command → incorporate them and run the appropriate full pipeline for the updated request.
- Previous audit artifacts exist → run the appropriate full pipeline anyway (reuse artifacts only if they pass the gate).

Do not skip phases. Do not collapse phases. Do not do any phase's work yourself. Do not stop early because the request "seems simple." The pipeline is the mechanism for all audit work.

Default mode phases:

1. Context → 2. Research → 3. Evaluation → 4. Implementation

Report-only mode phases:

1. Context → 2. Research → 3. Evaluation

## Judgment Model

The audit must separate four different questions. Do not blur them together.

1. `Platform validity` — supported syntax, correct frontmatter fields, correct file placement, deprecated field detection, and obviously broken routing.
2. `Primitive fit` — whether instructions, prompts, agents, skills, and hooks are being used for the right job.
3. `Recommendation strength` — whether a conclusion is required, recommended, optional, or illustrative only.
4. `Project fit` — whether the recommendation actually helps this repository and the user's stated focus.

Apply these rules throughout the pipeline:

- Start from the smallest viable target state. For many repositories, that is repo-wide instructions plus a few scoped instructions.
- Do not treat prompts, agents, skills, hooks, model pinning, subagent topology, or research caches as mandatory unless the evidence shows a clear payoff for this project.
- When the repository deliberately includes a repo-local Copilot workflow such as an audit system, evaluate it like any other project asset: by correctness, scope, maintenance cost, and developer value.
- A strong recommendation explains why it is good in platform terms such as token cost, routing reliability, least privilege, maintainability, and workflow fit.

## Target Surface Selection

Before invoking the first phase, classify the requested audit target surface from the user's wording. Keep this separate from the user focus and derive it from the request contract, not from a default assumption.

- `workspace-runtime` — the live workspace Copilot files, usually under `.github/`
- `product-source` — repo-owned source files that define shipped Copilot behavior, such as `copilot-config/`
- `user-install` — installed user-level Copilot assets outside the repo
- `mixed` — more than one surface must be audited together

Rules:

- Do not rewrite a request about `copilot-config/`, audit-system source files, shipped prompts, shipped skills, or shipped instructions into a `.github/` audit.
- If the repository's own baseline instructions declare a source-of-truth location for Copilot assets, the context phase must honor that declaration and return it explicitly.
- If the user's request is clearly about auditing the audit workflow itself, that is still an audit request. Audit the workflow's source surface rather than treating it as a router-debug request.
- Pass the target surface classification, the specific paths in scope, and the reason for that choice into every phase prompt.

## Global Helper Integrity Checks

When the audited Copilot workflow is intended to operate as a global or cross-workspace helper, portability and truthfulness become mandatory audit surfaces, not optional extras.

The audit must verify these surfaces together:

- the repo-local source files that define the workflow, whether that is `.github/`, `copilot-config/`, or another declared Copilot source surface
- the user-level install locations or packaging surfaces that make the helper available across workspaces
- the public entrypoints that expose the helper, including deterministic slash-command entrypoints and any best-effort natural-language routing
- the truthfulness of README, man page, installer, and runtime claims about installability, accessibility, and behavior

Claims that the helper installs alongside Copilot extensions, is globally accessible, or is reachable through a specific public entrypoint must match actual installer and runtime behavior. If those claims cannot be verified, the evaluation must treat that as an integrity problem and the implementation handoff must include explicit follow-up actions for any required non-`.github/` fixes.

## User Focus Forwarding

Everything the user typed after the slash command, plus any follow-up messages, is the **user focus**. Pass the user focus verbatim into every phase subagent prompt. Each phase must receive the focus and optimize its work toward it.

The user focus is not the same thing as the request contract. The request contract tells you what job to do and what not to do. The user focus tells you what aspect of that job matters most.

When writing the subagent prompt for each phase, include the user focus near the top, not buried at the bottom. Example:

> "The user focus for this audit is: [exact user text]. Optimize your [context/research/evaluation/implementation] for this focus."

If no user focus exists, tell each phase: "No specific focus was provided. The entire codebase is the scope."

If the user explicitly says not to make edits, include that near the top of every subagent prompt. The evaluation phase should still produce an implementation-ready plan, but the orchestrator must not invoke the implementation phase.

When the user focus is a question or advisory request and does not explicitly ask to change files, treat that as report-only intent even if the user did not say "no edits" verbatim. Requests like "how do I write a good Copilot setup", "what should this look like", "review this design", or "what are best practices" are advisory unless the user also asks to apply the recommendations.

The user focus determines what the research targets. If the user said "build a visual testing flow," the research must find evidence about visual testing patterns in Copilot customization, not just generic Copilot docs. If the user said "qt qss," the research must target Qt/QSS-specific Copilot guidance. The focus is not a tag — it drives the research direction.

Do not let one example focus mutate into a default recommendation for every project. The orchestrator's job is to keep the audit specific to the current repository and current request, not to spread specialized workflows into repos that do not need them.

Use isolated subagents for the runtime workflow. In each subagent prompt, tell the subagent exactly which audit skill to load, what inputs to use, what boundaries to respect, and what output to return.

If the repo includes a versioned community-cache contract or shared-cache manifest for the audit, pass that to the research phase as an allowed input. Shared-cache pull should be treated as normal bootstrap behavior when configured, but shared-cache publication must remain opt-in and should not be triggered unless the user explicitly asked for it.

For natural-language-routed runs, keep the current main session as the coordinator and invoke the phase specialists directly. Do not first launch `DevOpsAudit` as a subagent. Nested orchestration is less reliable than keeping the coordinator at the top level, and it contradicts this skill's own execution model.

When invoking a phase subagent, use the exact named audit specialist for that phase. Never intentionally fall back to a generic exploration agent.

Judge routing by observable runtime behavior, not by hidden implementation assumptions. The debug UI may show a generic model turn while still loading the correct audit skill and continuing through the intended workflow. Do not reject a run solely because the UI does not expose the custom agent identity clearly.

Treat routing as valid when the observable signals line up:

- the audit prompt loads the audit orchestration skill or equivalent audit instructions
- the main session is acting as the coordinator, whether that main session is `DevOpsAudit` or a natural-language-routed top-level session
- the run preserves the four audit phases in order
- the named audit phase specialists are the intended targets for subagent calls
- the main session does not silently collapse into doing the phase work itself

Treat routing as failed only when runtime evidence is concrete, such as:

- an explicit wrong-agent or unknown-agent error
- the run nests `DevOpsAudit` under another agent and then tries to orchestrate the pipeline from inside that nested subagent
- a clearly named non-audit subagent such as `Explore` being invoked for a phase that should use an audit specialist
- the main session performing context, research, evaluation, or implementation work itself without the audit pipeline

Judge phase outputs by substance, not ceremony. A phase is valid only if it covers its full scope with the required evidence, correct source weighting, current guidance for normative claims, and project-specific detail. If a phase is incomplete, generic, unsupported, anchored on stale guidance, or unable to recover from a weak line of inquiry by trying another path, reject it.

Run the required subagents in order for the active mode. Each one finishes before the next starts. Pass only compact handoff summaries to the next phase, not giant raw transcripts or repeated file contents.

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

1. **Gather Context** → start an isolated subagent. Tell it to load `devops-audit-context`. Give it the request contract, the user's request, user focus, and the preclassified target surface if one is obvious. It returns a compact project profile and a complete inventory for the selected Copilot target surface(s). The context agent must preserve the request contract, capture the user focus prominently in the project profile, and justify the target surface it chose.
2. **Research Best Practices** → start a new isolated subagent. Tell it to load `copilot-research`. Give it the accepted context handoff, the request contract, the user focus, and the target surface. Tell it to optimize research for the user focus — if the user asked about visual testing, research visual testing patterns; if they asked about Qt, research Qt-specific guidance. It returns current Copilot customization guidance for this project type, including an explicit target-state blueprint optimized for the user focus and target surface. The research subagent must treat workspace exploration as out of scope, must use `fetch` and `githubRepo` before any terminal fallback, and must return concrete external evidence rather than a workspace-derived opinion.
3. **Evaluate Setup** → start a new isolated subagent. Tell it to load `devops-audit-evaluation`. Give it the accepted context and research handoffs, the request contract, and the user focus. It returns file-by-file verdict coverage, prioritized problems and gaps, and an implementation-ready change plan that addresses the user focus.
4. **Implement Fixes** → only in default mode, start a new isolated subagent. Tell it to load `devops-audit-fix`. Give it the approved file-by-file change plan, the request contract, and supporting research evidence. It applies the changes.

## Phase Gates

### Context Gate

Accept context only if all of these are true:

- The project profile includes build, test, and key workflow information when that information exists in the workspace.
- The Copilot inventory covers every relevant file in the selected target surface(s).
- Each inventory entry includes the path, frontmatter, content summary, and notes.
- The output is compact. Do not accept giant pasted file bodies when summaries would suffice.

### Research Gate

Accept research only if all of these are true:

- Every mandatory topic from `copilot-research` is addressed.
- The output includes many concrete evidence references, not generic claims.
- The output shows why the guidance is current for normative claims, how source types were weighted, and which older examples remain useful only as illustrative patterns.
- If a shared community cache was available, the output states which manifest or snapshot was used and which cached items were revalidated, downgraded, or rejected.
- Model validation is either verified or reported as a specific blocked check with the exact fallback used.
- At least 3 real repositories were inspected at the file level, not just their landing pages.
- Those repository examples were explored beyond the README: the research read actual `.github/` files and at least one other useful project artifact such as build config, CI, tests, or developer docs to understand why the customization fits the project.
- At least 1 recent product-team video transcript was used, and preferably 2 when the topic is broad enough to support it. If transcript tooling or source availability blocks this, the blocker must be explicit and concrete.
- The output clearly states what correct or better target-state customization should look like for this project or user focus.
- The output avoids pushing specialized workflows or tooling into the target state unless the evidence shows they fit this project or the user explicitly asked for them.
- The recommendations are specific enough that the evaluator could rewrite the audited target-surface files without guessing.
- The output identifies concrete opportunities for improvement, simplification, or bug-finding rather than only validating current state.
- The evidence set is broad enough that a result based mostly on overview docs or community meta-guides would fail.
- The phase demonstrates resilience by pursuing alternate sources or tools when one path fails instead of giving up at the first dead end.
- The transcript evidence is actually used to extract practical workflow principles, not just listed as a citation.
- The research explains the relevant system and workflow in simple plain English rather than hiding behind jargon or source dumping.
- The research leaves no blocker-level confusion unresolved for the evaluator or implementer. Minor uncertainty is acceptable only when it does not prevent defining the best-practices target state or producing an implementation-ready plan.

### Evaluation Gate

Accept evaluation only if all of these are true:

- Every inventoried Copilot file receives an explicit verdict such as keep, fix, merge, move, or delete.
- Problems and gaps are tied back to research evidence and are labeled by recommendation strength.
- If few or no problems are found, the evaluator still explains why the remaining files are correct.
- The evaluation is specific to this workspace. If it would fit any repo unchanged, reject it.
- The evaluation includes a concrete file-by-file change plan that an implementation agent could execute without doing more research.

### Implementation Gate

Accept implementation only if all of these are true:

- The changed files match the approved fixes.
- The run was actually in implementation mode because the user explicitly asked for file changes, or gave a direct audit command with no advisory-only wording.
- The implementation input was concrete enough that the implementer did not need to infer the target state from raw links or a broad doc packet.
- Verification was performed and reported.
- VS Code diagnostics for the relevant target-surface files or folders were checked after the edits, any resulting errors or warnings caused by the implementation were fixed, and the check was repeated until the edited target surface was clean or a concrete blocker was reported.
- No blocked audit artifacts were created.
- Any refused changes are explained clearly.

If report-only mode is active, skip the implementation gate entirely and require the final report to state that no files were changed.

The orchestrator never performs context-gathering, research, evaluation, or implementation itself. It only invokes subagents, reviews results, and gives the final sign-off.

Phase acceptance rule: accept only outputs that clearly pass the gate for that phase. Do not restart a completed phase because the subagent described its own runtime in an unexpected way, but do reject it if the substance is weak.

If any required subagent cannot be invoked, stop immediately. Report the concrete runtime problem and do not fall back to manual execution. A manual fallback defeats the architecture and produces misleading behavior.

After the last required agent finishes, review the accepted outputs against the approved findings, then report to the user what was found, whether anything changed, and whether any phase had to be rejected or retried. In report-only mode, explicitly say that no files were changed.

## Automatic Post-run Community Submission

Community submission runs automatically after the required audit phases succeed. The submit script handles permission gating — the orchestrator does not check modes or settings.

After the last required phase finishes and the orchestrator accepts the final result:

1. Invoke `DevOpsAuditCommunitySubmit` with the accepted final result and tell it to load `devops-audit-community-submit`.
2. Tell it to extract ALL generalizable conclusions (target 3-10 per audit), not just one.
3. The submit agent will check community participation settings and skip gracefully if submission is not allowed for this environment.
4. Do not gate this invocation on participation mode — always invoke, let the submit pipeline decide.

If the submit agent reports that submission was skipped due to settings, that is normal. If it reports an error, include it in the final report but do not fail the audit.
