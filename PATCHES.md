# VS Code Workbench Patches

This project applies two minimal patches to VS Code's workbench bundle to enable branch-per-chat navigation. Both are candidates for upstreaming as VS Code PRs.

## Patch Management

```bash
# Check status of all patches
node scripts/patch-vscode-apply-all.js --check

# Apply all patches (creates pristine backup on first run)
node scripts/patch-vscode-apply-all.js

# Revert to pristine bundle
node scripts/patch-vscode-apply-all.js --revert
```

**Important**: After applying or reverting patches, you must **quit and restart VS Code** (Cmd+Q → reopen). `Reload Window` is NOT sufficient — Electron caches the workbench bundle in the main process.

The extension automatically checks patch status on activation and offers to apply missing patches.

## Patches

### 1. Chat Bridge (`patch-vscode-chat-bridge.js`)

**Type**: Behavioral gap / missing API surface
**Upstream candidate**: Expose `onDidChangeFocusedSession` to extensions

**Problem**: VS Code's extension API has no event for "focused chat session changed." The `onDidChangeTabs` API fires when switching between panels (e.g. editor → Chat), but NOT when switching conversations within the Chat panel. Extensions that need session-level focus tracking have no API surface.

**What it changes** (2 injection points):

1. `setLastFocusedWidget()` — After firing `_onDidChangeFocusedSession`, writes the focused widget's `sessionResource` URI to `~/.cache/gsh/active-chat-session.json`.

2. ViewModel change handler — After firing `_onDidChangeFocusedSession` on conversation switch, writes the new session's resource URI (or `null` when navigating to sessions list).

**File written**: `~/.cache/gsh/active-chat-session.json`
```json
{"s": "vscode-chat-editor:///chat-session-id", "t": 1711700000000}
```

**Lines changed**: ~2 lines of logic added at each injection point (4 total), wrapped in async `import("fs")` with silent error handling.

### 2. Folder Switch (`patch-vscode-folder-switch.js`)

**Type**: Requested behavior change
**Upstream candidate**: Add `suppressDialogs` option to `updateWorkspaceFolders()`

**Problem**: When an extension calls `updateWorkspaceFolders()` to switch the workspace root (e.g. to a worktree directory), VS Code shows a blocking confirmation dialog. This prevents automated workflows like branch-per-chat worktree switching.

**What it changes** (1 injection point):

In `enterWorkspace()`, replaces:
```
if (!await this.extensionService.stopExtensionHosts(reason)) return;
```
with:
```
await this.extensionService._doStopExtensionHosts();
```

This skips the veto/dialog event chain. Extension hosts still restart cleanly — only the dialog is removed.

**Lines changed**: 1 line modified.

## VS Code Update Behavior

When VS Code auto-updates, it replaces the workbench bundle. The patches are lost and must be re-applied. The extension detects this on activation and prompts the user.

## File Inventory

| File | Purpose |
|------|---------|
| `scripts/patch-vscode-apply-all.js` | Coordinator: backup, apply all, check, revert |
| `scripts/patch-vscode-chat-bridge.js` | Chat bridge patch definition |
| `scripts/patch-vscode-folder-switch.js` | Folder switch patch definition |
| `PATCHES.md` | This document |

## Backup

A single pristine (pre-any-patch) backup is maintained at:
```
/Applications/Visual Studio Code.app/.../workbench.desktop.main.js.pristine
```

Legacy per-patch backup files (`.bak`, `.folder-switch.bak`) are cleaned up automatically.
