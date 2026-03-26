# Editor & IDE Architecture — Modal Editing, Language Understanding & Protocols

## Conceptual Divide: Modeless vs Modal

### Modeless Editors (Most Modern Defaults)

In modeless editors (VS Code, Sublime, mainstream use):

- Type to insert text
- Ctrl+Z to undo
- Ctrl+X/C/V for clipboard
- Mouse and keyboard both active at all times
- Focus: single, unified mental model for all operations

### Modal Editors (Vi/Vim/Neovim)

Modal editors have distinct operating modes:

- **Normal mode**: Key commands for navigation, deletion, copying, searching
- **Insert mode**: Type text (similar to modeless)
- **Visual mode**: Select ranges of text
- **Command mode**: Colon-based meta-commands

The mental model: mode is your state. `i` enters insert, `Esc` exits to normal.

**Design philosophy behind modal editing:**

1. **Minimize hand motion**: Most operations use hjkl or home row; no heavy modifier key reaching
2. **Composability**: Operators (`d` for delete) compose with motions (`w` for word), creating language-like expressions: `dw` = delete word
3. **Remote-friendly**: Modal editing predates local, fast terminals; designed for slow, laggy connections where keystroke count matters

**Trade-off:** Steep learning curve. New users find mode-switching disorienting. Experts often find modal more efficient for intensive text manipulation.

## Vi/Vim/Neovim: Lineage & Modern Evolution

### Vi (Original, 1976)

Ultra-minimal modal editor, came with Unix. Still on every server.

```bash
vi filename
# i = insert mode
# Esc = return to normal
# :w = write, :q = quit
```

### Vim (Vi Improved, 1991)

Extended Vi with:
- Undo history (Vi: undo only last change)
- Syntax highlighting
- Plugins (vimscript)
- Splits and windows
- Regular expression power

Dominant for decades. Configuration: `.vimrc` (vimscript language).

### Neovim (Fork, 2014–Present)

Vim refactor with modern architecture:

**Key improvements:**
- **Lua scripting** (instead of vimscript): More approachable, faster plugin ecosystem
- **Builtin LSP client**: Language Server integration without plugins
- **Treesitter integration**: Uses treesitter for semantic highlighting (vs regex-based Vim)
- **Better plugin ecosystem**: Plugin managers (`lazy.nvim`, `packer.nvim`) with lazy-loading
- **External UI support**: Neovim separates rendering from logic; GUIs can attach

```lua
-- Neovim config (init.lua)
require('mason').setup()  -- Language server installer
require('lspconfig').pyright.setup{}  -- Attach Pyright (Python LSP)
```

**Vim vs Neovim adoption:**
- Vim: Stable, traditional, vimscript ecosystem
- Neovim: Faster-moving, Lua-based, LSP-first, popular with recent generations

## Emacs: The Lisp Machine

Emacs is not a text editor; it's an interactive Lisp interpreter that happens to edit text.

### Architecture: Elisp at the Core

Emacs starts with a C core (basic text operations, display) and exposes nearly everything through Emacs Lisp (elisp). Most of Emacs is written in elisp, including:

- Keybindings
- Modes (major + minor)
- Packages
- Configuration

```elisp
;; ~/.emacs.d/init.el
(setq default-directory "~/projects")
(global-set-key (kbd "C-x C-r") 'rename-file)
(add-hook 'python-mode-hook
  (lambda () (setq indent-tabs-mode nil)))
```

**Consequence:** Emacs is self-modifying. You can redefine core functions at runtime; the line between "editor" and "customization" blurs.

### The Emacs Mindset

**Modal? No.** Emacs uses modifier keys (Ctrl, Meta/Alt) as prefixes. Ctrl+X Ctrl+S = save. Ctrl+X Ctrl+F = find file. This is sequential command chaining, not modal state.

Emacs terms: 
- **Major modes**: One per buffer. Defines syntax highlighting, keybindings, behavior (e.g., `python-mode`, `org-mode`)
- **Minor modes**: Toggleable features overlaid on major mode (e.g., `line-number-mode`, `flyspell-mode`)

### Org Mode: Outlining & Literate Programming

One of Emacs's most famous features: **Org mode** is a markup format for hierarchical organization, planning, and literate programming.

```org
* Project Alpha
** TODO Implement API endpoint
   Deadline: 2026-04-01
** DONE Write tests
   CLOSED: [2026-03-25 Wed 10:30]
   
* Literate Code
#+BEGIN_SRC python
# This code block can be evaluated in-place
def greet(name):
    return f"Hello, {name}!"
#+END_SRC
```

Org mode enables:
- Project planners
- Notebook-like documentation (like Jupyter, but in Emacs)
- Bibliography management
- Export to HTML/PDF/LaTeX

### Magit: Git Porcelain

Magit provides a Git UI inside Emacs. Paradigm: press `?` to see all commands. No need to memorize Git CLI.

```
Stage hunks: s
Commit: c
Push: P
Interactive rebase: r
```

This Git inside your editor workflow appeals strongly to Emacs users.

## VS Code: Electron-Based, Extension Host

VS Code is not a native application; it's a web app running in Chromium (Electron).

### Architecture

```
Electron Shell (Chromium renderer + Node.js)
├── Main Process (Node.js)
├── Renderer Process (Web app)
│   └── Monaco Editor (Microsoft's in-browser code editor)
├── Language Client (connects to LSP servers)
└── Extension Host (isolated processes for extensions)
```

**Surprising fact:** The editor itself (Monaco) is a web component. VS Code re-renders on every keystroke, but Monaco is optimized for this (virtual scrolling, batched updates). This is why VS Code is fast despite Electron.

### Extension Host Isolation

Extensions run in isolated processes, not in the main renderer. Benefits:

- Crash: Extension crash doesn't kill VS Code
- Security: Extensions can't read arbitrary files (requires explicit permission)
- Performance: Main UI thread stays responsive

Downside: Extension communication with editor goes through message passing (slightly slower than direct API calls).

### LSP Client

VS Code ships with a built-in Language Client that connects to LSP servers:

```json
{
  "languageServerHosts": {
    "python": { "command": "pylance", "languages": ["python"] },
    "rust": { "command": "rust-analyzer", "languages": ["rust"] }
  }
}
```

The LSP protocol (`textDocument/completion`, `textDocument/hover`, etc.) is the abstraction layer. Theoretically, any editor + any LSP server = universal IDE experience.

### Debugger Integration: DAP

VS Code uses the Debug Adapter Protocol (DAP) to communicate with debuggers.

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "python",
      "request": "launch",
      "program": "${file}"
    }
  ]
}
```

DAP parallels LSP; the abstraction enables any debugger to work with any editor that supports DAP.

## JetBrains: PSI & Deep Code Understanding

JetBrains IDEs (IntelliJ, PyCharm, GoLand, etc.) are language-specific optimized experiences.

### PSI: Program Structure Interface

The core abstraction: **PSI** is an in-memory representation of code as a tree.

```
File
├── Import "java.util.List"
├── Class "MyClass"
│   ├── Field "items: List"
│   ├── Method "add()"
│   │   ├── Parameter "item"
│   │   └── Return statement
```

This tree is the source of truth for:
- Syntax highlighting
- Go-to-definition
- Find references
- Refactoring

JetBrains pre-computes the PSI (similar to how a compiler builds an AST). This enables sophisticated IDE features:

- **Find usages in an instant**: PSI already indexed; no need to re-parse the entire project
- **Refactoring safety**: Rename a method and the IDE knows exactly which references to update
- **Inspections**: Rules run over the PSI to detect bugs or code smells

### Inspections & Quick Fixes

Inspections are analyses that run on the PSI tree:

```
Warning: Method 'foo' is private but never referenced
Quick fix: Remove the method
```

Thousands of built-in inspections. Custom inspections via plugins.

### Build System Integration

JetBrains IDEs understand project structure by parsing build files:

- Maven (`pom.xml`): Parse dependencies, source directories, plugins
- Gradle (`build.gradle`): Evaluate Groovy/Kotlin to determine what's buildable
- Go modules (`go.mod`): Index vendored dependencies

This tight integration with build systems avoids the IDE falling out of sync with the actual build.

## Language Server Protocol (LSP)

A standardized protocol for communication between editors and language servers, eliminating the N×M matrix of editor–language pairs.

### Abstract Model

```
Editor (LSP client)
  ↓ textDocument/didOpen (opened a file)
  ↓ textDocument/completion (request completions at line:col)
  ← completionItem[] (responses)
LSP Server (language-specific daemon)
  ├── Parser/Analyzer (understands code)
  ├── Semantic database
```

### Key Methods

- `initialize`: Handshake; server declares capabilities
- `textDocument/completion`: Auto-complete suggestions
- `textDocument/hover`: Tooltip information
- `textDocument/definition`: Go-to-definition
- `workspace/symbol`: Find symbols across project
- `textDocument/formatting`: Format code
- `textDocument/diagnostics`: Lint/error reporting

### Limitation: Semantic Understanding

LSP provides building blocks (AST, symbol table, diagnostics) but not full semantic unification:

- Complex refactorings still differ by impl (JetBrains' refactorings are more aggressive)
- Performance varies (Pylance vs Pyright vs native Python LSP servers)
- Some tasks (e.g., cross-project find references) require external indexing beyond LSP

## Tree-Sitter: Incremental Parsing

**Tree-sitter** is a parser generator that produces incremental, error-tolerant parses suitable for editors.

**Key properties:**

1. **Incremental**: If you edit one line, only rescan nearby nodes, not the entire file
2. **Error-tolerant**: Partial parse succeeds even if syntax is incomplete (you're typing)
3. **Fast**: Written in C, exposed to higher-level languages via bindings

### Application: Syntax Highlighting & Structural Queries

Rather than regex-based coloring (Vim's traditional approach), tree-sitter parses to an AST and applies rules:

```bash
# Query: "Find all function definitions"
(function_definition name: (identifier) @func-name)
```

Queries are applied to the tree; matches are highlighted.

### Neovim & Tree-Sitter

Neovim integrates tree-sitter for:

- **Syntax highlighting** (more accurate than regex)
- **text-objects** (select a function or class by keystroke)
- **Indentation** (compute indent level from tree structure, not heuristics)

## Debugger Architecture: Debug Adapter Protocol (DAP)

DAP abstracts debugger operations (breakpoints, stepping, variable inspection) into a standard protocol, enabling any IDE + any debugger pair.

### Key Commands

- `launch` / `attach`: Start or attach to a process
- `setBreakpoints`: Set breakpoint at line
- `continue` / `stepIn` / `stepOut`: Execution control
- `variables` / `scopes`: Inspect local variables
- `evaluate`: Evaluate expression in debugged context

### Implementation

DAP server (debugger adapter) translates between:

- **Editor side**: DAP protocol (JSON-RPC over stdio or network)
- **Debugger side**: Native debugger protocol (GDB/LLDB wire format, etc.)

Example: VS Code + Python debugging

```
VS Code (DAP client)
  → debugpy adapter (DAP server, Python-specific)
    → Python runtime (raw GDB-like commands)
```

The adapter bridges the gap; end users see unified experience.