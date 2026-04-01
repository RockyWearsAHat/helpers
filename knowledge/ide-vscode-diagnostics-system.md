# VS Code Diagnostics System — Architecture & Language Server Integration

The **diagnostics system** is how VS Code surfaces code problems (errors, warnings, suggestions) from language servers and linters. It's the technical foundation behind the Problems panel and the agent `#read/problems` tool. Understanding diagnostics means understanding exactly what information is available to agents trying to self-correct generated code.

## Core Concept: DiagnosticCollection

At the API level, diagnostics are organized into **collections** — containerized groups of diagnostics scoped to a resource (file) and diagnostic provider. Think of a collection as a namespace: TypeScript owns its own collection, ESLint owns another, Pylance another.

Extensions create collections via `createDiagnosticCollection(name)` and publish diagnostics into them:

```typescript
const collection = languages.createDiagnosticCollection("my-linter");
collection.set(Uri.file("/path/to/file.ts"), [
  new Diagnostic(range, message, severity)
]);
```

Once set, VS Code immediately displays the diagnostics in:
- The Problems panel (filtered/searchable)
- Inline code (squiggly underlines, decorations)
- Error minimap in the scrollbar
- Agent tools that access `#read/problems`

Multiple collections can exist simultaneously; VS Code merges diagnostics from all providers.

## Diagnostic Structure

Each `Diagnostic` object contains:

- **Range** — Start and end position (line, column) in the file
- **Message** — Human-readable error/warning text
- **Severity** — One of: Error, Warning, Information, Hint
- **Source** — Provider name (e.g., "eslint", "typescript", "pylance")
- **Code** — Machine code for the error (e.g., "TS2322"); used for quick fixes and rule lookup
- **RelatedInformation** — Array of related locations (e.g., "other usage of this symbol" or "declaration at line X")
- **Tags** — Array of semantic metadata: `Unnecessary` (code that can be removed) or `Deprecated` (using outdated APIs)
- **CodeActions** — Attached quick fixes: automated corrections the IDE can apply

The **RelatedInformation** array is powerful for agents: it provides context about *why* a diagnostic occurred. For example, a TypeScript "unused variable" diagnostic includes a link to where the variable was declared.

## Severity Levels

- **Error** — Prevents compilation or valid interpretation. Agents must fix these to proceed.
- **Warning** — Code compiles but may cause runtime issues or violates conventions. Agents should fix if the user cares.
- **Information** — Neutral advice (improvements, style suggestions). Lower priority than errors/warnings.
- **Hint** — Subtle suggestions (e.g., "this variable could be const"). Lowest priority.

Agents can use severity to triage: fix all Errors first, then Warnings if capacity allows.

## Diagnostic Tags

Tags add semantic information:

- **Unnecessary** — Code is redundant and can be removed. Example: unused imports, dead code branches.
- **Deprecated** — Using old/deprecated APIs. Should migrate to alternatives.

## Code Actions Attached to Diagnostics

Language servers can attach `CodeAction` objects to diagnostics: automated fixes the IDE can apply. Examples:

- TypeScript diagnostic "unused variable" → CodeAction "remove statement"
- ESLint diagnostic "missing semicolon" → CodeAction "add semicolon"
- Pylance diagnostic "unused import" → CodeAction "remove import"

Agents don't directly invoke code actions; instead, they see the diagnostic and can generate similar fixes themselves. Code actions are primarily for IDE UI (Quick Fix menus).

## Language Server Protocol (LSP)

VS Code doesn't generate diagnostics itself; language servers do. The Language Server Protocol defines how diagnostics flow from server → client.

### Request/Response Cycle

1. **Client (VS Code) sends `textDocument/didOpen` notification** — "I opened a file"
2. **Server analyzes the file** — Runs parser, type checker, linter
3. **Server sends `textDocument/publishDiagnostics` notification** — "Here are the diagnostics I found"
4. **VS Code disseminates** — Updates Problems panel, inline decorations, agent tools

This is **notification-based** (one-way from server to client), not request-response. The server doesn't wait for acknowledgment.

### Update Triggers

Diagnostics typically update on:
- **File save** — Full re-analysis after user presses Ctrl+S
- **Keystroke** (optional) — Incremental updates as user types (more expensive; many servers debounce this)
- **Configuration change** — User changes settings
- **Dependency change** — Another file in the project changed (e.g., type definition)

Different servers have different strategies. TypeScript updates incrementally on keystroke (with debouncing). ESLint traditionally updates on save. Pylance (Microsoft's Python LSP) updates on keystroke with aggressive caching.

## DiagnosticCollection API

Extensions working with diagnostics:

### Setting diagnostics
```typescript
collection.set(uri, diagnostics);  // Replace all diagnostics for this file
collection.set(uri, undefined);     // Clear all diagnostics for this file
```

### Deleting a collection
```typescript
collection.dispose();  // Collection is garbage-collected
```

### Listening for changes
```typescript
onDidChange() // Event fires when diagnostics update
```

## Workspace-Wide vs. File-Scoped Diagnostics

Most diagnostics are **file-scoped**: a TypeScript error at line 42 of `src/app.ts` is tied to that file.

Some diagnostics are **workspace-wide**: a dependency resolution error, a build configuration error. These are published against a "null" URI or a workspace root URI. VS Code displays them in the Problems panel but doesn't tie them to specific files.

## Diagnostic Updates & Debouncing

Language servers must balance responsiveness (user sees errors quickly) with performance (analyzing on every keystroke is expensive).

- **No debouncing:** Server re-analyzes on every keystroke → responsive but CPU-intensive
- **Aggressive debouncing (e.g., 1 second):** User types, hits the delay, then diagnostics update → faster performance but delayed feedback

TypeScript debounces at ~200ms. VSCode itself configures diagnostic update delays via `[language].validate.delay` setting.

**Incremental diagnostics:** Smart servers don't re-analyze the entire file; they re-analyze only affected scopes. Example: you edit a function body; the server re-analyzes that function and its dependents but not unrelated parts. This is why language server complexity is high.

## Diagnostic Caching

When the user opens a file that was previously analyzed, the server may return cached diagnostics rather than re-analyzing if the file hasn't changed. Caching prevents thundering herd problems when reopening many files.

Extensions can also cache diagnostics locally to speed up diagnostics for unchanged content.

## JSON Schema Validation via Diagnostics

VS Code publishes diagnostics for configuration files (`.vscode/settings.json`, `tasks.json`, `launch.json`) via built-in JSON schema validators. These diagnostics appear in the Problems panel and inline, enabling agents to catch configuration errors just like code errors.

## Filtering & Querying Diagnostics

Via API, extensions can:
- Query all diagnostics in a range: `languages.getDiagnostics(uri)`
- Filter by severity, source, or code
- Subscribe to diagnostic changes: `languages.onDidChangeDiagnostics`

Agents use `#read/problems` which returns a curated list (not raw API; user-facing, filtered by configuration).

## Tool Integration

The agent `#read/problems` tool returns:
- All diagnostics currently in the Problems panel
- Filtered by severity (user/agent can ignore hints or information)
- With file, line, message, severity, and related information
- Structured as diagnostic objects (not just text)

This enables agents to:
1. Generate code
2. Invoke `#read/problems`
3. Parse diagnostics
4. Understand failures precisely
5. Refine code and retry

The **self-correcting loop** is why agents are effective at code generation: immediate, structured feedback enables tight iteration.

## Performance Considerations

Diagnostics create visual feedback overhead:
- Decorations (squiggly lines, icons) require rendering updates
- Problems panel updates are expensive at scale (large number of diagnostics)
- Diagnostic collection creates memory overhead

This is why:
- Rules can be disabled (turn off noisy linters)
- Severity levels exist (ignore non-critical diagnostics visually)
- Servers debounce and cache

For agents, performance is less critical (they don't render visually), but the logic remains: too many diagnostics = slow analysis loop.

## Custom Diagnostics

Extensions can contribute custom diagnostics via `createDiagnosticCollection`. Examples:
- Security linter (finds potential vulnerabilities)
- Performance analyzer (finds inefficient patterns)
- Accessibility checker (finds WCAG violations)
- Domain-specific linter (project-specific conventions)

Each extension owns its collection. VS Code merges all source collections into the unified Problems panel.

## See Also

- [ide-vscode-agent-tools.md](ide-vscode-agent-tools.md) — How agents use `#read/problems`
- [tools-editors-ide.md](tools-editors-ide.md) — LSP architecture overview
- [api-error-handling.md](api-error-handling.md) — Error representation patterns