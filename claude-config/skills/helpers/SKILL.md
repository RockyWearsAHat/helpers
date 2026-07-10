---
name: helpers
description: Discover, control, and use Helpers (formerly GSH / Git Shell Helpers) — the helpers MCP tools (research, knowledge memory, checkpoint, lint, local subagents, vision) and the helpers CLI for enabling/disabling Helpers and individual tools. Use when the user mentions Helpers OR its former names GSH / gsh / Git Shell Helpers, asks to enable/disable Helpers or a Helpers tool, asks what Helpers/GSH tools exist, wants to run a `gsh …`/`helpers …` command, or wants to install/configure Helpers. "GSH", "gsh", and "Git Shell Helpers" all mean Helpers.
---

# Helpers (formerly GSH / Git Shell Helpers)

> "GSH", "gsh", and "Git Shell Helpers" are the former names for Helpers. If the user asks to
> "use GSH" or run a `gsh …` command, that means Helpers / `helpers …`.

Helpers ships AI-agent tooling as a standard MCP server plus a `helpers` control CLI. It is
agent-agnostic; this skill is the Claude-side guide.

## Control the surface (run in a shell)

| Goal | Command |
| ---- | ------- |
| See what's installed & healthy | `helpers status` / `helpers doctor` |
| Turn ALL of Helpers off (bypass) | `helpers disable` |
| Turn Helpers back on | `helpers enable` |
| Toggle master switch | `helpers bypass` |
| List tools + on/off state | `helpers tool list` |
| Disable one tool | `helpers tool disable <name>` |
| Enable one tool | `helpers tool enable <name>` |
| Re-enable everything | `helpers tool enable all` |
| (Re)install into an agent | `helpers install [--agent auto\|claude\|copilot\|all]` |

Toggles are **live** — the MCP server re-reads `~/.config/helpers-server/tools.json`
on every request, so no restart is needed. After `helpers install`, run `/mcp` or restart
Claude Code so the `helpers` server connects.

If Helpers is disabled but the user explicitly asks for a Helpers action, you may call the tool
once with `{ "force": true }` to override the kill-switch.

## Tools (prefer these over shell emulation when they fit)

All tools are deterministic native Rust (no AI), except web search/scrape (Node).

**Workflow & quality**
- `lint` — prioritized CS2420/CS3500 violation list (file:line + fix); fix violations as you go.
- `lint` — run each language's own linters for a file/folder/workspace after edits.
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
4. `lint` after edits; `checkpoint` only after validation passes.

## CS2420 / CS3500 quality
`lint` enforces the CS2420 / CS3500 principles directly: it learns them from
`corpus/cs-principles.md` (alongside the official language rules it learns from the docs) and
flags violations with the exact `file:line` to fix. Followed to a T those principles ~guarantee
an A+, so a clean `lint` is the signal — there is no separate grader to run.
