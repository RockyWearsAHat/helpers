# AGENTS.md — Git Shell Helpers (GSH)

> Discovery file for AI coding agents. GSH is agent-agnostic tooling shipped as a
> standard **stdio MCP server** plus a `gsh` control CLI. Any MCP-capable agent
> (Claude Code, GitHub Copilot, Cursor, …) can use it.

## What GSH gives an agent

A persistent toolset that helps you write **longer, more accurate, better code**:

- **Project index** — a cheap, static map of the repo (files, symbols, reference
  graph, ranked) so you orient in one call instead of grepping or reading widely
  (`index_project`, `project_map`, `lookup`). Shareable across projects as a `.dxbundle`.
- **Knowledge** — a knowledge index/cache so you recall durable facts and community
  knowledge instead of re-searching (`search_knowledge_index`, `search_knowledge_cache`).
- **Research** — web search + page scraping gated behind the index and knowledge
  (`search_web`, `scrape_webpage`).
- **Quality gates** — `strict_lint` after edits, `checkpoint` for disciplined commits.
- **Local subagents & tool building** — delegate to local models, build reusable
  per-project tools (`ollama_subagent`, `build_workspace_tool`).
- **CS A+ grading** — objectively grade a CS2420/CS3500 project and drive it to an A+
  (`gsh grade`).

## Install (any agent)

```sh
# From the GSH source checkout:
./gsh install                 # auto-detect installed agents (Claude, Copilot)
./gsh install --agent claude  # Claude Code only
./gsh install --agent all     # everything
```

`gsh install` registers the MCP server and installs agent-native config
(Claude: CLAUDE.md core block, skills, slash commands; Copilot: agents/instructions/skills).

### Manual MCP registration

The server is plain stdio MCP — register it however your agent expects:

```sh
# Claude Code
claude mcp add -s user gsh -- node /absolute/path/to/git-shell-helpers-mcp

# Generic mcp.json
{ "mcpServers": { "gsh": { "command": "node",
  "args": ["/absolute/path/to/git-shell-helpers-mcp"] } } }
```

## Control & toggling (live — no restart)

```sh
gsh status              # what's installed, master switch, tool counts
gsh disable | enable    # master kill-switch for the whole GSH surface
gsh bypass              # toggle the master switch
gsh tool list           # every tool + on/off state
gsh tool disable <name> # turn one tool off
gsh tool enable all     # turn everything back on
gsh doctor              # health checks
```

Tool state lives in `~/.config/git-shell-helpers-mcp/tools.json` and is re-read by the
running server on every request, so toggles apply immediately. A disabled tool can be
overridden for a single call with `{ "force": true }`.

## Fast startup (auto-managed background server)

`gsh install` registers a tiny **C launcher** (`gsh-mcp`) instead of launching Node
directly. It starts in ~1ms and connects to a background server
(`git-shell-helpers-mcpd.js`) that stays resident, so sessions start fast instead of
paying cold Node startup. It is fully automatic: the server starts on demand,
idle-exits after ~15 min, is per-workspace (keyed by cwd + GSH env), and re-reads
tool state per request so `gsh` toggles stay live.

If the server isn't instantly ready — or there's no C compiler — the launcher falls
back to running Node directly within ~2s, so it's always reliable and never worse than
plain Node. Controls: `gsh daemon status`, `gsh daemon restart`, `gsh build`.

## Notes for agents

- Prefer a GSH tool over shell emulation when one fits; don't build one-off tools for
  trivial tasks.
- Tools needing the VS Code extension (branch sessions, VS Code-backed `strict_lint`,
  vision, `list_language_models`) degrade gracefully elsewhere; memory, research,
  checkpoint, subagent, and grading tools work standalone.
- See `README.md` for full per-tool docs and packaging.
