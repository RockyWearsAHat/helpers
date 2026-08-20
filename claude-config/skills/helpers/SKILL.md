---
name: helpers
description: Discover, control, and use Helpers (formerly GSH / Git Shell Helpers) — the helpers MCP tools (research, knowledge memory, checkpoint, lint, local subagents, vision) and the helpers CLI for enabling/disabling Helpers and individual tools. Use when the user mentions Helpers OR its former names GSH / gsh / Git Shell Helpers, asks to enable/disable Helpers or a Helpers tool, asks what Helpers/GSH tools exist, wants to run a `gsh …`/`helpers …` command, or wants to install/configure Helpers. "GSH", "gsh", and "Git Shell Helpers" all mean Helpers.
---

# Helpers (formerly GSH / Git Shell Helpers)

> "GSH", "gsh", and "Git Shell Helpers" are the former names for Helpers. If the user asks to
> "use GSH" or run a `gsh …` command, that means Helpers / `helpers …`.

Helpers ships AI-agent tooling as a standard MCP server plus a `helpers` control CLI. It is
agent-agnostic; this skill is the Claude-side guide.

## Working discipline

1. **Map before exploring.** Build/refresh the project index (`index_project`) and read
   `project_map` to orient in one call instead of grepping many files; use `lookup
   <symbol|file>` to find where something is defined and what references it. For durable
   facts, consult knowledge (`search_knowledge_index`, `search_knowledge_cache`) before
   reaching for the web. Stop once evidence suffices — over-exploration burns tokens.
2. **Sound foundation before building — confirm, then fix it first.** Bad architecture,
   spotty code, and shaky implementations are the #1 source of errors, and the hardest to
   solve, because the defect lives in the structure, not one line. When the map shows the
   ground you'd build on is unsound (tangled responsibilities, duplicated or dead code,
   leaky boundaries, no separation of concerns, missing/violated invariants), treat cleanup
   as the immediate first step, not a later pass. Confirm with the user before changing
   anything: name what's wrong and why it blocks the request, propose the refactor, get the
   go-ahead. When you find such rot mid-task, surface and fix it ASAP rather than coding
   around it.
3. **Prefer a Helpers tool over shell emulation** when one fits, but don't over-tool: no
   one-off tools for trivial tasks. When a multi-step task will recur, register it once as a
   project flow (`register_workspace_tool` → callable by name); check `list_workspace_tools`
   first.
4. **Loop: inspect → edit → validate → report.** No success claim without validation. Run
   `lint` (or the project's linter/build/tests) on changed files after edits.
5. **Checkpoint automatically at every verified milestone — do not wait to be told.** The
   moment step 4's validation passes on a coherent unit of work, call `checkpoint`: write
   your own `message` and stage a precise subset (`paths` / `lines`) — never `git add -A` of
   unrelated edits. Never stage generated or massive build artifacts; if tracked, remove from
   the index and gitignore them. This is default incremental-work behavior, not something
   reserved for when the user explicitly asks for a commit.
6. **Keep the workspace clean.** Generated files must never contaminate the repo.
7. **Documentation stays true, in one place.** Every change that makes any documentation
   stale — module docs, READMEs, handoff notes, instruction files, contract comments —
   updates it in the same edit. Up to date does not mean add more: it means centralize and
   validate — fold duplicates into the one authoritative place, delete what no longer
   matches the code, never describe a thing twice when one doc can be pointed at.

## Code quality bar — non-negotiable

Documentation and CS2420/CS3500 software principles are a behavior you always follow, not a
style preference. Whenever you touch code:

- **Document as you write.** Every public/exported function, type, and module gets a concise
  contract comment. Undocumented public surface is a defect, not a later task.
- **Hold the principles every edit:** clear naming, small single-responsibility units, no
  dead code, proper error handling (never swallow errors), appropriate data structures and
  complexity, tested behavior.
- **Composition over inheritance — always, beneath every other rule.** When a behavior can be
  reached by composing (fields, traits/interfaces, delegation, injected collaborators), do
  that instead of inheriting. Inheritance still fits a few genuinely fixed, fully-known
  hierarchies; treat such a base like a template — keep fixed functionality
  private/sealed, mark the slots subtypes must implement, document the contract. Prefer
  "has-a / uses-a" over "is-a".
- **Separate functions from data — but encapsulate what holds invariants.** Default to free
  functions over plain, open data: no hidden state, easiest thing to test and reuse. When a
  type carries rules that must always hold (balance never negative, list stays sorted),
  bundle the operations with the data and guard the invariant in one place instead of
  scattering it across call sites. Reusability comes from a narrow, honest interface, not
  from the paradigm.
- **Code reads like the project's own language.** Learn the project's domain terms and write
  code whose own lines read as that language: intent lives in the code, not propped up by
  comments. Public contracts still get their comment; the code carries the meaning.

Run `lint` after edits — it returns one prioritized CS2420/CS3500 violation list with
`file:line` and a fix. Treat its output like compiler errors: clear it (or justify each
remainder) before claiming done. `helpers grade` gives the rubric grade and gap-to-A+
checklist; `lint` gives the exact lines. Followed to a T, these principles ~guarantee an A+.

## Research — direct Google, no SearXNG

`search_web` / `scrape_webpage` drive a real (automated) Chrome straight against Google —
Node-free, no SearXNG, no local search service. Use after local memory/knowledge checks fail.
Ask Google direct questions like a human; don't mix many subjects in one query — learn
subjects individually, then combine into smarter searches. Stop once evidence suffices.

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
4. `lint` after edits; `checkpoint` immediately once validation passes — automatically, on
   every verified incremental milestone, never gated on the user asking for a commit.
