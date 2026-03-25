# NEW COPILOT FEATURE, AUDIT ADDED

`git-copilot-devops-audit` is a global helper for auditing GitHub Copilot customization in the current workspace. The goal is narrow and practical: keep `.github/` Copilot setup current, project-specific, and honest about what is actually installed and supported.

After the global audit surfaces are installed in VS Code:

- `/copilot-devops-audit` is the deterministic manual entrypoint.
- Natural-language audit routing is available through a user-level router instruction and should be treated as best-effort, not guaranteed.
- The audit can run as a full edit pass or as a report-only pass.
- A normal repository run seeds `.github/devops-audit-context.md` and `.github/devops-audit-research.md`; findings themselves are returned in chat.

Install or refresh the global audit surfaces from a cloned copy of this repo with:

```sh
./git-copilot-devops-audit --update-agent --force
```

If the helpers are already installed (i.e. `git-copilot-devops-audit` is in your PATH), `git copilot-devops-audit --update-agent --force` also works.

That command installs or refreshes:

- private audit agents under `~/.copilot/agents/`
- the natural-language router under `~/.copilot/instructions/`
- audit skills under `~/.copilot/skills/`
- the `/copilot-devops-audit` prompt in `~/Library/Application Support/Code/User/prompts/`

For VS Code use, you also need the GitHub Copilot and GitHub Copilot Chat extensions enabled. The script installer can optionally install or update those extensions when a usable VS Code CLI is available. Natural-language routing depends on current Copilot routing behavior, so keep `/copilot-devops-audit` as the fallback when you want deterministic invocation.

Each full audit uses a four-phase flow:

1. Context
2. Research
3. Evaluation
4. Implementation

# Git Shell Helpers

Small quality-of-life helpers wrapped as git subcommands:

- `git upload` – stage, commit, and push, with optional AI-generated commit messages using GitHub Copilot CLI.
- `git get` – initialize a local repo from a remote (like a lightweight `git clone` flow).
- `git initialize` – initialize the current directory as a repo, create an initial commit, set `origin`, and push.
- `git fucked-the-push` – destructive recovery helper to undo the last pushed commit while keeping changes staged.
- `git resolve` – safe helper for resolving merge/rebase conflicts with automatic backup branches.
- `git remerge` – merge a branch (typically a detached-work branch from `git upload`) back into a target branch; aborts cleanly on conflicts.
- `git copilot-quickstart` – scaffold a `.github/` Copilot self-iteration workflow for any repository, with Plan and Implement agents.
- `git scan-for-leaked-envs` – scan the current repository for leaked secrets, API keys, and environment variables using GitHub Copilot.
- `git help-i-pushed-an-env` – emergency tool to scrub secrets and sensitive files from git history, including batch operations across all GitHub repos.
- `git copilot-devops-audit` – install and run the Copilot customization audit workflow described above.
- `git-research-mcp` – MCP server providing web search (via local SearXNG) and per-project knowledge-cache tools for AI assistants.
- `git-shell-helpers-mcp` – combined MCP server that exposes all git-shell-helpers tools (research + vision) under one server entry.

## Installation options

### 1. macOS .pkg installer (recommended)

On macOS, the preferred way to install is via the native `.pkg` installer:

- Installs binaries into `/usr/local/bin` and man pages into `/usr/local/share/man/man1`.
- Does **not** modify your shell config files.

Grab the latest packaged macOS installer from the latest release:

- [github-shell-helpers latest release](https://github.com/RockyWearsAHat/github-shell-helpers/releases/latest)

After downloading `github-shell-helpers-<version>.pkg`:

1. Double-click the `.pkg`.
2. Follow the standard macOS Installer flow.

Once complete, the commands and man pages should be available immediately in any new shell:

- `git upload`
- `git get`
- `git initialize`
- `git fucked-the-push`
- `git resolve`
- `git remerge`
- `git copilot-quickstart`
- `git scan-for-leaked-envs`
- `git help-i-pushed-an-env`
- `git copilot-devops-audit`
- `git help upload|get|initialize|fucked-the-push|copilot-devops-audit`

The `.pkg` installer now also attempts to complete the VS Code global audit setup for the logged-in macOS user by:

- installing or updating the GitHub Copilot and GitHub Copilot Chat extensions when a usable VS Code CLI is available
- running `git copilot-devops-audit --update-agent --force` as the logged-in user to install the router, agents, skills, and prompt

If that automatic postinstall step cannot complete on your machine, run this manually:

```sh
git copilot-devops-audit --update-agent --force
```

### 2. One-line script installer (portable alternative)

If you prefer a script-based install into your home directory, use the installer script. This works well on macOS and other Unixy environments where you control your shell config.

Direct download of the installer script:

- [Git-Shell-Helpers-Installer.sh](https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh)

```sh
curl -fsSL \
  https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
  | zsh
```

After installation, either open a new terminal or run:

```sh
source ~/.zshrc
```

Then you can use the same commands and help pages as with the `.pkg` installer.
The script installer can also optionally:

- install or update GitHub Copilot CLI support for `git upload -ai`
- install or update the GitHub Copilot and GitHub Copilot Chat VS Code extensions when a usable VS Code CLI is available
- install the global audit router, agents, skills, and prompt for VS Code

## Why two installer methods?

- **Script installer (`Git-Shell-Helpers-Installer.sh`)**
  - Cross-shell friendly, very easy to share as a copy-pastable command.
  - Installs into your home directory and updates your shell config.
  - Offers optional Copilot CLI, VS Code extension, and global audit-surface setup.
- **macOS `.pkg` installer**
  - Integrates with the native macOS Installer UI.
  - Installs into system-level locations (`/usr/local/...`) without touching your `~/.zshrc`.
  - Attempts the same VS Code global audit setup automatically for the logged-in macOS user during postinstall.

Both install the same command-line tools and man pages; they just target different installation styles.

## git-research-mcp (MCP server)

`git-research-mcp` is a zero-dependency Node.js MCP server that provides web search and repo-local knowledge-cache tools over stdio. It requires Node.js >= 18 (uses built-in `fetch`).

### Prerequisites

A running SearXNG Docker container on port 8888:

```sh
docker run -d --name searxng -p 8888:8080 searxng/searxng:latest
```

No API keys or external credentials are required. SearXNG queries Google, Bing, DuckDuckGo, and other engines simultaneously.

### Configuration

The installer writes a default config to `~/.config/git-research-mcp/.env`:

```
SEARXNG_URL=http://localhost:8888
```

Edit this file to change the SearXNG URL. The `.env` is never written into the install directory.

### MCP registration

Add to your project's `.vscode/mcp.json` (or user-level mcp.json):

```json
{
  "servers": {
    "git-research-mcp": {
      "type": "stdio",
      "command": "node",
      "args": ["git-research-mcp"]
    }
  }
}
```

### Exposed MCP tools

| Tool                       | Description                                                |
| -------------------------- | ---------------------------------------------------------- |
| `search_web`               | Web search via local SearXNG (Google, Bing, DDG, etc.)     |
| `scrape_webpage`           | Fetch up to 5 URLs, strip HTML chrome, return cleaned text |
| `search_knowledge_cache`   | Search `.github/knowledge/` Markdown files by keyword      |
| `read_knowledge_note`      | Read a specific knowledge note                             |
| `write_knowledge_note`     | Create or overwrite a knowledge note                       |
| `update_knowledge_note`    | Replace a section by heading in a note                     |
| `append_to_knowledge_note` | Append content to an existing note                         |

## git-shell-helpers-mcp (combined MCP server)

`git-shell-helpers-mcp` is a combined Node.js MCP server that exposes all git-shell-helpers tools under a single server entry. It delegates to the individual modules (`git-research-mcp` for web search and knowledge-cache tools, `vision-tool` for image analysis) which remain separate files for maintainability.

When the Git Shell Helpers VS Code extension is installed, it publishes this MCP server globally across workspaces so tools such as `checkpoint` are discoverable without editing a user-profile `mcp.json`.

### MCP registration

```json
{
  "servers": {
    "gsh": {
      "type": "stdio",
      "command": "node",
      "args": ["git-shell-helpers-mcp"]
    }
  }
}
```

Environment variables for selective disabling:

- `GIT_SHELL_HELPERS_MCP_DISABLE_RESEARCH=1` – skip research tools
- `GIT_SHELL_HELPERS_MCP_DISABLE_VISION=1` – skip vision tools

## Development

- Update the version number in `VERSION` before cutting a new release.
- Add or update `release-notes/v<version>.md`; GitHub Releases now use that file as the release body.
- Build artifacts locally:
  - Script installer dist: `./scripts/build-dist.sh` → `dist/Git-Shell-Helpers-Installer.sh` and `dist/Git-Shell-Helpers-Installer-<version>.sh`
  - macOS pkg: `./scripts/build-pkg.sh` → `dist/github-shell-helpers-<version>.pkg`
- VS Code tasks:
  - **Build installer** – runs `./scripts/build-dist.sh`.
  - **Build macOS pkg** – runs `./scripts/build-pkg.sh`.

CI (see `.github/workflows/build-installer.yml`) ensures both installers build cleanly on each push to `main`, and publishes the versioned script installer plus the macOS package using the matching release-notes file.
