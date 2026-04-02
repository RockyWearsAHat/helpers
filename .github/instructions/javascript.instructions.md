---
applyTo: "**/*.js,**/*.mjs"
description: "JavaScript and Node.js conventions for MCP servers and the VS Code extension."
---

# JavaScript / Node.js Conventions

## General

- Use `'use strict'` in all Node.js files.
- Prefer `const` over `let`; never use `var`.
- Use `async/await` over raw Promises or callbacks.
- Handle errors at system boundaries (stdin/stdout parsing, HTTP responses, file I/O). Do not wrap every function in try/catch.
- Use `process.exit(1)` with stderr for fatal errors.

## MCP Servers (`git-research-mcp`, `git-shell-helpers-mcp`)

- These are single-file servers communicating via JSON-RPC over stdin/stdout.
- The `send()` function is the only output path — never write to stdout directly.
- Keep tool handler functions pure where possible: take args, return result objects.
- When adding new tools, register them in BOTH `listTools` and the handler dispatch.

## VS Code Extension (`vscode-extension/extension.js`)

- Extension lifecycle: `activate()` registers everything, `deactivate()` cleans up.
- Commands register via `vscode.commands.registerCommand` — keep handlers short, delegate to helper functions.
- Configuration reads via `vscode.workspace.getConfiguration('gitShellHelpers')`.
- Disposables must be pushed to `context.subscriptions`.

## Module Extraction

When `extension.js` or an MCP server exceeds 1000 lines, extract cohesive groups of functions into `lib/` or `src/` modules:

```javascript
// In the main file:
const {
  searchKnowledgeIndex,
  buildKnowledgeIndex,
} = require("./lib/mcp-knowledge-index");
```

Each extracted module should:

- Export named functions (no default exports for multi-function modules).
- Declare its own dependencies via `require()`.
- Be independently testable.
