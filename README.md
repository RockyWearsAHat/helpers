# Git Shell Helpers

Quality-of-life git subcommands, an MCP research server, a Copilot audit tool, and a VS Code extension that makes AI agents work transparently on feature branches — without switching tabs or changing directories.

- [Branch Sessions (VS Code extension)](#branch-sessions-vs-code-extension)
- [Git subcommands](#git-subcommands)
- [MCP servers](#mcp-servers)
- [Copilot DevOps Audit](#copilot-devops-audit)
- [Installation](#installation)
- [Development & Contributing](#development--contributing)

---

## Branch Sessions (VS Code extension)

The VS Code extension enables **per-chat branch isolation**: each Copilot chat can work on a different feature branch, and switching between chats automatically switches the visible branch in VS Code's Explorer, Source Control panel, and git status — all without leaving VS Code or running `git checkout`.

### How it works

1. An agent calls `branch_session_start({ branch: "feature/my-work" })`
2. The extension creates a git worktree in `~/.cache/gsh/worktrees/` and checks out the branch **in your main repo view** via `git symbolic-ref`
3. You see the feature branch in VS Code's status bar, SCM panel, and Explorer — it looks like a normal checkout
4. When you switch to a different Copilot chat, the extension transparently switches the visible branch to match that chat's session
5. When the agent calls `branch_session_end`, the extension restores your original branch and pops any stashed work

Multiple chats work on different branches in parallel. The worktree mechanics are invisible to the user.

### Enabling branch sessions

Branch sessions are off by default. Enable them in VS Code settings:

```
Settings → Git Shell Helpers → Branch Sessions → Enabled
```

Then reload the window. The `gsh` MCP server exposes `branch_session_start`, `branch_session_end`, `branch_read_file`, and `branch_status` once the setting is on.

### VS Code extension installation

The extension is bundled as a `.vsix`. Build it locally:

```sh
./scripts/build-vsix.sh
```

Then install via **Extensions → Install from VSIX…** in VS Code, or:

```sh
code --install-extension vscode-extension/git-shell-helpers-*.vsix
```

### VS Code patches (optional)

Two optional patches improve the branch session experience by modifying VS Code's compiled bundles:

| Patch              | Effect                                                                        | Requires      |
| ------------------ | ----------------------------------------------------------------------------- | ------------- |
| `folder-switch`    | Removes the "do you want to switch folders?" dialog when worktrees change     | Full restart  |
| `git-head-display` | Shows the worktree branch name in the status bar via `.git/gsh-head-override` | Window reload |

Apply both patches:

```sh
node scripts/patch-vscode-apply-all.js
```

Check status, revert, or apply individually:

```sh
node scripts/patch-vscode-apply-all.js --check
node scripts/patch-vscode-apply-all.js --revert
node scripts/patch-vscode-folder-switch.js
node scripts/patch-vscode-git-head-display.js
```

The patch scripts detect VS Code's install location automatically on macOS, Linux (including Snap), and Windows.

#### Upstream PR status

These patches address real VS Code gaps. Corresponding PRs are open against `microsoft/vscode`:

| Proposal                                               | PR                                                         | Status                                                   |
| ------------------------------------------------------ | ---------------------------------------------------------- | -------------------------------------------------------- |
| Suppress folder switch dialog (`suppressConfirmation`) | [#306519](https://github.com/microsoft/vscode/pull/306519) | Open — awaiting review; needs community upvotes          |
| Branch name display override (`headLabelOverride`)     | [#306517](https://github.com/microsoft/vscode/pull/306517) | Open — assigned [@lszomoru](https://github.com/lszomoru) |
| Chat session focus stability                           | [#306518](https://github.com/microsoft/vscode/pull/306518) | Open — assigned [@jrieken](https://github.com/jrieken)   |

If any of these PRs land in VS Code, the corresponding local patch becomes unnecessary. The extension will detect the native API and skip the patched code path automatically when the real API becomes available.

### Known limitations

- The `which code` / `where code` dynamic path detection requires `code` to be in your PATH. If it isn't, add it via **Shell Command: Install 'code' command in PATH** in the VS Code command palette.
- Patches are applied to VS Code's compiled bundles. They may need to be re-applied after VS Code auto-updates. Run `node scripts/patch-vscode-apply-all.js --check` to verify status after an update.
- Stash recovery after a VS Code reload is best-effort. If you manually run `git stash` between a `branch_session_start` and `branch_session_end`, the extension uses the stash message `"gsh-session-focus: auto-stash"` to find and restore the correct stash entry.

---

## Git subcommands

| Command                    | What it does                                                                    |
| -------------------------- | ------------------------------------------------------------------------------- |
| `git upload`               | Stage, commit, and push with optional AI-generated commit messages              |
| `git get`                  | Initialize a local repo from a remote (lightweight clone flow)                  |
| `git initialize`           | Initialize the directory as a repo, create initial commit, set `origin`, push   |
| `git checkpoint`           | Commit current state with an AI-generated message (used by `gsh` MCP tools)     |
| `git fucked-the-push`      | Destructive recovery: undo the last pushed commit while keeping changes staged  |
| `git resolve`              | Safe merge/rebase conflict resolution with automatic backup branches            |
| `git remerge`              | Merge a detached-work branch back into a target; aborts cleanly on conflicts    |
| `git copilot-quickstart`   | Scaffold a `.github/` Copilot self-iteration workflow for any repository        |
| `git scan-for-leaked-envs` | Scan for leaked secrets, API keys, and environment variables using Copilot      |
| `git help-i-pushed-an-env` | Emergency: scrub secrets from git history, including batch ops across all repos |
| `git copilot-devops-audit` | Run the Copilot customization audit workflow (see below)                        |

Man pages are installed for all commands. Use `git help <subcommand>` or `man git-<subcommand>` after installation.

---

## MCP servers

### git-shell-helpers-mcp (combined server — recommended)

`git-shell-helpers-mcp` exposes all tooling under one MCP server entry. When the VS Code extension is installed, it publishes this server globally so `checkpoint`, `branch_session_start`, and other tools are available in every workspace without editing `mcp.json` manually.

Manual registration if needed:

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

#### Exposed tools

| Tool                       | Description                                                    |
| -------------------------- | -------------------------------------------------------------- |
| `checkpoint`               | Stage and commit with an AI-generated message; optionally push |
| `branch_session_start`     | Start an isolated branch session for the current chat          |
| `branch_session_end`       | End the session; restore original branch and stash             |
| `branch_read_file`         | Read a file from a branch session worktree                     |
| `branch_status`            | Get current session branch and status                          |
| `search_web`               | Web search via local SearXNG                                   |
| `scrape_webpage`           | Fetch and clean up to 5 URLs                                   |
| `search_knowledge_cache`   | Search `knowledge/` Markdown notes by keyword                  |
| `read_knowledge_note`      | Read a specific knowledge note                                 |
| `write_knowledge_note`     | Create or overwrite a knowledge note                           |
| `update_knowledge_note`    | Replace a section by heading                                   |
| `append_to_knowledge_note` | Append content to an existing note                             |
| `analyze_images`           | Describe or compare images via vision model                    |

Environment variables to selectively disable groups:

```
GIT_SHELL_HELPERS_MCP_DISABLE_RESEARCH=1
GIT_SHELL_HELPERS_MCP_DISABLE_VISION=1
```

### git-research-mcp (standalone research server)

`git-research-mcp` is a standalone MCP server for web search and knowledge-cache tools. It requires a running SearXNG Docker container:

```sh
docker run -d --name searxng -p 8888:8080 searxng/searxng:latest
```

Configuration is in `~/.config/git-research-mcp/.env`:

```
SEARXNG_URL=http://localhost:8888
```

---

## Copilot DevOps Audit

`git-copilot-devops-audit` audits GitHub Copilot customization in any workspace — keeping `.github/` setup current, project-specific, and aligned with what VS Code and Copilot actually support.

### Install the audit surfaces

```sh
git copilot-devops-audit --update-agent --force
```

This installs:

- Audit agents under `~/.copilot/agents/`
- Natural-language router under `~/.copilot/instructions/`
- Audit skills under `~/.copilot/skills/`
- The `/copilot-devops-audit` slash command prompt in VS Code's user prompts folder

### Running an audit

Use the `/copilot-devops-audit` slash command in VS Code Copilot Chat for the deterministic entry point. Natural-language routing ("audit my Copilot setup") works through the installed router instruction but is best-effort.

Each full audit runs four phases: Context → Research → Evaluation → Implementation.

---

## Installation

### macOS .pkg (recommended)

Download the latest `.pkg` from the [releases page](https://github.com/RockyWearsAHat/github-shell-helpers/releases/latest) and run the installer. It places binaries in `/usr/local/bin` and man pages in `/usr/local/share/man/man1` without touching shell config files.

The postinstall script also attempts to install the VS Code extensions and run `git copilot-devops-audit --update-agent --force` for the logged-in user. If it can't (no VS Code CLI in PATH), run that command manually.

### Homebrew

If the optional tap-publish workflow is configured, install from the tap:

```sh
brew tap RockyWearsAHat/gsh
brew install github-shell-helpers
```

If the tap is not configured yet, the release still publishes `github-shell-helpers.rb` so you can install from the formula file directly.

### Debian / Ubuntu

Each GitHub release publishes a `.deb` package:

```sh
sudo apt install ./github-shell-helpers_<version>_all.deb
```

The workflow currently publishes the `.deb` as a release asset. A dedicated apt repository is not automated yet.

### Arch Linux (AUR)

If the optional AUR publish step is configured, install with your preferred helper:

```sh
yay -S github-shell-helpers
```

If AUR publishing is not configured yet, the release still includes `PKGBUILD` and `.SRCINFO` assets.

### npm

If npm publishing is configured in GitHub Actions:

```sh
npm install -g github-shell-helpers
```

The release also includes the generated `.tgz` package for manual installation.

### Portable tarball

Each release includes a portable archive containing the same command/support-file tree used by the package-manager builds:

```sh
tar -xzf github-shell-helpers-<version>.tar.gz
```

### Script installer (cross-platform)

```sh
curl -fsSL \
  https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Git-Shell-Helpers-Installer.sh \
  | zsh
```

Then `source ~/.zshrc` or open a new terminal.

---

## Development & Contributing

### Build & test

```sh
bash ./scripts/test.sh                  # full test suite
bash ./scripts/test-git-upload-states.sh  # state recovery tests
./scripts/build-dist.sh                 # script installer + portable tarball
./scripts/build-deb.sh                  # Debian package
./scripts/build-homebrew-formula.sh     # Homebrew formula
./scripts/build-aur-package.sh          # AUR metadata
./scripts/build-npm-package.sh          # npm package tarball
./scripts/build-pkg.sh                  # macOS pkg
./scripts/build-vsix.sh                 # VS Code extension .vsix
```

### Versioning

Update `VERSION` (single-line semver) and add `release-notes/v<version>.md` before cutting a release. CI builds the shell installer, portable tarball, `.deb`, Homebrew formula, AUR metadata, npm package tarball, macOS `.pkg`, and VSIX, then publishes them with the release notes file as the release body.

### Release configuration

macOS signing and notarization are configured with GitHub Actions secrets:

- `INSTALLER_CERT_BASE64`
- `INSTALLER_CERT_PASSWORD`
- `PKG_SIGN_IDENTITY`
- `NOTARIZE_APPLE_ID`
- `NOTARIZE_PASSWORD`
- `NOTARIZE_TEAM_ID`

Optional publish channels are configured with these secrets and repository variables:

- npm publish: secret `NPM_TOKEN`
- Homebrew tap publish: secret `HOMEBREW_TAP_TOKEN`, variable `HOMEBREW_TAP_REPOSITORY`
- AUR publish: secret `AUR_SSH_PRIVATE_KEY`, variable `AUR_PACKAGE_NAME` (defaults to `github-shell-helpers`)

Without those optional publish credentials, the workflow still uploads the generated formula, AUR metadata, `.deb`, and npm tarball as GitHub release assets.

### Pull requests

Before opening a PR:

- Run the full test suite and confirm it passes
- Run `node scripts/patch-vscode-apply-all.js --check` if you touched patch scripts
- Describe what changed, what was tested, and any breaking changes
- Reference related issues or upstream PRs where relevant

CI runs the test suite automatically on every PR. Copilot will offer an automated review pass on each push — treat its findings as a first-pass linting signal, not a final verdict.

### Architecture notes

The codebase has several large files. Before editing any file over 500 lines, read the function index (`grep -n 'function ' <file>`) and understand the call chain you're modifying. See `.github/instructions/modular-architecture.instructions.md` for decomposition guidance.

Key files:

| File                                       | Domain                                                     |
| ------------------------------------------ | ---------------------------------------------------------- |
| `vscode-extension/extension.js`            | Extension entry point, command registration                |
| `vscode-extension/src/worktree-manager.js` | Branch session focus/unfocus, binding, stash, git ops      |
| `vscode-extension/src/ipc-servers.js`      | Unix socket IPC between MCP server and extension           |
| `git-shell-helpers-mcp`                    | MCP server — branch sessions, checkpoint, research, vision |
| `git-upload`                               | Stage/commit/push with AI messages and test detection      |
| `git-help-i-pushed-an-env`                 | Secret scrubbing from git history                          |
| `scripts/patch-vscode-apply-all.js`        | Coordinator for VS Code bundle patches                     |
