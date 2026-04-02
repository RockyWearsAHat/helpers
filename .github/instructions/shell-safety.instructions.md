---
applyTo: "git-*,**/*.sh,scripts/**,install-git-shell-helpers"
description: "Shell safety rules for git-shell-helpers. Prohibits heredocs, enforces safe patterns, and requires modular thinking for large scripts."
---

# Shell Safety Rules

These rules apply to ALL shell scripts in this repository, including the extensionless `git-*` commands at the repo root.

## Heredoc Prohibition

**Never use heredocs** (`<<EOF`, `<<-EOF`, `<<'EOF'`, `cat <<`, etc.) in this codebase. This is a hard rule.

Why: Heredocs cause parsing failures in agent terminal flows, are fragile across shells, resist static analysis, and make scripts harder to review. Under higher autonomy modes (e.g. Autopilot), failed heredocs trigger **automatic retry loops** — the agent keeps trying heredoc variants without stopping for user input, compounding wasted time and context. A single heredoc failure can burn 5-10 retries before the agent gives up. See `.github/knowledge/shell-heredoc-antipattern.md` for the distilled rationale note.

Instead:

- Use `printf '%s\n'` for short multi-line output.
- Use template files in a `templates/` directory for large content blocks (especially for `git-copilot-quickstart` scaffold content).
- Use `create_file` / editor tools when generating files in agent workflows — never `cat > file <<EOF`.

**Legacy heredoc hotspots still exist.** Refresh the current list with:

- `rg -n "<<[-'[:alnum:]]|cat <<" git-* scripts/ Git-Shell-Helpers-Installer.sh install-git-shell-helpers`

## Unsafe Pattern Prevention

- **No file writes via redirection** (`> file`, `>> file`, `| tee file`) in agent-generated terminal commands. Use editor file tools instead.
- **No inline interpreter scripts** (`node -e`, `python -c`, `bash -c` with multi-line bodies). Create real script files.
- **No unquoted variable expansions**. Always use `"$var"` and `"${arr[@]}"`.
- **No `echo -e`** — use `printf` for portable formatted output.
- **No `sed -i`** without a backup extension or portability guard (macOS vs GNU).

## Shell Script Conventions

- Start every script with `#!/usr/bin/env bash` and `set -euo pipefail`.
- Include a header comment: command name, Usage, Description, Options, Examples.
- Name commands `git-<verb>` (no `.sh` extension) so Git discovers them as subcommands.
- Log with `[git-<command>]` prefix for output attribution.
- Parse arguments with `for arg in "$@"` / `case` statement; use flag variables.
- Define ANSI color constants at the top when using colored output.
- Use `exit 1` with stderr message for user errors; rely on `set -e` for unexpected failures.
- Prefer portable coreutils; when macOS-specific paths are needed, set `PATH` explicitly.
