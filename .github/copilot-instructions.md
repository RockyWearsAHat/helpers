# Repository Baseline

- This repository ships shell-based Git helper commands, installer scripts, packaging assets, and man pages. Most code changes are in shell scripts and release packaging files.
- Use `./scripts/test.sh` for repo sanity checks. When installer behavior changes, build the script installer with `./scripts/build-dist.sh` and the macOS package with `./scripts/build-pkg.sh`.
- Keep user-facing command behavior, installer scripts, and matching man pages aligned when changing flags, command names, or help text.
- Prefer small, repo-specific Copilot guidance. Add prompts, agents, skills, or hooks only when a repeated workflow clearly benefits from them.
- When editing `.github/` customization files, keep repo-wide instructions factual, keep prompts as manual entrypoints, keep agents focused on role and tool boundaries, and keep step-by-step methodology in skills.
