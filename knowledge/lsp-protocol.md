# Language Server Protocol (LSP) — Architecture, Specification & Design Philosophy

## Overview

The Language Server Protocol (LSP) is an open-source, JSON-RPC-based protocol standardized by Microsoft to decouple editors and IDEs from language tooling. Before LSP (pre-2016), each editor had to integrate deeply with language-specific compilers, type checkers, linters, and other tools. VSCode + TypeScript required TypeScript bindings in VSCode. Vim + Python needed separate Vim plugins. This created exponential integration work: M editors × N languages = M×N custom integrations, each with different APIs, update cycles, and maintenance costs.

LSP inverts the problem: define a single, language-neutral protocol. Now M editors can all use N language servers over the same interface. An editor author builds LSP client support once; a language maintainer builds an LSP server once. Integration cost becomes M + N instead of M×N.

The protocol is maintained by Microsoft as open source and has become the de facto standard for language tooling in modern editors (VSCode, Neovim, Emacs, Sublime, JetBrains IDEs, and dozens of others).

## Client-Server Architecture

LSP defines two roles: **client** (the editor) and **server** (the language tool). The client initiates communication; the server responds to requests and may publish notifications asynchronously.

### Communication Layer

Messages flow over a **transport mechanism** — typically stdio (stdin/stdout), but also TCP sockets, WebSockets, or named pipes. The application layer uses **JSON-RPC 2.0** (RFC 7159), which frames requests, responses, and one-way notifications as JSON objects.

A request expects a response:
```json
{"jsonrpc": "2.0", "id": 1, "method": "textDocument/hover", "params": {...}}
{"jsonrpc": "2.0", "id": 1, "result": {...}}
```

A notification is one-way — the client sends it without expecting a reply:
```json
{"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {...}}
```

### Initialization Handshake

LSP communication always begins with a **handshake**. The client sends an `initialize` request with capabilities it supports:

```json
{
  "jsonrpc": "2.0",
  "id": 0,
  "method": "initialize",
  "params": {
    "processId": 1234,
    "rootPath": "/home/user/project",
    "capabilities": {
      "textDocument": {
        "completion": {"dynamicRegistration": true},
        "hover": {},
        "definition": {}
      }
    }
  }
}
```

The server responds with its **capabilities** — which LSP features it implements:

```json
{
  "jsonrpc": "2.0",
  "id": 0,
  "result": {
    "capabilities": {
      "textDocumentSync": "Full",
      "completionProvider": {"resolveProvider": true},
      "hoverProvider": true,
      "definitionProvider": true,
      "referencesProvider": true,
      "renameProvider": true,
      "diagnosticProvider": {"interFileDependencies": true}
    },
    "serverInfo": {"name": "my-language-server", "version": "1.0.0"}
  }
}
```

Both client and server now know what's mutually supported and can optimize communication. The client doesn't request features the server doesn't advertise; the server doesn't send features the client didn't request.

After initialization, the client sends a `initialized` notification, and the server changes into "running" mode. If the client wants to disconnect gracefully, it sends `shutdown` (waits for confirmation), then `exit` (terminates the connection).

## Core Language Features

### Documents and Text Synchronization

A **text document** is identified by a URI (e.g., `file:///home/user/project/main.py`). When a document opens, the client sends `textDocument/didOpen` with the full content:

```json
{
  "method": "textDocument/didOpen",
  "params": {
    "textDocument": {
      "uri": "file:///home/user/project/main.py",
      "languageId": "python",
      "version": 1,
      "text": "def hello():\n    return 42"
    }
  }
}
```

As the user edits, the client sends `textDocument/didChange` notifications. The **synchronization strategy** can be one of:

- **Full sync**: Send the entire file content on every change (simpler for the server, higher bandwidth)
- **Incremental sync**: Send only the changed ranges (more efficient, requires the server to maintain a copy of the file)
- **None**: Server doesn't track file content; client is responsible for maintaining state

When a document closes, the client sends `textDocument/didClose`.

### Completion

When the user requests or the editor auto-triggers completion, the client sends `textDocument/completion`:

```json
{
  "method": "textDocument/completion",
  "params": {
    "textDocument": {"uri": "file:///home/user/project/main.py"},
    "position": {"line": 5, "character": 12}
  }
}
```

The server responds with a list of **CompletionItem** objects:

```json
{
  "result": [
    {
      "label": "hello",
      "kind": 12,
      "detail": "def hello() -> int",
      "documentation": "Greet the user",
      "sortText": "hello",
      "insertText": "hello()"
    }
  ]
}
```

The completion item includes metadata: the display label, kind (function, variable, module, etc.), documentation, and the text to insert. Optionally, the server can defer expensive resolution until the user selects an item, using `completionItem/resolve`.

### Hover Information

When the user hovers over a symbol, the client sends `textDocument/hover`:

```json
{
  "method": "textDocument/hover",
  "params": {
    "textDocument": {"uri": "file:///home/user/project/main.py"},
    "position": {"line": 0, "character": 5}
  }
}
```

The server responds with **MarkedString** (plain text or code block) or structured **MarkupContent**:

```json
{
  "result": {
    "contents": {
      "kind": "markdown",
      "value": "```python\ndef hello() -> int\n```\n\nGreet the user and return the meaning of life."
    }
  }
}
```

### Definition and References

`textDocument/definition` tells the client where a symbol is declared:

```json
{
  "method": "textDocument/definition",
  "params": {
    "textDocument": {"uri": "file:///home/user/project/main.py"},
    "position": {"line": 5, "character": 10}
  }
}
```

The server responds with one or more **Location** objects (file URI + range):

```json
{
  "result": [
    {
      "uri": "file:///home/user/project/module.py",
      "range": {"start": {"line": 20, "character": 0}, "end": {"line": 22, "character": 10}}
    }
  ]
}
```

`textDocument/references` returns all places where a symbol is used, optionally including the declaration.

### Rename

`textDocument/rename` asks the server to compute all locations that must change when a symbol is renamed:

```json
{
  "method": "textDocument/rename",
  "params": {
    "textDocument": {"uri": "file:///home/user/project/main.py"},
    "position": {"line": 0, "character": 5},
    "newName": "greeting"
  }
}
```

The server returns a **WorkspaceEdit** describing all file changes needed (insertions, deletions, full file replacements). This allows renames to be atomic and IDE-coordinated.

### Diagnostics

The server publishes diagnostics (errors, warnings, hints) asynchronously via `textDocument/publishDiagnostics` notifications. These aren't responses to client requests; the server initiates them whenever it detects issues:

```json
{
  "method": "textDocument/publishDiagnostics",
  "params": {
    "uri": "file:///home/user/project/main.py",
    "diagnostics": [
      {
        "range": {
          "start": {"line": 2, "character": 8},
          "end": {"line": 2, "character": 15}
        },
        "severity": 1,
        "code": "E501",
        "source": "pylint",
        "message": "Line too long (105 > 100 characters)"
      }
    ]
  }
}
```

The client displays these in the editor's problem panel, gutter, or inline annotations. Severity levels are: 1 (error), 2 (warning), 3 (information), 4 (hint).

## Why LSP Succeeded Where Earlier Efforts Failed

Before LSP, there were ad hoc attempts at language-editor integration: Language-specific IDE plugins (think Eclipse + Java plugins), editor scripting APIs (Vim's python interface, Emacs Lisp), or tool-specific integrations (Sublime's LSP plugin in 2018 predates LSP adoption but required per-language implementation).

LSP won because:

1. **Standardization + Simplicity**: One protocol for all editors and languages. The JSON-RPC baseline is straightforward to implement in any language.
2. **Network-transparent**: Runs over TCP, SSH tunnels, or other network transports. Enables remote development, cloud IDEs, and centralized language services.
3. **Incremental adoption**: Editors and language maintainers can adopt LSP gradually. No requirement to rewrite existing tooling at once.
4. **Vendor neutrality**: Open spec, multiple implementations (Pylance, Rust-Analyzer, Clangd, Gopls). No single company controls the standards evolution—though Microsoft's leadership is significant.
5. **Asynchronous, non-blocking**: Requests include IDs; responses can arrive out of order. Servers can be slow or asynchronous (e.g., spawning subprocesses) without blocking the editor.

## Limitations and Pragmatic Trade-offs

LSP is text-centric: it operates on source files as strings with line/column positions. It doesn't assume an AST, IR, or compiled representation. This makes LSP language-agnostic but also means some language features require workarounds:

- **Structural or semantic refactoring** beyond rename requires custom LSP extensions or composite protocols
- **Build-system integration** (detecting which files are part of a project, managing dependencies) is non-standard; each server implements its own heuristics
- **Performance at scale**: Large projects (100K+ lines) can overwhelm incremental parsing and analysis. Servers must implement caching, debouncing, and prioritization
- **Latency expectations**: LSP is not real-time. A completion request might take 200-500ms on a large codebase. Editor UX must handle this gracefully (show cached results, cancel stale requests)
- **Two-way coupling**: Despite protocol decoupling, editor and server knowledge are still interdependent. If the server crashes, the editor loses all language features

---

## See Also

- `lsp-implementation-patterns.md` — Building efficient language servers
- `ide-debug-adapter-protocol.md` — DAP architecture and debugging protocols
- `tools-editors-ide.md` — Broader editor architecture context