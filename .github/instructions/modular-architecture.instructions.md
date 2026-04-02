---
applyTo: "**"
description: "Modular architecture principles for the git-shell-helpers codebase. Prevents shallow patches and enforces decomposition of monolithic files."
---

# Modular Architecture Principles

This codebase has several monolithic files (2000–4400 lines). These rules prevent shallow patches and guide toward decomposition.

## The Thorough Fix Mandate

**Before modifying any file over 500 lines**, you MUST:

1. Read the function index: `grep -n 'function \|^[a-z_]*()' <file>` (for Bash) or `grep -n 'function \|^[a-zA-Z_]*()' <file>` (for JS).
2. Understand the call chain of the function(s) you're touching — read 200+ lines of surrounding context.
3. Identify whether the change affects callers, callees, or shared state.
4. If the fix involves a function over 100 lines, consider whether it should be extracted first.

**An 8-line patch to a 3000-line file is almost always wrong.** Either: (a) you don't understand the surrounding context, or (b) the function should be extracted and fixed as a unit.

## File Size Limits

| Category            | Soft Limit | Hard Limit | Action                                      |
| ------------------- | ---------- | ---------- | ------------------------------------------- |
| Shell scripts       | 500 lines  | 1000 lines | Extract functions into `lib/` sourced files |
| Node.js files       | 500 lines  | 1000 lines | Extract modules into `lib/` or `src/`       |
| Any single function | 100 lines  | 200 lines  | Extract helper functions                    |

When a file exceeds the soft limit, new features should be added to extracted modules, not appended to the monolith.

## Decomposition Patterns

### Bash: Sourced library files

```bash
# In git-upload, replace 200-line function bodies with:
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib/upload-test-detection.sh"
```

Library files go in `lib/` at the repo root. Each library:

- Contains related functions (test detection, risk scoring, AI message generation, etc.)
- Uses the same `set -euo pipefail` and logging conventions
- Does not define `main()` — only functions
- Is tested via the existing test suite

### Node.js: Module extraction

```javascript
// In git-research-mcp, replace inline function blocks with:
const {
  buildKnowledgeIndex,
  searchKnowledgeIndex,
} = require("./lib/mcp-knowledge-index");
```

Module files go in `lib/` adjacent to the main script, or `src/` for the VS Code extension.

## Current Monoliths and Target Modules

| File                            | Current Lines | Target Modules                                                                                                                                   |
| ------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `git-upload`                    | ~2,960        | `lib/upload-spinner.sh`, `lib/upload-test-detection.sh`, `lib/upload-ai-message.sh`, `lib/upload-diff-analysis.sh`, `lib/upload-risk-scoring.sh` |
| `git-research-mcp`              | ~3,030        | `lib/mcp-knowledge-index.js`, `lib/mcp-web-search.js`, `lib/mcp-google-headless.js`, `lib/mcp-knowledge-rw.js`                                   |
| `vscode-extension/extension.js` | ~4,390        | `vscode-extension/src/mcp-client.js`, `vscode-extension/src/command-handlers.js`, `vscode-extension/src/configuration.js`                        |
| `git-help-i-pushed-an-env`      | ~2,500        | `lib/env-scan-patterns.sh`, `lib/env-history-rewrite.sh`, `lib/env-batch-ops.sh`                                                                 |
| `git-copilot-quickstart`        | ~1,180        | `lib/quickstart-templates.sh` + `templates/` directory for scaffold content                                                                      |

## Refactoring Process

When extracting from a monolith:

1. Identify a cohesive group of functions (e.g., all test-detection functions in `git-upload`: `detect_vscode_test_task`, `detect_test_cmd`, `summarize_test_output`, `compute_testing_status`, `extract_test_failure_count`, `extract_test_total_count`).
2. Create the target library file with the extracted functions.
3. Add a `source` or `require` statement in the original file.
4. Run `bash ./scripts/test.sh` to verify nothing broke.
5. Update `man/man1/` and `--help` if behavior descriptions changed.

Do NOT refactor and add features in the same change. Refactor first, then add features to the clean module.

## Separation of Concerns

Each file should have ONE clear responsibility:

- `git-upload`: orchestrate the stage/commit/push flow (delegate detection, scoring, AI to libraries)
- `git-research-mcp`: MCP protocol handling and tool dispatch (delegate knowledge ops, search, scraping to modules)
- `vscode-extension/extension.js`: VS Code lifecycle and command registration (delegate MCP client, config management to modules)

If a file does more than one thing well, it should be split so each part can be understood, tested, and modified independently.
