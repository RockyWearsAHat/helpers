---
description: "Lean reference for gsh MCP tools."
---

# gsh MCP Tools (Lean)

When a gsh MCP tool exists for the requested action, prefer it over terminal emulation.

Core tools to prioritize:

1. `index_project` + `project_map` (and `lookup`) to map the repo cheaply before grepping or reading widely.
2. `search_knowledge_index` and `search_knowledge_cache` before external web search.
3. `strict_lint` after file edits.
4. `cs_lint` after touching code — fix the CS2420/CS3500 violations it lists (mandatory, not optional).
5. `checkpoint` at validated milestones — stage `paths` or `lines` for a focused commit.
6. `register_workspace_tool` to capture a recurring multi-step task as a one-call project flow; `list_workspace_tools` to reuse an existing one.

Load deeper tool details only when the current task requires them.

## Checkpoint Rules

- **Prefer `message`** with a specific description over AI generation.
- **Never include generated content.** Set `all: false` and name only the source files you authored. Generated outputs, build artifacts, and AI-drafted files must NOT be staged.
- **Use `model: "auto"`** for AI message generation.
- Checkpoint often at specific file granularity rather than blanket staging.
- The `branch` guard parameter is not used — no branch session tooling.

## strict_lint Protocol

- Run `strict_lint` with `filePath` for each edited file before reporting progress complete.
- Run `strict_lint` with `folderPath` or workspace scope only when a change can affect shared wiring across files.
- Success gate: zero errors and zero warnings on edited files, unless warnings are explicitly accepted and documented in report.
- If strict_lint reports diagnostics provider inactivity for a scope, treat that as a failure and fix provider setup/state before continuing.
- If strict_lint reports diagnostics from your edits, fix first, then rerun until clean.

## Research Order

- Project structure first: `project_map` / `lookup` (refresh with `index_project`).
- Project knowledge next: `search_knowledge_index`, then `search_knowledge_cache` if exact terms matter.
- External web only when the index and knowledge do not answer the question.
