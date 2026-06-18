---
description: "Lean project-orientation loop: map the repo cheaply with the project index before grepping or reading widely."
applyTo: "git-*,scripts/**,copilot-config/**,vscode-extension/**"
---

# Project Orientation (Lean)

For non-trivial tasks, orient from the project index before exploring by hand —
it is the cheapest way to work:

1. `index_project` to build/refresh the static repo map (files, symbols, and the
   reference graph), then `project_map` for a ranked overview in one call.
2. `lookup <symbol|file>` to find where something is defined and what references
   it, instead of grepping or reading many files.
3. Only fall back to manual search when the index does not answer the question.

Keep the index current; prefer it over re-deriving structure.
