# VS Code Patches & Proposed API Usage

Branch-per-chat navigation requires two non-standard VS Code integrations:

1. **Proposed API** (`chatParticipantPrivate`) — for chat session focus events. Enabled via `~/.vscode/argv.json`. No bundle patch needed.
2. **Workbench patch** (folder-switch) — to suppress the workspace folder switch confirmation dialog. Applied to the bundle.

## Chat Session Focus: Proposed API

VS Code already has `vscode.window.onDidChangeActiveChatPanelSessionResource` in the `chatParticipantPrivate` proposed API. This fires whenever the user switches between chat conversations — exactly what we need.

**How it's enabled:**

1. `~/.vscode/argv.json` includes `"enable-proposed-api": ["RockyWearsAHat.git-shell-helpers"]`
2. `vscode-extension/package.json` includes `"enabledApiProposals": ["chatParticipantPrivate"]`
3. The extension subscribes to the event in `activate()` and maps session URIs to worktree bindings.

**Why not a bundle patch?** We originally patched the workbench to write session focus to a JSON file (`~/.cache/gsh/active-chat-session.json`), but the renderer is sandboxed — `import("fs")` silently fails. The proposed API routes through the extension host via `$acceptActiveChatSession` IPC, which is the correct architecture.

**Survives VS Code updates**: `argv.json` is user config, not part of the application bundle. The `enable-proposed-api` flag persists across updates.

**Upstream candidate**: Promote `onDidChangeActiveChatPanelSessionResource` to stable API so extensions don't need the proposed API flag.

## Workbench Patch: Folder Switch

### Patch Management

```bash
node scripts/patch-vscode-apply-all.js --check    # check status
node scripts/patch-vscode-apply-all.js             # apply patch
node scripts/patch-vscode-apply-all.js --revert    # revert to pristine
```

**Important**: After applying or reverting the patch, **quit and restart VS Code** (Cmd+Q → reopen). `Reload Window` is NOT sufficient — Electron caches the workbench bundle.

The extension checks patch status on activation and offers to apply if missing.

### What it changes (`patch-vscode-folder-switch.js`)

**Type**: Requested behavior change
**Upstream candidate**: Add `suppressDialogs` option to `updateWorkspaceFolders()`

**Problem**: `updateWorkspaceFolders()` triggers a blocking confirmation dialog in `enterWorkspace()`. This prevents automated worktree switching.

In `enterWorkspace()`, replaces:

```
if (!await this.extensionService.stopExtensionHosts(reason)) return;
```

with:

```
await this.extensionService._doStopExtensionHosts();
```

Skips the veto/dialog chain. Extension hosts still restart cleanly.

**Lines changed**: 1 line modified.

## VS Code Update Behavior

When VS Code auto-updates, the workbench bundle is replaced and the folder-switch patch is lost. The extension detects this and prompts re-application. The proposed API (argv.json) is unaffected by updates.

## File Inventory

| File                                    | Purpose                                         |
| --------------------------------------- | ------------------------------------------------ |
| `scripts/patch-vscode-apply-all.js`     | Coordinator: backup, apply, check, revert        |
| `scripts/patch-vscode-folder-switch.js` | Folder switch patch definition                   |
| `PATCHES.md`                            | This document                                    |
| `~/.vscode/argv.json`                   | Proposed API enablement (user-local, not in repo) |

## Backup

A single pristine (pre-any-patch) backup is maintained at:

```
/Applications/Visual Studio Code.app/.../workbench.desktop.main.js.pristine
```

Legacy per-patch backup files (`.bak`, `.folder-switch.bak`) are cleaned up automatically.
