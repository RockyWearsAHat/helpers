# Helpers

Quality-of-life git subcommands, an MCP tool server, an AI-agent control CLI (`helpers`),
a Copilot audit tool, and a VS Code extension that makes AI agents work transparently on
feature branches — without switching tabs or changing directories.

Helpers is **agent-agnostic**: its tools ship as a standard stdio MCP server, so Claude Code,
GitHub Copilot, and any MCP-capable agent can use them. See [`AGENTS.md`](AGENTS.md) for
the agent-facing quickstart.

- [Use with any AI agent (helpers CLI)](#use-with-any-ai-agent-helpers-cli)
- [CS2420 / CS3500 A+ grading](#cs2420--cs3500-a-grading)
- [Branch Sessions (VS Code extension)](#branch-sessions-vs-code-extension)
- [Git subcommands](#git-subcommands)
- [MCP servers](#mcp-servers)
- [Research Search (GitHub Pages)](#research-search-github-pages)
- [helpers setup (project build-out)](#helpers-setup-project-build-out)
- [Installation](#installation)
- [Development & Contributing](#development--contributing)

---

## Use with any AI agent (helpers CLI)

`helpers` is the single control surface for Helpers. It installs Helpers into your AI agent(s),
toggles the whole tool surface or individual tools live, and reports health.

```sh
helpers install              # auto-detect agents (Claude Code, Copilot) and wire Helpers in
helpers install --agent claude   # Claude Code only ( --agent copilot | all )
helpers status               # what's installed, master switch, tool counts, agents
helpers doctor               # health checks
```

### Toggling (live — no agent restart)

```sh
helpers disable | enable     # master kill-switch for the entire Helpers tool surface
helpers bypass               # toggle the master switch
helpers tool list            # every tool + on/off state
helpers tool disable <name>  # turn one tool off
helpers tool enable all      # turn everything back on
```

Tool state lives in `~/.config/helpers-server/tools.json` and is re-read by the
running MCP server on every request, so toggles take effect immediately. A disabled tool
can be overridden for a single call with `{ "force": true }`.

### Updating

```sh
helpers update           # upgrade to the latest release, rebuild, and re-register
helpers update --check   # only report whether a newer release is available
```

`helpers update` upgrades a git checkout with `git pull` and a packaged install by
downloading the latest release tarball, then rebuilds the native tools and re-registers
with your agents. `helpers status` also shows a non-blocking "update available" hint,
refreshed in the background at most once a day. Restart your agent (or run `/mcp
reconnect`) afterward to load the new tools.

### Fast startup (C launcher + auto-managed background server)

`helpers install` registers a small **C launcher** (`helpers-mcp`, compiled on install) instead of
launching Node directly. It starts in ~1ms and connects to a background server
(`helpers-serverd.js`) that loads the tool modules once and stays resident, so
sessions start fast (~tens of ms) instead of paying cold Node startup every time.

It just works — nothing to manage:

- the background server **starts automatically** on first use and **exits after ~15 min
  idle**;
- it's per-workspace (keyed by cwd + Helpers env) so scope is preserved;
- it re-reads tool state per request, so `helpers` toggles stay live;
- if it isn't instantly ready (or there's no C compiler), the launcher **falls back to
  running Node directly within ~2s** — so startup is always reliable, never worse than
  plain Node.

```sh
helpers build            # (re)compile the launcher (optional; auto-run by install)
helpers daemon status    # is the background server running?
helpers daemon restart   # restart it (use after changing Helpers code)
```

### What `helpers install --agent claude` does

- Registers the `helpers` MCP server with Claude Code (`claude mcp add -s user`).
- Writes the Helpers core behavior into `~/.claude/CLAUDE.md` as a managed block (no clobber).
- Installs the `helpers` and `cs-grade` **skills**, the `/helpers` **slash command**, and the
  `cs-grade-improver` **subagent** into `~/.claude/`.

Run `/mcp` (or restart Claude Code) afterward so the server connects. The same artifacts
live under [`claude-config/`](claude-config) and the Copilot equivalents under
[`copilot-config/`](copilot-config).

---

## CS2420 / CS3500 A+ grading

`git-cs-grade` (also `helpers grade`) scores a Java course project against an objective
structural rubric and writes `GRADE.md`: a numeric+letter grade, a per-category scorecard
with the evidence behind each score, and a prioritized **Path to A+** checklist.

```sh
helpers grade .                      # auto-detect course
helpers grade ./hw3 --course cs3500  # object-oriented design rubric
helpers grade ./lab --course cs2420  # data-structures/algorithms rubric
git-cs-grade . --json            # machine-readable, for an automated loop
```

The intended loop: grade → fix the highest-impact checklist items → re-grade, until A+.
Claude Code users can hand the whole job to the **`cs-grade-improver`** subagent or the
**`cs-grade`** skill, which implement that loop (MVC separation, programming to interfaces,
design patterns, JUnit coverage, Javadoc, and cleanliness).

> The rubric grades what you can restructure (design, tests, docs, style); it does **not**
> run the course autograder's correctness suite, so pair it with the official tests.

---

## VS Code extension

The companion VS Code extension surfaces the Helpers community-cache panel, live tool
activity, strict-lint diagnostics, and model controls.

### Installation

The extension is bundled as a `.vsix`. Build it locally:

```sh
./scripts/build-vsix.sh
```

Then install via **Extensions → Install from VSIX…** in VS Code, or:

```sh
code --install-extension vscode-extension/helpers-*.vsix
```

> The `code --install-extension` form needs the `code` command on your PATH. If
> it isn't, add it via **Shell Command: Install 'code' command in PATH** from the
> VS Code command palette.

---

## Git subcommands

The standalone `git-*` CLIs are native Rust (the `gitcli` module of the
`helpers-native` crate), built and symlinked by `helpers build`. They are deterministic;
`git upload` is the only one that touches AI, and only as an opt-in.

| Command                    | What it does                                                                    |
| -------------------------- | ------------------------------------------------------------------------------- |
| `git upload`               | Stage, commit, and push with safe recovery; deterministic message by default, optional `-ai` via Claude/Copilot |
| `git get`                  | Initialize a local repo from a remote (lightweight clone flow)                  |
| `git initialize`           | Initialize the directory as a repo, create initial commit, set `origin`, push   |
| `git checkpoint`           | Commit current state with a deterministic message (used by `helpers` MCP tools)     |
| `git fucked-the-push`      | Destructive recovery: undo the last pushed commit while keeping changes staged  |
| `git resolve`              | Safe merge/rebase conflict resolution with automatic backup branches            |
| `git remerge`              | Merge a detached-work branch back into a target; aborts cleanly on conflicts    |
| `git scan-for-leaked-envs` | Scan for leaked secrets, API keys, and env vars with deterministic patterns     |
| `git help-i-pushed-an-env` | Emergency: scrub secrets from git history, including batch ops across all repos |

Plus `helpers setup` — a deterministic project build-out plan (see **helpers setup** below).

Man pages are installed for all commands. Use `git help <subcommand>` or `man git-<subcommand>` after installation.

---

## MCP servers

### helpers-server (combined server — recommended)

`helpers-server` exposes all tooling under one MCP server entry. When the VS Code extension is installed, it publishes this server globally so `checkpoint`, `branch_session_start`, and other tools are available in every workspace without editing `mcp.json` manually.

Manual registration if needed:

```json
{
  "servers": {
    "helpers": {
      "type": "stdio",
      "command": "node",
      "args": ["helpers-server"]
    }
  }
}
```

#### Exposed tools

Every Helpers tool below is implemented in native Rust (the `helpers-native` binary),
except web search/scrape which stay in Node (they drive a headless browser).
All tools are deterministic — no tool calls an AI model.

Core workflow and quality:

- `strict_lint` - run each language's own linters on a file/folder/workspace.
- `cs_lint` - scan for CS2420/CS3500 software-principle violations (single responsibility, documentation gaps, error handling, maintainability) and return one prioritized list with `file:line` + fix. Complements `helpers grade`; re-run to track the count to zero.
- `checkpoint` - stage and commit (deterministic message from the diff, or your own); optionally push. Stage a precise subset with `paths` (specific files) or `lines` (specific line ranges) for a focused checkpoint.

Project index (cheap repo map — orient without grepping):

- `index_project` - build/refresh a static map of files, symbols, and the reference graph (ranked), written to `.helpers/index/`.
- `project_map` - return a compact, token-cheap overview of the top modules plus a Mermaid graph; orient in one call.
- `lookup` - find where a symbol is defined and what references it, from the index graph instead of a grep sweep.
- `project_setup` - analyze the repo deterministically and return a concise build-out plan (purpose, stack + build/test/lint commands, gap checklist, and questions to ask the user). Drives a project to a complete, well-structured state fast; writes `.helpers/SETUP.md`. Also available as `helpers setup`.

Project flows (agent-agnostic reusable tools, scoped to the project):

- `register_workspace_tool` - register a named shell command/flow as a callable MCP tool (stored in `.helpers/tools/manifest.json`); turns a repetitive multi-step task into one tool call. Live immediately — no restart.
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

1. `project_map` (and `index_project` to refresh) to orient cheaply instead of reading/grepping many files.
2. `lookup <symbol|file>` to jump straight to definitions and references.
3. `list_workspace_tools` to reuse an existing project flow before re-implementing a task.
4. Call one specialized tool for the user goal (for example `search_web` or `strict_lint`).
5. Use `scrape_webpage` only for top hits that need deeper evidence.
6. End with `checkpoint` only after validation passes.

Environment variables to selectively disable groups:

```
HELPERS_MCP_DISABLE_RESEARCH=1
HELPERS_MCP_DISABLE_VISION=1
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

## helpers setup (project build-out)

`helpers setup` replaces the old Copilot DevOps audit with a **deterministic** project
build-out engine — no AI, no agent install. It analyzes the repository and prints
(and writes to `.helpers/SETUP.md`) a concise, structured plan that drives any project
to a complete, well-structured state quickly, while enforcing three rules:

1. **Minimal context** — the plan is a tight, ranked summary, never a file dump.
2. **Understand goals first** — purpose/goals are surfaced (or flagged as unknown)
   before any build-out steps are proposed.
3. **Clarify with the user** — ambiguities become explicit questions to ask first.

```sh
helpers setup            # analyze, print the plan, write .helpers/SETUP.md
helpers setup --no-write # print only
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
brew tap RockyWearsAHat/helpers
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
  https://raw.githubusercontent.com/RockyWearsAHat/github-shell-helpers/main/Helpers-Installer.sh \
  | bash
```

Then open a new terminal so your shell profile reloads the installed PATH and MANPATH snippet.

### No toolchain needed — prebuilt binaries

`helpers build` (run automatically by every install path) **downloads a prebuilt
`helpers-native` binary for your platform** from the GitHub release, so a normal install
needs **no Rust or C toolchain**. The one binary hosts the MCP tools, `git-cs-grade`, and
the ported `git-*` CLIs. Prebuilts are published for:

- macOS (universal: Apple Silicon + Intel)
- Linux x86_64 and aarch64 (glibc), Linux x86_64 (musl, e.g. Alpine)
- Windows x86_64 and arm64

If there's no prebuilt for your platform, or the download is unavailable, `helpers build`
**falls back to compiling from source** (and you can force that with
`helpers build --from-source`). Run **`helpers doctor`** to see status; the source-build
toolchain below is only needed for that fallback.

| Tool (source build only) | When needed | Install |
| --- | --- | --- |
| **Rust** (`cargo`) | `helpers build --from-source` | <https://rustup.rs> · macOS `brew install rust` · Windows `winget install Rustlang.Rustup` |
| **C compiler** (`cc`/`clang`/`gcc`) | the optional fast C launcher (always built locally) | macOS `xcode-select --install` · Linux `apt install build-essential` · Windows MinGW-w64 (e.g. [WinLibs](https://winlibs.com/)) on `PATH` |
| **MinGW-w64** (`dlltool`) | source build on the Windows **GNU** Rust host | WinLibs on `PATH`, or use the MSVC host: `rustup default stable-x86_64-pc-windows-msvc` |

Notes:

- The C launcher is a perf optimization, built locally and **never fatal** — without a C
  compiler the server just runs via Node (cold start). It is not shipped prebuilt because it
  bakes install-specific paths.
- `helpers build` reports failure (non-zero exit) if it can neither download nor build
  working tools — it never claims success with 0 tools.
- The `git-*` CLI shortcuts are symlinks to the one binary; on Windows symlinks need
  elevation, so they may be skipped — non-fatal, and the MCP tools (and `helpers grade`)
  work regardless.

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
| `vscode-extension/src/ipc-servers.js`      | Unix socket IPC between MCP server and extension           |
| `helpers-server`                    | MCP server — project index, checkpoint, strict_lint, knowledge, project flows, web research |
| `git-upload`                               | Stage/commit/push with AI messages and test detection      |
| `git-help-i-pushed-an-env`                 | Secret scrubbing from git history                          |
| `scripts/patch-vscode-apply-all.js`        | Coordinator for VS Code bundle patches                     |
