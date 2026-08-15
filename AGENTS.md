# AGENTS.md — Helpers (Helpers)

> Discovery file for AI coding agents. Helpers is agent-agnostic tooling shipped as a
> standard **stdio MCP server** plus a `helpers` control CLI. Any MCP-capable agent
> (Claude Code, GitHub Copilot, Cursor, …) can use it.

## What Helpers gives an agent

A persistent toolset that helps you write **longer, more accurate, better code**:
project index (`index_project`, `project_map`, `lookup`), knowledge memory, gated
web research, quality gates (`lint`, `checkpoint`), reusable project flows
(`register_workspace_tool`), and CS2420/CS3500-grade linting. Full tool list and
what each does: `helpers.dx` (read via the dx MCP tools, e.g. `dx_read`, or
`dx read helpers.dx`).

## Install (any agent)

```sh
# From the Helpers source checkout:
./helpers install                 # auto-detect installed agents (Claude, Copilot)
./helpers install --agent claude  # Claude Code only
./helpers install --agent all     # everything
```

`helpers install` registers the MCP server and installs agent-native config
(Claude: CORE.md block, skills, slash commands; Copilot: agents/instructions/skills;
Codex too).

### Manual MCP registration

The server is plain stdio MCP, served directly by the native binary (no Node) —
register it however your agent expects:

```sh
# Claude Code
claude mcp add -s user helpers -- /absolute/path/to/helpers-native mcp

# Generic mcp.json
{ "mcpServers": { "helpers": { "command": "/absolute/path/to/helpers-native",
  "args": ["mcp"] } } }
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

The server is one compiled Rust binary with ~1ms cold start (no Node, no daemon needed);
architecture detail: `helpers.dx`.

## Notes for agents

- Prefer a Helpers tool over shell emulation when one fits; don't build one-off tools for
  trivial tasks — but capture *recurring* multi-step tasks as project flows.
- Every tool is deterministic and standalone (no AI), and works in any agent. The tools
  are native Rust for speed and type safety; only `search_web` / `scrape_webpage` run in
  Node (headless browser).
- **Documentation must be kept up to date with every change** — and up to date means
  **centralized and valid**, not longer: update the module docs / README / handoff notes your
  change made stale in the same edit, fold duplicates into the one authoritative doc, and delete
  anything that no longer matches the code. The next agent should read the docs cold and know
  exactly where the project stands. (The always-on rule lives in `agent-config/CORE.md`,
  Working discipline #7 — edit it there, never in the generated per-agent copies.)
- Full docs (every MCP tool, CLI command, architecture, dev/release process): see
  `helpers.dx` — read via the dx MCP tools (`dx_read`) or `dx read helpers.dx`.
