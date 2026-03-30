```instructions
# {{PROJECT_NAME}} — Copilot Instructions

This file is intentionally short. Detailed guidance lives in `.github/instructions/`.

## Model Selection

- **Default model**: GPT-4.1 (general tasks, documentation, code review)
- **Plan agent** (`@Plan`): Claude Opus 4.5 (complex research and planning)
- **Implement agent** (`@Implement`): Raptor Mini (execution and implementation)

## Agents

- **`@Plan`** — Diagnose → Write EXACT CODE to `.github/plan.md`. Never implements directly.
- **`@Implement`** — Execute `.github/plan.md` step-by-step. Never improvises or interprets.

**Workflow:** User request → `@Plan` (diagnose + write code plan) → `@Implement` (execute exactly)

## Golden Rules

1. **Documentation-first TDD**: update docs/spec → write tests that mirror the docs → implement.
2. **Spec-driven development**: prefer official documentation and project docs over assumptions.
3. **No hacks**: avoid shortcuts that compromise maintainability or correctness.
4. **Keep code clean**: small diffs, clear names, consistent style, and documentation updates.
5. **Clean only after user verification**: once the user confirms the issue is solved, clean workspace artifacts.
6. **Plans contain CODE**: `@Plan` writes exact code blocks, not prose. `@Implement` copy-pastes.

## Canonical Docs (read first)

- `.github/instructions/memory.md` — living "memory" of the codebase (update alongside changes)
- `.github/instructions/tdd.md` — documentation-first TDD workflow (tests mirror docs)
- `.github/instructions/workspace-hygiene.md` — what to delete, when to delete it
- `.github/instructions/code-style.md` — conventions and style guide

## Build / Test

{{BUILD_SECTION}}
{{TEST_SECTION}}

## Cleanup

- Remove build artifacts before committing large changes
- Keep logs and debug output out of version control
- Run cleanup only after user verification

```
