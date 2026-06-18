# AGENTS.md — Helpers (Helpers)

> Discovery file for AI coding agents. Helpers is agent-agnostic tooling shipped as a
> standard **stdio MCP server** plus a `helpers` control CLI. Any MCP-capable agent
> (Claude Code, GitHub Copilot, Cursor, …) can use it.

## What Helpers gives an agent

A persistent toolset that helps you write **longer, more accurate, better code**:

- **Project index** — a cheap, static map of the repo (files, symbols, reference
  graph, ranked) so you orient in one call instead of grepping or reading widely
  (`index_project`, `project_map`, `lookup`). Shareable across projects as a `.dxbundle`.
- **Knowledge** — a knowledge index/cache so you recall durable facts and community
  knowledge instead of re-searching (`search_knowledge_index`, `search_knowledge_cache`).
- **Research** — web search + page scraping gated behind the index and knowledge
  (`search_web`, `scrape_webpage`).
- **Quality gates** — `strict_lint` after edits, `checkpoint` for disciplined commits.
- **Project flows** — register a recurring multi-step task once as a named tool any
  agent can call (`register_workspace_tool`, `list_workspace_tools`), scoped to the
  project — turning context repetition into one tool call.
- **CS A+ grading** — objectively grade a CS2420/CS3500 project and drive it to an A+
  (`helpers grade`).

## Install (any agent)

```sh
# From the Helpers source checkout:
./helpers install                 # auto-detect installed agents (Claude, Copilot)
./helpers install --agent claude  # Claude Code only
./helpers install --agent all     # everything
```

`helpers install` registers the MCP server and installs agent-native config
(Claude: CLAUDE.md core block, skills, slash commands; Copilot: agents/instructions/skills).

### Manual MCP registration

The server is plain stdio MCP — register it however your agent expects:

```sh
# Claude Code
claude mcp add -s user helpers -- node /absolute/path/to/helpers-server

# Generic mcp.json
{ "mcpServers": { "helpers": { "command": "node",
  "args": ["/absolute/path/to/helpers-server"] } } }
```

## Control & toggling (live — no restart)

```sh
helpers status              # what's installed, master switch, tool counts
helpers disable | enable    # master kill-switch for the whole Helpers surface
helpers bypass              # toggle the master switch
helpers tool list           # every tool + on/off state
helpers tool disable <name> # turn one tool off
helpers tool enable all     # turn everything back on
helpers doctor              # health checks
```

Tool state lives in `~/.config/helpers-server/tools.json` and is re-read by the
running server on every request, so toggles apply immediately. A disabled tool can be
overridden for a single call with `{ "force": true }`.

## Fast startup (auto-managed background server)

`helpers install` registers a tiny **C launcher** (`helpers-mcp`) instead of launching Node
directly. It starts in ~1ms and connects to a background server
(`helpers-serverd.js`) that stays resident, so sessions start fast instead of
paying cold Node startup. It is fully automatic: the server starts on demand,
idle-exits after ~15 min, is per-workspace (keyed by cwd + Helpers env), and re-reads
tool state per request so `helpers` toggles stay live.

If the server isn't instantly ready — or there's no C compiler — the launcher falls
back to running Node directly within ~2s, so it's always reliable and never worse than
plain Node. Controls: `helpers daemon status`, `helpers daemon restart`, `helpers build`.

## Notes for agents

- Prefer a Helpers tool over shell emulation when one fits; don't build one-off tools for
  trivial tasks — but capture *recurring* multi-step tasks as project flows.
- Every tool is deterministic and standalone (no AI), and works in any agent. The tools
  are native Rust for speed and type safety; only `search_web` / `scrape_webpage` run in
  Node (headless browser).
- See `README.md` for full per-tool docs and packaging.
