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

## MCP tools, git subcommands, architecture, dev/release

Exposed to the agent via the `helpers` MCP server (`helpers-native mcp`): project
index (`index_project`/`project_map`/`lookup`), project flows
(`register_workspace_tool`), knowledge memory, gated web research, quality gates
(`lint`, `checkpoint`), and the busybox-style `git-*` subcommands. CS2420/CS3500
grading is a property of a clean `lint` run — there is no separate grader.

Full docs — every MCP tool with its schema, the complete `git-*` subcommand list,
architecture (native binary dispatch, MCP startup, live tool toggling), the VS Code
extension, and the dev/release process — live in `helpers.dx`: read via the dx MCP
tools (`dx_read`) or `dx read helpers.dx`.

## License

MIT.
