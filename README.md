# Helpers

**AI-agent tooling as a single native binary.** Helpers gives Claude Code, GitHub
Copilot, or any MCP-capable agent a fast, deterministic toolset — project indexing,
knowledge memory, web search, linting, CS grading, and safe git helpers — plus a
small control CLI. It's one prebuilt Rust binary: **no Node, no toolchain, no
runtime dependencies.** Download it for your platform and it just works.

```sh
curl -fsSL https://raw.githubusercontent.com/RockyWearsAHat/helpers/main/Helpers-Installer.sh | bash
```

That downloads the prebuilt binary for your OS, wires the `helpers` CLI onto your
PATH, and registers the MCP server with any AI agent it detects.

---

## Why a single binary

- **No Node.** The MCP server and the `helpers` CLI are the same native Rust
  binary (`helpers-native`). Tools start in ~1ms and the install needs nothing
  installed beforehand.
- **Prebuilt for every major platform.** macOS (Apple Silicon + Intel), Linux
  x86_64/aarch64 (glibc and musl/Alpine), and Windows x86_64/arm64. CI builds and
  install-tests all of them on every release.
- **Source build is the fallback.** On an unsupported platform, `helpers build
  --from-source` compiles it with Rust.
- **Agent-agnostic.** It speaks the Model Context Protocol over stdio; Claude Code
  and Copilot are first-class, but any MCP client can use it.

## Install

The one-line installer above is the recommended path. Alternatives:

| Method | How |
| --- | --- |
| Script installer (any OS) | `curl -fsSL …/Helpers-Installer.sh \| bash` |
| npm | `npm i -g @rockywearsahat/helpers` |
| Homebrew (macOS) | `brew install rockywearsahat/helpers/helpers` |
| Scoop (Windows) | `scoop bucket add helpers …; scoop install helpers` |
| Winget (Windows) | `winget install RockyWearsAHat.Helpers` |
| apt (Debian/Ubuntu) | add the [apt repo](https://rockywearsahat.github.io/helpers), then `apt install helpers` |
| Direct binary | download `helpers-native-<platform>.tar.gz` from [Releases](https://github.com/RockyWearsAHat/helpers/releases), extract, symlink `helpers` → `helpers-native`, run `helpers install` |
| macOS `.pkg` / Debian `.deb` | see [Releases](https://github.com/RockyWearsAHat/helpers/releases) |
| From source | clone, then `helpers build --from-source` (needs Rust) |

After installing, run `helpers install` once to register with your agent (the
installer does this automatically), then restart the agent or run `/mcp reconnect`.

## Use it (the `helpers` CLI)

```text
helpers status                 Install state, master switch, tool counts, agents
helpers doctor                 Health checks
helpers install [--agent auto|claude|copilot|all]
helpers uninstall [--agent claude|copilot|all]
helpers enable | disable | bypass [on|off]      Master switch (live, no restart)
helpers tool list | tool {enable,disable} <name|all> | tool reset
helpers update [--check]       Download the latest prebuilt binary for this platform
helpers build [--from-source]  (Re)create the helpers/git-* symlinks (or compile)
helpers index build|map|lookup <query>          Cheap project index
helpers setup                  Deterministic project build-out plan
```

Toggles are **live** — the MCP server re-reads its config each request, so enabling
or disabling tools takes effect without restarting the agent. A disabled tool can
still be forced for one call with `{ "force": true }`.

## MCP tools

Exposed to the agent via the `helpers` MCP server (`helpers-native mcp`):

**Workflow & quality**
- `cs_lint` — prioritized CS2420/CS3500 violations (`file:line` + fix).
- `strict_lint` — run each language's linters for a file/folder/workspace.
- `checkpoint` — stage/commit a precise subset with your own message.

**Project index** (orient without grepping)
- `index_project`, `project_map`, `lookup`, `project_setup`.

**Project flows** (reusable, project-scoped, callable by any agent)
- `register_workspace_tool`, `unregister_workspace_tool`, `list_workspace_tools`.

**Knowledge & web**
- `search_knowledge_index`, `search_knowledge_cache`, `read_knowledge_note`,
  `write_knowledge_note`, `update_knowledge_note`, `append_to_knowledge_note`,
  `build_knowledge_index`, `submit_community_research`.
- `search_web`, `scrape_webpage` — drive a real Chrome (CDP). Automated like a
  person; if Google shows a CAPTCHA, a visible Chrome opens for you to solve once,
  then the verified session is reused. **Requires Google Chrome** — the tools say
  so if it's missing.

## Git subcommands

The same binary is symlinked busybox-style to standalone `git-*` helpers:
`git-resolve`, `git-remerge`, `git-checkpoint`, `git-upload`, `git-get`,
`git-initialize`, `git-scan-for-leaked-envs`, `git-fucked-the-push`, and
`git-help-i-pushed-an-env`.

## CS2420 / CS3500 quality

The `lint` tool enforces the CS2420 / CS3500 principles directly: it learns them from
`corpus/cs-principles.md` (alongside the official language rules it learns from the docs) and
reports the exact `file:line` to fix. Followed to a T those principles ~guarantee an A+, so a
clean `lint` is the signal — there is no separate grader.

## VS Code extension

An optional VS Code extension (`vscode-extension/`) registers the Helpers MCP
server and ships agent guidance for Copilot. It's the only TypeScript component and
is editor-specific; the binary works without it in any MCP agent.

## Development & contributing

```sh
bash ./scripts/test.sh                          # JS/shell test suite
cargo test --manifest-path native/Cargo.toml    # native Rust tests
cargo build --release --manifest-path native/Cargo.toml
```

- **Architecture:** the runtime is one Rust crate (`native/`). The binary dispatches
  busybox-style: `helpers` → the control CLI, `helpers-native mcp` → the MCP server,
  `git-*` → the ported git helpers, and `helpers-native schemas|call` for tooling.
  Agent config is embedded in the binary; web tools use `headless_chrome` (CDP, no
  Node).
- **Releasing:** bump `VERSION` (single-line semver) and add
  `release-notes/v<version>.md`. On merge to `main`, CI cross-builds the binary for
  all platforms, runs a Node-free install-test on macOS/Linux/Windows, and publishes
  the release with prebuilt tarballs, checksums, and packages.

## License

MIT.
