---
name: gsh
description: Discover, control, and use Git Shell Helpers (GSH) — the gsh MCP tools (research, knowledge memory, checkpoint, strict_lint, local subagents, vision) and the gsh CLI for enabling/disabling GSH and individual tools. Use when the user mentions GSH, asks to enable/disable GSH or a GSH tool, asks what GSH tools exist, or wants to install/configure GSH.
---

# GSH — Git Shell Helpers

GSH ships AI-agent tooling as a standard MCP server plus a `gsh` control CLI. It is
agent-agnostic; this skill is the Claude-side guide.

## Control the surface (run in a shell)

| Goal | Command |
| ---- | ------- |
| See what's installed & healthy | `gsh status` / `gsh doctor` |
| Turn ALL of GSH off (bypass) | `gsh disable` |
| Turn GSH back on | `gsh enable` |
| Toggle master switch | `gsh bypass` |
| List tools + on/off state | `gsh tool list` |
| Disable one tool | `gsh tool disable <name>` |
| Enable one tool | `gsh tool enable <name>` |
| Re-enable everything | `gsh tool enable all` |
| (Re)install into an agent | `gsh install [--agent auto\|claude\|copilot\|all]` |

Toggles are **live** — the MCP server re-reads `~/.config/git-shell-helpers-mcp/tools.json`
on every request, so no restart is needed. After `gsh install`, run `/mcp` or restart
Claude Code so the `gsh` server connects.

If GSH is disabled but the user explicitly asks for a GSH action, you may call the tool
once with `{ "force": true }` to override the kill-switch.

## Tools (prefer these over shell emulation when they fit)

All tools are deterministic native Rust (no AI), except web search/scrape (Node).

**Workflow & quality**
- `cs_lint` — prioritized CS2420/CS3500 violation list (file:line + fix); fix violations as you go.
- `strict_lint` — run each language's own linters for a file/folder/workspace after edits.
- `checkpoint` — stage/commit with your own `message` (or a deterministic one); stage a precise subset with `paths` (files) or `lines` (line ranges).

**Project index** (cheap repo map — orient without grepping)
- `index_project` — build/refresh the static map of files, symbols, and the reference graph.
- `project_map` — ranked module overview + Mermaid graph in one cheap call.
- `lookup` — where a symbol is defined / what references it, from the graph.

**Project flows** (reusable, scoped to the project, callable by any agent)
- `register_workspace_tool` — register a named shell command/flow as a one-call MCP tool.
- `unregister_workspace_tool` / `list_workspace_tools` — manage and discover flows.

**Knowledge & web**
- `search_knowledge_index` / `search_knowledge_cache` / `read_knowledge_note` — repo + community knowledge.
- `write_knowledge_note` / `update_knowledge_note` / `append_to_knowledge_note` — persist findings.
- `search_web`, `scrape_webpage` — external facts after the index + knowledge miss.

## Efficient order
1. `project_map` / `lookup` (refresh with `index_project`) to orient cheaply instead of grepping.
2. Consult knowledge before web search.
3. One specialized tool for the goal; `scrape_webpage` only for top hits needing depth.
4. `strict_lint` after edits; `checkpoint` only after validation passes.

## Grading CS projects
For CS2420 (data structures/algorithms) or CS3500 (OOD/MVC) projects, use the `cs-grade`
skill or run `gsh grade <path> --course cs2420|cs3500` to produce an objective `GRADE.md`
and a prioritized path to an A+.
