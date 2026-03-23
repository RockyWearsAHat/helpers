---
applyTo: "**/*.sh"
description: "Shell scripting conventions for git-shell-helpers commands."
---

# Shell Script Conventions

- Start every script with a shebang (`#!/usr/bin/env bash` or `#!/bin/zsh`) and `set -euo pipefail`.
- Include a header comment block with: command name, Usage, Description, Options, and Examples sections.
- Name commands `git-<verb>` so Git discovers them as subcommands automatically.
- Log messages with a `[git-<command>]` prefix for consistent output attribution.
- Parse arguments with a `for arg in "$@"` loop and `case` statement; use flag variables for state.
- Quote all variable expansions (`"$var"`, `"${arr[@]}"`) to prevent word splitting and globbing.
- Define ANSI color constants (`RED`, `GREEN`, `NC`, etc.) at the top when using colored output.
- Keep each command's man page (`man/man1/git-<command>.1`) aligned with its `--help` output and header comment.
- Use `exit 1` with a stderr message for user-facing errors; rely on `set -e` for unexpected failures.
- Prefer portable coreutils; when macOS-specific paths are needed (e.g. Homebrew), set `PATH` explicitly.
