---
name: devops-audit-evaluation
description: "Checklist for evaluating project-specific Copilot customization quality."
user-invocable: false
---

# DevOps Audit Evaluation

Inspect every file in `.github/` that Copilot uses. Decide whether each one is correct, useful, and properly written for this project, and whether it is the cleanest practical implementation of the intended workflow.

## The Two Assumptions

1. The project source code is correct. Do not question it. The developers wrote what they wanted.
2. The `.github/` Copilot files are not trustworthy until you have verified them yourself. Assume every file might be wrong, outdated, duplicated, or poorly written. Prove it is correct before moving on.

This is the core of the evaluation: the project is right, the Copilot setup might not be.

## What You Are Evaluating

You are evaluating whether the `.github/` Copilot configuration actually helps someone develop in this project. Not whether it looks nice, not whether it follows some template, but whether it makes Copilot more useful for real work in this specific codebase.

Do not stop at "not broken." If the research shows a cleaner, clearer, more maintainable, or more efficient way to implement the intended workflow, that is a valid finding even when the current file technically works.

## For Every File, Answer These Questions

1. Is this file technically correct? (Valid YAML frontmatter, correct field names, proper file naming, no deprecated syntax)
2. Is the content accurate for this project? (Does it describe things that are actually true about this codebase?)
3. Even if the formatting is wrong, is there useful content worth preserving or rewriting? (Separate technical invalidity from informational value.)
4. Does it actually help a developer? (Would removing this file make the Copilot experience worse?)
5. Is it concise? (Is it burning context window space with content that adds no value?)
6. Does it duplicate something else? (Is another file already covering this?)
7. Does it reference audit tools or audit processes? (This is a serious problem — audit files should help the project, not describe the audit itself)

## What To Look For

### File Type Purpose Violations

Each file type in `.github/` has a specific purpose. Content in the wrong file type causes real problems — agents that do the work themselves instead of delegating, skills that shape identity instead of methodology, instructions that define behaviors no one asked for.

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
- Conflicting instructions across files
- Files that were clearly auto-generated and never reviewed
- Files whose structure is broken but whose underlying project knowledge may still be worth salvaging into a correct format

## What To Ignore

- Source code quality (not your job)
- Build output or logs (not your job)
- Anything outside `.github/` unless a Copilot file directly references it
- Style preferences that do not change behavior or correctness
- The absence of global audit tools from the workspace. `DevOpsAudit` and its specialist agents may be installed in standard user-level locations on disk. They are not required to exist in the audited repository.

## Output

Return two sections:

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
- **Target state**: what the file should look like after the audit and why
- **Evidence**: the research conclusion or source that justifies the target state
- **Implementation notes**: the exact kind of edit needed, concise but concrete

This plan must be executable by the implementation agent without more research.

### Problems

For each problem found:

- **Severity**: critical / significant / minor
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

Do not hide behind a tiny findings list. If you conclude that only one or two files need changes, your file verdict coverage must still show that you evaluated the rest and found them sound.
