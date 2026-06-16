# Git Shell Helpers (GSH)

GSH is installed. It adds an MCP server (`gsh`) plus a `gsh` CLI for control. These
notes are always on; they shape how to work, not what to conclude.

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
2. **Prefer a GSH tool over shell emulation** when one fits, but don't over-tool: no
   one-off tools for trivial tasks. Built-in Claude tools (Read/Edit/Grep/Bash) remain
   first choice for ordinary file and shell work.
3. **Loop: inspect → edit → validate → report.** No success claim without validation.
   Run `strict_lint` (or the project's linter/build/tests) on changed files after edits.
4. **Checkpoint at verified milestones** with `checkpoint`: write your own `message`,
   set `all: false`, and name only files you actually changed. Never stage generated or
   massive build artifacts — if they're already tracked, remove from the index and
   gitignore them.
5. **Keep the workspace clean.** Generated files must never contaminate the repo. Use
   minimal tokens for maximal results.

## Code quality bar

Write code to CS2420/CS3500 A+ standards regardless of language: clear naming, small
single-responsibility units, documented public surface, no dead code, tested behavior,
appropriate data structures and complexity. Run `gsh grade` on a CS project to get an
objective rubric and a gap-to-A+ checklist.

## Control surface

- `gsh status` / `gsh doctor` — what's installed and healthy.
- `gsh index build|map|lookup <q>` — build the project index, print the ranked module
  map, or find a symbol/file. `gsh index export|install` shares a project's map as a
  portable `.dxbundle` so you can reference another project's structure.
- `gsh disable` / `gsh enable` — master switch; hides/shows the entire GSH tool surface
  live (no restart). `gsh bypass` toggles.
- `gsh tool list|enable|disable <name>` — turn individual tools on/off.
- Any disabled tool can be overridden for a single call with `{ force: true }`.

The project index, knowledge, checkpoint, strict_lint, subagent, and grading tools work
standalone. Tools that need the VS Code extension (branch sessions, VS Code-backed
`strict_lint`, vision/screenshot, `list_language_models`) degrade gracefully outside it.
