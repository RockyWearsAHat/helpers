---
description: "Lean reference for helpers MCP tools."
---

# helpers MCP Tools (Lean)

When a helpers MCP tool exists for the requested action, prefer it over terminal emulation.

Core tools to prioritize:

1. `index_project` + `project_map` (and `lookup`) to map the repo cheaply before grepping or reading widely.
2. `search_knowledge_index` and `search_knowledge_cache` before external web search.
3. `lint` after file edits.
4. `lint` after touching code — fix the CS2420/CS3500 violations it lists (mandatory, not optional).
5. `checkpoint` at validated milestones — stage `paths` or `lines` for a focused commit.
6. `register_workspace_tool` to capture a recurring multi-step task as a one-call project flow; `list_workspace_tools` to reuse an existing one.

Load deeper tool details only when the current task requires them.

## Checkpoint Rules

- **Prefer `message`** with a specific description over AI generation.
- **Never include generated content.** Set `all: false` and name only the source files you authored. Generated outputs, build artifacts, and AI-drafted files must NOT be staged.
- **Use `model: "auto"`** for AI message generation.
- Checkpoint often at specific file granularity rather than blanket staging.
- The `branch` guard parameter is not used — no branch session tooling.

## lint Protocol

- Run `lint` (with `root` = project path, optional `max`) after edits. It returns a prioritized CS2420/CS3500 violation list (file:line + fix) **plus** the official rules from Helpers' webscraped, always-current lint index — **no local toolchain** (clippy/ruff/eslint/staticcheck) required.
- Success gate: clear the violations it lists (or explicitly justify any remainder) before reporting done; re-run to watch the count drop.
- Pair with `helpers grade` for the rubric.

## Research Order

- Project structure first: `project_map` / `lookup` (refresh with `index_project`).
- Project knowledge next: `search_knowledge_index`, then `search_knowledge_cache` if exact terms matter.
- External web only when the index and knowledge do not answer the question.
