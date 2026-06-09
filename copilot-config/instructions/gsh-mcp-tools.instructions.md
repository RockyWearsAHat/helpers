---
description: "Lean reference for gsh MCP tools."
---

# gsh MCP Tools (Lean)

When a gsh MCP tool exists for the requested action, prefer it over terminal emulation.

Core tools to prioritize:

1. `workspace_context` for workspace orientation at session start.
2. `strict_lint` after file edits.
3. `checkpoint` at validated milestones — see rules below.
4. Native `session_store_sql` (chronicle skill) for prior-session recall.
5. `list_language_models` when model choice impacts cost/quality.
6. `build_workspace_tool` when user asks to create a runnable tool.

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
- If strict_lint reports diagnostics from your edits, fix first, then rerun until clean.
