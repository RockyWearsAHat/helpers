# Debug Adapter Protocol (DAP) — Architecture, Breakpoints & Debugging Interface

## Overview

The Debug Adapter Protocol (DAP) mirrors the success of LSP but for debuggers. Before DAP, each debugger (GDB, LLDB, Python debugger, Node debugger) required custom integration in every IDE. VSCode integrated GDB differently than JetBrains CLion, differently than emacs-gdb. Language maintainers couldn't control the debugging experience.

DAP (standardized by Microsoft, co-designed with JetBrains) defines a protocol between a **debug UI** (the IDE) and a **debug adapter** (which controls the target runtime). Now a language or runtime can build a single debug adapter; any IDE supporting DAP can use it.

## Client-Server Architecture

Like LSP, DAP uses a client-server model:

- **Client** (debug UI) — The IDE. Sends requests to inspect state, execute operations (step, continue, pause)
- **Server** (debug adapter) — Bridges the IDE and the debugged process/runtime. Translates IDE requests into debugger-specific operations

The communication layer is **JSON-RPC 2.0** over a transport (typically stdio or TCP).

```
┌─────────────────┐         JSON-RPC              ┌──────────────────┐
│                 │  ◄──── (requests/responses)   │                  │
│   IDE           │  ───► (events/notifications)  │  Debug Adapter   │
│  (VSCode)       │                               │  (e.g. GDB)      │
│                 │                               │                  │
└─────────────────┘                               └────────┬─────────┘
                                                           │ (GDB protocol)
                                                           ▼
                                                   ┌──────────────────┐
                                                   │  Debugged        │
                                                   │  Process/Runtime │
                                                   └──────────────────┘
```

The debug adapter communicates with the debugged process using language/runtime-specific protocols (e.g., GDB machine interface, LLDB debugserver, Python debugpy protocol).

## Initialization and Capabilities

Like LSP, DAP begins with an `initialize` request:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "clientID": "vscode",
    "clientName": "Visual Studio Code",
    "adapterID": "python",
    "pathFormat": "path",
    "linesStartAt1": true,
    "columnsStartAt1": true,
    "supportsVariableType": true,
    "supportsVariablePaging": true,
    "supportsRunInTerminalRequest": true,
    "locale": "en-us"
  }
}
```

The adapter responds with capabilities:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "capabilities": {
      "supportsConfigurationDoneRequest": true,
      "supportsConditionalBreakpoints": true,
      "supportsHitConditionalBreakpoints": true,
      "supportsLogPoints": true,
      "supportsEvaluateForHovers": true,
      "supportsSetExpression": true,
      "supportsClipboardContext": true,
      "supportsStepBack": false,
      "supportsRestartFrame": true,
      "supportsGotoTargetsRequest": true,
      "supportsStepInTargetsRequest": true,
      "supportsCompletionsRequest": true,
      "supportsReadMemoryRequest": false
    }
  }
}
```

During initialization, the **debugger registers for events** and prepares to receive requests. The client then sends `launch` (start a new process) or `attach` (connect to a running process).

## Launch vs. Attach

### Launch

The debug adapter **starts a new instance** of the debugged program:

```json
{
  "method": "launch",
  "params": {
    "program": "/path/to/app",
    "args": ["arg1", "arg2"],
    "cwd": "/path/to/project",
    "stopOnEntry": false,
    "console": "integratedTerminal"
  }
}
```

The adapter spawns the process, attaches its debugger, and waits for initialization. The program may stop at entry (first line) or run to the first breakpoint.

### Attach

The debug adapter **connects to a running process** that's already instrumented for debugging:

```json
{
  "method": "attach",
  "params": {
    "processId": 1234,
    "stopOnEntry": false
  }
}
```

This is used for:

- Debugging a long-running service (web server, background worker) without restarting
- Debugging a process started outside the IDE
- Remote debugging (attach to a process on another machine via SSH tunnel or network protocol)

## Breakpoint Model

DAP supports multiple breakpoint types via `setBreakpoints`:

### Simple Breakpoint

Stop execution at a line:

```json
{
  "method": "setBreakpoints",
  "params": {
    "source": {"path": "/home/user/project/app.py"},
    "breakpoints": [
      {"line": 42},
      {"line": 100}
    ]
  }
}
```

### Conditional Breakpoint

Stop only if a condition is true:

```json
{
  "breakpoints": [
    {"line": 42, "condition": "count > 10"}
  ]
}
```

The adapter evaluates the condition in the debugged program's context. If the condition is false, execution continues; if true, the debugger stops.

### Hit Count Breakpoint

Stop after N hits:

```json
{
  "breakpoints": [
    {"line": 42, "hitCondition": "5"}
  ]
}
```

The first 4 times the breakpoint is reached, execution continues. On the 5th hit, the debugger stops.

### Logpoint

Print a message without stopping (non-stopping breakpoint):

```json
{
  "breakpoints": [
    {"line": 42, "logMessage": "Reached line 42, count={count}"}
  ]
}
```

The adapter expands variable references (in braces) and prints the message to the debug console. The program continues running.

## Execution Control

### Continue and Pause

Resume execution or pause a running program:

```json
{"method": "continue"}      // Resume
{"method": "pause"}         // Pause
```

### Step Operations

The IDE sends step requests, and the adapter moves the execution pointer:

- **`stepIn`** — Step into a function call
- **`stepOut`** — Execute to the end of the current function
- **`next`** (step over) — Execute the current line; skip function calls
- **`stepBack`** — Undo the last step (if supported)
- **`goto`** — Jump execution to a specific line (if supported)

Each step request results in a **`stopped` event** sent by the adapter when the debugger reaches a new location:

```json
{
  "method": "stopped",
  "params": {
    "reason": "breakpoint",
    "threadId": 1,
    "description": "Stopped at breakpoint"
  }
}
```

The `reason` indicates why execution stopped: `"breakpoint"`, `"step"`, `"pause"`, `"exception"`, etc.

## Stack Frames and Variables

When paused, the IDE requests the **call stack** via `stackTrace`:

```json
{
  "method": "stackTrace",
  "params": {
    "threadId": 1,
    "startFrame": 0,
    "levels": 20
  }
}
```

The adapter responds with a list of stack frames:

```json
{
  "result": {
    "stackFrames": [
      {
        "id": 100,
        "name": "my_function",
        "source": {"path": "/home/user/project/app.py"},
        "line": 42,
        "column": 5
      },
      {
        "id": 101,
        "name": "main",
        "source": {"path": "/home/user/project/app.py"},
        "line": 200,
        "column": 0
      }
    ]
  }
}
```

For each frame, the IDE can request **variables** via `variables`:

```json
{
  "method": "variables",
  "params": {
    "variablesReference": 100  // Reference to the first frame
  }
}
```

The adapter lists local variables and their values:

```json
{
  "result": {
    "variables": [
      {"name": "x", "value": "42", "variablesReference": 0},
      {"name": "obj", "value": "MyObject(...)", "variablesReference": 101},
      {"name": "list", "value": "[1, 2, 3]", "variablesReference": 102}
    ]
  }
}
```

Complex objects have a `variablesReference` > 0, allowing the IDE to request the object's children:

```json
{
  "method": "variables",
  "params": {
    "variablesReference": 101  // Request children of obj
  }
}
```

This lazy expansion of nested structures keeps the protocol efficient: the IDE only requests details it displays.

## Watch Expressions and Evaluation

The user can add **watch expressions** in the debugger UI (e.g., `myVar.length`). The IDE sends `evaluate` requests:

```json
{
  "method": "evaluate",
  "params": {
    "expression": "myVar.length",
    "frameId": 100,
    "context": "watch"
  }
}
```

The adapter evaluates the expression in the debugged process's context:

```json
{
  "result": {
    "result": "5",
    "type": "int",
    "variablesReference": 0
  }
}
```

Other evaluation contexts:

- **`"watch"`** — Watch expression; evaluate in the paused frame's context
- **`"repl"`** — Debug console (REPL); the user typed the expression interactively
- **`"hover"`** — Hover in the editor; show value of a symbol

Evaluation can support **`setExpression`** to modify variables in place:

```json
{
  "method": "setExpression",
  "params": {
    "expression": "myVar",
    "value": "42",
    "frameId": 100
  }
}
```

## Threads and Async Execution

DAP supports **multi-threaded debugging**. The adapter sends `thread` events when threads are created/destroyed:

```json
{
  "method": "thread",
  "params": {
    "threadId": 2,
    "reason": "started"
  }
}
```

The IDE can request a list of all threads via `threads`:

```json
{
  "method": "threads"
}
```

And receive:

```json
{
  "result": {
    "threads": [
      {"id": 1, "name": "Main"},
      {"id": 2, "name": "Worker-1"},
      {"id": 3, "name": "Worker-2"}
    ]
  }
}
```

When paused, the IDE can switch focus between threads to inspect each thread's call stack and variables independently.

## Source and Disassembly

The `source` request retrieves source code for a file (used when source isn't available locally, e.g., remote debugging):

```json
{
  "method": "source",
  "params": {
    "sourceReference": 123
  }
}
```

For languages with compiled bytecode or assembly output, adapters can support `disassemble` requests to show low-level instructions.

## Reverse Debugging

Some debuggers support **reverse debugging** — stepping backward through execution. The adapter advertises this with `supportsStepBack` in capabilities, and the IDE calls `stepBack` to reverse a step.

## Configuration and Termination

### Configuration Done

After the client sets breakpoints and configures the session, it sends `configurationDone`:

```json
{
  "method": "configurationDone"
}
```

This signal tells the adapter to start accepting breakpoints and begin execution. Without this, the debuged process remains suspended.

### Disconnect and Terminate

To end the session:

```json
{
  "method": "disconnect",
  "params": {
    "terminateDebuggee": true  // Also kill the debugged process
  }
}
```

If `terminateDebuggee` is false, the debugged process continues running; the debugger just disconnects.

## Common Implementation Patterns

### Pattern 1: Wrapping an Existing Debugger

Most DAP adapters are thin wrappers around language-specific debuggers:

```
VSCode (DAP client) ──► My Adapter (DAP server) ──► GDB (over MI protocol)
```

The adapter:

1. Parses DAP requests from VSCode
2. Translates them to GDB machine interface commands
3. Parses GDB responses
4. Translates back to DAP events

**Example:** The C++ DAP adapter (`lldb-dap`) wraps LLDB's debugserver. The Python DAP adapter (`debugpy`) is a native Python debugger that implements DAP directly.

### Pattern 2: Native DAP Implementation

Some languages implement DAP natively:

- **Kotlin/Java**: The Kotlin language server includes debugging support with DAP
- **Go**: An independent Delve debugger implements DAP
- **Node.js**: The Node debugger protocol was adapted to DAP

These don't wrap external debuggers; they implement the debugging logic directly.

## Comparison with LSP

| Aspect | LSP | DAP |
|--------|-----|-----|
| **Purpose** | Language tooling (completion, hover, definition) | Debugging (breakpoints, stepping, inspection) |
| **Directionality** | Mostly client → server (mostly requests) | Bidirectional (requests + async events) |
| **State** | Minimal; document content is the source of truth | Heavy; execution state (stack, threads, variables) is critical |
| **Latency tolerance** | Higher; 500ms completion is acceptable | Lower; 50ms pause response expected |
| **Multi-server support** | One language server per language typical | One debug adapter per debugger typical |
| **Activation** | Implicit; server runs when needed | Explicit; user launches a debug session |

## Decoupling and Standardization Benefits

DAP achieves the same goal as LSP: decoupling the debugging **interface** (command set) from the debugging **implementation** (how breakpoints actually work in GDB vs. LLDB vs. Python debugger).

Before DAP:

- VSCode + GDB: Custom GDB integration in VSCode code
- JetBrains + GDB: Custom GDB integration in JetBrains code
- Vim + GDB: Custom Vim plugin

After DAP:

- Any editor + GDB: That editor uses a DAP client; GDB has a DAP adapter
- Any editor + LLDB: That editor uses a DAP client; LLDB has a DAP adapter

The multiplication issue (M editors × N debuggers) is solved, as with LSP.

---

## See Also

- `lsp-protocol.md` — LSP, the parallel protocol for language services
- `ide-vscode-architecture.md` — How debuggers integrate into VS Code
- `ide-extension-development.md` — Writing debug adapters as VS Code extensions