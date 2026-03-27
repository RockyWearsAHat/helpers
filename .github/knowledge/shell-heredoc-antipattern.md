# Heredocs and Unsafe Shell Patterns in Agent Workflows

## Summary

Heredocs (`<<EOF`), long inline interpreter scripts (`node -e`, `python -c`), and shell redirection file writes (`> file`, `>> file`, `| tee file`) are **anti-patterns** in AI agent terminal workflows. They cause parsing failures, trigger automatic retry loops, and bypass safety guardrails.

## Why Heredocs Fail in Agent Workflows

1. **Shell parsing mismatches**: VS Code uses bash grammar for terminal parsing regardless of the actual shell. Complex constructs like heredocs are fragile in this context.
2. **Auto-retry amplification**: Under higher autonomy modes (e.g., Autopilot), failed heredocs trigger automatic retry loops — the agent keeps trying variants without stopping for user input. A single failure can burn 5–10 retries.
3. **Minimal write detection**: VS Code's detection of file writes via shell is minimal. Heredoc-based file creation can slip past intended guardrails.
4. **Quoting hazards**: Nested quoting, variable expansion conflicts, and delimiter mismatches make heredocs brittle across shells.
5. **Not reviewable**: Inline scripts embedded in terminal commands cannot be easily reviewed, tested, or version-controlled.

## Safe Alternatives

| Unsafe Pattern | Safe Alternative |
|---|---|
| `cat <<EOF > file.txt` | Use `create_file` / editor tools |
| `node -e '...'` (multi-line) | Create `scripts/task.mjs`, run `node scripts/task.mjs` |
| `python -c '...'` (multi-line) | Create `scripts/task.py`, run `python scripts/task.py` |
| `bash -c '...'` (multi-line) | Create `scripts/task.sh`, run `bash scripts/task.sh` |
| `echo "..." > file` | Use `printf '%s\n'` for short output, or editor tools for files |
| `| tee output.log` | Capture output in code, or use editor tools |

## In Existing Codebases

When heredocs exist in committed scripts (not agent terminal commands), refactor them to:

1. **Template files** in a `templates/` directory, loaded via `cat` or variable assignment
2. **`printf '%s\n'`** for short multi-line strings
3. **Dedicated script files** for any inline interpreter code

## Enforcement Layers

1. **Soft (instructions)**: Repository `.instructions.md` files that prohibit heredocs
2. **Hard (hooks)**: `PreToolUse` agent hooks that deny heredoc-containing terminal commands
3. **CI (scan)**: Automated repo scans that flag new heredoc introductions

## Key Insight

The cost of heredoc failures is **multiplicative, not linear**. Each failed attempt consumes agent context budget and can trigger multiple automatic retries. Prevention at the instruction layer is cheap; cleanup after a retry spiral is expensive.