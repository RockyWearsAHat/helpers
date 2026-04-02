---
name: devops-audit-evaluation
description: "Checklist for evaluating project-specific Copilot customization quality."
user-invocable: false
---

# DevOps Audit Evaluation

Inspect every file in the selected Copilot target surface. Decide whether each one is correct, useful, and properly written for this project, and whether it is the cleanest practical implementation of the intended workflow.

## The Two Assumptions

1. The project source code is correct. Do not question it. The developers wrote what they wanted.
2. The Copilot files in the selected target surface are not trustworthy until you have verified them yourself. Assume every file might be wrong, outdated, duplicated, or poorly written. Prove it is correct before moving on.

This is the core of the evaluation: the project is right, the Copilot setup might not be.

## What You Are Evaluating

You are evaluating whether the selected Copilot target surface actually helps someone develop in this project. Not whether it looks nice, not whether it follows some template, but whether it makes Copilot more useful for real work in this specific codebase.

For globally shipped helpers or workflows that claim to work across many repositories, evaluation stays centered on the selected target surface, but it must validate the external surfaces that make those claims true. If installability, accessibility, or behavioral claims depend on README text, man pages, installer scripts, user-level install locations, or observable runtime entrypoints, those surfaces are mandatory validation inputs because they determine whether the target surface is truthful.

Do not stop at "not broken." If the research shows a cleaner, clearer, more maintainable, or more efficient way to implement the intended workflow, that is a valid finding even when the current file technically works.

Do not invent improvements just because a workflow is impressive or sophisticated. A recommendation is valid only when the research supports it for this project. Specialized flows such as visual tooling, replay capture, or autonomous debugging helpers should be treated as project-specific options, not default upgrades.

Evaluate in this order:

1. `Platform validity` — is the file supported and technically correct right now?
2. `Primitive fit` — is this the right Copilot surface for this kind of content?
3. `Project value` — does it help this repository's real workflows?
4. `Recommendation strength` — is any change required, recommended, optional, or merely illustrative?

## For Every File, Answer These Questions

1. Is this file technically correct? (Valid YAML frontmatter, correct field names, proper file naming, no deprecated syntax)
2. Is the content accurate for this project? (Does it describe things that are actually true about this codebase?)
3. Even if the formatting is wrong, is there useful content worth preserving or rewriting? (Separate technical invalidity from informational value.)
4. Does it actually help a developer? (Would removing this file make the Copilot experience worse?)
5. Is it concise? (Is it burning context window space with content that adds no value?)
6. Does it duplicate something else? (Is another file already covering this?)
7. If it references audit tools or audit processes, is that a legitimate repo-local workflow asset or misplaced meta-process?
8. Is it pushing a specialized workflow the project did not ask for and the research did not justify? (Optional workflows are not universal best practice.)
9. Does it guide Copilot with clear principles, expectations, goals, and boundaries, or is it mostly trying to make the output "seem right"?
10. Does it work with Copilot's intended workflow and primitive model, or does it fight the system by over-prescribing internals, duplicating methodology, or demanding fake certainty?
11. If you recommend a change, how strong is that recommendation: required, recommended, optional, or illustrative only?

## What To Look For

### File Type Purpose Violations

Each file type in the selected Copilot target surface has a specific purpose. Content in the wrong file type causes real problems — agents that do the work themselves instead of delegating, skills that shape identity instead of methodology, instructions that define behaviors no one asked for.

Research the current intended purpose of each file type before evaluating. These purposes could change over time, so verify them — do not assume. As of the last verified check:

- **Agent files** shape how a model behaves — identity, role, personality. They should not contain task methodology, step-by-step procedures, output format specs, or detailed instructions about what to read or produce. That belongs in skills.
- **Skill files** shape how a model performs a specific task — methodology, steps, sources, output format. They are not tied to a specific agent. They should not contain identity or behavioral shaping.
- **Instruction files** provide context that should be included in every request (or every request matching an `applyTo` pattern). They should not define agent behaviors or task procedures.
- **Prompt files** are reusable entry points — things users run over and over. They should not contain detailed methodology.

When an agent file contains detailed task instructions, the model reads them, sees it has the tools, and does the work itself instead of loading the skill and following the methodology there. This is one of the most common and damaging mistakes in Copilot customization.

Flag any file where content is in the wrong file type. This is a significant or critical problem depending on severity.

### Technical Issues

- YAML frontmatter errors (missing opening `---`, wrong field names like `mode:` instead of `agent:`, invalid tool names)
- Hardcoded model IDs that may not exist anymore
- Instructions that describe a different project or are too generic to be useful
- Files with `applyTo: ""` or overly broad `applyTo` patterns that load on every request
- Files that say nothing specific to this project and could apply to any codebase
- Missing coverage for workflows that this project clearly uses
- Recommendations or files that hardcode optional workflow preferences without project-specific evidence
- Conflicting instructions across files
- Files that were clearly auto-generated and never reviewed
- Files whose structure is broken but whose underlying project knowledge may still be worth salvaging into a correct format

### Tool and Discovery Design Issues

- Agent descriptions and skill descriptions that are vague, generic, or missing. These are the discovery surface — the model selects tools, skills, and agents based on how well descriptions match the user's intent. A description that says "does stuff" is like a tool with no description.
- Skill descriptions that don't state when to use AND when NOT to use the skill. Negative boundaries prevent mis-routing.
- Agent descriptions that lack output format expectations. If an agent or skill produces structured output, the description should say so — models handle structured results better when they know the shape upfront.
- Skills or agents with no concrete examples of expected invocation patterns. Anthropic's testing showed tool examples improve parameter accuracy from 72% to 90%. Skills that demonstrate concrete inputs and outputs are more reliably invoked.
- Agents with overly broad tool access when they only need a focused subset. This is the `allowed_callers` / least-privilege principle applied to Copilot: give each agent only the tools it actually needs.
- Multi-step workflows that keep all intermediate results in a single agent's context instead of delegating to subagents. This mirrors the programmatic tool calling principle: process intermediate data out of the main context window.
- Skills that pre-load all their methodology into the agent description instead of using progressive loading. This is the `defer_loading` principle: load full definitions only when actually needed.

### Prompting Quality and Workflow Fit Issues

- Prompts, instructions, skills, or agents that judge quality by tone, confidence, polish, or whether something "looks right" instead of whether it gives Copilot a clear job and truthful constraints
- Files that bury the real goal under style rules, forcing the model to optimize for appearances instead of useful behavior
- Content that duplicates step-by-step methodology in prompts or agent files instead of letting skills carry the method
- Guidance that fights Copilot's intended workflow by forcing rigid internal reasoning scripts, fake certainty, or unnecessary ceremony
- Prompts that do not define success criteria, evidence expectations, or boundaries, leaving Copilot to guess what "good" means
- Examples copied from strong repositories without translating them into this project's actual workflows and constraints

When you find one of these, evaluate it as a real quality problem, not a style nit. If the wording drives Copilot toward confusion, false confidence, or misuse of primitives, that is a legitimate finding.

## What To Ignore

- Source code quality (not your job)
- Build output or logs (not your job)
- Product-source critique outside the selected target surface unless a Copilot file directly references it
- External install, docs, packaging, or runtime surfaces only when they are unrelated to Copilot setup availability, accessibility, or truthfulness. If README, man pages, installer scripts, user-level install locations, or runtime behavior define whether the Copilot workflow is actually available or whether its claims are true, they are in scope and must be validated.
- Style preferences that do not change behavior or correctness
- The absence of global audit tools from the workspace. `DevOpsAudit` and its specialist agents may be installed in standard user-level locations on disk. They are not required to exist in the audited repository.

## Output

Return two sections:

Before writing findings, classify every proposed change with both of these labels:

- `Recommendation strength`: required / recommended / optional / illustrative
- `Evidence grade`: supported-current / supported-but-optional / deprecated / weakly-supported-opinionated

Use these labels consistently in the implementation plan and problems list.

### File Verdict Coverage

Before problems and gaps, list every inventoried Copilot file exactly once with:

- **File**: which file
- **Verdict**: keep / fix / merge / move / delete
- **Reason**: one or two sentences tied to the research and project context

If the context inventory listed 10 files, your coverage section must contain 10 verdicts. An evaluation that skips files is incomplete.

### Implementation Plan

After file verdict coverage and before problems and gaps, produce a concrete file-by-file change plan with:

- **File**: which file
- **Operation**: keep / edit / merge / move / delete / add
- **Recommendation strength**: required / recommended / optional / illustrative
- **Evidence grade**: supported-current / supported-but-optional / deprecated / weakly-supported-opinionated
- **Target state**: what the file should look like after the audit and why
- **Evidence**: the research conclusion or source that justifies the target state
- **Implementation notes**: the exact kind of edit needed, concise but concrete

This plan must be executable by the implementation agent without more research.

### Problems

For each problem found:

- **Severity**: critical / significant / minor
- **Recommendation strength**: required / recommended / optional / illustrative
- **Evidence grade**: supported-current / supported-but-optional / deprecated / weakly-supported-opinionated
- **File**: which file
- **What is wrong**: plain description
- **Evidence**: what you saw that proves it
- **Why it matters**: how this hurts the developer experience
- **Fix**: what should be done about it

### Gaps

Things that should exist but do not. Only list gaps where:

- The project clearly has a workflow or pattern that Copilot should know about
- The research findings show a correct way to address it
- Adding it would noticeably improve the development experience

Do not invent problems. Do not list things that are fine. If the setup is good, say it is good and explain why.

If a file is strong, explain why in behavioral terms: it gives Copilot a clear role, a concrete goal, honest boundaries, and a workflow that matches the intended primitive. Do not praise a file merely because it sounds polished.

Do not hide behind a tiny findings list. If you conclude that only one or two files need changes, your file verdict coverage must still show that you evaluated the rest and found them sound.
