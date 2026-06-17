# Git Shell Helpers

Quality-of-life git subcommands, an MCP tool server, an AI-agent control CLI (`gsh`),
a Copilot audit tool, and a VS Code extension that makes AI agents work transparently on
feature branches — without switching tabs or changing directories.

GSH is **agent-agnostic**: its tools ship as a standard stdio MCP server, so Claude Code,
GitHub Copilot, and any MCP-capable agent can use them. See [`AGENTS.md`](AGENTS.md) for
the agent-facing quickstart.

- [Use with any AI agent (gsh CLI)](#use-with-any-ai-agent-gsh-cli)
- [CS2420 / CS3500 A+ grading](#cs2420--cs3500-a-grading)
- [Branch Sessions (VS Code extension)](#branch-sessions-vs-code-extension)
- [Git subcommands](#git-subcommands)
- [MCP servers](#mcp-servers)
- [Research Search (GitHub Pages)](#research-search-github-pages)
- [Copilot DevOps Audit](#copilot-devops-audit)
- [Installation](#installation)
- [Development & Contributing](#development--contributing)

---

## Use with any AI agent (gsh CLI)

`gsh` is the single control surface for GSH. It installs GSH into your AI agent(s),
toggles the whole tool surface or individual tools live, and reports health.

```sh
gsh install              # auto-detect agents (Claude Code, Copilot) and wire GSH in
gsh install --agent claude   # Claude Code only ( --agent copilot | all )
gsh status               # what's installed, master switch, tool counts, agents
gsh doctor               # health checks
```

### Toggling (live — no agent restart)

```sh
gsh disable | enable     # master kill-switch for the entire GSH tool surface
gsh bypass               # toggle the master switch
gsh tool list            # every tool + on/off state
gsh tool disable <name>  # turn one tool off
gsh tool enable all      # turn everything back on
```

Tool state lives in `~/.config/git-shell-helpers-mcp/tools.json` and is re-read by the
running MCP server on every request, so toggles take effect immediately. A disabled tool
can be overridden for a single call with `{ "force": true }`.

### Fast startup (C launcher + auto-managed background server)

`gsh install` registers a small **C launcher** (`gsh-mcp`, compiled on install) instead of
launching Node directly. It starts in ~1ms and connects to a background server
(`git-shell-helpers-mcpd.js`) that loads the tool modules once and stays resident, so
sessions start fast (~tens of ms) instead of paying cold Node startup every time.

It just works — nothing to manage:

- the background server **starts automatically** on first use and **exits after ~15 min
  idle**;
- it's per-workspace (keyed by cwd + GSH env) so scope is preserved;
- it re-reads tool state per request, so `gsh` toggles stay live;
- if it isn't instantly ready (or there's no C compiler), the launcher **falls back to
  running Node directly within ~2s** — so startup is always reliable, never worse than
  plain Node.

```sh
gsh build            # (re)compile the launcher (optional; auto-run by install)
gsh daemon status    # is the background server running?
gsh daemon restart   # restart it (use after changing GSH code)
```

### What `gsh install --agent claude` does

- Registers the `gsh` MCP server with Claude Code (`claude mcp add -s user`).
- Writes the GSH core behavior into `~/.claude/CLAUDE.md` as a managed block (no clobber).
- Installs the `gsh` and `cs-grade` **skills**, the `/gsh` **slash command**, and the
  `cs-grade-improver` **subagent** into `~/.claude/`.

Run `/mcp` (or restart Claude Code) afterward so the server connects. The same artifacts
live under [`claude-config/`](claude-config) and the Copilot equivalents under
[`copilot-config/`](copilot-config).

---

## CS2420 / CS3500 A+ grading

`git-cs-grade` (also `gsh grade`) scores a Java course project against an objective
structural rubric and writes `GRADE.md`: a numeric+letter grade, a per-category scorecard
with the evidence behind each score, and a prioritized **Path to A+** checklist.

```sh
gsh grade .                      # auto-detect course
gsh grade ./hw3 --course cs3500  # object-oriented design rubric
gsh grade ./lab --course cs2420  # data-structures/algorithms rubric
git-cs-grade . --json            # machine-readable, for an automated loop
```

The intended loop: grade → fix the highest-impact checklist items → re-grade, until A+.
Claude Code users can hand the whole job to the **`cs-grade-improver`** subagent or the
**`cs-grade`** skill, which implement that loop (MVC separation, programming to interfaces,
design patterns, JUnit coverage, Javadoc, and cleanliness).

> The rubric grades what you can restructure (design, tests, docs, style); it does **not**
> run the course autograder's correctness suite, so pair it with the official tests.

---

## Branch Sessions (VS Code extension)

The VS Code extension enables **per-chat branch isolation**: each Copilot chat can work on a different feature branch, and switching between chats changes which branch is visible in the workspace.

### How it works

1. An agent calls `branch_session_start({ branch: "feature/my-work" })`
2. The extension creates a git worktree in `~/.cache/gsh/worktrees/` and checks out the branch **in your main repo view** via `git symbolic-ref`
3. You see the feature branch in VS Code's status bar, SCM panel, and Explorer — it looks like a normal checkout
4. When you switch to a different Copilot chat, the extension switches the visible branch to match that chat's session
5. When no bound chat is active, the main repo returns to its baseline branch and the feature session stays parked in its worktree
6. When the agent calls `branch_session_end`, the extension restores your original branch and pops any stashed work

Multiple chats can keep different branches parked in parallel. If a branch seems to disappear from the workspace, it is usually parked in another chat rather than lost; switch back to that chat or run `branch_status`.

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

Optional patches improve the branch session experience by modifying VS Code's compiled bundles:

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
- Branch sessions are chat-bound. If the current chat does not own a session, the workspace can return to baseline even though other sessions still exist. Use `branch_status` or the Branch Files view to find parked sessions.

---

## Git subcommands

The standalone `git-*` CLIs are native Rust (the `gitcli` module of the
`gsh-native` crate), built and symlinked by `gsh build`. They are deterministic;
`git upload` is the only one that touches AI, and only as an opt-in.

| Command                    | What it does                                                                    |
| -------------------------- | ------------------------------------------------------------------------------- |
| `git upload`               | Stage, commit, and push with safe recovery; deterministic message by default, optional `-ai` via Claude/Copilot |
| `git get`                  | Initialize a local repo from a remote (lightweight clone flow)                  |
| `git initialize`           | Initialize the directory as a repo, create initial commit, set `origin`, push   |
| `git checkpoint`           | Commit current state with a deterministic message (used by `gsh` MCP tools)     |
| `git fucked-the-push`      | Destructive recovery: undo the last pushed commit while keeping changes staged  |
| `git resolve`              | Safe merge/rebase conflict resolution with automatic backup branches            |
| `git remerge`              | Merge a detached-work branch back into a target; aborts cleanly on conflicts    |
| `git scan-for-leaked-envs` | Scan for leaked secrets, API keys, and env vars with deterministic patterns     |
| `git help-i-pushed-an-env` | Emergency: scrub secrets from git history, including batch ops across all repos |

Plus `gsh setup` — a deterministic project build-out plan (see **gsh setup** below).

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

Every GSH tool below is implemented in native Rust (the `gsh-native` binary),
except web search/scrape which stay in Node (they drive a headless browser).
All tools are deterministic — no tool calls an AI model.

Core workflow and quality:

- `workspace_context` - summarize active workspace roots and branch state.
- `strict_lint` - run each language's own linters on a file/folder/workspace.
- `checkpoint` - stage/commit (deterministic message from the diff, or your own); optionally push.

Project index (cheap repo map — orient without grepping):

- `index_project` - build/refresh a static map of files, symbols, and the reference graph (ranked), written to `.gsh/index/`.
- `project_map` - return a compact, token-cheap overview of the top modules plus a Mermaid graph; orient in one call.
- `lookup` - find where a symbol is defined and what references it, from the index graph instead of a grep sweep.
- `project_setup` - analyze the repo deterministically and return a concise build-out plan (purpose, stack + build/test/lint commands, gap checklist, and questions to ask the user). Drives a project to a complete, well-structured state fast; writes `.gsh/SETUP.md`. Also available as `gsh setup`.

Project flows (agent-agnostic reusable tools, scoped to the project):

- `register_workspace_tool` - register a named shell command/flow as a callable MCP tool (stored in `.gsh/tools/manifest.json`); turns a repetitive multi-step task into one tool call. Live immediately — no restart.
- `unregister_workspace_tool` - remove a registered flow.
- `list_workspace_tools` - list the flows registered for this project.

Knowledge:

- `search_knowledge_cache`
- `search_knowledge_index`
- `build_knowledge_index`
- `read_knowledge_note`
- `write_knowledge_note`
- `update_knowledge_note`
- `append_to_knowledge_note`
- `submit_community_research`

Web research (Node — headless browser):

- `search_web`
- `scrape_webpage`

Context-efficient usage order (minimal context, maximal output):

1. `workspace_context` once per task, then `project_map` (and `index_project` to refresh) to orient cheaply instead of reading/grepping many files.
2. `lookup <symbol|file>` to jump straight to definitions and references.
3. `list_workspace_tools` to reuse an existing project flow before re-implementing a task.
4. Call one specialized tool for the user goal (for example `search_web` or `strict_lint`).
5. Use `scrape_webpage` only for top hits that need deeper evidence.
6. End with `checkpoint` only after validation passes.

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

## Research Search (GitHub Pages)

The public note-search site is published at [rockywearsahat.github.io/github-shell-helpers](https://rockywearsahat.github.io/github-shell-helpers/).

It indexes three layers:

- generalized Copilot guidance from the community-cache snapshot plus the Copilot research studybase
- the broad CS/coding corpus under `knowledge/`
- the archived raw source material under `research-sources/legacy-root-dumps/`

The site serves a ranked client-side search UI with in-browser previews, source labels, evidence links, and direct links back to GitHub.

### Local build

```sh
node ./scripts/build-pages-search-site.js
python3 -m http.server -d build/pages-search 4173
```

The GitHub Pages workflow lives at `.github/workflows/pages-search.yml`. Pull requests validate the site build without deploying; pushes to `main` publish the contents of `build/pages-search/` to GitHub Pages.

### Live-site browser flow capture

To capture the published GitHub Pages experience for visual QA, run:

```sh
node ./scripts/capture-live-site-browser-flows.js
```

The script targets `https://rockywearsahat.github.io/github-shell-helpers/` by default, captures the `browse`, `search`, `about`, and `reader` flows as compressed JPEG scroll shots, and writes the results to `build/visual-captures/` with a per-run `manifest.json`. The output directory is gitignored through `build/`.

Useful overrides:

```sh
node ./scripts/capture-live-site-browser-flows.js --flows browse,search
node ./scripts/capture-live-site-browser-flows.js --base-url http://127.0.0.1:4173/
node ./scripts/capture-live-site-browser-flows.js --output-dir /tmp/atlas-live-site-check
```

---

## gsh setup (project build-out)

`gsh setup` replaces the old Copilot DevOps audit with a **deterministic** project
build-out engine — no AI, no agent install. It analyzes the repository and prints
(and writes to `.gsh/SETUP.md`) a concise, structured plan that drives any project
to a complete, well-structured state quickly, while enforcing three rules:

1. **Minimal context** — the plan is a tight, ranked summary, never a file dump.
2. **Understand goals first** — purpose/goals are surfaced (or flagged as unknown)
   before any build-out steps are proposed.
3. **Clarify with the user** — ambiguities become explicit questions to ask first.

```sh
gsh setup            # analyze, print the plan, write .gsh/SETUP.md
gsh setup --no-write # print only
```

The plan contains: detected purpose, the technology stack with its build/test/lint
commands, the project shape (languages, top-level dirs, entry points), a prioritized
gap checklist (missing tests, CI, license, lint config, …), and clarifying questions.
The same engine is exposed to agents as the `project_setup` MCP tool, so an agent can
orient and build out in one call.

---

## Installation

### macOS .pkg (recommended)

Download the latest `.pkg` from the [releases page](https://github.com/RockyWearsAHat/github-shell-helpers/releases/latest) and run the installer. It places binaries in `/usr/local/bin` and man pages in `/usr/local/share/man/man1` without touching shell config files.

The postinstall script also attempts to install the VS Code extensions for the logged-in user.

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
  | bash
```

Then open a new terminal so your shell profile reloads the installed PATH and MANPATH snippet.

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

The workflow also uses repository variables to explicitly enable or disable the optional channels:

- `RELEASE_ENVIRONMENT`
- `ENABLE_MACOS_SIGNING`
- `ENABLE_NPM_PUBLISH`
- `ENABLE_HOMEBREW_PUBLISH`
- `ENABLE_AUR_PUBLISH`

Seed the safe defaults into the repo now:

```sh
bash ./scripts/setup-github-release-config.sh --defaults-only
```

For tighter scoping, store the release-only values in a GitHub Environment named `release` so only jobs that declare `environment: release` can access them:

```sh
bash ./scripts/setup-github-release-config.sh --env release --defaults-only
```

To install the real credentials later, copy `.github/release-config.example.env` to a private local file, fill in the values you actually have, export it, then run:

```sh
set -a
source /path/to/your/release-config.env
set +a
bash ./scripts/setup-github-release-config.sh --env release
```

This is the closest GitHub Actions gets to “owner/internal only” for workflow credentials: the values live on GitHub, are not committed to the repo, are only exposed to GitHub-hosted jobs, and can be restricted to the `release` environment instead of the whole repository.

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
| `git-shell-helpers-mcp`                    | MCP server — project index, checkpoint, strict_lint, knowledge, project flows, web research |
| `git-upload`                               | Stage/commit/push with AI messages and test detection      |
| `git-help-i-pushed-an-env`                 | Secret scrubbing from git history                          |
| `scripts/patch-vscode-apply-all.js`        | Coordinator for VS Code bundle patches                     |
