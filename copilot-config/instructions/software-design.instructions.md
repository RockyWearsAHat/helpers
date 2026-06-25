---
applyTo: "**"
description: "General CS principles and AI development practices. Language-specific rules live in the linter corpus — run lint after edits."
---

# Software Design & AI Development

Language-specific rules (naming, idioms, unsafe patterns) are enforced by the AI linter — run `lint` after edits. These principles apply regardless of language.

## Core CS Principles

1. **Clarity over cleverness.** Code is read far more than written. Prefer explicit, traceable logic.
2. **Single responsibility.** Every function, module, and type does one thing well. If you need "and" to describe it, split it.
3. **Validate at boundaries only.** Trust internal code and framework guarantees. Validate user input, external APIs, and file I/O — not internal calls.
4. **Error handling is not optional.** Every error path either recovers, propagates with context, or terminates explicitly. Never swallow.
5. **Preserve behavior unless change is intentional.** Refactor and feature work are separate commits.
6. **Complexity is a liability.** Big-O matters. Don't reach for a hash map when a list suffices; don't reach for a list when a single value suffices.

## MCP Tool Design

This repo ships MCP tools. Tool quality directly affects how well AI agents use them.

- **Schema descriptions are contracts.** Write them for an AI reader: state what the tool does, what inputs mean, what the output contains, and what to do next. Vague descriptions produce wrong calls.
- **Handler purity.** Tool handlers take args, return content. Side-effects (disk, network, git) are the explicit purpose — no hidden state, no global mutation beyond the operation itself.
- **Fail loudly and specifically.** Return `Err("tool_name: what went wrong and why")` — not a generic string. Agents retry based on error messages.
- **One tool, one job.** Composability beats monoliths. An agent can chain `lint_languages` → `lint_learn` → `lint_submit`; it cannot decompose a single bloated tool.
- **Output is read by an AI.** Structured prose (consistent columns, clear labels, explicit next steps) beats raw data dumps.

## AI Development Practices

- **Determinism where it matters.** Model calls and network fetches are non-deterministic; everything else should be. Keep the non-deterministic surface minimal and isolated.
- **Prompt injection defense.** Tool handlers that process user-supplied text or crawled content must not concatenate it into instructions or shell commands. Treat external data as data, never code.
- **Context efficiency.** Agents have limited context windows. Tool output should answer the question and no more — include what the agent needs for the next decision, not everything you could say.
- **Graceful degradation.** When a model, API, or network resource is unavailable, fall back to cached data or report clearly — never silently return empty or wrong results.
- **Test with real calls, not just mocks.** Agent behavior emerges from actual tool interactions. Unit tests verify logic; integration tests with a real agent verify the tool is actually useful.

## Code Organization

- Keep modules focused. When a file exceeds ~500 lines, extract cohesive groups into sibling modules.
- Prefer composition over inheritance. A type that holds a collaborator and delegates is easier to test and change than a subclass.
- Public surface gets a contract comment. Private helpers do not need one unless the why is non-obvious.
- Dead code is deleted, not commented out.
