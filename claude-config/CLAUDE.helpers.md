# Helpers (formerly GSH / Git Shell Helpers)

Helpers is installed. It adds an MCP server (`helpers`) plus a `helpers` CLI for control. These
notes are always on; they shape how to work, not what to conclude.

**Naming:** Helpers was previously called **GSH** / **Git Shell Helpers** — same thing. If asked
to "use GSH" (or to run a `gsh …` command), use Helpers: the command is `helpers …` and the MCP
server is `helpers`. Treat "GSH", "gsh", and "Git Shell Helpers" as aliases for Helpers.

## Working discipline

1. **Map before exploring — it's the cheapest way to work.** Build/refresh the project
   index (`index_project`) and read `project_map` to orient in a single call instead of
   grepping and reading many files; use `lookup <symbol|file>` to find where something is
   defined and what references it. The index is a static, ~instant repo map (files,
   symbols, and the reference graph, ranked by importance), so leaning on it directly
   cuts tokens and time — prefer it over re-deriving structure by hand, and keep it
   current. For durable facts, consult knowledge (`search_knowledge_index`,
   `search_knowledge_cache`) before reaching for the web. Stop once evidence suffices —
   over-exploration burns tokens.
2. **Prefer a Helpers tool over shell emulation** when one fits, but don't over-tool: no
   one-off tools for trivial tasks. Built-in Claude tools (Read/Edit/Grep/Bash) remain
   first choice for ordinary file and shell work. When a multi-step task will recur,
   register it once as a project flow (`register_workspace_tool` → callable by name);
   check `list_workspace_tools` before re-implementing a task.
3. **Loop: inspect → edit → validate → report.** No success claim without validation.
   Run `lint` (or the project's linter/build/tests) on changed files after edits.
4. **Checkpoint at verified milestones** with `checkpoint`: write your own `message` and
   stage a precise subset — pass `paths` for specific files, or `lines` for specific line
   ranges (a focused checkpoint, never `git add -A` of unrelated edits). Never stage
   generated or massive build artifacts — if tracked, remove from the index and gitignore
   them.
5. **Keep the workspace clean.** Generated files must never contaminate the repo. Use
   minimal tokens for maximal results.

## Code quality bar — non-negotiable behavior

Documentation and CS2420/CS3500 software principles are **not optional and not a
style preference** — they are a behavior you always follow. Whenever you touch code:

- **Document as you write.** Every public/exported function, type, and module gets a
  concise contract comment. Writing undocumented public surface is a defect, not a
  later task.
- **Hold the principles every edit:** clear naming, small single-responsibility units,
  no dead code, proper error handling (never swallow errors), appropriate data
  structures and complexity, tested behavior.
- **Fix violations immediately.** Run **`lint`** after edits — it returns one clean,
  prioritized list of CS2420/CS3500 violations with `file:line` and a fix. Treat its
  output like compiler errors: resolve them before claiming done; re-run until the count
  is zero (or each remaining item is explicitly justified). Do not introduce new
  violations, and leave code at least as clean as you found it.
- `helpers grade` gives the rubric grade and gap-to-A+ checklist; `lint` gives the exact
  lines to fix. Use both: grade to know where you stand, `lint` to drive it to clean.

## Control surface

- `helpers status` / `helpers doctor` — what's installed and healthy.
- `helpers index build|map|lookup <q>` — build the project index, print the ranked module
  map, or find a symbol/file. `helpers index export|install` shares a project's map as a
  portable `.dxbundle` so you can reference another project's structure.
- `helpers disable` / `helpers enable` — master switch; hides/shows the entire Helpers tool surface
  live (no restart). `helpers bypass` toggles.
- `helpers tool list|enable|disable <name>` — turn individual tools on/off.
- Any disabled tool can be overridden for a single call with `{ force: true }`.

Every Helpers tool is deterministic and standalone — no tool calls an AI model, and they
all work in any agent. The tools (project index, knowledge, checkpoint, lint,
project flows, grading) are native Rust for speed and type safety; only `search_web` /
`scrape_webpage` stay in Node (they drive a headless browser).
