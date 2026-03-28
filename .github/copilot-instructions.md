---
description: "Git Shell Helpers workspace baseline: architecture, build/test, coding standards, and boundaries."
---

# Workspace Baseline

Git Shell Helpers (`github-shell-helpers`) v0.3.4 — shell-based git helpers, MCP servers, Copilot customization automation, a VS Code extension, and a community-backed research cache.

## Non-Negotiable Boundary

- `copilot-config/` is product source shipped to users.
- `.github/` is repository development config and supporting project assets.

When updating shipped Copilot customization behavior, edit `copilot-config/`. When updating repository workflows, docs, and governance assets, edit `.github/`.

## Build and Test

- **Test**: `bash ./scripts/test.sh` (from repo root)
- **State recovery test**: `bash ./scripts/test-git-upload-states.sh`
- **Build installer**: `./scripts/build-dist.sh`
- **Build macOS pkg**: `./scripts/build-pkg.sh`
- **Build VSIX**: `./scripts/build-vsix.sh`
- **Version**: stored in `VERSION` (single-line semver)

Always run tests after modifying shell commands or the VS Code extension.

## Architecture Map

### Monolithic files — handle with care

These files are large and tightly coupled. **Check actual size with `wc -l` before planning edits** — line counts change as modules are extracted. Do not make small patches without understanding surrounding context.

| File                            | Domain                                                                 |
| ------------------------------- | ---------------------------------------------------------------------- |
| `vscode-extension/extension.js` | VS Code extension (MCP client, commands, config)                       |
| `git-research-mcp`              | Node.js MCP server (web search, knowledge index, headless Chrome)      |
| `git-upload`                    | Bash: stage/commit/push with AI messages, test detection, risk scoring |
| `git-help-i-pushed-an-env`      | Bash: secret scrubbing from git history                                |
| `git-copilot-quickstart`        | Bash: scaffold Copilot workflows                                       |
| `git-copilot-devops-audit`      | Bash: install/run audit agents                                         |

**Before editing any file over 500 lines**: check `wc -l`, read at least the function index (`grep -n 'function \|^[a-z_]*()' <file>`), and understand the call chain you are modifying. Do not submit an 8-line patch to a large file without understanding the surrounding 200+ lines of context.

### Directory structure

- Shell commands: repo root (`git-upload`, `git-checkpoint`, etc.) — **no `.sh` extension**
- MCP servers: `git-research-mcp` (Node.js), `git-shell-helpers-mcp` (Node.js)
- VS Code extension: `vscode-extension/`
- Vision tools: `vision-tool/`, `aioserver-vision-tool/`
- Community cache: `community-cache/`
- Build/test scripts: `scripts/`
- Man pages: `man/man1/`
- Knowledge corpus: `knowledge/`
- Copilot product source (shipped to users): `copilot-config/`
- Repo dev config: `.github/`

## Coding Standards

- See `.github/instructions/shell-safety.instructions.md` for shell rules (heredoc prohibition, safe patterns).
- See `.github/instructions/modular-architecture.instructions.md` for file size limits and decomposition principles.
- See `.github/instructions/javascript.instructions.md` for Node.js/extension conventions.
- Keep command `--help` output, man page, and installer behavior aligned in the same change.

## Engineering Defaults

- Prefer MCP tools over shell emulation when tool coverage exists.
- Run strict diagnostics before and after edits.
- Keep command/man page/installer behavior aligned in the same change.
- Treat remote repository state as canonical for community-driven data; local cache copies are accelerators, not source of truth.

## Detailed Sources of Truth

- Copilot system assets: `copilot-config/agents/`, `copilot-config/skills/`, `copilot-config/instructions/`, `copilot-config/prompts/`
- Build/test/release workflows: `.github/workflows/`
- Knowledge corpus and philosophy: `knowledge/`, `knowledge/knowledge-philosophy.md`
- Community cache automation scripts: `scripts/community-cache-*.sh`, `scripts/community-research-submit.sh`
- Repo-only local reference cache: `.github/devops-audit-community-cache/`
