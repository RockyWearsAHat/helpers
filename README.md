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

## Quick Start

### 1. Install

```sh
curl -fsSL https://raw.githubusercontent.com/RockyWearsAHat/helpers/main/Helpers-Installer.sh | bash
```

### 2. Verify Installation

```sh
helpers status
```

You should see agent detection and tool counts. If your agent isn't detected, manually register:

```sh
helpers install --agent claude    # Claude Code
helpers install --agent copilot   # GitHub Copilot
helpers install --agent all       # All available agents
```

### 3. Use in Your Agent

Restart your agent or run `/mcp reconnect`, then the `helpers` tools are available automatically in your agent's MCP interface.

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

The quick-start installer above is the recommended path. Alternatives:

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

## Usage Examples

### Project Indexing

Index your project to give your agent fast, deterministic project structure knowledge:

```sh
helpers index build          # Build the project index
helpers index map            # Show the project map
helpers index lookup <term>  # Search the index for symbols, files, or functions
```

When called from your agent, use `index_project` to create a project index that the agent can then query with `project_map` or `lookup` without additional filesystem scans.

### Git Helpers (Busybox-Style Subcommands)

Helpers provides safe, idempotent git operations:

```sh
git checkpoint               # Create a safe checkpoint before risky operations
git checkpoint --revert      # Revert to a previous checkpoint
git status-report            # Agent-friendly git status
```

These integrate seamlessly with your agent's workflow — the agent calls them as regular git subcommands.

### Quality Gates (Linting & Checking)

The `lint` tool provides configurable linting with built-in CS2420/CS3500 grading:

```sh
helpers tool list            # See all available linters and their state
helpers tool disable <name>  # Disable a specific linter
helpers tool enable all      # Re-enable all linters
```

Lint runs gate your code quality — a clean lint run satisfies course requirements automatically (no separate grading).

### Knowledge & Memory

Your agent can register project flows and knowledge that persist across sessions:

```sh
helpers setup                # Show the deterministic project build-out plan
```

### Web Research (Gated)

Controlled web search for your agent:

```sh
# Use in agent with { "force": true } to override safety gates if needed
```

## Architecture Overview

### Design Philosophy

**Single Binary, No Runtime**: Helpers is compiled to a native binary (`helpers-native`) using Rust. It contains both:
- The MCP server (stdio-based Model Context Protocol server)
- The CLI tool (`helpers` command)

Starting any tool takes ~1ms because there's no interpreter startup, no dependency loading, and no daemon process — just native code execution.

### Component Structure

1. **Native Binary (`helpers-native`)**: Single Rust executable for all platforms
   - MCP server mode: Listens on stdio for agent requests
   - CLI mode: Direct invocation via the `helpers` command
   - Hot-reloadable config: Tool state changes take effect instantly

2. **Git Subcommands**: Symlinked from the binary (`git-checkpoint`, `git-*`, etc.)
   - Git sees them as native subcommands
   - Agent can call them as regular git operations

3. **Configuration**: Stored in `~/.config/helpers-server/tools.json`
   - Agent detection and registration
   - Per-tool on/off state
   - Live reloaded on every request

### Platform Support

| Platform | Status | Build |
| --- | --- | --- |
| macOS (Apple Silicon) | ✓ | Native ARM64 |
| macOS (Intel) | ✓ | Native x86_64 |
| Linux x86_64 (glibc) | ✓ | Native |
| Linux aarch64 (glibc) | ✓ | Native ARM64 |
| Linux (musl/Alpine) | ✓ | Native |
| Windows x86_64 | ✓ | Native |
| Windows ARM64 | ✓ | Native |
| Unsupported OS | ✓ | Source build via Rust |

## Troubleshooting

### Agent Not Detecting Helpers

```sh
helpers status              # Check if Helpers is installed and agents are detected
helpers install --agent auto # Auto-detect and register with found agents
```

If still not detected, try manual registration:

```sh
helpers install --agent claude    # Claude Code
helpers install --agent copilot   # GitHub Copilot
```

### Tools Not Working in Agent

Restart your agent or use `/mcp reconnect` (Claude Code):

```
/mcp reconnect
```

### Performance Issues

Check the health of your installation:

```sh
helpers doctor               # Run health checks
helpers update --check       # Check for available updates
```

### Updating to Latest Version

```sh
helpers update              # Download and install the latest prebuilt binary
```

Updates are automatic for package managers (Homebrew, Scoop, npm, apt) but manual for direct binary installs.

### Disabling Specific Tools

If a tool is problematic, disable it without restarting:

```sh
helpers tool disable <name>  # Disable one tool
helpers tool list            # See what's available and its state
helpers tool reset           # Reset all tools to default state
```

## MCP tools, git subcommands, architecture details, dev/release

Exposed to the agent via the `helpers` MCP server (`helpers-native mcp`): project
index (`index_project`/`project_map`/`lookup`), project flows
(`register_workspace_tool`), knowledge memory, gated web research, quality gates
(`lint`, `checkpoint`), and the busybox-style `git-*` subcommands. CS2420/CS3500
grading is a property of a clean `lint` run — there is no separate grader.

Full docs — every MCP tool with its schema, the complete `git-*` subcommand list,
architecture (native binary dispatch, MCP startup, live tool toggling), the VS Code
extension, and the dev/release process — live in `helpers.dx`: read via the dx MCP
tools (`dx_read`) or `dx read helpers.dx`.

## Development

### Building from Source

Clone the repository and build with Rust:

```sh
git clone https://github.com/RockyWearsAHat/helpers.git
cd helpers
helpers build --from-source    # Compiles the Rust binary
npm test                        # Run tests
npm run setup-git-hooks        # Install pre-commit hooks
```

### Project Structure

```
.
├── bin/                       # Binary entry points (CLI)
├── native/                    # Rust native binary source
├── lib/                       # Shared library code
├── lint-checkers/            # Linting configurations per course
├── test/                     # Test suite
├── scripts/                  # Build and install scripts
├── AGENTS.md                 # Agent discovery file
└── helpers.dx                # Comprehensive documentation
```

### Running Tests

```sh
npm test                       # Run the full test suite
npm run setup-git-hooks       # Set up git hooks for commits
```

The test suite verifies:
- Binary compilation for all platforms
- MCP server initialization
- CLI command execution
- Installation on multiple OS variants
- Git integration

### Code Standards

- **Deterministic**: All tools produce the same output given the same input, every time
- **Type-safe**: Rust provides compile-time safety; schema-based tool definitions prevent runtime errors
- **Live-reloadable**: Config changes take effect without restarting the server
- **Agent-agnostic**: Works with any MCP-capable agent via the Model Context Protocol

## Contributing

Helpers welcomes contributions. Before submitting a PR:

1. **Run tests locally**: `npm test`
2. **Check linting**: Tools are auto-linted on commit
3. **Document changes**: Update relevant sections in `helpers.dx`
4. **Cross-platform**: Verify changes work on macOS, Linux, and Windows

## Resources

- **Bug reports**: [GitHub Issues](https://github.com/RockyWearsAHat/helpers/issues)
- **Discussions**: [GitHub Discussions](https://github.com/RockyWearsAHat/helpers/discussions)
- **Documentation**: See `helpers.dx` for full API docs
- **Releases**: [GitHub Releases](https://github.com/RockyWearsAHat/helpers/releases)

## License

MIT.
