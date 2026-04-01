# VS Code Extension Development — API, Lifecycle & Patterns

## Overview

A VS Code extension is a Node.js module that runs in the extension host process and communicates with the editor via the `vscode` API. Extensions range from simple (auto-format on save) to complex (full language servers, debuggers, terminal emulators). This note covers the API surface, extension lifecycle, and common patterns.

## Extension Structure

A minimal extension:

```
my-extension/
├── package.json          # Manifest
├── src/
│   └── extension.ts      # Entry point
├── tsconfig.json         # TypeScript config
└── vscode.d.ts           # API types (bundled by VS Code)
```

### package.json Manifest

The manifest declares what the extension does:

```json
{
  "name": "my-extension",
  "displayName": "My Extension",
  "version": "0.0.1",
  "description": "Does cool things",
  "main": "dist/extension.js",
  "engines": {
    "vscode": "^1.80.0"
  },
  "activationEvents": [
    "onLanguage:python",
    "onCommand:myext.sayHello",
    "onView:myext.explorerView"
  ],
  "contributes": {
    "commands": [
      {
        "command": "myext.sayHello",
        "title": "Say Hello",
        "category": "My Extension"
      }
    ],
    "languages": [
      {
        "id": "mylang",
        "extensions": [".mylang"],
        "configuration": "./language-configuration.json"
      }
    ],
    "grammars": [
      {
        "language": "mylang",
        "scopeName": "source.mylang",
        "path": "./syntaxes/mylang.tmLanguage.json"
      }
    ]
  }
}
```

Key fields:

- **activationEvents**: Which situations trigger the extension to load. Lazy loading keeps VSCode lean.
- **contributes**: What the extension adds to VSCode (commands, views, language support, keybindings, etc.)
- **engines**: Minimum VSCode version required

## Activation Events

Activation events are triggers that cause VSCode to load and activate an extension. They're lazy-loaded: the extension doesn't run until it's needed.

### Common Activation Events

**onLanguage: LANGUAGE** — Load when a file of a specific language is opened:

```json
{"activationEvents": ["onLanguage:python", "onLanguage:javascript"]}
```

This is used by LSP clients: when the user opens a Python file, the Python LSP extension activates and starts the language server.

**onCommand: COMMAND** — Load when a specific command is executed:

```json
{"activationEvents": ["onCommand:myext.doSomething"]}
```

If the command palette doesn't yet show your command, the first execution triggers activation.

**onView: VIEW_ID** — Load when a view (sidebar panel) is requested:

```json
{"activationEvents": ["onView:myext.customView"]}
```

**onViewsWelcome** — Load when showing a welcome message in an empty view

**onStartupFinished** — Load after VSCode finishes loading (slightly slower startup, but guaranteed VSCode is ready)

**workspaceContains: GLOB** — Load if the workspace folder matches a glob pattern:

```json
{"activationEvents": ["workspaceContains:Makefile", "workspaceContains:**/*.py"]}
```

Useful for project-specific tools (e.g., a C++ extension loads if any .cpp files are found).

### Activation Penalty

Each activation event and each extension adds delay to VSCode startup. Extensions should minimize their footprint:

- Use lazy activation events, not `onStartupFinished` when possible
- Keep the `activate` function lightweight (quick initialization, defer expensive work)
- Avoid polling or timers that run unconditionally

## The `vscode` API Module

Inside an extension's Node.js code, the global `vscode` module provides access to editor functionality. It's not a regular npm package; it's provided by the extension host at runtime.

### Common API Namespaces

**vscode.window**: Interact with the active window and editor

```typescript
// Show a message
vscode.window.showInformationMessage("Hello!");

// Get the active editor
const editor = vscode.window.activeTextEditor;
if (editor) {
  const text = editor.document.getText();
  const selection = editor.selection;
}

// Show a quick pick (command palette-like menu)
const choice = await vscode.window.showQuickPick(["Option A", "Option B"]);
```

**vscode.commands**: Register and execute commands

```typescript
vscode.commands.registerCommand("myext.sayHello", () => {
  vscode.window.showInformationMessage("Hello from my extension!");
});

// Execute a command
await vscode.commands.executeCommand("editor.action.formatDocument");
```

**vscode.workspace**: Access workspace folders and files

```typescript
// List all open text documents
const docs = vscode.workspace.textDocuments;

// Watch files
const fileWatcher = vscode.workspace.createFileSystemWatcher("**/*.py");
fileWatcher.onDidChange(uri => {
  console.log(`File changed: ${uri.fsPath}`);
});

// Read a file
const uri = vscode.Uri.file("/path/to/file");
const bytes = await vscode.workspace.fs.readFile(uri);
```

**vscode.languages**: Register language features (completion, hover, definition, etc.)

```typescript
vscode.languages.registerCompletionItemProvider("python", {
  provideCompletionItems(document, position) {
    return [
      new vscode.CompletionItem("myKeyword", vscode.CompletionItemKind.Keyword)
    ];
  }
});
```

**vscode.debug**: Access the debugger

```typescript
vscode.debug.registerDebugAdapterDescriptorFactory("mylang", {
  createDebugAdapterDescriptor(session) {
    return new vscode.DebugAdapterExecutable("node", ["./debugAdapter.js"]);
  }
});
```

## Extension Lifecycle

### 1. Registration Phase (package.json)

VSCode parses all `package.json` manifests and builds a registry of available extensions without activating them.

### 2. Activation Trigger

When an activation event fires (e.g., file opened, command called), VSCode:

1. Loads the extension's main module
2. Calls the `activate(extensionContext)` function
3. The extension initializes, registers commands, language features, etc.

```typescript
export function activate(context: vscode.ExtensionContext) {
  console.log("Extension activated!");

  // Register a command
  const disposable = vscode.commands.registerCommand(
    "myext.sayHello",
    () => vscode.window.showInformationMessage("Hello!")
  );

  // Attach to the context to ensure cleanup
  context.subscriptions.push(disposable);
}
```

### 3. Deactivation (Cleanup)

When an extension is unloaded or VSCode exits, the `deactivate()` function is called:

```typescript
export function deactivate() {
  // Cleanup: close servers, free resources, etc.
  console.log("Extension deactivated!");
}
```

The `context.subscriptions` array is automatically disposed; all registered disposables are cleaned up.

### 4. Storage and Persistence

The `ExtensionContext` provides two storage APIs:

```typescript
// Global extension storage (shared across all workspaces)
context.globalState.update("key", value);
const val = context.globalState.get("key");

// Workspace-specific storage (per folder)
context.workspaceState.update("key", value);
```

Used for storing user preferences, cache, or state.

## Common Extension Patterns

### Pattern 1: Language Support (Syntax + LSP)

An extension that adds language support typically:

1. **Registers a language**: Declares the language ID (e.g., "mylang")
2. **Provides grammar**: TextMate or tree-sitter grammar for syntax highlighting
3. **Spawns LSP server**: Starts an external LSP process and connects via LSP client

```typescript
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

let client: LanguageClient;

export async function activate(context: vscode.ExtensionContext) {
  client = new LanguageClient(
    "mylang",
    "My Language Server",
    { command: "node", args: ["./server.js"] },
    { documentSelector: ["mylang"] }
  );

  await client.start();
  context.subscriptions.push(client);
}

export function deactivate() {
  return client?.stop();
}
```

### Pattern 2: Custom Command with Code Action

A command that provides a code action (fix or refactor):

```typescript
vscode.languages.registerCodeActionsProvider("python", {
  provideCodeActions(document, range, context) {
    const actions: vscode.CodeAction[] = [];
    if (context.diagnostics.some(d => d.code === "E501")) {
      const action = new vscode.CodeAction(
        "Wrap line",
        vscode.CodeActionKind.QuickFix
      );
      action.command = { command: "myext.wrapLine", arguments: [range] };
      actions.push(action);
    }
    return actions;
  }
});
```

### Pattern 3: TreeView (Sidebar)

A custom view in the sidebar:

```typescript
class MyTreeDataProvider implements vscode.TreeDataProvider<MyTreeItem> {
  getTreeItem(element: MyTreeItem) {
    return element;
  }

  getChildren(element?: MyTreeItem): MyTreeItem[] {
    if (!element) {
      return [new MyTreeItem("Root Item", vscode.TreeItemCollapsibleState.Expanded)];
    }
    return [];
  }
}

export function activate(context: vscode.ExtensionContext) {
  const provider = new MyTreeDataProvider();
  vscode.window.registerTreeDataProvider("myext.treeView", provider);

  vscode.commands.registerCommand("myext.treeView.click", (item: MyTreeItem) => {
    vscode.window.showInformationMessage(`Clicked: ${item.label}`);
  });
}

class MyTreeItem extends vscode.TreeItem {
  constructor(label: string, collapsibleState: vscode.TreeItemCollapsibleState) {
    super(label, collapsibleState);
  }
}
```

### Pattern 4: Webview Panel

A custom UI (HTML/CSS/JavaScript):

```typescript
vscode.commands.registerCommand("myext.showPanel", () => {
  const panel = vscode.window.createWebviewPanel(
    "myPanel",
    "My Custom Panel",
    vscode.ViewColumn.One,
    { enableScripts: true }
  );

  panel.webview.html = `
    <html>
      <body>
        <h1>Hello from Webview</h1>
        <script>
          const vscode = acquireVsCodeApi();
          vscode.postMessage({ command: "ready" });
        </script>
      </body>
    </html>
  `;

  panel.webview.onDidReceiveMessage(message => {
    if (message.command === "ready") {
      vscode.window.showInformationMessage("Webview ready!");
    }
  });
});
```

## Extension API Surface (High-Level Reference)

| Category | Examples |
|----------|----------|
| **Window** | `activeTextEditor`, `showQuickPick`, `showInputBox`, `showErrorMessage` |
| **Workspace** | `workspaceFolders`, `openTextDocument`, `findFiles`, `createFileSystemWatcher` |
| **Languages** | `registerCompletionItemProvider`, `registerHoverProvider`, `registerDefinitionProvider`, `registerCodeActionsProvider` |
| **Commands** | `registerCommand`, `executeCommand` |
| **Themes** | `activeColorTheme`, `onDidChangeActiveColorTheme` |
| **Status Bar** | `createStatusBarItem`, `setStatusBarMessage` |
| **Debug** | `registerDebugAdapterDescriptorFactory`, `startDebugging` |
| **Terminal** | `createTerminal`, `activeTerminal`, `onDidChangeActiveTerminal` |

The full API is defined in `vscode.d.ts` (shipped with VSCode). Type definitions ensure strong IDE support in extension code.

## Extension Packaging and Distribution

### Local Development

Use the **VS Code Extension Generator**:

```bash
npm install -g yo generator-code
yo code
```

Run locally with the **Debug Extension** launch configuration (built into the VS Code extension template).

### Publishing to Marketplace

1. Create a **PAT (Personal Access Token)** on dev.azure.com
2. Install **vsce** (VSCode Extension CLI):
   ```bash
   npm install -g vsce
   ```
3. Package and publish:
   ```bash
   vsce package           # Creates .vsix file
   vsce publish           # Publishes to marketplace
   ```

Extensions require:

- Unique `name` across the marketplace
- `LICENSE` file
- `README.md` with description and screenshots
- `CHANGELOG.md` (recommended)

Marketplace extensions are auto-updated by VSCode, ensuring users stay current.

## Anti-Patterns and Common Mistakes

1. **Synchronous file I/O**: Blocks the extension host
   - Fix: Use `vscode.workspace.fs.*` (async) instead of `require("fs")`

2. **Activation in `onStartupFinished`**: Delays VSCode startup
   - Fix: Use more specific activation events

3. **Polling without debouncing**: Floods IPC with frequent requests
   - Fix: Debounce or use events (`onDidChangeTextDocument`)

4. **Memory leaks**: Storing references to editors/documents indefinitely
   - Fix: Clean up in the `deactivate()` function

5. **Unhandled promise rejections**: Silent failures
   - Fix: Always add `.catch()` handlers or use try/catch in async functions

---

## See Also

- `ide-vscode-architecture.md` — How extensions fit into VS Code's architecture
- `lsp-protocol.md` — Building and integrating LSP servers
- `api-design.md` — API design principles (relevant for extension APIs)