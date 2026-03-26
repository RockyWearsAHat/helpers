# Language Server Implementation Patterns — Building Efficient Language Servers

## Overview

Building an LSP server requires decisions about parsing strategy, document state management, incremental analysis, and performance optimization. There's no single "correct" architecture; different languages and use cases have different constraints. A Python LSP server optimizing for a 5-year-old codebase has different needs than one for a compiler language like Rust.

This note covers common patterns, their trade-offs, and frequent pitfalls that implementers encounter.

## Document Synchronization Strategies

### Full Synchronization

The simplest strategy: on every `textDocument/didChange`, the server receives the entire file content. The server replaces its in-memory copy completely.

**Pros:**
- No ambiguity; no risk of client-server state divergence
- Implementation is straightforward: no need to track edits or compute deltas
- Good for small files or low-frequency changes

**Cons:**
- Bandwidth overhead for large files (kilobytes or more)
- The server can't perform true incremental analysis; must re-analyze the entire file on every keystroke

**Used by:** Simple servers, debugging-focused implementations (where state correctness > performance), or protocols where bandwidth is cheap (local stdio).

### Incremental Synchronization

The client sends only the changed text ranges:

```json
{
  "method": "textDocument/didChange",
  "params": {
    "textDocument": {"uri": "file:///project/main.py", "version": 42},
    "contentChanges": [
      {
        "range": {
          "start": {"line": 5, "character": 0},
          "end": {"line": 5, "character": 15}
        },
        "text": "def new_name():"
      }
    ]
  }
}
```

The server applies the delta to its cached copy, then re-analyzes.

**Pros:**
- Lower bandwidth
- Enables true incremental parsing (incrementally update the AST instead of re-parsing from scratch)
- Better responsiveness for interactive features on large files

**Cons:**
- More complex synchronization logic; risk of client-server divergence if edits are lost or out-of-order
- Version numbers must match; if versions drift, the server gets corrupt state
- Requires the server to maintain a file buffer in memory

**Used by:** Performance-sensitive servers (Rust-Analyzer, TypeScript, Pyright). The tradeoff is worth it for servers supporting large projects.

### Pragmatic Hybrid: Document Versioning

Most production servers use incremental sync with version tracking. The client includes a `version` field in each change:

```json
{
  "textDocument": {"uri": "file:///...", "version": 42},
  "contentChanges": [...]
}
```

If the server receives a change with an unexpected version, it can either:
1. **Request full re-sync**: Send a request to the client asking for the full document content
2. **Drop the change**: Assume it's stale and ignore it
3. **Log and continue**: Accept incremental changes but log version mismatches for debugging

Rust-Analyzer and other mature servers use this to recover from transient corruption.

## Parsing and Analysis Architecture

### Incremental Parsing

When the user edits a single line in a 10,000-line file, re-parsing the entire file is wasteful. **Incremental parsers** update only the affected syntax subtrees.

Parsers like **Tree-Sitter** and **Lezer** are designed for this:

- **Tree-Sitter** (parser generator) produces parsers that re-parse only changed regions using memoization and a rolling buffer
- **Lezer** (JavaScript parser library) uses recovery-based incremental parsing

**Trade-off:** Incremental parsing is complex to implement. Languages with simple, regular syntax (Go) benefit from it. Languages with nested scopes and complex precedence (C++) need it but also need sophisticated cache invalidation.

### Query-Based Analysis

Some servers decouple parsing from analysis using a **query engine**:

1. **Parse phase**: Maintain a current syntax tree (incremental or full)
2. **Query phase**: On demand (hover, completion, references), run specific queries over the tree

The TypeScript language server uses this pattern: it maintains an incremental parser, then answers queries by traversing the tree and type-checking on demand.

**Pros:**
- Clear separation: parser updates aggressively, analysis is lazy
- Scales to large codebases with selective type-checking (only analyze what's needed for the current request)

**Cons:**
- Complex bookkeeping to cache query results and invalidate when the tree changes

### Single-Pass vs. Multi-Pass Analysis

Early LSP servers (and many scripting language servers) use a single pass: parse the file, extract symbols, compute type information, all in one walk.

More sophisticated servers (Rust-Analyzer) use multi-pass:

1. **Syntax pass**: Parse to AST
2. **Name resolution pass**: Build symbol table, resolve references
3. **Type inference pass**: Infer types, check consistency
4. **Semantic pass**: Compute semantic meaning for code actions

Each pass can be incremental and cached. When a single line changes, only the affected passes re-run.

## Cancellation and Prioritization

A key LSP feature: the client can cancel long-running requests. If the user hits a key before a completion request finishes, the client sends a `$/cancelRequest` notification with the request ID:

```json
{
  "method": "$/cancelRequest",
  "params": {"id": 42}
}
```

The server should stop computing and clean up. **Important:** The server must check for cancellation frequently, not only at the end of analysis. A server that computes for 5 seconds then checks for cancellation is unresponsive.

Many servers use:
- **Cooperative cancellation**: Check a `cancellation_token` at loop boundaries in the analysis
- **Thread-based timeout**: Spawn the request in a thread, interrupt after a timeout
- **Prioritization queue**: Put completion requests ahead of diagnostics, so user-facing features respond faster

## Diagnostic Publishing

Servers typically publish diagnostics in a background thread or async task. The naive approach: on every file change, re-analyze and publish new diagnostics. This works for small projects but creates problems at scale:

1. **Latency**: If diagnostics take 2 seconds to compute on a 100-line file, users see a 2-second delay after typing
2. **Thrashing**: If the user types 5 characters per second, 5 diagnostic runs per second are queued; the server can't catch up

Production servers use **debouncing**: delay diagnostic computation until no edits have arrived for N milliseconds (typically 100-500ms). This lets the user type without triggering analysis on every keystroke while still providing timely feedback.

Another pattern: **prioritized analysis**. Compute diagnostics for the currently-edited file first, then other open files, then project-wide checks. The user always sees feedback for their current context quickly.

## Completion Providers

Two strategies:

### Eager Computation

On every `textDocument/completion` request, compute the full completion list from scratch:

```python
def completion(self, uri, position):
    symbols = self.analyze(uri)
    return [
        CompletionItem(name, detail)
        for name in symbols if name.startswith(prefix)
    ]
```

**Pros:** Simple, guarantees correctness
**Cons:** Slow for large symbol tables (a codebase with 100K symbols must filter all 100K)

### Two-Phase Completion

Many servers (TypeScript) implement `textDocument/completion` and `completionItem/resolve`:

1. **Completion request**: Return items quickly (just names, no details)
2. **Resolve request**: Client sends back the selected item; server computes expensive details (docs, type info)

```json
{
  "method": "textDocument/completion",
  "result": [{"label": "function_name"}, {"label": "another"}, ...]
}
```

Then, if the user selects an item:

```json
{
  "method": "completionItem/resolve",
  "params": {"label": "function_name"},
  "result": {"label": "function_name", "detail": "def function_name(x: int) -> str", "documentation": "..."}
}
```

**Pros:** Responsive first render (showing 100+ items instantly), efficient (only resolve the selected item)
**Cons:** Requires coordination; user might see incomplete info if resolve is slow

## Symbol Searching and Workspace Symbols

The `workspace/symbol` request asks the server to find symbols matching a query across all files:

```json
{
  "method": "workspace/symbol",
  "params": {"query": "my_function"}
}
```

This is expensive: the server must search potentially thousands of files. Common optimizations:

1. **Symbol caching**: Pre-compute a symbol table on startup and keep it current as files change
2. **Parallel search**: Search multiple files in parallel
3. **Fuzzy filtering**: Use substring or edit-distance matching to cap results (return top 50, not all 10,000)

Rust-Analyzer caches the entire project's symbol table and incrementally updates it. Most servers build an index in the background.

## Semantic Tokens and Syntax Highlighting

A newer LSP feature: `textDocument/semanticTokens`. Instead of relying on the editor's built-in regex-based syntax highlighting, the server can provide semantic coloring:

```json
{
  "method": "textDocument/semanticTokens/full",
  "result": {
    "resultId": "v1",
    "data": [0, 0, 3, 0, 0, 5, 3, 4, 1, 0, 12, 10, 1, 0, ...]
  }
}
```

The data array encodes position and token type deltas. The editor uses this to color code with context-aware highlighting (e.g., unused variables in gray, constants in blue).

Trade-off: semantic tokens require the server to have deep semantic understanding. A simple server might skip this; a sophisticated one (Rust-Analyzer) uses it to provide precise, context-aware highlighting.

## Common Pitfalls

### State Divergence

The most common bug: client and server file content drifts. Causes:

- Version mismatch on incremental syncs
- Missing or duplicate edits
- Files opened in multiple editors without coordination

**Mitigation:** Always validate received versions. If version is unexpected, ask the client to resend the full file.

### Blocking on I/O

LSP servers often call out to external tools (compilers, linters, type checkers). Blocking on these calls stalls all client requests. Example: a Python LSP server that calls `mypy` synchronously blocks hover + completion while `mypy` runs.

**Mitigation:** Run external tools asynchronously. Use thread pools or async I/O. Cancel or timeout external operations when the client cancels the request.

### Memory Leaks on Long Sessions

Servers run continuously, processing changes for hours. If the server doesn't clean up closed documents or cache entries, memory grows without bound.

**Mitigation:** Implement cache eviction (LRU), explicitly free closed documents, profile memory usage regularly.

### Assuming One Client

LSP doesn't mandate one client. In theory, multiple editors could attach to the same server process. Few servers handle this well. Shared state (file buffers, diagnostics) can conflict.

**Pragmatic stance:** Most servers assume one client. If multiple attach, the behavior is undefined (first wins, last wins, or chaos).

---

## See Also

- `lsp-protocol.md` — LSP specification and core features
- `compiler-design-backend.md` — Incremental compilation concepts
- `algorithm-caching.md` — Cache invalidation strategies