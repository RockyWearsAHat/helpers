---
applyTo: "**"
description: "Critical rules for using VS Code's built-in agent tools correctly. Prevents common failures with terminal, file editing, and search tools."
---

# VS Code Tool Safety Rules

For the full reference, see the `vscode-tool-quirks.md` knowledge note.

## Terminal Commands

- **Never use `#` comments in terminal commands.** zsh's `INTERACTIVE_COMMENTS` is off by default. Inline comments are treated as literal arguments and cause errors. Use the tool's `explanation` parameter instead.
- **Never call `run_in_terminal` in parallel.** Non-background calls share one shell. Run one command, wait for output, run the next.
- **Filter large output proactively.** Pipe through `head`, `tail`, `grep`, or `awk` before output exceeds 60KB. Don't let `find`, `ls -R`, or `grep -r` run unbounded.
- **Set generous timeouts.** If unsure how long a command takes, use `timeout: 0` rather than guessing short.
- **If the terminal hangs**, the issue is shell integration, not the command. Don't retry — the shell config is the problem.

## File Edits

- **Always read before editing.** Never construct `oldString` from memory. Read the exact current content first.
- **Include 3+ lines of context** in `oldString` to ensure unique matching. One-line matches are fragile.
- **Watch for auto-format interference.** If multiple edits fail on the same file, VS Code may be reformatting between calls.
- **Use `multi_replace_string_in_file` for batched edits.** But order carefully — replacements run sequentially and earlier changes affect later matches.

### "Mismatched context" — What It Means and How to Recover

`replace_string_in_file` fails with "mismatched context" when `oldString` doesn't match the file's actual bytes. This is **always caused by one of**:

1. **Stale read** — `oldString` was constructed from memory or a cached file view, not a fresh `read_file`. Fix: call `read_file` on the exact line range immediately before constructing `oldString`. Never skip this.
2. **Auto-formatter ran between edits** — Prettier, ESLint, or another format-on-save tool reformatted the file after a prior edit. The reformatted content no longer matches the `oldString` you built. Fix: after a formatter-caused mismatch, `read_file` the current file state and rebuild `oldString` from the reformatted content.
3. **Sequential batch edits shifted content** — An earlier edit in `multi_replace_string_in_file` removed or added lines, invalidating a later `oldString` that was built from the pre-edit state. Fix: order edits bottom-to-top in the file (later line numbers first) so earlier edits don't shift the target of later ones.
4. **Large patch, context window staleness** — Building a 1000+ line patch causes the model to lose precision on exact whitespace and punctuation. `oldString` drifts from the actual file. Fix: split large file rewrites into ≤200-line sections. Read each section fresh before editing it.

**Recovery protocol when a batch fails mid-way:**

1. `read_file` the entire affected file to get the current state (some edits may have succeeded).
2. Identify which edits landed and which did not.
3. Re-read the specific sections that failed.
4. Reconstruct only the failed `oldString` values from the fresh read.
5. Retry only the failed edits (do not re-apply succeeded ones).

## File Creation

- **`create_file` fails on existing files.** Use `replace_string_in_file` for edits. The tool auto-creates parent directories.

## Paths

- **Always use absolute paths** in tool calls. Relative paths fail silently.

## Compiler & Diagnostics — Your Most Reliable Tool

The compiler, type checker, linter, and VS Code's Problems panel (`read/problems` / `get_errors`) are **the ground truth for correctness**. They do not guess. They do not hallucinate. Use them.

**Before and after every code change:**

1. Call `get_errors` on the edited file(s) immediately after editing. Do not wait until the end.
2. If errors are present, fix them before moving on. Do not accumulate errors across multiple edits.
3. After fixing, call `get_errors` again to confirm clean. Repeat until the errors list is empty or a concrete external blocker is reported.

**The hierarchy for resolving errors:**

1. **Compiler / type checker says X is wrong** → it is wrong. Fix it.
2. **Linter / static analysis says X is a problem** → treat it as a real problem unless you have a specific, documented reason to suppress it.
3. **You think X might be wrong** → run the tool. Find out. Don't reason without evidence.

**Never:**

- Skip the diagnostics check to save time. It costs far less time to check now than to debug a broken build later.
- Argue with a compiler error by reasoning from first principles. The compiler parsed the actual source tree. You did not.
- Assume a file is correct because it "looks right." Read the errors.

**The compiler is almost always correct.** The rare case where it appears to be wrong is almost always one of:

- A stale build cache — clean and rebuild.
- A missing import or uninstalled dependency — install it.
- An outdated language server — reload the VS Code window.

If you genuinely believe the compiler is wrong, say so explicitly and explain why, then verify with a second source before overriding it.

## Format Control — Bypass Formatters on Agent Save

When `gitShellHelpers.formatControl.bypassOnAgentSave` is enabled, `editor.formatOnSave` and `editor.codeActionsOnSave` are suppressed. This prevents Prettier, ESLint auto-fix, and other formatters from running on every agent-triggered file save.

**Rules:**

- **Subagents MUST NEVER run formatting.** No subagent may call `gitShellHelpers.formatOpenFiles` or `editor.action.formatDocument`. Formatting mid-pipeline corrupts file content that other subagents are reading.
- **Only the top-level orchestrating agent** may run formatting, and only **once at the very end** of the entire request — after all edits, all subagent work, and all validation are complete.
- To format at the end: call `vscode.commands.executeCommand('gitShellHelpers.formatOpenFiles')` via `run_vscode_command`, or run the command palette entry "GitHub Shell Helpers: Format All Open Files".
- When this setting is off (default), normal `editor.formatOnSave` behavior applies and no special handling is needed.
