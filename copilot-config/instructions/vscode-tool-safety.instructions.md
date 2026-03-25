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
