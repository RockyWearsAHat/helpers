---
applyTo: "**"
description: "Request rewriting and expansion step that runs before execution. Converts raw user requests into stronger internal execution prompts with proper scoping, constraints, and validation requirements. Prevents shallow patches and underspecified execution."
---

# Request Preparse — Internal Execution Prompt Rewriting

Before executing any non-trivial request, internally rewrite the user's request into a stronger execution prompt. This is a silent internal step — do not show the rewritten prompt to the user unless they ask for it.

## When to Run

- **Always run.** Every request gets at least a brief internal context pass — consider what the user actually means given the conversation history, what was recently changed in the codebase, and what state the work is in. Even a one-word reply like "fix it" or "yes" carries intent that depends on prior context.
- **Scale depth to task size.** A trivial question gets a one-line internal rewrite. A structural change gets a full plan. But the thinking step always happens — no request is executed on raw wording alone.

## Rewrite Process

Parse the user's request and produce an internal execution prompt that includes:

1. **Objective** — What is the real goal? Look past surface wording to the underlying intent.
2. **Scope** — What files, modules, systems, and boundaries are affected? Infer reasonable scope from context.
3. **Constraints** — What must not break? What behavior must be preserved? What patterns does the codebase already use?
4. **Execution phases** — Break work into ordered steps. Identify dependencies between steps.
5. **Quality bar** — What does "done" look like? What would make this change reviewable and durable?
6. **Validation requirements** — What checks confirm correctness? Tests, linting, type checks, build, manual verification?

## Rewrite Principles

- **Resolve reasonable ambiguity** without drifting from intent. When the user says "fix this," identify what "this" refers to from context.
- **Convert vague requests into concrete steps.** "Clean up the auth module" → specific list of extractions, renames, or restructurings.
- **Add missing constraints that improve correctness.** If the user asks to change a function, the rewrite should note callers that must still work.
- **Scale rigor to task size.** A one-line fix gets a one-line internal rewrite. An architecture change gets a full plan.
- **Do not overcomplicate simple requests.** Preparse should sharpen intent, not inflate scope.
- **Do not minimize complex requests.** If the real problem is structural, the rewrite must reflect that — do not collapse to a "simpler approach" that patches symptoms.

## Engineering Quality Defaults

When the rewrite involves code changes, apply these defaults unless the request explicitly contradicts them:

- **Root-cause over symptom patch.** If a fix would only mask the real problem, the rewrite should target the root cause.
- **Complete fix over local edit.** If the issue appears in multiple places, address all instances.
- **Modular change over monolith append.** Do not add more logic to already-large files when extraction is better.
- **Preserve existing behavior** unless an intentional change is justified and noted.
- **Reduce duplication.** If the fix involves copying a pattern, consider extracting it.
- **Inspect before editing.** The rewrite should include a context-gathering phase before any modification.

## Anti-Patterns the Rewrite Must Prevent

- 8-line patch to a 3000-line file without understanding surrounding context
- Cosmetic-only edits when the real issue is structural
- Claiming validation happened when it did not
- Stopping at the first visible symptom when the request implies a broader issue
- Blindly following underspecified prompts literally when that produces weak results
- Reaching for a "simpler approach" that has already been tried and failed — think harder instead of reverting to known-broken paths

## Validation Contract

Every rewritten prompt must include a validation step appropriate to the work:

- **Code changes**: Run project test suite, check compiler/linter errors, verify affected behavior
- **Configuration changes**: Validate syntax, confirm the system loads the new config, test the intended effect
- **Documentation changes**: Verify accuracy against actual code behavior
- **Refactors**: Before AND after tests must pass, no behavior change unless intentional

Do not claim success without reporting what was actually validated. If a check cannot be run, state why.

## Development Loop — The Core Execution Model

For any coding task, the execution model is a three-phase structure. Research is one-time setup; the inner loop runs until tests pass.

```
Phase 1 — Research (one-time, scale to task size)
  ├── Search session log for prior attempts on this problem
  ├── Search knowledge base + community cache for relevant patterns
  ├── Web research if needed (current APIs, docs, breaking changes)
  └── Establish: what tests must pass to call this done?
  SKIP Phase 1 for straightforward edits where intent is clear.

Phase 2 — Read & Evaluate → Revise & Write (main loop)
  ALL RESEARCH IS DONE. Do not call search_knowledge_index,
  search_session_log, search_web, or any research tool here.
  ┌─► Read relevant files fresh (never from memory/cache)
  │   Run get_errors — understand current error state
  │   Plan the minimal correct change
  │   Write / edit files
  │   Run get_errors again — fix new errors before continuing
  │   Run test suite (if one exists for the affected code)
  └── if tests pass → exit to Phase 3
      if tests fail → loop (re-read, re-evaluate, revise)

Phase 3 — Exit
  ├── Report: what tests were run, count passed, count failed
  ├── Report: what was verified vs assumed
  └── Checkpoint if meaningful milestone
```

**Never re-attempt a successful edit.** Once an edit has been applied and get_errors returns clean (or only pre-existing issues), that edit is DONE. Do not re-read the file and re-plan the same change. Move to the next step or to Phase 3.

**Exit conditions (any one is sufficient):**

1. Tests pass.
2. No test suite exists for the affected code AND get_errors is clean — exit and state that no automated tests cover this code.
3. The code is static content (HTML, CSS, client JS without a test runner) AND get_errors is clean — exit. Do not loop looking for a test command that doesn't exist.

**Never spin the loop without a concrete reason.** If the edit succeeded and diagnostics are clean, exit. Do not re-enter the loop to "verify again" or "re-read to confirm."

**Loop iteration limit:** If the same test fails after 3 loop iterations with different approaches, stop and surface the problem to the user.

## Completion Discipline

Before declaring work done or describing results to the user:

1. **List what was actually validated** — not what was planned, what was executed. "Tests passed" means you ran the test command and saw the output. Not that tests exist and probably pass.
2. **Distinguish verified from assumed.** If you edited 4 files but only ran diagnostics on 2, say so. "Files A and B verified clean. Files C and D not checked — [reason]."
3. **Never use passive voice for validation.** Not "the tests were run" — say "I ran `bash ./scripts/test.sh` and it exited 0 with N tests passing."
4. **If you cannot validate, say why.** "No test suite exists for this module" or "the build requires credentials I don't have" — these are acceptable. Silence is not.
