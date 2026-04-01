# VS Code Agent Tools — Comprehensive Reference

VS Code agents are autonomous code assistants that accomplish complex engineering tasks by combining reasoning, planning, and tool invocation. The power of an agent lies entirely in its **tool suite** — what actions it can take on your behalf. Understanding these tools means understanding exactly what an agent can and cannot do in your workspace.

## The Tool Ecosystem

VS Code agents access three categories of tools:

1. **Built-in tools** — Native capabilities from VS Code itself (reading files, running terminals, searching code, etc.)
2. **Extension tools** — Tools contributed by installed extensions via the Language Model Tools API
3. **MCP tools** — Tools provided by Model Context Protocol servers (database queries, API integrations, custom capabilities)

When you enable a tool in the chat interface, the agent gains permission to invoke it during problem-solving. The agent autonomously decides *when* to use each tool based on your prompt and the current context.

## Built-in Read Tools (#read)

Read tools give agents access to workspace state and diagnostics:

### #read/problems
The most powerful read tool: provides access to **every compile error, lint warning, and diagnostic** currently visible in the VS Code Problems panel. This includes:
- TypeScript/JavaScript errors and type mismatches
- Python linting violations (from Pylance, Ruff, etc.)
- CSS/markup errors
- Compiler warnings (C++, Rust, Go)
- Custom diagnostics from language servers

The agent sees the full diagnostic object: line number, column, severity (Error/Warning/Information/Hint), message, and related code actions. This enables **self-correcting code generation** — the agent can generate code, see diagnostics, understand what failed, and fix it.

### #read/readFile
Read arbitrary files from the workspace. The agent specifies a path and line range; VS Code returns the file contents. Works for source code, configuration files, documentation, and any text file. Essential for understanding existing code before making changes.

### #read/terminalLastCommand
Retrieve the last command run in the terminal and its context. Useful for understanding what the agent previously executed or what a user ran.

### #read/terminalSelection
Access selected text from the integrated terminal. Allows the agent to work with command output or terminal state.

### #read/getNotebookSummary
For Jupyter notebooks: get a summary of all cells (type, language, execution state, outputs). Returns cell IDs and metadata without fetching full content.

### #read/readNotebookCellOutput
For Jupyter notebooks: read the saved output of a previously executed cell. Enables agents to analyze notebook computation results without re-running.

## Built-in Edit Tools (#edit)

Edit tools allow persistent modifications to the workspace:

### #edit/createFile
Create a new file with content. The agent specifies path and initial content. Paths can be deeply nested; parent directories are auto-created. Fails safely if the file already exists.

### #edit/createDirectory
Create nested directory structures. Equivalent to `mkdir -p`. Safe if already exists.

### #edit/editFiles
Modify existing files. The agent specifies one or more text replacements (find/replace operations). Each replacement includes: old string (with context to avoid ambiguity), new string, and explanation. Multiple replacements are applied sequentially in a single edit operation. This is the core text-editing tool for code generation.

### #edit/editNotebook
Edit Jupyter notebooks: insert new cells, delete cells, or modify cell content. Supports markdown and code cells. Cell IDs identify which cells to modify.

## Built-in Execute Tools (#execute)

Execute tools perform actions outside the editor:

### #execute/runInTerminal
**The most dangerous tool.** Run arbitrary shell commands in the integrated terminal. Command output is captured and returned. The agent can:
- Compile code: `npm run build`, `cargo build`, `python -m pytest`
- Install packages: `npm install`, `pip install`
- Run tests
- Start servers
- Execute arbitrary system commands

The terminal runs in the workspace directory. The agent can see output and respond. This is how agents drive build systems, testing frameworks, and external tools.

**Security implications:** This tool can modify files, network access, and system state. VS Code provides approval dialogs with configurable auto-approve rules and experimental terminal sandboxing (macOS/Linux) to restrict file system and network access.

### #execute/getTerminalOutput
Read the full output from a terminal that the agent or user previously executed. Useful after a long-running command completes.

### #execute/createAndRunTask
Execute a VS Code task (defined in `.vscode/tasks.json`). Tasks are reusable commands for build, test, watch, and custom workflows. Returns task output. More structured than raw terminal execution; used for build systems and test runners.

### #execute/runNotebookCell
Execute a single Jupyter notebook cell and get its output. Allows interactive notebook workflows where the agent generates cells, runs them, analyzes output, and generates follow-up cells.

### #execute/testFailure
Access test failure information from the last test run. Provides detailed error context for test-driven debugging.

## Built-in Search Tools (#search)

Search tools enable code exploration and understanding:

### #search/codebase
Full-text semantic search across the codebase (not indexed). The agent specifies a query; VS Code searches and returns matching code snippets with file paths and line numbers. Useful for understanding patterns, finding examples, or locating related code before making changes.

### #search/textSearch
Regex or literal text search across files. Returns paths and matching lines. Fast for targeted searches by identifier, function name, or pattern.

### #search/fileSearch
File name and path search. Find files matching a glob pattern or name prefix.

### #search/listDirectory
List contents of a directory (files and subdirectories). Used to explore structure.

### #search/changes
Access git changes: modified files, staged changes, merge conflicts. Returns paths and diff summaries. Useful for understanding what's changed in the current branch.

### #search/usages
Find all references to a symbol (function, class, variable) in the codebase. Language-aware; understands scope and imports.

## Built-in Web Tool (#web)

### #web/fetch
Fetch content from any URL. The agent specifies the URL; returns the HTML body or JSON. VS Code shows approval dialogs for URL access to prevent unintended data leaks to untrusted domains. Both request and response can require approval (response approval prevents prompt injection from fetched content).

## Agent Runtime Tools (#agent)

### #agent/runSubagent
Delegate a sub-task to a specialized agent. The parent agent can invoke another agent (e.g., a @testing agent) and receive results. **Note:** VS Code disables `runSubagent` for subagents (hardcoded in the Copilot extension core). Only top-level agents and the main chat session can invoke subagents. Agents that may run as subagents should detect the absence of `runSubagent` and fall back to skill-driven execution.

## VS Code Integration Tools (#vscode)

### askQuestions
Pause the agent and ask the user one or more questions. Useful when the agent needs clarification before proceeding.

### installExtension
Install a VS Code extension by ID (e.g., `ms-python.python`). Used to add new capabilities to the IDE mid-session.

### runCommand
Execute any VS Code command via ID (e.g., `workbench.action.openFolder`). Used for UI automation and workspace manipulation.

The `VSCodeAPI` tool provides low-level access to VS Code's extension API, allowing agents to call arbitrary extension APIs directly.

## Tool Approval & Permission Levels

Agents respect a **three-tier permission system**:

- **Default:** Tools show approval dialogs before execution. The agent can ask clarifying questions if needed.
- **Bypass Approvals:** Tools auto-approve without dialogs. The agent auto-retries on errors without asking.
- **Autopilot (Preview):** Tools auto-approve, errors auto-retry, and clarifying questions auto-respond. The agent continues autonomously until it judges the task complete.

Each tool can be pre-approved (skip the dialog) or post-approved (skip reviewing the result). Users can manage approvals centrally via **Chat: Manage Tool Approval** command.

## Terminal Sandboxing (Preview)

On macOS/Linux, agents can run terminal commands in a **sandbox** that restricts file system and network access. When enabled:
- Commands default to read/write access in the current working directory only
- Network access is blocked by default; specific domains can be allowlisted
- File system can be configured with allow/deny rules

Sandboxing provides defense-in-depth against prompt injection: even if an attacker tricks the agent into running a malicious command, the sandbox limits damage.

## Tool Sets

Related tools can be grouped into **reusable tool sets** and referenced as a single entity in prompts. For example, `#edit` is a predefined tool set containing `edit/createFile`, `edit/createDirectory`, and `edit/editFiles`. Custom tool sets can be defined in `.jsonc` files to organize domain-specific tools.

## How Agents Use Tools

The agent reasoning loop follows this pattern:

1. **Analyze prompt** — Understand the task
2. **Choose tools** — Decide which tools are needed
3. **Invoke tools** — Call each tool with appropriate parameters
4. **Analyze results** — Read tool output and diagnostics
5. **Iterate** — Call more tools as needed, or generate a response

The `#read/problems` tool closes the loop: after an edit, the agent can immediately see diagnostics, understand what failed, and refine its approach. This self-correction is what makes agents effective generators rather than naive code generation.

## Permission & Safety Considerations

Tools that modify files or run commands require approval because they can:
- Corrupt or delete data
- Run arbitrary code on your system
- Leak sensitive information to untrusted domains
- Be exploited via prompt injection (malicious content in tool output)

The approval system is a user control surface, not a security boundary. Treat Bypass Approvals and Autopilot like root access: they're powerful, they're convenient, and they bypass protections. Use them only when you trust the agent fully or in isolated/disposable environments.

VS Code's sandboxing and auto-approve rules add layers, but they're not hermetic. If security is critical, consider running agents in containers or separated network zones.

## Context Window & Tool Parallelism

Agents maintain a context window (limited token budget). Large codebases mean strategic tool use: the agent may use `#search/codebase` to understand architecture rather than `#read/readFile` on every file. Parallel tool invocation (within a single request) is supported; the agent invokes multiple tools and waits for all results before responding.

## See Also

- [ide-mcp-integration.md](ide-mcp-integration.md) — How MCP extends agent tool capabilities
- [ide-vscode-diagnostics-system.md](ide-vscode-diagnostics-system.md) — What the `#read/problems` tool accesses
- [ide-vscode-terminal-integration.md](ide-vscode-terminal-integration.md) — How terminal execution works under the hood