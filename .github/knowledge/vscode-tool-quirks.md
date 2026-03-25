# VS Code Agent Tool Quirks & Pitfalls

Practical reference for the built-in tools exposed to AI models in VS Code agent mode. These are the real-world failure modes and workarounds discovered through direct testing, VS Code source analysis, and community reports. Everything here is verified behavior as of VS Code 1.106+ (mid-2025).

---

## Terminal Tool (`run_in_terminal`)

### The zsh Comment Bug

**The single most common error in agent workflows on macOS.**

In zsh interactive shells, `INTERACTIVE_COMMENTS` is **OFF by default**. The `#` character is NOT treated as a comment — it's a literal character. When the model appends `# explanation` to a command:

```
ls #list files          → ls: #list: No such file or directory
cat #check config       → cat: #check: No such file or directory
grep -r "test" . # RHS → searches for "test" in ".", "#", "RHS"
```

**Root cause**: VS Code's zsh shell integration script (`shellIntegration-rc.zsh`) only calls `setopt HIST_IGNORE_SPACE`. It does **not** call `setopt INTERACTIVE_COMMENTS`. The user's `.zshrc` typically doesn't set it either.

**Rule**: Never use `#` comments in terminal commands. If you need to explain what a command does, explain it in the `explanation` parameter of the tool call, not inline.

### Shell Grammar Mismatch

VS Code uses a **tree-sitter bash grammar** to parse terminal commands for all non-PowerShell shells. There is no zsh grammar. Consequences:
- zsh-specific syntax that differs from bash may not be correctly parsed
- The `;` operator in zsh may be handled differently than the parser expects
- Auto-approve patterns that rely on subcommand detection can miss zsh-specific constructs
- Quote concatenation can bypass auto-approve rules (`find -e"x"ec` bypasses the `find -exec` blocker)

### Persistent Session State

Non-background `run_in_terminal` calls share a **single persistent zsh session**:
- Working directory (`cd`) persists between calls
- Environment variables (`export`) persist between calls
- Shell aliases and functions loaded from `.zshrc` are available
- If one command changes state (sets a variable, changes directory), all subsequent commands inherit that state

Background processes (`isBackground: true`) get their **own fresh shell** starting at the workspace root. They don't share state with the main session.

### Output Handling

- Output is **automatically truncated at ~60KB**. When exceeded, the content is written to a temp file and a path is returned. Always filter large output with `head`, `tail`, `grep`, or `awk` BEFORE it gets too large.
- Prefer `| head -N` or `| tail -N` to limit output proactively, especially for `find`, `ls -R`, `grep -r`, or any command that might produce unbounded output.
- Long-running commands should use `isBackground: true` with `get_terminal_output` for checking later.
- The `timeout` parameter stops *tracking* after the specified milliseconds, returning whatever output was collected. The command may still be running. Be conservative — always set timeouts longer than you think necessary, or use 0 for no timeout rather than guessing too low.

### Shell Integration Fragility

The terminal tool depends on VS Code's shell integration to detect when commands start and finish. This can break with:
- Custom zsh themes that use **RPROMPT / RPS1** (right-side prompts). These interfere with VS Code's prompt detection, causing the agent to hang waiting for command completion.
- The `DISABLE_MAGIC_FUNCTIONS=true` setting in `.zshrc`, which disables zsh hooks VS Code relies on.
- Heavily customized oh-my-zsh or powerlevel10k configurations that override prompt sequences.
- Stale terminals — after extended idle time, shell integration can lose sync. If the terminal hangs, a new terminal session may resolve it.

**Workaround**: If the terminal hangs indefinitely, the issue is almost certainly shell integration, not the command itself. Don't retry the same command — that won't help. Consider the shell config.

### Sequential Only

Never call `run_in_terminal` multiple times in parallel. The tool shares a single non-background session — parallel calls would interleave commands in the same shell. Run one command, wait for output, then run the next.

### The "Simplified" Message

Every terminal tool invocation shows "Note: The tool simplified the command to..." — this is informational, not a warning. It shows the exact command being executed (with leading whitespace stripped). It does NOT mean the command was materially modified.

---

## File Editing Tool (`replace_string_in_file` / `multi_replace_string_in_file`)

### Exact Match: Character-for-Character

The `oldString` must match the file content **exactly**:
- Every space, tab, newline, and invisible character must match
- Tab-indented code cannot be matched with space-indented strings
- Trailing whitespace on lines must match (even if invisible)
- Line endings (LF vs CRLF) must match the file's actual endings
- NO regex, NO globbing, NO fuzzy matching — strictly literal

### Uniqueness Requirement

The `oldString` must appear **exactly once** in the file:
- Zero matches → failure ("could not find exact match")
- Multiple matches → failure (ambiguous)
- Include **at least 3 lines of context** before and after the target to ensure uniqueness
- Common single-line strings like `return null;` or `import React from 'react';` will likely match multiple locations

### Auto-Format Race Condition

If VS Code auto-formats a file between successive `replace_string_in_file` calls, the file's whitespace/indentation may change. The next call's `oldString` — based on the pre-format state — will no longer match.

**Mitigation**: When making multiple edits to the same file, read the file content between replacements if auto-formatting is enabled, or use `multi_replace_string_in_file` to batch them (sequential execution within a single tool call).

### Cascading Failure in Multi-Replace

`multi_replace_string_in_file` applies replacements **sequentially**. If replacement #1 changes the file content in a region that replacement #2's `oldString` references, #2 will fail because the text it expects is no longer there. Order replacements carefully — work from bottom to top of the file, or ensure non-overlapping regions.

### Never Replace What You Haven't Read

Always read the current file content before constructing `oldString`. Guessing at indentation, variable names, or surrounding context is the #1 cause of edit failures. A single wrong character means failure.

---

## File Creation Tool (`create_file`)

- **Fails on existing files**: Will NOT overwrite. Use `replace_string_in_file` to edit existing files.
- **Auto-creates directories**: Parent directories are created if they don't exist. No need to `mkdir -p` first.
- **For full file rewrites**: There is no built-in tool for replacing entire file contents. Options: (a) use `replace_string_in_file` with the full file content as `oldString` (fragile for large files), or (b) use the terminal to write files (bypasses VS Code's undo stack and edit tracking).

---

## File Reading Tool (`read_file`)

- **1-indexed lines**: Line numbers start at 1, not 0. `startLine: 1, endLine: 10` reads the first 10 lines.
- **Prefer large ranges**: One call reading 200 lines is better than twenty calls reading 10 lines each. The tool is optimized for larger reads.
- **Binary files**: `startLine`/`endLine` are interpreted as byte offsets for binary files.
- **Read before edit**: Always read the region you plan to edit to get the exact current content. Never rely on stale/cached mental models of file content.

---

## Search Tools

### `grep_search`

- **Case-insensitive by default**.
- **Respects ignore rules**: `.gitignore`, `search.exclude`, and `files.exclude` settings apply. Files in `node_modules/`, build outputs, etc. are excluded unless `includeIgnoredFiles: true` is set.
- **`includePattern` uses glob syntax, not regex**: No `|` operator. Use patterns like `**/*.ts` or `src/folder/**`. Can also be an absolute path.
- **Limited results by default**: Not all matches are returned. Use `maxResults` only when needed — it slows the search.
- **Can scope to a single file**: Set `includePattern` to a relative file path to search within one file. Useful for getting an overview of a file's structure.

### `semantic_search`

- **Never call in parallel**: Must be called sequentially — likely due to embedding/indexing constraints.
- **Natural language queries**: Works best with terms that might appear in code comments, function names, or variable names.
- **Full workspace for small repos**: Returns complete file contents for small workspaces, reducing the need for targeted reads.

---

## Cross-Tool Gotchas

### Don't Use Terminal for File Edits

The terminal can write files (`cat >`, `echo >>`, `sed -i`), but these bypass VS Code's edit tracking, undo stack, and diff view. The user won't see what changed. Always use `replace_string_in_file` or `create_file` for file modifications unless explicitly asked to use the terminal.

### Tool Availability Can Change Mid-Session

Users can enable/disable tools at any time via the tools picker. A tool that worked at the start of a conversation may become unavailable later. If a tool call fails with "tool not available," don't retry — adapt.

### Large File Scalability

For files over ~1000 lines, `replace_string_in_file` becomes increasingly fragile because:
- More potential for duplicate matches
- More context needed for uniqueness
- Higher chance of stale state between reads and edits

For large files, prefer multiple targeted small edits over ambitious multi-hundred-line replacements.

### Path Requirements

All tool calls that take file paths require **absolute paths**. Relative paths will fail silently or target the wrong location. Always construct paths from the workspace root.
